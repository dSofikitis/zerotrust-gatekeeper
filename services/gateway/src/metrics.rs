//! Prometheus metrics for the gateway.
//!
//! Two surfaces:
//!
//! 1. A counter `zt_gateway_decisions_total{decision}` that the
//!    middleware increments on every authn/authz/quota decision —
//!    this is what the Grafana audit dashboard renders.
//! 2. A `/metrics` HTTP endpoint on a dedicated port (default 9100)
//!    so Prometheus can scrape without cohabiting with the gateway's
//!    main router.
//!
//! The exporter uses an in-process recorder, so calls to
//! [`metrics::counter!`] from anywhere in the binary route into the
//! same Prometheus registry without any plumbing.

use std::net::SocketAddr;

use anyhow::Result;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

/// Decision labels used by [`record_decision`]. Kept as constants so
/// a typo in a middleware call site is a compile error rather than
/// a silent label-cardinality bug.
pub const DECISION_ALLOW: &str = "allow";
pub const DECISION_DENY: &str = "deny";
pub const DECISION_AUTH_FAILED: &str = "auth_failed";
pub const DECISION_RATE_LIMITED: &str = "rate_limited";

/// Install the Prometheus recorder and return a handle that can be
/// rendered into the OpenMetrics text format on demand.
pub fn install_recorder() -> Result<PrometheusHandle> {
    let handle = PrometheusBuilder::new()
        .install_recorder()
        .map_err(|e| anyhow::anyhow!("install prometheus recorder: {e}"))?;
    Ok(handle)
}

/// Spawn a tiny axum server on `addr` that serves `/metrics` from
/// `handle`. Runs in the background for the lifetime of the process.
pub async fn serve(addr: SocketAddr, handle: PrometheusHandle) -> Result<()> {
    let app = axum::Router::new().route(
        "/metrics",
        axum::routing::get(move || {
            let handle = handle.clone();
            async move { handle.render() }
        }),
    );
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(addr = %addr, "metrics endpoint listening");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Increment the decision counter. Tenant is included as a label so
/// the dashboard can break out per-tenant ratios; cardinality stays
/// bounded to (decisions × tenants) which is fine for the demo.
pub fn record_decision(decision: &'static str, tenant: &str) {
    metrics::counter!(
        "zt_gateway_decisions_total",
        "decision" => decision,
        "tenant" => tenant.to_string(),
    )
    .increment(1);
}
