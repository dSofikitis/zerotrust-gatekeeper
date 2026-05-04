# deploy/terraform

Terraform modules for cloud deploys. Two targets, same service
shape — the gateway → auth-issuer / backend-echo wiring and the
Redis-backed rate limiter look identical from the application's
point of view:

- [`gcp/`](gcp/) — Cloud Run services + Memorystore (Redis) +
  IAM bindings.
- [`aws/`](aws/) — Fargate on ECS + ALB (HTTPS) + ElastiCache
  Redis + service discovery + per-service security groups +
  Secrets Manager for the auth-issuer signing key.

Both modules **deliberately ship without a state backend** —
bring your own GCS / S3 + lock table. Each module's README has
the exact `backend.tf` snippet to drop in next to `main.tf`.
