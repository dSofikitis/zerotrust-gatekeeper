# certs

Holds **dev-only** mTLS materials for local Compose runs. Generated
by `scripts/gen-certs.sh` (lands in phase 2) and gitignored — see
`.gitignore`.

| File | Purpose |
|---|---|
| `ca.crt` / `ca.key` | Project CA. Both gateway and clients trust it. |
| `server.crt` / `server.key` | Gateway server cert; CN=gateway, SAN=localhost. |
| `client-1.crt` / `client-1.key` | A test client cert, signed by `ca`. |

**Never commit anything from this directory.** Production deploys
must use cert-manager / ACM PCA / equivalent and rotate
automatically.
