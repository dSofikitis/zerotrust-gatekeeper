//! zt-gateway: the ZeroTrust Gatekeeper API gateway.
//!
//! Module map:
//! - [`config`]    — runtime knobs from env.
//! - [`tls`]       — rustls server config builder for mTLS.
//! - [`jwt`]       — JWKS-backed JWT validator.
//! - [`opa`]       — OPA HTTP client + the policy input contract.
//! - [`ratelimit`] — per-tenant fixed-window counter (in-memory by
//!   default; Redis-backed when `GATEWAY_REDIS_URL` is set).
//! - [`metrics`]   — Prometheus recorder + `/metrics` endpoint.
//! - [`auth`]      — tower middleware tying jwt + opa + ratelimit
//!   into an axum-friendly chain.
//! - [`server`]    — axum [`Router`](axum::Router) builder.

pub mod auth;
pub mod config;
pub mod jwt;
pub mod metrics;
pub mod opa;
pub mod ratelimit;
pub mod server;
pub mod tls;

/// Bumped per release.
pub const GATEWAY_VERSION: &str = "0.4.0";
