//! axum router for zt-gateway.
//!
//! Layer order on the protected route (innermost first, axum
//! applies them outside-in):
//!
//!   handler -> opa_layer -> jwt_layer -> TraceLayer
//!
//! /healthz stays public and is unprotected by either auth layer.

use std::sync::Arc;

use axum::routing::{any, get};
use axum::{response::Json, Extension, Router};
use serde_json::{json, Value};
use tower_http::trace::TraceLayer;

use crate::jwt::{Claims, JwksClient};
use crate::opa::OpaClient;

/// Build the gateway's Router.
///
/// `jwks` and `opa` are independently optional; both `Some` is the
/// real production shape. Either set to `None` to disable that
/// stage in dev runs.
pub fn router(jwks: Option<Arc<JwksClient>>, opa: Option<Arc<OpaClient>>) -> Router {
    let public = Router::new().route("/healthz", get(handle_healthz));

    let mut protected = Router::new().fallback(any(handle_proxy_stub));
    if let Some(opa) = opa {
        protected = protected.layer(axum::middleware::from_fn_with_state(
            opa,
            crate::auth::opa_layer,
        ));
    }
    if let Some(jwks) = jwks {
        protected = protected.layer(axum::middleware::from_fn_with_state(
            jwks,
            crate::auth::jwt_layer,
        ));
    }

    public.merge(protected).layer(TraceLayer::new_for_http())
}

async fn handle_healthz() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

async fn handle_proxy_stub(claims: Option<Extension<Claims>>) -> Json<Value> {
    let identity = match claims {
        Some(Extension(c)) => json!({
            "sub": c.subject,
            "tenant": c.tenant,
            "roles": c.roles,
            "country": c.country,
        }),
        None => Value::Null,
    };
    Json(json!({
        "status": "scaffolded",
        "message": "Rate limit + proxy land in phase 7.",
        "identity": identity,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn build_open_router() -> Router {
        router(None, None)
    }

    #[tokio::test]
    async fn healthz_returns_ok() {
        let app = build_open_router();
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
    async fn fallback_returns_scaffold_message_when_open() {
        let app = build_open_router();
        let req = Request::builder()
            .uri("/api/anything")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["status"], "scaffolded");
        assert_eq!(body["identity"], Value::Null);
    }

    #[tokio::test]
    async fn fallback_rejects_missing_token_when_jwt_layer_attached() {
        let jwks = Arc::new(JwksClient::new("test-iss", "test-aud"));
        let app = router(Some(jwks), None);
        let req = Request::builder()
            .uri("/api/anything")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn healthz_open_even_when_layers_attached() {
        let jwks = Arc::new(JwksClient::new("test-iss", "test-aud"));
        let opa = Arc::new(OpaClient::new("http://nonexistent:8181"));
        let app = router(Some(jwks), Some(opa));
        let req = Request::builder()
            .uri("/healthz")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
