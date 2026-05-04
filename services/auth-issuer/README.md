# auth-issuer

Toy IdP for the local stack. Mints short-lived RS256 JWTs and
exposes the public verification key as a JWKS document so the
gateway can validate without a shared secret.

Real-world parallel: Keycloak, Auth0, Cognito, GCP IAM.

## Endpoints

| Method | Path | Description |
|---|---|---|
| GET | `/healthz` | Liveness; `{"status":"ok"}`. |
| GET | `/.well-known/jwks.json` | Public RSA key (single entry, RS256, current `kid`). |
| POST | `/auth/token` | Issue a JWT. Body: `{"username":"...", "tenant":"...", "roles":[...], "country":"..."}`. Tenant / roles / country default if omitted. |

## Token shape

```json
{
  "iss": "zt-auth-issuer",
  "sub": "alice",
  "aud": "zt-gateway",
  "iat": 1746273600,
  "nbf": 1746273600,
  "exp": 1746274500,
  "tenant": "acme",
  "roles": ["user"],
  "country": "GR"
}
```

The JWS header carries `alg: RS256` and a `kid` matching the JWKS.

## Dev defaults (no user store)

The IdP accepts any non-empty `username` (no password check). To
keep the curl examples natural the dev derivation is:

| `username` | `tenant` | `roles` |
|---|---|---|
| `alice`, `bob` | `acme` | `[user]` |
| `carl`, `dan` | `globex` | `[user]` |
| `admin` | `acme` | `[admin]` |
| anything else | `acme` | `[user]` |

Override any field via the request body.

## Run it

```bash
go run .                 # listens on :8081
curl -s http://localhost:8081/.well-known/jwks.json | jq
curl -s -X POST http://localhost:8081/auth/token \
     -H 'Content-Type: application/json' \
     -d '{"username":"alice"}' | jq
```

Or via Compose: `make compose-up` brings the service up alongside
Redis / OPA / Prometheus / Grafana.

## Configuration

| Env var | Default | Notes |
|---|---|---|
| `AUTH_ADDR` | `:8081` | HTTP listen address. |
| `AUTH_TOKEN_TTL` | `15m` | Lifetime of issued tokens (Go duration or seconds). |

## Key rotation

`NewSigner()` generates a fresh 2048-bit RSA key on startup. A
production rotation strategy lands in a follow-up — likely a small
keyring + a `kid` per slot, exposed together via JWKS so existing
in-flight tokens stay valid through a rotation.
