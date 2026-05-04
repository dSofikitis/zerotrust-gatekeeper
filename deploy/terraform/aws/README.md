# deploy/terraform/aws

Fargate module for ZeroTrust Gatekeeper. Mirrors the GCP module
under `../gcp/` — same service shape, different cloud primitives.

## What it provisions

- `aws_ecs_cluster` + Fargate task definitions for gateway,
  auth-issuer, and backend-echo.
- `aws_lb` (Application Load Balancer) in the public subnets,
  HTTPS-only listener on port 443 backed by the ACM cert you pass
  in via `gateway_certificate_arn`. mTLS termination still happens
  on the gateway itself.
- `aws_elasticache_replication_group` (single-node Redis 7.1)
  backing the gateway's rate-limit counter — wired into the
  gateway task via `GATEWAY_REDIS_URL`.
- `aws_service_discovery_private_dns_namespace` so the gateway
  reaches `auth-issuer` and `backend-echo` by stable internal DNS
  without a second ALB.
- `aws_security_group` per service so the gateway → auth-issuer /
  backend-echo path is internal-only and ElastiCache only accepts
  connections from the gateway's SG.
- A Secrets Manager secret ARN (passed in via
  `auth_signing_secret_arn`) is mounted into the auth-issuer task
  as the `AUTH_SIGNING_KEY` env var, with an IAM policy granting
  read access only to the auth-issuer task role.
- CloudWatch log groups per service with 30-day retention.

## Apply

This module ships **without a state backend** by design — bring
your own S3 bucket + DynamoDB lock table. Wire it up in a
`backend.tf` next to `main.tf`:

```hcl
terraform {
  backend "s3" {
    bucket         = "your-state-bucket"
    key            = "zerotrust-gatekeeper/aws/terraform.tfstate"
    region         = "eu-west-1"
    dynamodb_table = "your-tf-lock-table"
    encrypt        = true
  }
}
```

Then:

```bash
aws sso login --profile your-profile
export AWS_PROFILE=your-profile
terraform -chdir=deploy/terraform/aws init
terraform -chdir=deploy/terraform/aws plan \
  -var "vpc_id=vpc-..." \
  -var 'public_subnet_ids=["subnet-pub-a","subnet-pub-b"]' \
  -var 'private_subnet_ids=["subnet-priv-a","subnet-priv-b"]' \
  -var "gateway_image=<acct>.dkr.ecr.eu-west-1.amazonaws.com/zt-gateway:0.4.0" \
  -var "auth_issuer_image=<acct>.dkr.ecr.eu-west-1.amazonaws.com/zt-auth-issuer:0.4.0" \
  -var "backend_echo_image=<acct>.dkr.ecr.eu-west-1.amazonaws.com/zt-backend-echo:0.4.0" \
  -var "gateway_certificate_arn=arn:aws:acm:..." \
  -var "auth_signing_secret_arn=arn:aws:secretsmanager:..."
terraform -chdir=deploy/terraform/aws apply ...
```

## Extension points

- **WAF** — attach `aws_wafv2_web_acl_association` to
  `aws_lb.gateway` for an L7 firewall in front of the ALB.
- **Custom domain** — add an `aws_route53_record` with the ALB as
  alias target.
- **OIDC service-to-service auth** between gateway → auth-issuer /
  backend-echo: today the upstreams are reachable from anywhere in
  the gateway's SG; the right answer is short-lived signed
  identity tokens minted from the gateway task role.
- **Audit log retention** beyond CloudWatch's 30 days — add a
  Kinesis Firehose subscription to the log group with an S3
  destination, or a Lambda that streams to Athena.
