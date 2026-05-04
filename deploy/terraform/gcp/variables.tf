variable "project_id" {
  description = "GCP project that holds the gateway + auxiliary services."
  type        = string
}

variable "region" {
  description = "Cloud Run region (and Memorystore region)."
  type        = string
  default     = "europe-west1"
}

variable "gateway_image" {
  description = "Fully qualified container image for zt-gateway. e.g. eu.gcr.io/<project>/zt-gateway:0.2.0"
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

variable "service_invoker_members" {
  description = "Principals allowed to invoke the gateway. The dev default 'allUsers' is fine for the demo; production should pin to an IAP-protected backend."
  type        = list(string)
  default     = ["allUsers"]
}

variable "labels" {
  description = "Common labels applied to every resource."
  type        = map(string)
  default = {
    project = "zerotrust-gatekeeper"
    managed = "terraform"
  }
}
