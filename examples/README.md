# examples

Ready-to-run scripts and JSON payloads for talking to the running
stack. Lands alongside the demo flow in phase 8.

Layout:
- `curl/` — shell scripts: get a token, call protected, hit a denied
  endpoint, exhaust the rate limit, fetch metrics.
- `jwt/` — sample decoded JWT payloads, useful while wiring claims
  through Rego.
