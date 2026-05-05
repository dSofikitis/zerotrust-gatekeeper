#!/usr/bin/env bash
# Generate dev mTLS material into certs/.
#
#   certs/ca.{key,crt}            project CA, trusted by both sides
#   certs/server.{key,crt}        gateway server cert (CN=gateway, SAN=localhost)
#   certs/client-1.{key,crt}      a sample client cert signed by the CA
#
# Anything in certs/ is gitignored. Production deploys must use
# cert-manager / ACM PCA / equivalent and rotate automatically.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Git Bash on Windows rewrites a leading `/CN=...` as a filesystem
# path (`C:/Program Files/Git/CN=...`). Setting MSYS_NO_PATHCONV
# would also break the `-out path` arguments, so we use the
# double-slash escape for subject strings only.
SUBJ_ROOT="//"
if [ "${OSTYPE-}" != "msys" ] && [ "${OSTYPE-}" != "cygwin" ]; then
    SUBJ_ROOT="/"
fi
DST="$HERE/certs"
mkdir -p "$DST"

DAYS="${DAYS:-365}"

# ---------- CA ----------

if [ ! -f "$DST/ca.key" ]; then
    echo "[certs] generating CA"
    openssl genrsa -out "$DST/ca.key" 4096 >/dev/null 2>&1
    openssl req -x509 -new -nodes -key "$DST/ca.key" \
        -sha256 -days "$DAYS" \
        -subj "${SUBJ_ROOT}CN=ZeroTrust Dev CA/O=zt-dev" \
        -out "$DST/ca.crt" >/dev/null 2>&1
fi

# ---------- helper to mint a leaf cert ----------

mint_leaf() {
    local name="$1"  # e.g. server, client-1
    local cn="$2"    # subject CN
    local extra="$3" # extension block (san or empty)

    local key="$DST/$name.key"
    local csr="$DST/$name.csr"
    local crt="$DST/$name.crt"
    local cnf
    cnf="$(mktemp)"
    cat > "$cnf" <<EOF
[req]
distinguished_name = dn
req_extensions = v3_req
prompt = no
[dn]
CN = $cn
O = zt-dev
[v3_req]
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth, clientAuth
$extra
EOF

    openssl genrsa -out "$key" 2048 >/dev/null 2>&1
    openssl req -new -key "$key" -out "$csr" -config "$cnf" >/dev/null 2>&1
    openssl x509 -req -in "$csr" \
        -CA "$DST/ca.crt" -CAkey "$DST/ca.key" -CAcreateserial \
        -days "$DAYS" -sha256 \
        -extensions v3_req -extfile "$cnf" \
        -out "$crt" >/dev/null 2>&1
    rm -f "$cnf" "$csr"
    echo "[certs] minted $name"
}

if [ ! -f "$DST/server.key" ]; then
    mint_leaf server "gateway" "subjectAltName = DNS:localhost, DNS:gateway, IP:127.0.0.1"
fi

if [ ! -f "$DST/client-1.key" ]; then
    mint_leaf client-1 "client-1" ""
fi

echo "[certs] done. Files in $DST:"
ls -1 "$DST" | grep -E '\.(crt|key)$' || true
