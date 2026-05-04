//! Runtime configuration loaded from the environment.

use std::env;
use std::net::SocketAddr;

/// Top-level runtime config. TLS, Auth, and OPA are all optional so
/// the gateway can run in dev mode without certs / IdP / policy
/// engine; in any real deployment all three must be set.
#[derive(Debug, Clone)]
pub struct Config {
    pub addr: SocketAddr,
    pub tls: Option<TlsConfig>,
    pub auth: Option<AuthConfig>,
    pub opa_url: Option<String>,
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

/// JWKS endpoint + the JWT validation parameters. When this is
/// `Some`, the gateway gates non-`/healthz` requests on a verifiable
/// bearer token whose `iss` and `aud` match.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub jwks_url: String,
    pub issuer: String,
    pub audience: String,
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

        let auth = env::var("GATEWAY_AUTH_JWKS_URL")
            .ok()
            .map(|jwks_url| AuthConfig {
                jwks_url,
                issuer: env::var("GATEWAY_AUTH_ISSUER")
                    .unwrap_or_else(|_| "zt-auth-issuer".to_string()),
                audience: env::var("GATEWAY_AUTH_AUDIENCE")
                    .unwrap_or_else(|_| "zt-gateway".to_string()),
            });

        let opa_url = env::var("GATEWAY_OPA_URL").ok();

        let upstream_url = env::var("GATEWAY_UPSTREAM_URL")
            .unwrap_or_else(|_| "http://backend-echo:8082".to_string());

        Self {
            addr,
            tls,
            auth,
            opa_url,
            upstream_url,
        }
    }
}
