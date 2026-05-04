# backend-echo

Sample protected upstream. Echoes the request body + the identity
the gateway stamped via `X-Auth-Identity` so we can verify the full
mTLS → JWT → OPA → rate-limit → forward chain end to end.

Implementation lands in phase 8. The current commit ships a stub to
keep CI green.
