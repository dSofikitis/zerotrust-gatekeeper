#!/usr/bin/env bash
# Print the decoded payload of a JWT minted by auth-issuer.
#
#   bash examples/curl/get-token.sh | bash examples/curl/decode-token.sh
#
# Or pass the token as $1.

set -euo pipefail

token="${1:-$(cat)}"
payload="$(printf '%s' "$token" | cut -d. -f2)"

# JWT base64url -> base64 (pad with =) -> decode.
# Only pad when needed: `printf '=%.0s' $(seq 1 0)` would still print
# one literal '=' because printf runs the format once even with no
# args, breaking base64 -d when the payload was already aligned.
pad=$(( (4 - ${#payload} % 4) % 4 ))
b64="$payload"
if [ "$pad" -gt 0 ]; then
    b64="$b64$(printf '=%.0s' $(seq 1 "$pad"))"
fi
printf '%s' "$b64" | tr '_-' '/+' | base64 -d 2>/dev/null
echo
