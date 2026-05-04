# zt-gateway

The star service. A Rust HTTP/HTTPS gateway that:

1. Terminates TLS and **requires** a valid client certificate
   (mTLS) signed by the project CA.
2. Validates the bearer JWT against the issuer's JWKS, extracting
   claims into request extensions.
3. Calls **OPA** at `POST /v1/data/zt/authz/allow` with the
   request + claims + client-cert subject and refuses to proceed
   on `allow == false`.
4. Applies a per-`(tenant, route)` token-bucket rate limit, backed
   by Redis for horizontal scale.
5. Forwards the request to the configured upstream, stamping
   `X-Auth-Identity`.
6. Emits one structured JSON audit event per request and Prometheus
   metrics for ops.

Built on **axum + tower** so each step is a composable middleware
layer; that's why this service is in Rust — the hot path is
per-request, runs in front of the internet, and benefits from the
type system catching auth bugs at compile time.

Implementation lands in phases 4-7. The current commit ships a
trivial lib + bin so the cargo + clippy + fmt CI matrix passes from
commit 1.
