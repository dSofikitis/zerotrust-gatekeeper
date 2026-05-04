//! tower middleware for the gateway's auth/authz/quota pipeline.
//!
//! [`jwt_layer`] strips the bearer token, validates it via
//! [`JwksClient`], stamps [`Claims`] into the request extensions.
//!
//! [`opa_layer`] reads those claims, builds an [`OpaInput`], calls
//! the policy engine, and either passes the request through or
//! rejects with 403 + the deny reasons.
//!
//! [`rate_limit_layer`] runs downstream of both — denied requests
//! and unauthenticated requests should never count against a
//! tenant's quota. Emits 429 + Retry-After when a bucket is
//! exhausted, and stamps X-RateLimit-Remaining on success.
//!
//! All three stages emit structured audit log lines under
//! `target: "audit"` so a separate subscriber can route them to a
//! SIEM without grepping the regular request log.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderValue, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::jwt::{Claims, JwksClient};
use crate::opa::{input_from, OpaClient};
use crate::ratelimit::InMemoryRateLimiter;

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

/// Token-bucket rate limit keyed by `<tenant>:<method>:<first-path-segment>`.
/// Must be downstream of [`jwt_layer`] (and ideally [`opa_layer`])
/// so authenticated, authorized requests are the only ones that
/// consume budget.
pub async fn rate_limit_layer(
    State(limiter): State<Arc<InMemoryRateLimiter>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let claims = match req.extensions().get::<Claims>().cloned() {
        Some(c) => c,
        None => {
            tracing::error!("rate_limit_layer ran without claims; jwt_layer must run first");
            return reject(
                StatusCode::INTERNAL_SERVER_ERROR,
                "auth pipeline misconfigured",
            );
        }
    };

    let method = req.method().clone();
    let path_str = req.uri().path().to_string();
    let first_seg = path_str
        .trim_start_matches('/')
        .split('/')
        .next()
        .unwrap_or("");
    let key = format!("{}:{}:{}", claims.tenant, method, first_seg);

    match limiter.check(&key).await {
        Ok(remaining) => {
            let mut response = next.run(req).await;
            if let Ok(value) = HeaderValue::from_str(&remaining.to_string()) {
                response
                    .headers_mut()
                    .insert("X-RateLimit-Remaining", value);
            }
            response
        }
        Err(retry_after) => {
            tracing::info!(
                target: "audit",
                decision = "rate_limited",
                tenant = %claims.tenant,
                subject = %claims.subject,
                method = %method,
                path = %path_str,
                key = %key,
                retry_after_secs = retry_after,
                "rate limit exceeded"
            );
            let mut response = (
                StatusCode::TOO_MANY_REQUESTS,
                axum::Json(serde_json::json!({
                    "error": "rate limit exceeded",
                    "retry_after_secs": retry_after,
                })),
            )
                .into_response();
            if let Ok(value) = HeaderValue::from_str(&retry_after.to_string()) {
                response.headers_mut().insert("Retry-After", value);
            }
            response
        }
    }
}

fn reject(status: StatusCode, message: &'static str) -> Response {
    let body = serde_json::json!({"error": message});
    (status, axum::Json(body)).into_response()
}
