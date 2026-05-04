# deploy/terraform/gcp

Cloud Run module for ZeroTrust Gatekeeper.

## What it provisions

- `google_cloud_run_v2_service.gateway` — the Rust gateway, public
  ingress, scales 1..10.
- `google_cloud_run_v2_service.auth_issuer` — internal-only.
- `google_cloud_run_v2_service.backend_echo` — internal-only.
- `google_redis_instance.rate_limit` — Memorystore Basic tier, 1
  GB. Backs the gateway's token-bucket rate limiter.
- `google_cloud_run_v2_service_iam_binding.gateway_invokers` — by
  default opens the gateway to `allUsers` for the demo; pin to your
  org's IAP backend or a single principal for anything real.

## Apply

This module ships **without a state backend** by design — bring your
own GCS bucket. Wire it up in a `backend.tf` next to `main.tf`:

```hcl
terraform {
  backend "gcs" {
    bucket = "your-state-bucket"
    prefix = "zerotrust-gatekeeper/gcp"
  }
}
```

Then:

```bash
gcloud auth application-default login
terraform -chdir=deploy/terraform/gcp init
terraform -chdir=deploy/terraform/gcp plan \
  -var "project_id=your-project" \
  -var "gateway_image=eu.gcr.io/your-project/zt-gateway:0.2.0" \
  -var "auth_issuer_image=eu.gcr.io/your-project/zt-auth-issuer:0.2.0" \
  -var "backend_echo_image=eu.gcr.io/your-project/zt-backend-echo:0.2.0"
terraform -chdir=deploy/terraform/gcp apply ...
```

## What's deferred to a follow-up

- Cloud Armor WAF in front of the gateway. Comes online when
  there's a real domain to protect.
- Cert manager / managed cert for a custom domain. Until then,
  rely on the Cloud Run-issued `*.run.app` cert.
- Service-to-service auth between gateway → auth-issuer /
  backend-echo (today they're "INTERNAL_LOAD_BALANCER" ingress and
  unauthenticated; the right answer is OIDC tokens minted on the
  gateway's service account).
- Logging / monitoring sinks for the audit log (today you read it
  out of Cloud Logging stdout).
