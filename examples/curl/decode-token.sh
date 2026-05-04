#!/usr/bin/env bash
# Print the decoded payload of a JWT minted by auth-issuer.
#
#   bash examples/curl/get-token.sh | bash examples/curl/decode-token.sh
#
# Or pass the token as $1.

set -euo pipefail

token="${1:-$(cat)}"
payload="$(printf '%s' "$token" | cut -d. -f2)"

# JWT base64url -> base64 (pad with =) -> decode
pad=$(( (4 - ${#payload} % 4) % 4 ))
b64="$payload$(printf '=%.0s' $(seq 1 $pad))"
echo "$b64" | tr '_-' '/+' | base64 -d 2>/dev/null
echo
