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
| **Rate limiting** | Fixed-window counter per `(tenant, route)` behind a `RateLimiter` enum — in-memory by default, Redis-backed (atomic `INCR` + `EXPIRE`, fail-open) when `GATEWAY_REDIS_URL` is set. |
| **Audit + observability** | Structured JSON audit log per request, Prometheus metrics (request count, latency, allow/deny ratio, rate-limit hits), Grafana provisioned alongside. |
| **IaC** | Runnable Terraform module skeletons for GCP (Cloud Run) and AWS (Fargate). |
| **Polyglot monorepo** | Rust for the gateway (perf + memory safety on the hot path), Go for the IdP and the sample protected service. |

## Architecture at a glance

```
                                  ┌────────────────┐
                                  │   auth-issuer  │  (Go) — POST /auth/token, JWKS
                                  └───────┬────────┘
                                          │ JWT
                                          ▼
client (mTLS) ──► gateway (Rust) ──► [JWT verify] ──► [OPA authz] ──► [rate limit] ──► 200 + identity
                       │
                       │ deny
                       ▼
                 audit log
                 + metrics

           backend-echo (Go) — sample downstream that consumes
                                X-Auth-* headers and echoes them
                                back; demonstrates the upstream
                                contract for a real protected
                                service.
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for the data flow per request,
the policy data contract, and the rationale behind each language pick.

## Quickstart

```bash
make certs             # one-shot: generates dev CA + server + client-1 in certs/
make compose-up        # builds + brings the stack up
make demo              # token issue, allowed call, denied call, rate-limit hit
```

After `make compose-up`, the stack is reachable on localhost:

| Port | Service | What |
|---|---|---|
| 8443 | gateway | the policy enforcement endpoint |
| 9100 | gateway | Prometheus `/metrics` |
| 8081 | auth-issuer | `POST /auth/token`, JWKS at `/.well-known/jwks.json` |
| 8082 | backend-echo | sample protected upstream |
| 8181 | opa | policy engine, `POST /v1/data/zt/authz/allow` |
| 6379 | redis | rate-limit backend |
| 9090 | prometheus | scrapes the gateway every 5s |
| 3000 | grafana | `admin` / `admin` → *Dashboards → ZeroTrust → Audit* |

Tear down with `make compose-down`.

## What to look at after `make demo`

1. **Audit dashboard** — <http://localhost:3000>, log in `admin`/`admin`,
   *Dashboards → ZeroTrust → Audit*. Set the time range to "Last 15
   minutes" while the demo run is fresh: 1h totals (allow / deny /
   auth-failed / rate-limited), per-decision timeseries, per-tenant
   allow + refusal-by-category breakdowns.

2. **Structured audit log** — every middleware decision emits one
   JSON line under the `audit` target:

   ```bash
   docker compose -f deploy/compose/docker-compose.yml logs gateway \
     | jq 'select(.target=="audit")'
   ```

3. **OPA decisions directly** — useful for understanding *why* a
   request was denied; the companion `data.zt.authz.reasons` rule
   names which sub-policy vetoed:

   ```bash
   curl -s -X POST http://localhost:8181/v1/data/zt/authz/reasons \
        -H 'Content-Type: application/json' \
        -d '{"input":{"method":"DELETE","path":["tenants","globex","users"],
                      "claims":{"tenant":"acme","roles":["user"],"country":"KP"}}}'
   ```

4. **Trip the rate limit yourself** — hammer one route with the same
   token until you get a 429:

   ```bash
   TOKEN=$(USER_FILE=alice bash examples/curl/get-token.sh)
   for i in $(seq 1 120); do
     curl -s -o /dev/null -w '%{http_code}\n' \
          http://localhost:8443/tenants/acme/users \
          -H "Authorization: Bearer $TOKEN"
   done | sort | uniq -c
   ```

   The 429 count and the *Rate-Limited (1h)* tile on the dashboard
   should rise in lockstep, and the response carries a `Retry-After`
   header.

## Gateway middleware chain

Innermost first:
`handler → rate_limit_layer → opa_layer → jwt_layer → TraceLayer`. Each
layer is independently testable, and the order is enforced by the
router builder so a misconfigured chain fails closed (e.g. `opa_layer`
without `jwt_layer` upstream returns 500 instead of silently allowing).

CI runs the full Rust + Go + Rego matrix (build + lint + tests) in
parallel on every push to `main` and every PR.

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
MIT — © 2026 [Dimitris Sofikitis](https://dimitrisofikitis.com). See [LICENSE](LICENSE).
