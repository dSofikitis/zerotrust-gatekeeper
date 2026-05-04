# deploy/terraform/aws

AWS module for ZeroTrust Gatekeeper. **Not implemented in v0.1** —
the GCP module under `../gcp/` is the reference, and porting to AWS
is a focused follow-up rather than a separate piece of architecture.

The shape, when it ships, will be:

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

Until that lands, the locally-runnable Compose stack
(`deploy/compose/`) and the GCP module (`../gcp/`) cover the
"multi-cloud-ready" claim by demonstrating the architecture is
cloud-agnostic.
