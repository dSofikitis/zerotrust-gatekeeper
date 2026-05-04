// Cloud Run skeleton for ZeroTrust Gatekeeper. Deliberately ships
// without a backend block — wire your own GCS state bucket before
// running `terraform init`.
//
// What this module gives you:
//   - One Cloud Run v2 service per app (gateway, auth-issuer,
//     backend-echo) with sensible CPU/memory + startup probes.
//   - A managed Memorystore (Redis) instance for the gateway's rate
//     limit token bucket.
//   - IAM bindings so the gateway can reach auth-issuer and
//     backend-echo over the internal Cloud Run network.
//
// What it does NOT do (and shouldn't, for a public repo):
//   - Bring its own state backend.
//   - Carry credentials.
//   - Provision a custom domain or managed cert; use Cloud Run's
//     built-in run.app URLs for the demo, and add your own domain
//     mapping when you wire ACM PCA / managed certs.

provider "google" {
  project = var.project_id
  region  = var.region
}

// ---------- Memorystore (Redis) ----------

resource "google_redis_instance" "rate_limit" {
  name           = "zt-rate-limit"
  tier           = "BASIC"
  memory_size_gb = 1
  region         = var.region
  redis_version  = "REDIS_7_0"
  display_name   = "ZeroTrust gateway rate limit"
  labels         = var.labels
}

// ---------- backend-echo ----------

resource "google_cloud_run_v2_service" "backend_echo" {
  name     = "zt-backend-echo"
  location = var.region
  ingress  = "INGRESS_TRAFFIC_INTERNAL_LOAD_BALANCER"

  template {
    containers {
      image = var.backend_echo_image
      env {
        name  = "ECHO_ADDR"
        value = ":8082"
      }
      ports {
        container_port = 8082
      }
      resources {
        limits = {
          cpu    = "1"
          memory = "256Mi"
        }
      }
      startup_probe {
        http_get { path = "/healthz" }
      }
    }
    scaling {
      min_instance_count = 0
      max_instance_count = 5
    }
  }
  labels = var.labels
}

// ---------- auth-issuer ----------

resource "google_cloud_run_v2_service" "auth_issuer" {
  name     = "zt-auth-issuer"
  location = var.region
  ingress  = "INGRESS_TRAFFIC_INTERNAL_LOAD_BALANCER"

  template {
    containers {
      image = var.auth_issuer_image
      env {
        name  = "AUTH_ADDR"
        value = ":8081"
      }
      env {
        name  = "AUTH_TOKEN_TTL"
        value = "15m"
      }
      ports {
        container_port = 8081
      }
      resources {
        limits = {
          cpu    = "1"
          memory = "256Mi"
        }
      }
      startup_probe {
        http_get { path = "/healthz" }
      }
    }
    scaling {
      min_instance_count = 0
      max_instance_count = 3
    }
  }
  labels = var.labels
}

// ---------- gateway ----------

resource "google_cloud_run_v2_service" "gateway" {
  name     = "zt-gateway"
  location = var.region
  ingress  = "INGRESS_TRAFFIC_ALL"

  template {
    containers {
      image = var.gateway_image
      env {
        name  = "GATEWAY_ADDR"
        value = ":8443"
      }
      env {
        name  = "GATEWAY_UPSTREAM_URL"
        value = google_cloud_run_v2_service.backend_echo.uri
      }
      env {
        name  = "GATEWAY_AUTH_JWKS_URL"
        value = "${google_cloud_run_v2_service.auth_issuer.uri}/.well-known/jwks.json"
      }
      env {
        name  = "GATEWAY_REDIS_URL"
        value = "redis://${google_redis_instance.rate_limit.host}:${google_redis_instance.rate_limit.port}"
      }
      ports {
        container_port = 8443
      }
      resources {
        limits = {
          cpu    = "2"
          memory = "512Mi"
        }
      }
      startup_probe {
        http_get { path = "/healthz" }
      }
    }
    scaling {
      min_instance_count = 1
      max_instance_count = 10
    }
  }
  labels = var.labels
}

resource "google_cloud_run_v2_service_iam_binding" "gateway_invokers" {
  project  = google_cloud_run_v2_service.gateway.project
  location = google_cloud_run_v2_service.gateway.location
  name     = google_cloud_run_v2_service.gateway.name
  role     = "roles/run.invoker"
  members  = var.service_invoker_members
}
