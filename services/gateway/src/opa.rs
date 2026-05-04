//! OPA client + the input contract the gateway sends per request.
//!
//! The gateway calls `POST /v1/data/zt/authz` once per request with
//! the input shape the policies expect, and gets back the
//! `{"allow", "reasons"}` document. The reasons list (when allow is
//! false) is folded into the audit log + the 403 response body.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::jwt::Claims;

#[derive(Debug, Serialize)]
pub struct OpaInput {
    pub method: String,
    pub path: Vec<String>,
    pub tenant: String,
    pub claims: ClaimsForOpa,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_cert_subject: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ClaimsForOpa {
    pub sub: String,
    pub tenant: String,
    pub roles: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

impl ClaimsForOpa {
    pub fn from_claims(c: &Claims) -> Self {
        Self {
            sub: c.subject.clone(),
            tenant: c.tenant.clone(),
            roles: c.roles.clone(),
            country: c.country.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct OpaResult {
    #[serde(default)]
    pub allow: bool,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OpaResponse {
    result: OpaResult,
}

pub struct OpaClient {
    url: String,
    client: reqwest::Client,
}

impl OpaClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let mut url: String = base_url.into();
        while url.ends_with('/') {
            url.pop();
        }
        url.push_str("/v1/data/zt/authz");
        Self {
            url,
            client: reqwest::Client::new(),
        }
    }

    pub async fn evaluate(&self, input: &OpaInput) -> Result<OpaResult> {
        let body = serde_json::json!({"input": input});
        let resp: OpaResponse = self
            .client
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("POST {}", self.url))?
            .error_for_status()
            .context("opa response status")?
            .json()
            .await
            .context("decode opa response")?;
        Ok(resp.result)
    }
}

/// Build an [`OpaInput`] from a request method, path, and the
/// validated claims attached by [`auth::jwt_layer`](crate::auth).
pub fn input_from(method: &str, path: &str, claims: &Claims) -> OpaInput {
    let path: Vec<String> = path
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    OpaInput {
        method: method.to_ascii_uppercase(),
        path,
        tenant: claims.tenant.clone(),
        claims: ClaimsForOpa::from_claims(claims),
        client_ip: None,
        client_cert_subject: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn alice() -> Claims {
        Claims {
            subject: "alice".into(),
            tenant: "acme".into(),
            roles: vec!["user".into()],
            country: Some("GR".into()),
        }
    }

    #[test]
    fn input_from_splits_path_and_lifts_claims() {
        let input = input_from("get", "/tenants/acme/users", &alice());
        assert_eq!(input.method, "GET");
        assert_eq!(input.path, vec!["tenants", "acme", "users"]);
        assert_eq!(input.tenant, "acme");
        assert_eq!(input.claims.sub, "alice");
        assert_eq!(input.claims.roles, vec!["user".to_string()]);
        assert_eq!(input.claims.country.as_deref(), Some("GR"));
    }

    #[test]
    fn input_from_handles_root_path() {
        let input = input_from("GET", "/", &alice());
        assert_eq!(input.path, Vec::<String>::new());
    }

    #[tokio::test]
    async fn evaluate_returns_allow_true() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/data/zt/authz"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "result": {"allow": true, "reasons": []}
            })))
            .mount(&server)
            .await;

        let client = OpaClient::new(server.uri());
        let result = client
            .evaluate(&input_from("GET", "/healthz", &alice()))
            .await
            .expect("evaluate");
        assert!(result.allow);
        assert!(result.reasons.is_empty());
    }

    #[tokio::test]
    async fn evaluate_returns_deny_with_reasons() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/data/zt/authz"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "result": {"allow": false, "reasons": ["tenant_scope", "method"]}
            })))
            .mount(&server)
            .await;

        let client = OpaClient::new(server.uri());
        let result = client
            .evaluate(&input_from("DELETE", "/tenants/globex/users", &alice()))
            .await
            .expect("evaluate");
        assert!(!result.allow);
        assert_eq!(
            result.reasons,
            vec!["tenant_scope".to_string(), "method".to_string()]
        );
    }

    #[tokio::test]
    async fn evaluate_surfaces_5xx() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/data/zt/authz"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let client = OpaClient::new(server.uri());
        let result = client.evaluate(&input_from("GET", "/", &alice())).await;
        assert!(result.is_err());
    }

    #[test]
    fn opa_url_is_normalized() {
        let c = OpaClient::new("http://opa:8181/");
        assert!(c.url.ends_with("/v1/data/zt/authz"));
        assert!(!c.url.contains("//v1"));
    }
}
