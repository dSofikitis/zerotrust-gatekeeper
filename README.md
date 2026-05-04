# ZeroTrust Gatekeeper

> **Zero trust** — verify every request, every time. No implicit
> trust because of where the request came from.

A policy-as-code API gateway. Every HTTP request is authenticated
(mTLS *and* JWT), authorized via OPA / Rego, rate-limited per
tenant, and audited with structured logs + Prometheus metrics.

## What this is for

The hard part of any production HTTP system isn't the routing — it's
the *enforcement layer*: who's calling, what are they allowed to do,
how often, and how do we prove what happened later. ZeroTrust
Gatekeeper implements that layer end-to-end as a learning + portfolio
piece, with the seams that match how real edge-stacks (Envoy + OPA,
Kong, Tyk, AWS API Gateway + Lambda authorizers) are composed.

| Capability | Where it shows up |
|---|---|
| **Mutual TLS at the edge** | Gateway's TLS config requires + validates client certs against a trust bundle. Cert generator script for local dev. |
| **JWT validation** | Gateway fetches the issuer's JWKS, verifies signatures, extracts claims into request extensions for downstream layers. |
| **Policy as code** | OPA / Rego sidecar (`policies/*.rego`) — tenant scoping, RBAC, geo-blocking. Gateway calls `POST /v1/data/zt/authz/allow` per request. |
| **Rate limiting** | Token bucket per `(tenant, route)` with Redis backing for horizontal scale. |
| **Audit + observability** | Structured JSON audit log per request, Prometheus metrics (request count, latency, allow/deny ratio, rate-limit hits), Grafana dashboards. |
| **IaC** | Terraform skeleton for GCP (Cloud Run) and AWS (Fargate). |
| **Polyglot monorepo** | Rust for the gateway (perf + memory safety on the hot path), Go for the IdP and the protected upstream. |

## Architecture at a glance

```
                                  ┌────────────────┐
                                  │   auth-issuer  │  (Go) — POST /auth/token, JWKS
                                  └───────┬────────┘
                                          │ JWT
                                          ▼
client (mTLS) ──► gateway (Rust) ──► [JWT verify] ──► [OPA authz] ──► [rate limit]
                       │                                                   │
                       │ deny                                              │ allow
                       │                                                   ▼
                       ▼                                           backend-echo (Go)
                 audit log
                 + metrics
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for the data flow per request,
the policy data contract, and the rationale behind each language pick.

## Quickstart

```bash
make compose-up        # gateway + auth-issuer + backend-echo + opa + redis + prometheus + grafana
make demo              # walk through token issue, allowed call, denied call, rate-limit hit
```

## Status (v0.1)

The repo is built phase by phase. CI matrix runs Rust + Go + Rego
in parallel on every push.

| Phase | What | Status |
|---|---|---|
| 1 | Repo skeleton + CI matrix + dependabot + CodeQL | ✅ |
| 2 | Compose stack (Redis + OPA + Prometheus + Grafana) + cert generator | ✅ |
| 3 | `auth-issuer` (Go, RS256 + JWKS, 10 tests) | ✅ |
| 4 | `gateway` scaffold (Rust, axum + optional mTLS via rustls) | ✅ |
| 5 | JWT validation against JWKS (**6 tests**) | ✅ |
| 6 | OPA integration + audit log (**6 tests**) | ✅ |
| 7a | Sample Rego policies (tenants / methods / geo, **14 tests**) | ✅ |
| 7b | Rate limiting (in-memory token bucket; Redis is a v0.2 swap) (**4 tests**) | ✅ |
| 8a | `backend-echo` (Go, identity-stamp upstream) | ✅ |
| 8b | `examples/` with curl flows + JWT payloads | ✅ |
| 8c | Terraform skeleton (GCP Cloud Run) | ✅ |
| 8d | End-to-end demo script | ✅ |
| 8e | Grafana audit dashboard | ⏳ v0.2 |

**Gateway middleware chain** (innermost first):
`handler → rate_limit_layer → opa_layer → jwt_layer → TraceLayer`. Each
layer is independently testable, and the order is enforced by the
router builder so a misconfigured chain fails closed (e.g. opa_layer
without jwt_layer upstream returns 500 instead of silently allowing).

## Development

Polyglot monorepo. Each service builds and tests independently.

| Service | Toolchain | Build | Test |
|---|---|---|---|
| `services/gateway` | Rust stable | `cargo build` | `cargo test` |
| `services/auth-issuer` | Go ≥ 1.23 | `go build ./...` | `go test ./...` |
| `services/backend-echo` | Go ≥ 1.23 | `go build ./...` | `go test ./...` |
| `policies` | OPA ≥ 0.65 | `opa fmt --diff` | `opa test policies/` |

`make help` lists targets. CI (`.github/workflows/ci.yml`) runs the
same lint + test matrix across Rust, Go, and OPA on every push to
`main` and every PR.

## License
MIT. Copyright (c) 2026 @dSofikitis.
