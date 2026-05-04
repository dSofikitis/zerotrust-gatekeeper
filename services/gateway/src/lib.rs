//! zt-gateway: the ZeroTrust Gatekeeper API gateway. mTLS termination
//! lives behind [`tls`], the axum router behind [`server`], and
//! runtime knobs behind [`config`]. JWT validation, OPA authorization,
//! audit logging, and rate limiting layer onto the same router in
//! phases 5-7.

pub mod config;
pub mod server;
pub mod tls;

/// Bumped per release.
pub const GATEWAY_VERSION: &str = "0.2.0";
