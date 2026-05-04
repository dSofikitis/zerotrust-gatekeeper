//! zt-gateway: the ZeroTrust Gatekeeper API gateway.
//!
//! Module map:
//! - [`config`] — runtime knobs from env.
//! - [`tls`]    — rustls server config builder for mTLS.
//! - [`jwt`]    — JWKS-backed JWT validator.
//! - [`auth`]   — tower middleware that wraps [`jwt`] for axum.
//! - [`server`] — axum [`Router`](axum::Router) builder.

pub mod auth;
pub mod config;
pub mod jwt;
pub mod server;
pub mod tls;

/// Bumped per release.
pub const GATEWAY_VERSION: &str = "0.3.0";
