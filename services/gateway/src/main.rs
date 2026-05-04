//! Entry point for the ZeroTrust Gatekeeper. Real implementation
//! (mTLS termination, JWT validation, OPA authorization, token-bucket
//! rate limiting, audit log, Prometheus metrics) lands in phases
//! 4-7. This stub keeps `cargo build` honest from commit 1.

fn main() {
    println!(
        "zt-gateway {}: scaffolded; implementation lands in phases 4-7.",
        zt_gateway::GATEWAY_VERSION
    );
}
