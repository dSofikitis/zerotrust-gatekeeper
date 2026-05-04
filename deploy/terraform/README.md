# deploy/terraform

Terraform modules for cloud deploys. Two targets to demonstrate the
architecture is cloud-agnostic:

- `gcp/` — Cloud Run services + Cloud Armor + Memorystore (Redis).
- `aws/` — Fargate + ALB + ElastiCache.

Both ship as **runnable module skeletons**: the resources, variables,
and outputs are defined, but `terraform apply` against a real account
needs credentials this repo deliberately doesn't carry. Wire up your
own backend / state / credentials before running.
