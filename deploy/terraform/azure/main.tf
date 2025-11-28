# MockForge Azure Deployment - Container Apps
#
# This module deploys MockForge on Microsoft Azure using Container Apps for serverless container deployment.

terraform {
  required_version = ">= 1.0"

  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~> 3.0"
    }
  }
}

provider "azurerm" {
  features {}
}

# Resource Group
resource "azurerm_resource_group" "main" {
  name     = "${var.project_name}-rg"
  location = var.location

  tags = {
    Environment = var.environment
    Project     = var.project_name
  }
}

# Log Analytics Workspace
resource "azurerm_log_analytics_workspace" "main" {
  name                = "${var.project_name}-logs"
  location            = azurerm_resource_group.main.location
  resource_group_name = azurerm_resource_group.main.name
  sku                 = "PerGB2018"
  retention_in_days   = var.log_retention_days
}

# Container Apps Environment
resource "azurerm_container_app_environment" "main" {
  name                       = "${var.project_name}-env"
  location                   = azurerm_resource_group.main.location
  resource_group_name        = azurerm_resource_group.main.name
  log_analytics_workspace_id = azurerm_log_analytics_workspace.main.id
}

# Container App
resource "azurerm_container_app" "main" {
  name                         = "${var.project_name}-app"
  container_app_environment_id = azurerm_container_app_environment.main.id
  resource_group_name          = azurerm_resource_group.main.name
  revision_mode                = "Single"

  template {
    min_replicas = var.min_instances
    max_replicas = var.max_instances

    container {
      name   = "mockforge"
      image  = var.container_image
      cpu    = var.cpu
      memory = "${var.memory}Gi"

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
    }
  }

  ingress {
    external_enabled = true
    target_port      = 3000
    transport        = "http"

    traffic_weight {
      percentage      = 100
      latest_revision = true
    }
  }

  tags = {
    Environment = var.environment
    Project     = var.project_name
  }
}
