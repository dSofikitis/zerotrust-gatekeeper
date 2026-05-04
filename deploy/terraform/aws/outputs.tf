output "gateway_alb_dns" {
  description = "Public DNS for the ALB. Hit it with curl + a JWT from auth-issuer."
  value       = aws_lb.gateway.dns_name
}

output "auth_issuer_dns" {
  description = "Internal DNS name (service discovery) for auth-issuer. JWKS at /.well-known/jwks.json."
  value       = "auth-issuer.${aws_service_discovery_private_dns_namespace.internal.name}"
}

output "backend_echo_dns" {
  description = "Internal DNS name (service discovery) for backend-echo."
  value       = "backend-echo.${aws_service_discovery_private_dns_namespace.internal.name}"
}

output "redis_endpoint" {
  description = "ElastiCache primary endpoint backing the gateway's rate-limit counter."
  value       = aws_elasticache_replication_group.rate_limit.primary_endpoint_address
}
