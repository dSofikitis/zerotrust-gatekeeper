//! Runtime configuration loaded from the environment.

use std::env;
use std::net::SocketAddr;

/// Top-level runtime config. TLS is optional so the gateway can run
/// in dev mode without certs; in any real deployment all three TLS
/// paths must be set.
#[derive(Debug, Clone)]
pub struct Config {
    pub addr: SocketAddr,
    pub tls: Option<TlsConfig>,
    pub upstream_url: String,
}

/// File-system paths to the TLS material. mTLS is enforced when this
/// is `Some` — the gateway both presents `server_cert` *and* requires
/// every client to present a cert chained to `client_ca`.
#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub server_cert: String,
    pub server_key: String,
    pub client_ca: String,
}

impl Config {
    /// Load from environment with sensible dev defaults.
    pub fn from_env() -> Self {
        let addr_raw = env::var("GATEWAY_ADDR").unwrap_or_else(|_| "0.0.0.0:8443".to_string());
        let addr: SocketAddr = addr_raw
            .parse()
            .unwrap_or_else(|_| panic!("GATEWAY_ADDR={addr_raw:?} is not a valid socket address"));

        let tls = match (
            env::var("GATEWAY_TLS_SERVER_CERT").ok(),
            env::var("GATEWAY_TLS_SERVER_KEY").ok(),
            env::var("GATEWAY_TLS_CLIENT_CA").ok(),
        ) {
            (Some(server_cert), Some(server_key), Some(client_ca)) => Some(TlsConfig {
                server_cert,
                server_key,
                client_ca,
            }),
            _ => None,
        };

        let upstream_url = env::var("GATEWAY_UPSTREAM_URL")
            .unwrap_or_else(|_| "http://backend-echo:8082".to_string());

        Self {
            addr,
            tls,
            upstream_url,
        }
    }
}
