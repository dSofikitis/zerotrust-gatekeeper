#!/usr/bin/env bash
# Mint a JWT from auth-issuer and print just the token to stdout.
#
#   bash examples/curl/get-token.sh                       -> alice
#   USER_FILE=alice bash examples/curl/get-token.sh       -> alice (explicit)
#   USER_FILE=admin bash examples/curl/get-token.sh       -> admin role
#   USER_FILE=blocked-country bash examples/curl/get-token.sh
#
# Override the issuer with AUTH=http://my-host:8081 ...

set -euo pipefail

AUTH="${AUTH:-http://localhost:8081}"
USER_FILE="${USER_FILE:-alice}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BODY_PATH="$HERE/jwt/token-request-${USER_FILE}.json"

if [ ! -f "$BODY_PATH" ]; then
    echo "no payload at $BODY_PATH; available:" >&2
    ls -1 "$HERE/jwt" >&2
    exit 1
fi

curl -sS -X POST "$AUTH/auth/token" \
    -H 'Content-Type: application/json' \
    --data-binary @"$BODY_PATH" \
  | sed -E 's/.*"access_token":"([^"]+)".*/\1/'
