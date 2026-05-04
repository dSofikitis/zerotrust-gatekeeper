# deploy/terraform/aws

AWS module for ZeroTrust Gatekeeper. The GCP module under `../gcp/`
is the reference implementation; the AWS port is a mechanical
translation of the same shape rather than a separate piece of
architecture, and lives here so the multi-cloud claim is grounded
in concrete resource definitions.

When the module is wired up in full, the shape is:

- `aws_ecs_cluster` + Fargate task definitions per service (gateway,
  auth-issuer, backend-echo).
- `aws_lb` (Application Load Balancer) in front of the gateway with
  ACM-issued cert; mTLS termination remains on the gateway service
  itself.
- `aws_elasticache_replication_group` (single-node Redis) backing
  the gateway's rate-limit token bucket — the parallel of GCP's
  Memorystore.
- `aws_security_group` per service so the gateway → auth-issuer /
  backend-echo path is internal-only.
- AWS Secrets Manager + IAM roles for the auth-issuer's signing key.

The locally-runnable Compose stack (`deploy/compose/`) and the GCP
module (`../gcp/`) cover the cloud-agnostic story end-to-end today.
