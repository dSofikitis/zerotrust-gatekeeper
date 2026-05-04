//! zt-gateway library: types and middleware for the ZeroTrust
//! Gatekeeper API gateway. The full implementation (axum + tower
//! + jsonwebtoken + governor) lands in phases 4-7.

/// Bumped per release.
pub const GATEWAY_VERSION: &str = "0.1.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!GATEWAY_VERSION.is_empty());
    }
}
