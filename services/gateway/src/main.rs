//! zt-gateway entry point: load env config, init structured
//! logging, build JwksClient + OpaClient + RateLimiter + axum
//! router, bind on HTTP or mTLS.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use axum_server::tls_rustls::RustlsConfig;
use tracing_subscriber::EnvFilter;

use zt_gateway::config::Config;
use zt_gateway::jwt::JwksClient;
use zt_gateway::metrics::{install_recorder, serve as serve_metrics};
use zt_gateway::opa::OpaClient;
use zt_gateway::ratelimit::{InMemoryRateLimiter, Limit, RateLimiter, RedisRateLimiter};
use zt_gateway::server::router;
use zt_gateway::tls::build_server_config;
use zt_gateway::GATEWAY_VERSION;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    let cfg = Config::from_env();

    let jwks: Option<Arc<JwksClient>> = match cfg.auth.as_ref() {
        Some(auth) => {
            let client = Arc::new(JwksClient::new(auth.issuer.clone(), auth.audience.clone()));
            client.refresh_from_url(&auth.jwks_url).await?;
            tracing::info!(
                jwks_url = %auth.jwks_url,
                issuer = %auth.issuer,
                audience = %auth.audience,
                "JWT validation enabled"
            );
            Some(client)
        }
        None => {
            tracing::warn!(
                "GATEWAY_AUTH_JWKS_URL not set — JWT validation OFF. \
                 Suitable for local dev only."
            );
            None
        }
    };

    let opa: Option<Arc<OpaClient>> = cfg.opa_url.as_ref().map(|url| {
        tracing::info!(opa_url = %url, "OPA authorization enabled");
        Arc::new(OpaClient::new(url.clone()))
    });
    if opa.is_none() {
        tracing::warn!(
            "GATEWAY_OPA_URL not set — OPA authorization OFF. \
             Suitable for local dev only."
        );
    }

    let limit = Limit {
        max: cfg.rate_limit.max,
        window: Duration::from_secs(cfg.rate_limit.window_secs),
    };
    let limiter = match cfg.redis_url.as_deref() {
        Some(url) => match RedisRateLimiter::connect(url, limit).await {
            Ok(rl) => {
                tracing::info!(
                    redis_url = %url,
                    max = cfg.rate_limit.max,
                    window_secs = cfg.rate_limit.window_secs,
                    "rate limit enabled (Redis-backed)"
                );
                Arc::new(RateLimiter::Redis(Box::new(rl)))
            }
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    redis_url = %url,
                    "Redis connect failed; falling back to in-memory rate limiter"
                );
                Arc::new(RateLimiter::InMemory(InMemoryRateLimiter::new(limit)))
            }
        },
        None => {
            tracing::info!(
                max = cfg.rate_limit.max,
                window_secs = cfg.rate_limit.window_secs,
                "rate limit enabled (in-memory)"
            );
            Arc::new(RateLimiter::InMemory(InMemoryRateLimiter::new(limit)))
        }
    };

    let metrics_handle = install_recorder()?;
    tokio::spawn(async move {
        if let Err(err) = serve_metrics(cfg.metrics_addr, metrics_handle).await {
            tracing::error!(error = %err, "metrics endpoint exited");
        }
    });

    let app = router(jwks, opa, Some(limiter));

    tracing::info!(
        version = GATEWAY_VERSION,
        addr = %cfg.addr,
        metrics_addr = %cfg.metrics_addr,
        upstream = %cfg.upstream_url,
        tls = cfg.tls.is_some(),
        auth = cfg.auth.is_some(),
        opa = cfg.opa_url.is_some(),
        redis = cfg.redis_url.is_some(),
        "zt-gateway starting"
    );

    if let Some(tls) = &cfg.tls {
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
