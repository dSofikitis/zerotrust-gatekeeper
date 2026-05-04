# Contributing

Polyglot monorepo. Every component is independently buildable and
tested with its own toolchain.

## Quick start

From the repo root:

```bash
make help        # list every target
make build       # build all services
make test        # run all tests (cargo + go + opa)
make lint        # rustfmt + clippy + go vet + opa fmt
make compose-up  # bring up the full stack
```

| Component | Toolchain | Where |
|---|---|---|
| `gateway` | Rust stable | `services/gateway/` (Cargo.toml) |
| `auth-issuer`, `backend-echo` | Go ≥ 1.23 | `services/<name>/` (go.mod) |
| `policies` | OPA ≥ 0.65 | `policies/*.rego` |

## Branches and PRs

- One logical change per commit; small PRs land faster.
- The PR template asks for the components touched, a test plan, and
  a quick security review (no committed secrets, no new external
  network calls without an allow-list, OPA policies covered by `opa
  test`).
- CI (`.github/workflows/ci.yml`) runs lint + test in parallel
  across Rust, Go, and Rego on every push to `main` and every PR.
  Keep all jobs green.

## Touching a Rego policy

- Add a new `.rego` file under `policies/` (one rule package per
  file, named after its concern: `tenants.rego`, `methods.rego`,
  `geo.rego`).
- Always add a sibling `*_test.rego` covering at least the
  default-deny path and one happy-path case.
- Run `make test-policies` (or `opa test policies/`) before pushing.

## Touching crypto / auth code

- The gateway's JWT validator and mTLS termination live behind
  trait-shaped seams so they're unit-testable without standing up
  the cert chain. Keep them that way: any change to those modules
  needs a test that exercises both the success and the failure
  path.
- Do not check in `.pem`, `.crt`, `.key`, or `.csr` files. The
  `.gitignore` enforces this; the dev certs are produced by
  `scripts/gen-certs.sh`.

## Adding a new service language

If you bring in a fourth toolchain (e.g. WASM filters, a UI), add a
job to `.github/workflows/ci.yml` and per-target rules in the
top-level `Makefile`. Make sure tests are runnable in CI without
external network access beyond `cargo` / `go mod download` / `opa`.
