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

variable "location" {
  description = "Azure region"
  type        = string
  default     = "eastus"
}

variable "container_image" {
  description = "Docker image for MockForge"
  type        = string
  default     = "ghcr.io/saasy-solutions/mockforge:latest"
}

variable "cpu" {
  description = "CPU allocation (0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0)"
  type        = number
  default     = 0.5
}

variable "memory" {
  description = "Memory allocation in GiB"
  type        = number
  default     = 1.0
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

variable "log_level" {
  description = "Log level (trace, debug, info, warn, error)"
  type        = string
  default     = "info"
}

variable "log_retention_days" {
  description = "Log retention in days"
  type        = number
  default     = 7
}
