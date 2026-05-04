# examples

Concrete payloads + shell scripts for poking at the running stack.

## Layout

```
examples/
├── jwt/             token-request bodies for auth-issuer
├── curl/            shell scripts wrapping common flows
└── README.md        this file
```

## Quickstart

Bring the stack up (`make compose-up`) then:

```bash
# Mint a token as alice (in tenant acme)
bash examples/curl/get-token.sh

# Pipe it to decode-token.sh to inspect the claims
bash examples/curl/get-token.sh | bash examples/curl/decode-token.sh

# Mint as admin
USER_FILE=admin bash examples/curl/get-token.sh \
  | bash examples/curl/decode-token.sh

# Hit backend-echo directly (sanity check — bypasses the gateway)
bash examples/curl/call-echo-direct.sh

# Send a request through the gateway end-to-end
bash examples/curl/call-gateway.sh GET /tenants/acme/users
bash examples/curl/call-gateway.sh POST /tenants/acme/users '{"name":"x"}' admin
```

## Token-request payloads (`jwt/*.json`)

| File | Notes |
|---|---|
| `token-request-alice.json` | tenant=acme, role=user, country=GR. The default. |
| `token-request-admin.json` | tenant=acme, role=admin. Allowed to POST/PUT/DELETE per `methods.rego`. |
| `token-request-blocked-country.json` | country=KP — fires the `geo` rule's deny when used through the gateway. |

## Scripts

| Script | What it does |
|---|---|
| `get-token.sh` | POST to `auth-issuer/auth/token` with one of the `jwt/*.json` payloads (selected via `USER_FILE=...`); prints just the JWT to stdout. |
| `decode-token.sh` | Stdin or `$1` JWT → prints the decoded payload. Useful for verifying the claim shape end-to-end. |
| `call-gateway.sh` | Mint a token + send a request through the gateway. Returns the gateway's response and the HTTP status. |
| `call-echo-direct.sh` | Hit backend-echo bypassing the gateway, with hand-stamped `X-Auth-*` headers — useful for confirming the upstream consumes the identity contract correctly. |

All scripts honour env overrides (`AUTH=...`, `GATEWAY=...`,
`ECHO=...`) for non-localhost targets.
