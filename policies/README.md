# policies

Rego policies for the ZeroTrust Gatekeeper, evaluated by the OPA
sidecar. The gateway calls `POST /v1/data/zt/authz/allow` per
request with the input contract documented in
[`ARCHITECTURE.md`](../ARCHITECTURE.md#policy-input-contract).

## Files

- `zt.rego` — placeholder `data.zt.authz.allow` so CI's `opa fmt` /
  `opa test` have something to evaluate from commit 1.
- `zt_test.rego` — companion test: default-deny + scaffold path.

## What lands in phase 7

- `tenants.rego` — a request can only touch resources belonging to
  the JWT's tenant claim.
- `methods.rego` — `GET` allowed for any valid identity; `POST` /
  `PUT` / `DELETE` require `roles` containing `admin`.
- `geo.rego` — block list of country codes from JWT claims.

## Local check

```bash
opa fmt --diff policies/
opa test policies/
```
