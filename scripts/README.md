# scripts

| Script | Lands | Purpose |
|---|---|---|
| `gen-certs.sh` | phase 2 | One-shot `openssl` recipe that produces `certs/{ca,server,client-1}.{crt,key}` for local Compose runs. |
| `demo.sh` | phase 8 | End-to-end walk: get a token, call a protected route, hit a denied route, trip the rate limit, show the audit log. |
