//! mTLS server config for the gateway: the server presents its own
//! cert *and* requires every client to present a cert chained to the
//! configured CA bundle. Loaded from PEM files on disk; deployments
//! point these at cert-manager / ACM PCA / equivalent.

use std::fs;
use std::io::BufReader;
use std::sync::Arc;

use anyhow::{Context, Result};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::WebPkiClientVerifier;
use rustls::{RootCertStore, ServerConfig};

use crate::config::TlsConfig;

/// Build a rustls ServerConfig that:
///   - presents the configured server cert + key
///   - requires every client to present a cert chained to client_ca
pub fn build_server_config(cfg: &TlsConfig) -> Result<ServerConfig> {
    let server_certs = load_certs(&cfg.server_cert)?;
    let server_key = load_key(&cfg.server_key)?;
    let client_ca = load_certs(&cfg.client_ca)?;

    let mut roots = RootCertStore::empty();
    for cert in client_ca {
        roots.add(cert).context("add client CA to root store")?;
    }
    let verifier = WebPkiClientVerifier::builder(Arc::new(roots))
        .build()
        .context("build client cert verifier")?;

    ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(server_certs, server_key)
        .context("build TLS server config")
}

fn load_certs(path: &str) -> Result<Vec<CertificateDer<'static>>> {
    let bytes = fs::read(path).with_context(|| format!("read {path}"))?;
    let mut reader = BufReader::new(bytes.as_slice());
    rustls_pemfile::certs(&mut reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("parse certs from {path}"))
}

fn load_key(path: &str) -> Result<PrivateKeyDer<'static>> {
    let bytes = fs::read(path).with_context(|| format!("read {path}"))?;
    let mut reader = BufReader::new(bytes.as_slice());
    rustls_pemfile::private_key(&mut reader)
        .with_context(|| format!("parse private key from {path}"))?
        .ok_or_else(|| anyhow::anyhow!("no private key found in {path}"))
}
