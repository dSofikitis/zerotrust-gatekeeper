# auth-issuer

Toy IdP for the local stack. Mints short-lived JWTs signed with a
rotating key, and exposes the verification keys at
`/.well-known/jwks.json` so the gateway can validate without a
shared secret.

Real-world parallel: Keycloak, Auth0, Cognito, Google IAM. Used here
purely so the gateway has a real JWKS source during dev.

Implementation lands in phase 3. The current commit ships a stub to
keep CI green.
