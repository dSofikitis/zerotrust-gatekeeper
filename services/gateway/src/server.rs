//! axum router for zt-gateway. Phase 4 ships /healthz and a fallback
//! stub; JWT validation, OPA authorization, audit logging, and rate
//! limiting layer onto this same Router in phases 5-7.

use axum::routing::{any, get};
use axum::{response::Json, Router};
use serde_json::{json, Value};
use tower_http::trace::TraceLayer;

/// Build the gateway's Router. Kept separate from the runtime
/// bootstrap so tests can hit it without standing up TLS or the
/// listener.
pub fn router() -> Router {
    Router::new()
        .route("/healthz", get(handle_healthz))
        .fallback(any(handle_proxy_stub))
        .layer(TraceLayer::new_for_http())
}

async fn handle_healthz() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

async fn handle_proxy_stub() -> Json<Value> {
    Json(json!({
        "status": "scaffolded",
        "message": "JWT verify + OPA authz + rate limit + proxy land in phases 5-7."
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn healthz_returns_ok() {
        let app = router();
        let req = Request::builder()
            .uri("/healthz")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["status"], "ok");
    }

    #[tokio::test]
    async fn fallback_returns_scaffold_message() {
        let app = router();
        let req = Request::builder()
            .uri("/api/anything")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["status"], "scaffolded");
    }
}
