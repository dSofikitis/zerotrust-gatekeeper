//! tower middleware for the gateway's auth/authz pipeline.
//!
//! [`jwt_layer`] strips the bearer token, validates it via
//! [`JwksClient`], stamps [`Claims`] into the request extensions.
//!
//! [`opa_layer`] reads those claims, builds an [`OpaInput`], calls
//! the policy engine, and either passes the request through or
//! rejects with 403 + the deny reasons. Both stages emit structured
//! audit log lines.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::jwt::{Claims, JwksClient};
use crate::opa::{input_from, OpaClient};

/// JWT validation. Use as `axum::middleware::from_fn_with_state(jwks, jwt_layer)`.
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

/// OPA authorization. Must be downstream of [`jwt_layer`] — reads
/// [`Claims`] from request extensions.
pub async fn opa_layer(
    State(opa): State<Arc<OpaClient>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let claims = match req.extensions().get::<Claims>().cloned() {
        Some(c) => c,
        None => {
            tracing::error!("opa_layer ran without claims in extensions; jwt_layer must run first");
            return reject(
                StatusCode::INTERNAL_SERVER_ERROR,
                "auth pipeline misconfigured",
            );
        }
    };

    let method = req.method().clone();
    let path_str = req.uri().path().to_string();
    let input = input_from(method.as_str(), &path_str, &claims);

    let result = match opa.evaluate(&input).await {
        Ok(r) => r,
        Err(err) => {
            tracing::error!(error = %err, "opa evaluate failed");
            return reject(StatusCode::SERVICE_UNAVAILABLE, "policy engine unavailable");
        }
    };

    if !result.allow {
        tracing::info!(
            target: "audit",
            decision = "deny",
            reasons = ?result.reasons,
            method = %method,
            path = %path_str,
            subject = %claims.subject,
            tenant = %claims.tenant,
            "request denied"
        );
        return (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "error": "forbidden",
                "reasons": result.reasons,
            })),
        )
            .into_response();
    }

    tracing::info!(
        target: "audit",
        decision = "allow",
        method = %method,
        path = %path_str,
        subject = %claims.subject,
        tenant = %claims.tenant,
        "request allowed"
    );
    next.run(req).await
}

fn reject(status: StatusCode, message: &'static str) -> Response {
    let body = serde_json::json!({"error": message});
    (status, axum::Json(body)).into_response()
}
