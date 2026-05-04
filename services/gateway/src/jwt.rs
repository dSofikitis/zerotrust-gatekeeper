//! JWT validation against the auth-issuer's JWKS.
//!
//! [`JwksClient`] holds the verification keys. In production they're
//! refreshed from the issuer's `/.well-known/jwks.json` at startup;
//! tests can populate the cache directly via [`JwksClient::install_key`].
//! [`JwksClient::validate`] checks the JWS signature, the `iss` and
//! `aud` claims, and exp / nbf, returning a strongly-typed [`Claims`].

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use tokio::sync::RwLock;

/// What the gateway extracts from a verified JWT and stamps onto
/// the request for downstream layers.
#[derive(Debug, Clone)]
pub struct Claims {
    pub subject: String,
    pub tenant: String,
    pub roles: Vec<String>,
    pub country: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawClaims {
    sub: String,
    tenant: String,
    roles: Vec<String>,
    #[serde(default)]
    country: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JwksDoc {
    keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize)]
struct Jwk {
    kid: String,
    n: String,
    e: String,
}

/// Holds the verification keys + the validation parameters.
pub struct JwksClient {
    cache: Arc<RwLock<HashMap<String, DecodingKey>>>,
    issuer: String,
    audience: String,
}

impl JwksClient {
    pub fn new(issuer: impl Into<String>, audience: impl Into<String>) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            issuer: issuer.into(),
            audience: audience.into(),
        }
    }

    /// Replace the cache with a fresh JWKS document fetched from `url`.
    pub async fn refresh_from_url(&self, url: &str) -> Result<()> {
        let doc: JwksDoc = reqwest::get(url)
            .await
            .with_context(|| format!("fetch jwks from {url}"))?
            .json()
            .await
            .context("decode jwks")?;
        let mut cache = self.cache.write().await;
        cache.clear();
        for jwk in doc.keys {
            let key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
                .with_context(|| format!("build key for kid={}", jwk.kid))?;
            cache.insert(jwk.kid, key);
        }
        Ok(())
    }

    /// Test seam: load a key directly without going through the wire.
    pub async fn install_key(&self, kid: impl Into<String>, key: DecodingKey) {
        self.cache.write().await.insert(kid.into(), key);
    }

    /// Validate a compact JWS and return the typed claims. Errors
    /// here mean the request must be rejected with 401.
    pub async fn validate(&self, token: &str) -> Result<Claims> {
        let header = decode_header(token).context("decode header")?;
        let kid = header.kid.ok_or_else(|| anyhow!("token has no kid"))?;
        let key = {
            let cache = self.cache.read().await;
            cache
                .get(&kid)
                .cloned()
                .ok_or_else(|| anyhow!("kid {kid} not in jwks cache"))?
        };
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        let data = decode::<RawClaims>(token, &key, &validation).context("validate")?;
        Ok(Claims {
            subject: data.claims.sub,
            tenant: data.claims.tenant,
            roles: data.claims.roles,
            country: data.claims.country,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use rsa::pkcs8::EncodePrivateKey;
    use rsa::traits::PublicKeyParts;
    use rsa::{RsaPrivateKey, RsaPublicKey};
    use serde::Serialize;
    use std::time::{SystemTime, UNIX_EPOCH};

    const ISSUER: &str = "test-issuer";
    const AUDIENCE: &str = "test-audience";
    const KID: &str = "test-key-1";

    #[derive(Serialize)]
    struct TestClaims {
        sub: String,
        tenant: String,
        roles: Vec<String>,
        country: Option<String>,
        iss: String,
        aud: String,
        exp: u64,
        nbf: u64,
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn keypair() -> (RsaPrivateKey, RsaPublicKey) {
        let mut rng = rand::thread_rng();
        let priv_key = RsaPrivateKey::new(&mut rng, 2048).expect("gen rsa");
        let pub_key = RsaPublicKey::from(&priv_key);
        (priv_key, pub_key)
    }

    fn mint(priv_key: &RsaPrivateKey, claims: &TestClaims) -> String {
        let pem = priv_key
            .to_pkcs8_pem(Default::default())
            .expect("pkcs8 pem")
            .to_string();
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(KID.to_string());
        encode(
            &header,
            claims,
            &EncodingKey::from_rsa_pem(pem.as_bytes()).expect("encoding key"),
        )
        .expect("sign")
    }

    async fn install_pub(jwks: &JwksClient, pub_key: &RsaPublicKey) {
        // JWKS-style: build the DecodingKey directly from RSA modulus
        // + exponent so the test exercises the same code path as the
        // production JWKS fetch.
        let n = URL_SAFE_NO_PAD.encode(pub_key.n().to_bytes_be());
        let e = URL_SAFE_NO_PAD.encode(pub_key.e().to_bytes_be());
        let key = DecodingKey::from_rsa_components(&n, &e).expect("decoding key");
        jwks.install_key(KID, key).await;
    }

    fn good_claims() -> TestClaims {
        TestClaims {
            sub: "alice".into(),
            tenant: "acme".into(),
            roles: vec!["user".into()],
            country: Some("GR".into()),
            iss: ISSUER.into(),
            aud: AUDIENCE.into(),
            exp: now() + 60,
            nbf: now() - 1,
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn validate_happy_path() {
        let (priv_key, pub_key) = keypair();
        let jwks = JwksClient::new(ISSUER, AUDIENCE);
        install_pub(&jwks, &pub_key).await;
        let token = mint(&priv_key, &good_claims());
        let claims = jwks.validate(&token).await.expect("validate");
        assert_eq!(claims.subject, "alice");
        assert_eq!(claims.tenant, "acme");
        assert_eq!(claims.roles, vec!["user".to_string()]);
        assert_eq!(claims.country.as_deref(), Some("GR"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejects_expired_token() {
        let (priv_key, pub_key) = keypair();
        let jwks = JwksClient::new(ISSUER, AUDIENCE);
        install_pub(&jwks, &pub_key).await;
        let mut c = good_claims();
        // jsonwebtoken's default leeway is 60s, so push exp well past
        // the boundary to make the test deterministic.
        c.exp = now() - 600;
        c.nbf = now() - 700;
        let token = mint(&priv_key, &c);
        assert!(jwks.validate(&token).await.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejects_wrong_audience() {
        let (priv_key, pub_key) = keypair();
        let jwks = JwksClient::new(ISSUER, AUDIENCE);
        install_pub(&jwks, &pub_key).await;
        let mut c = good_claims();
        c.aud = "someone-else".into();
        let token = mint(&priv_key, &c);
        assert!(jwks.validate(&token).await.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejects_wrong_issuer() {
        let (priv_key, pub_key) = keypair();
        let jwks = JwksClient::new(ISSUER, AUDIENCE);
        install_pub(&jwks, &pub_key).await;
        let mut c = good_claims();
        c.iss = "evil-issuer".into();
        let token = mint(&priv_key, &c);
        assert!(jwks.validate(&token).await.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejects_unknown_kid() {
        let (priv_key, _pub_key) = keypair();
        let jwks = JwksClient::new(ISSUER, AUDIENCE);
        // No key installed in the cache — kid will be unknown.
        let token = mint(&priv_key, &good_claims());
        assert!(jwks.validate(&token).await.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejects_tampered_signature() {
        let (priv_key, pub_key) = keypair();
        let jwks = JwksClient::new(ISSUER, AUDIENCE);
        install_pub(&jwks, &pub_key).await;
        let token = mint(&priv_key, &good_claims());
        // Flip a byte in the signature segment.
        let mut parts: Vec<&str> = token.splitn(3, '.').collect();
        let mut sig = parts[2].to_string();
        sig.replace_range(0..1, if &sig[0..1] == "A" { "B" } else { "A" });
        parts[2] = &sig;
        let tampered = parts.join(".");
        assert!(jwks.validate(&tampered).await.is_err());
    }
}
