variable "project_id" {
  description = "GCP Project ID"
  type        = string
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
  description = "GCP region"
  type        = string
  default     = "us-central1"
}

variable "container_image" {
  description = "Docker image for MockForge"
  type        = string
  default     = "gcr.io/saasy-solutions/mockforge:latest"
}

variable "cpu" {
  description = "CPU allocation (1000 = 1 vCPU)"
  type        = number
  default     = 1000
}

variable "memory" {
  description = "Memory allocation in MiB"
  type        = number
  default     = 1024
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

variable "concurrency" {
  description = "Number of concurrent requests per instance"
  type        = number
  default     = 80
}

variable "allow_unauthenticated" {
  description = "Allow unauthenticated access"
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

variable "enable_logging_sink" {
  description = "Enable Cloud Logging sink"
  type        = bool
  default     = false
}
