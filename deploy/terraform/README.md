# deploy/terraform

Terraform modules for cloud deploy. Two target clouds in v0.1 to
demonstrate multi-cloud claims:

- `gcp/` — Cloud Run services + Cloud Armor + Memorystore (Redis).
- `aws/` — Fargate + ALB + ElastiCache.

Both ship as **module skeletons**: the resources, variables, and
outputs are defined, but `terraform apply` against a real account
needs credentials this repo deliberately doesn't carry. Land your
own backend / state / credentials before running.

Modules land in phase 8.
