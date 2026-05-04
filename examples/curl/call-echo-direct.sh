#!/usr/bin/env bash
# Hit backend-echo directly (bypassing the gateway) with hand-stamped
# X-Auth-* headers — useful while the gateway middleware is being
# wired up in phases 5-7. Once the gateway terminates mTLS and
# stamps these headers itself, prefer call-gateway.sh.
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
