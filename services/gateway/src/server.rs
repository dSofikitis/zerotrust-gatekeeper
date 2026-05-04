//! axum router for zt-gateway.
//!
//! Phase 5 wires the [`auth`](crate::auth) middleware in front of
//! the catch-all route so any request to a non-`/healthz` path must
//! carry a verifiable JWT. OPA authorization (phase 6), audit
//! logging (phase 6), and rate limiting (phase 7) layer on top.

use std::sync::Arc;

use axum::routing::{any, get};
use axum::{response::Json, Extension, Router};
use serde_json::{json, Value};
use tower_http::trace::TraceLayer;

use crate::jwt::{Claims, JwksClient};

/// Build the gateway's Router.
///
/// When `jwks` is `Some`, the catch-all route is gated by the JWT
/// middleware. When `None`, the router is open — useful only for
/// dev runs that don't need a real auth-issuer.
pub fn router(jwks: Option<Arc<JwksClient>>) -> Router {
    let public = Router::new().route("/healthz", get(handle_healthz));

    let mut protected = Router::new().fallback(any(handle_proxy_stub));
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
        "message": "OPA authz + rate limit + proxy land in phases 6-7.",
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
        router(None)
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
        let app = router(Some(jwks));
        let req = Request::builder()
            .uri("/api/anything")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn healthz_open_even_when_jwt_layer_attached() {
        let jwks = Arc::new(JwksClient::new("test-iss", "test-aud"));
        let app = router(Some(jwks));
        let req = Request::builder()
            .uri("/healthz")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
