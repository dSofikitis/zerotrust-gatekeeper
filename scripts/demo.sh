#!/usr/bin/env bash
# End-to-end walkthrough for ZeroTrust Gatekeeper. Designed to be
# safe to re-run; idempotent everywhere it can be.
#
#   bash scripts/demo.sh
#
# Walks token issuance, JWKS publication, direct OPA decisions,
# upstream reachability, and the live gateway with JWT + OPA +
# rate-limit middleware engaged.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE="$HERE/deploy/compose/docker-compose.yml"
AUTH="${AUTH:-http://localhost:8081}"
ECHO="${ECHO:-http://localhost:8082}"
GATEWAY="${GATEWAY:-http://localhost:8443}"

step() { echo; echo "===== $* ====="; }

# ---------- 1. mTLS material ----------

step "1. Generate dev mTLS certs (idempotent)"
bash "$HERE/scripts/gen-certs.sh"

# ---------- 2. Bring the stack up ----------

step "2. Bring the Compose stack up"
docker compose -f "$COMPOSE" up -d --build

step "Waiting for services to settle"
for svc in auth-issuer backend-echo opa redis; do
    for i in $(seq 1 30); do
        if docker compose -f "$COMPOSE" ps "$svc" 2>/dev/null | grep -q "Up"; then
            echo "  $svc: up"
            break
        fi
        sleep 1
    done
done

# ---------- 3. Token issuance ----------

step "3. Mint a token as alice"
ALICE_TOKEN="$(USER_FILE=alice bash "$HERE/examples/curl/get-token.sh")"
echo "alice token: ${ALICE_TOKEN:0:20}...${ALICE_TOKEN: -20}"

step "Decoded payload:"
printf '%s' "$ALICE_TOKEN" | bash "$HERE/examples/curl/decode-token.sh"

step "Mint a token as admin (will be allowed past methods.rego for writes)"
ADMIN_TOKEN="$(USER_FILE=admin bash "$HERE/examples/curl/get-token.sh")"
printf '%s' "$ADMIN_TOKEN" | bash "$HERE/examples/curl/decode-token.sh"

# ---------- 4. JWKS sanity ----------

step "4. JWKS endpoint exposes the verification key"
curl -sS "$AUTH/.well-known/jwks.json"
echo

# ---------- 5. OPA decisions ----------

step "5. OPA decision: in-tenant GET as alice (should allow)"
curl -sS -X POST http://localhost:8181/v1/data/zt/authz/allow \
     -H 'Content-Type: application/json' \
     -d '{"input":{
            "method": "GET",
            "path": ["tenants", "acme", "users"],
            "claims": {"tenant": "acme", "roles": ["user"], "country": "GR"}
          }}'
echo

step "OPA decision: cross-tenant DELETE (should deny)"
curl -sS -X POST http://localhost:8181/v1/data/zt/authz/allow \
     -H 'Content-Type: application/json' \
     -d '{"input":{
            "method": "DELETE",
            "path": ["tenants", "globex", "users"],
            "claims": {"tenant": "acme", "roles": ["admin"], "country": "GR"}
          }}'
echo

step "OPA reasons for the same denial (audit-log fodder)"
curl -sS -X POST http://localhost:8181/v1/data/zt/authz/reasons \
     -H 'Content-Type: application/json' \
     -d '{"input":{
            "method": "DELETE",
            "path": ["tenants", "globex", "users"],
            "claims": {"tenant": "acme", "roles": ["user"], "country": "KP"}
          }}'
echo

# ---------- 6. Upstream sanity ----------

step "6. Hit backend-echo directly (sanity check — bypasses the gateway)"
bash "$HERE/examples/curl/call-echo-direct.sh"

# ---------- 7. Gateway end-to-end ----------

step "7. Gateway with JWT + OPA + rate-limit engaged"
gw_ver="$(curl -fsS "$GATEWAY/healthz" 2>/dev/null || true)"
if [[ "$gw_ver" == *'"status":"ok"'* ]]; then
    bash "$HERE/examples/curl/call-gateway.sh" GET /tenants/acme/users || true
    bash "$HERE/examples/curl/call-gateway.sh" POST /tenants/acme/users '{"new":"user"}' admin || true
else
    echo "Gateway is not reachable at $GATEWAY — skipping this section."
fi

step "Demo complete. Tear down with: docker compose -f deploy/compose/docker-compose.yml down"
