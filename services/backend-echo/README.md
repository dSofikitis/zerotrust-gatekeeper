# backend-echo

Sample protected upstream behind the gateway. Returns the request
shape + the identity headers the gateway stamped, so the demo can
prove the full mTLS → JWT → OPA → rate-limit → forward chain works
end to end.

## Endpoints

| Method | Path | Description |
|---|---|---|
| GET | `/healthz` | Liveness; `{"status":"ok"}`. |
| `*` | anything else | Echoes the request back as JSON. |

## Response shape

```json
{
  "method": "POST",
  "path": "/tenants/acme/users",
  "query": "verbose=1",
  "request_id": "abc-123",
  "identity": {
    "sub": "alice",
    "tenant": "acme",
    "roles": ["user"],
    "country": "GR"
  },
  "headers": { "Content-Type": "application/json", "...": "..." },
  "body": "<raw request body, capped at 1 MiB>"
}
```

The gateway stamps `X-Auth-Subject` / `X-Auth-Tenant` / `X-Auth-Roles`
(comma-separated) / `X-Auth-Country` from the validated JWT, and
`backend-echo` lifts those onto `identity`.

## What's intentionally dropped

Sensitive headers are never echoed: `Authorization`, `Cookie`,
`Proxy-Authorization`, plus the standard hop-by-hop set
(`Connection`, `Keep-Alive`, `Transfer-Encoding`, …). Authorization
in particular: a JWT must not land in audit-log payloads by accident.

## Run it

```bash
go run .                 # listens on :8082
curl -s http://localhost:8082/echo \
     -H 'X-Auth-Subject: alice' \
     -H 'X-Auth-Tenant: acme' \
     -H 'X-Auth-Roles: user,billing' \
     -H 'Content-Type: application/json' \
     -d '{"hello":"world"}' | jq
```

Or via Compose: `make compose-up` brings the service up alongside
the rest of the stack.

## Configuration

| Env var | Default | Notes |
|---|---|---|
| `ECHO_ADDR` | `:8082` | HTTP listen address. |
