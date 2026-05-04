#!/usr/bin/env bash
# Send a request through the gateway with a freshly-minted JWT.
# Usage:
#
#   bash examples/curl/call-gateway.sh GET /tenants/acme/users
#   bash examples/curl/call-gateway.sh POST /tenants/acme/users '{"name":"x"}' admin
#
# Args:
#   $1  HTTP method                      (default: GET)
#   $2  URL path                         (default: /healthz)
#   $3  request body                     (default: empty)
#   $4  user file in examples/jwt/       (default: alice)
#
# Env: GATEWAY=https://localhost:8443    target gateway base URL.
#      GATEWAY=http://localhost:8443     for non-TLS dev runs.

set -euo pipefail

GATEWAY="${GATEWAY:-http://localhost:8443}"
METHOD="${1:-GET}"
URL_PATH="${2:-/healthz}"
BODY="${3:-}"
USER_FILE="${4:-alice}"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

token="$(USER_FILE="$USER_FILE" bash "$HERE/get-token.sh")"
if [ -z "$token" ]; then
    echo "failed to mint token for $USER_FILE" >&2
    exit 1
fi

echo "→ $METHOD $GATEWAY$URL_PATH (as $USER_FILE)"
if [ -n "$BODY" ]; then
    curl -sS -k -X "$METHOD" "$GATEWAY$URL_PATH" \
        -H "Authorization: Bearer $token" \
        -H 'Content-Type: application/json' \
        --data-binary "$BODY" \
        -w "\n← HTTP %{http_code}\n"
else
    curl -sS -k -X "$METHOD" "$GATEWAY$URL_PATH" \
        -H "Authorization: Bearer $token" \
        -w "\n← HTTP %{http_code}\n"
fi
