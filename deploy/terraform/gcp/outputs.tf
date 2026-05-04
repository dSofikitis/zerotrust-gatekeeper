output "gateway_url" {
  description = "Public URL of the deployed gateway. Hit it with curl + a JWT from auth-issuer."
  value       = google_cloud_run_v2_service.gateway.uri
}

output "auth_issuer_url" {
  description = "Internal URL for auth-issuer. JWKS is at /.well-known/jwks.json."
  value       = google_cloud_run_v2_service.auth_issuer.uri
}

output "backend_echo_url" {
  description = "Internal URL for the upstream the gateway proxies to."
  value       = google_cloud_run_v2_service.backend_echo.uri
}

output "redis_endpoint" {
  description = "Memorystore endpoint backing the gateway's rate-limit token bucket."
  value       = "${google_redis_instance.rate_limit.host}:${google_redis_instance.rate_limit.port}"
}
