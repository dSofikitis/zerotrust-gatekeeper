# deploy/compose

The full ZeroTrust Gatekeeper stack via Docker Compose: gateway,
identity provider, protected upstream, policy engine, rate-limit
backend, and observability.

| Service | Image | Ports | Notes |
|---|---|---|---|
| `gateway` | built locally | 8443 | Rust gateway. mTLS termination, JWT validation, OPA authz, rate limit, audit log. |
| `auth-issuer` | built locally | 8081 | Go RS256 token minter, exposes JWKS at `/.well-known/jwks.json`. |
| `backend-echo` | built locally | 8082 | Go identity-stamping upstream — what the gateway proxies allowed requests to. |
| `redis` | `redis:7-alpine` | 6379 | Backs the gateway's rate-limit counter via `GATEWAY_REDIS_URL=redis://redis:6379` (atomic `INCR` + `EXPIRE`, fail-open). |
| `opa` | `openpolicyagent/opa:0.68.0` | 8181 | Loads `policies/*.rego` from the repo, mounted read-only. Query at `POST /v1/data/zt/authz/allow`. |
| `prometheus` | `prom/prometheus:v2.55.1` | 9090 | Scrapes gateway + auth-issuer + opa metrics. |
| `grafana` | `grafana/grafana:11.2.0` | 3000 | Prometheus datasource provisioned (uid `zt-prometheus`). |

Bring it up:

```bash
make compose-up
docker compose -f deploy/compose/docker-compose.yml ps
```

Open Grafana at <http://localhost:3000> (admin / admin).

OPA sanity check:

```bash
# default-deny
curl -s -X POST http://localhost:8181/v1/data/zt/authz/allow \
     -H 'Content-Type: application/json' \
     -d '{"input":{}}' \
     | jq
# {"result":false}
```

## Generate dev mTLS material

```bash
make certs   # or: bash scripts/gen-certs.sh
```

Produces `certs/{ca,server,client-1}.{crt,key}`. The directory is
gitignored (see `.gitignore`); never commit cert material.
