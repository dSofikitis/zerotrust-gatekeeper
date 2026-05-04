# deploy/compose

The full ZeroTrust Gatekeeper stack via Docker Compose.

## What's in this commit (phase 2)

The data + observability plane is up; the gateway, auth-issuer, and
backend-echo join in their own phases.

| Service | Image | Ports | Notes |
|---|---|---|---|
| `redis` | `redis:7-alpine` | 6379 | Backs the rate-limit token-bucket. |
| `opa` | `openpolicyagent/opa:0.68.0` | 8181 | Loads `policies/*.rego` from the repo's policies dir, mounted read-only. Query at `POST /v1/data/zt/authz/allow`. |
| `prometheus` | `prom/prometheus:v2.55.1` | 9090 | Scrapes gateway + auth-issuer + opa metrics. Targets show DOWN until those services join. |
| `grafana` | `grafana/grafana:11.2.0` | 3000 | Prometheus datasource provisioned (uid `zt-prometheus`); dashboards directory provider configured for phase 8. |

Bring it up:

```bash
make compose-up
docker compose -f deploy/compose/docker-compose.yml ps
```

Open Grafana at <http://localhost:3000> (admin / admin). Dashboards
folder is `ZeroTrust` (empty until phase 8).

OPA sanity check:

```bash
# default-deny check
curl -s -X POST http://localhost:8181/v1/data/zt/authz/allow \
     -H 'Content-Type: application/json' \
     -d '{"input":{}}' \
     | jq
# {"result":false}

# scaffold path (placeholder rule)
curl -s -X POST http://localhost:8181/v1/data/zt/authz/allow \
     -H 'Content-Type: application/json' \
     -d '{"input":{"scaffold":true}}' \
     | jq
# {"result":true}
```

## Generate dev mTLS material

```bash
make certs   # or: bash scripts/gen-certs.sh
```

Produces `certs/{ca,server,client-1}.{crt,key}`. The directory is
gitignored (see `.gitignore`); never commit cert material.

## What's coming
- Phase 3: `auth-issuer` joins (port 8081, `/auth/token`, JWKS).
- Phase 4: `gateway` joins (port 8443, mTLS).
- Phase 5-7: JWT validation, OPA integration, audit log, rate
  limiting; sample Rego policies.
- Phase 8: `backend-echo` joins, Grafana dashboards drop in,
  Terraform skeleton.
