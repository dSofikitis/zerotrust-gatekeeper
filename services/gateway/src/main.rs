//! zt-gateway entry point: load env config, init structured logging,
//! build the axum router, and bind on either HTTP or mTLS depending
//! on whether GATEWAY_TLS_* paths are set.

use std::sync::Arc;

use anyhow::Result;
use axum_server::tls_rustls::RustlsConfig;
use tracing_subscriber::EnvFilter;

use zt_gateway::config::Config;
use zt_gateway::server::router;
use zt_gateway::tls::build_server_config;
use zt_gateway::GATEWAY_VERSION;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    let cfg = Config::from_env();
    let app = router();

    tracing::info!(
        version = GATEWAY_VERSION,
        addr = %cfg.addr,
        upstream = %cfg.upstream_url,
        tls = cfg.tls.is_some(),
        "zt-gateway starting"
    );

    if let Some(tls) = &cfg.tls {
        // Install the rustls default CryptoProvider once per process.
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let server_cfg = build_server_config(tls)?;
        let rustls_cfg = RustlsConfig::from_config(Arc::new(server_cfg));
        tracing::info!(
            server_cert = %tls.server_cert,
            client_ca = %tls.client_ca,
            "mTLS enabled"
        );
        axum_server::bind_rustls(cfg.addr, rustls_cfg)
            .serve(app.into_make_service())
            .await?;
    } else {
        tracing::warn!(
            "GATEWAY_TLS_* env vars not set — listening on plain HTTP. \
             Suitable for local dev only."
        );
        let listener = tokio::net::TcpListener::bind(cfg.addr).await?;
        axum::serve(listener, app).await?;
    }
    Ok(())
}

fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .json()
        .init();
}
