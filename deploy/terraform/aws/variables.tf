variable "project_name" {
  description = "Name prefix for all resources"
  type        = string
  default     = "mockforge"
}

variable "environment" {
  description = "Environment name (dev, staging, prod)"
  type        = string
  default     = "dev"

  validation {
    condition     = contains(["dev", "staging", "prod"], var.environment)
    error_message = "Environment must be dev, staging, or prod."
  }
}

variable "region" {
  description = "AWS region"
  type        = string
  default     = "us-east-1"
}

variable "vpc_cidr" {
  description = "CIDR block for VPC"
  type        = string
  default     = "10.0.0.0/16"
}

variable "container_image" {
  description = "Docker image for MockForge"
  type        = string
  default     = "ghcr.io/saasy-solutions/mockforge:latest"
}

variable "cpu" {
  description = "CPU units for ECS task (256, 512, 1024, 2048, 4096)"
  type        = number
  default     = 512
}

variable "memory" {
  description = "Memory for ECS task in MB"
  type        = number
  default     = 1024
}

variable "min_instances" {
  description = "Minimum number of ECS tasks"
  type        = number
  default     = 1
}

variable "max_instances" {
  description = "Maximum number of ECS tasks"
  type        = number
  default     = 10
}

variable "enable_auto_scaling" {
  description = "Enable auto-scaling"
  type        = bool
  default     = true
}

variable "enable_monitoring" {
  description = "Enable CloudWatch Container Insights"
  type        = bool
  default     = true
}

variable "log_level" {
  description = "Log level (trace, debug, info, warn, error)"
  type        = string
  default     = "info"
}

variable "log_retention_days" {
  description = "CloudWatch log retention in days"
  type        = number
  default     = 7
}

variable "enable_ssl" {
  description = "Enable SSL/TLS (requires ACM certificate)"
  type        = bool
  default     = false
}

variable "certificate_arn" {
  description = "ACM certificate ARN for SSL (required if enable_ssl is true)"
  type        = string
  default     = ""
}
