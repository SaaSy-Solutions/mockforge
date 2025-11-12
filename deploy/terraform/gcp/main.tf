# MockForge GCP Deployment - Cloud Run
#
# This module deploys MockForge on Google Cloud Platform using Cloud Run for serverless container deployment.
# It includes auto-scaling, Cloud Logging, and Cloud Monitoring integration.

terraform {
  required_version = ">= 1.0"

  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }
  }
}

provider "google" {
  project = var.project_id
  region  = var.region
}

# Cloud Run Service
resource "google_cloud_run_service" "main" {
  name     = "${var.project_name}-service"
  location = var.region

  template {
    spec {
      containers {
        image = var.container_image

        ports {
          container_port = 3000
        }

        env {
          name  = "MOCKFORGE_HTTP_PORT"
          value = "3000"
        }

        env {
          name  = "MOCKFORGE_ADMIN_ENABLED"
          value = "true"
        }

        env {
          name  = "MOCKFORGE_ADMIN_PORT"
          value = "9080"
        }

        env {
          name  = "RUST_LOG"
          value = var.log_level
        }

        resources {
          limits = {
            cpu    = "${var.cpu}m"
            memory = "${var.memory}Mi"
          }
        }
      }

      container_concurrency = var.concurrency
      timeout_seconds       = 300
    }

    metadata {
      annotations = {
        "autoscaling.knative.dev/minScale" = tostring(var.min_instances)
        "autoscaling.knative.dev/maxScale" = tostring(var.max_instances)
        "run.googleapis.com/cpu-throttling" = "false"
      }
    }
  }

  traffic {
    percent         = 100
    latest_revision = true
  }
}

# IAM Policy for public access
resource "google_cloud_run_service_iam_member" "public" {
  count = var.allow_unauthenticated ? 1 : 0

  service  = google_cloud_run_service.main.name
  location = google_cloud_run_service.main.location
  role     = "roles/run.invoker"
  member   = "allUsers"
}

# Custom Domain Mapping (optional)
resource "google_cloud_run_domain_mapping" "main" {
  count = var.custom_domain != "" ? 1 : 0

  name     = var.custom_domain
  location = var.region

  metadata {
    namespace = var.project_id
  }

  spec {
    route_name = google_cloud_run_service.main.name
  }
}

# Cloud Logging Sink (optional)
resource "google_logging_project_sink" "main" {
  count = var.enable_logging_sink ? 1 : 0

  name        = "${var.project_name}-logs"
  destination = "logging.googleapis.com/projects/${var.project_id}"

  filter = "resource.type=cloud_run_revision AND resource.labels.service_name=${google_cloud_run_service.main.name}"

  unique_writer_identity = true
}
