variable "do_token" {
  description = "DigitalOcean API token"
  type        = string
  sensitive   = true
}

variable "project_name" {
  description = "Name prefix for all resources"
  type        = string
  default     = "mockforge"
}

variable "environment" {
  description = "Environment name (dev, staging, prod)"
  type        = string
  default     = "dev"
}

variable "region" {
  description = "DigitalOcean region"
  type        = string
  default     = "nyc1"
}

variable "container_image" {
  description = "Docker image for MockForge"
  type        = string
  default     = "ghcr.io/saasy-solutions/mockforge"
}

variable "image_tag" {
  description = "Docker image tag"
  type        = string
  default     = "latest"
}

variable "instance_size" {
  description = "App Platform instance size (basic-xxs, basic-xs, basic-s, basic-m, professional-xs, etc.)"
  type        = string
  default     = "basic-xs"
}

variable "min_instances" {
  description = "Minimum number of instances"
  type        = number
  default     = 1
}

variable "max_instances" {
  description = "Maximum number of instances"
  type        = number
  default     = 10
}

variable "enable_auto_scaling" {
  description = "Enable auto-scaling"
  type        = bool
  default     = true
}

variable "custom_domain" {
  description = "Custom domain name (optional)"
  type        = string
  default     = ""
}

variable "log_level" {
  description = "Log level (trace, debug, info, warn, error)"
  type        = string
  default     = "info"
}
