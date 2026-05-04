# policies

Rego policies for the ZeroTrust Gatekeeper, evaluated by the OPA
sidecar. The gateway calls `POST /v1/data/zt/authz/allow` per
request with the input contract documented in
[`ARCHITECTURE.md`](../ARCHITECTURE.md#policy-input-contract).

## Files

- [`zt.rego`](zt.rego) — top-level `data.zt.authz.allow` rule. The
  request is allowed iff every sub-rule below is satisfied.
- [`tenants.rego`](tenants.rego) — a request can only touch
  resources belonging to the JWT's tenant claim.
- [`methods.rego`](methods.rego) — `GET` allowed for any valid
  identity; `POST` / `PUT` / `DELETE` require `roles` containing
  `admin`.
- [`geo.rego`](geo.rego) — block list of country codes pulled from
  JWT claims.
- [`config.rego`](config.rego) — shared data (blocked countries,
  admin roles) consumed by the sub-rules.
- [`zt_test.rego`](zt_test.rego) — package-level tests covering
  default-deny and the allow path.

## Local check

```bash
opa fmt --diff policies/
opa test policies/
```
