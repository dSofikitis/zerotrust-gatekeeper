# scripts

| Script | Purpose |
|---|---|
| `gen-certs.sh` | One-shot `openssl` recipe that produces `certs/{ca,server,client-1}.{crt,key}` for local Compose runs. |
| `demo.sh` | End-to-end walk: get a token, call a protected route, hit a denied route, trip the rate limit, show the audit log. |
