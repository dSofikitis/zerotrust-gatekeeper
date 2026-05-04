variable "region" {
  description = "AWS region for the cluster, ALB, and ElastiCache."
  type        = string
  default     = "eu-west-1"
}

variable "name_prefix" {
  description = "Prefix applied to every named resource. Keep short — ALB target groups cap at 32 chars."
  type        = string
  default     = "zt"
}

variable "vpc_id" {
  description = "VPC to deploy into. Must already have at least two public + two private subnets."
  type        = string
}

variable "public_subnet_ids" {
  description = "Public subnets the ALB lives in (one per AZ)."
  type        = list(string)
}

variable "private_subnet_ids" {
  description = "Private subnets the Fargate tasks + ElastiCache live in."
  type        = list(string)
}

variable "gateway_image" {
  description = "Container image for zt-gateway. e.g. <acct>.dkr.ecr.<region>.amazonaws.com/zt-gateway:0.4.0"
  type        = string
}

variable "auth_issuer_image" {
  description = "Container image for auth-issuer."
  type        = string
}

variable "backend_echo_image" {
  description = "Container image for backend-echo."
  type        = string
}

variable "gateway_certificate_arn" {
  description = "ACM cert ARN for the ALB's HTTPS listener. mTLS termination still happens on the gateway, but the ALB needs server-side TLS for the public edge."
  type        = string
}

variable "auth_signing_secret_arn" {
  description = "Secrets Manager ARN holding the auth-issuer's RS256 private key. The auth-issuer task role gets read access."
  type        = string
}

variable "tags" {
  description = "Common tags applied to every resource."
  type        = map(string)
  default = {
    project = "zerotrust-gatekeeper"
    managed = "terraform"
  }
}
