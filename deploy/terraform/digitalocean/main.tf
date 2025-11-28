# MockForge DigitalOcean Deployment - App Platform
#
# This module deploys MockForge on DigitalOcean using App Platform for managed container deployment.

terraform {
  required_version = ">= 1.0"

  required_providers {
    digitalocean = {
      source  = "digitalocean/digitalocean"
      version = "~> 2.0"
    }
  }
}

provider "digitalocean" {
  token = var.do_token
}

# App Platform App
resource "digitalocean_app" "main" {
  spec {
    name   = var.project_name
    region = var.region

    service {
      name               = "mockforge"
      instance_count     = var.min_instances
      instance_size_slug = var.instance_size

      image {
        registry_type = "DOCKER_HUB"
        repository    = var.container_image
        tag           = var.image_tag
      }

      http_port    = 3000
      health_check {
        http_path             = "/health/live"
        initial_delay_seconds = 30
        period_seconds        = 10
        timeout_seconds       = 5
        success_threshold     = 1
        failure_threshold     = 3
      }

      env {
        key   = "MOCKFORGE_HTTP_PORT"
        value = "3000"
        type  = "GENERAL"
      }

      env {
        key   = "MOCKFORGE_ADMIN_ENABLED"
        value = "true"
        type  = "GENERAL"
      }

      env {
        key   = "MOCKFORGE_ADMIN_PORT"
        value = "9080"
        type  = "GENERAL"
      }

      env {
        key   = "RUST_LOG"
        value = var.log_level
        type  = "GENERAL"
      }

      # Auto-scaling
      dynamic "scaling" {
        for_each = var.enable_auto_scaling ? [1] : []
        content {
          min_instances = var.min_instances
          max_instances = var.max_instances
        }
      }
    }

    # Custom domain (optional)
    dynamic "domain" {
      for_each = var.custom_domain != "" ? [1] : []
      content {
        name = var.custom_domain
        type = "PRIMARY"
      }
    }
  }
}
