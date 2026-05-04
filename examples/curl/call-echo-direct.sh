#!/usr/bin/env bash
# Hit backend-echo directly (bypassing the gateway) with hand-stamped
# X-Auth-* headers. Useful as a sanity check that the upstream
# consumes the identity contract correctly; for the full enforcement
# chain go through call-gateway.sh.
#
# Usage: bash examples/curl/call-echo-direct.sh

set -euo pipefail

ECHO="${ECHO:-http://localhost:8082}"

curl -sS "$ECHO/api/echo?demo=1" \
    -H 'X-Auth-Subject: alice' \
    -H 'X-Auth-Tenant: acme' \
    -H 'X-Auth-Roles: user' \
    -H 'X-Auth-Country: GR' \
    -H 'X-Request-Id: demo-001'
echo
