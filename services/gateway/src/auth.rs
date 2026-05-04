//! tower middleware that pulls the bearer token off the
//! `Authorization` header, validates it via [`JwksClient`], and
//! stamps the resulting [`Claims`] into the request extensions for
//! downstream layers (OPA authz, audit log, proxy header rewrite).

use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::jwt::JwksClient;

/// Use as: `axum::middleware::from_fn_with_state(jwks, jwt_layer)`.
pub async fn jwt_layer(
    State(jwks): State<Arc<JwksClient>>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let token = match req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
    {
        Some(t) => t.to_string(),
        None => return reject(StatusCode::UNAUTHORIZED, "missing bearer token"),
    };
    match jwks.validate(&token).await {
        Ok(claims) => {
            req.extensions_mut().insert(claims);
            next.run(req).await
        }
        Err(err) => {
            tracing::debug!(error = %err, "jwt validation failed");
            reject(StatusCode::UNAUTHORIZED, "invalid token")
        }
    }
}

fn reject(status: StatusCode, message: &'static str) -> Response {
    let body = serde_json::json!({"error": message});
    (status, axum::Json(body)).into_response()
}
