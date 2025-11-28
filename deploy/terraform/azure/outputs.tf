output "service_url" {
  description = "Public URL to access MockForge"
  value       = azurerm_container_app.main.latest_revision_fqdn
}

output "admin_url" {
  description = "Admin UI URL"
  value       = "${azurerm_container_app.main.latest_revision_fqdn}:9080"
}

output "health_check_url" {
  description = "Health check endpoint URL"
  value       = "https://${azurerm_container_app.main.latest_revision_fqdn}/health/live"
}

output "resource_group_name" {
  description = "Resource group name"
  value       = azurerm_resource_group.main.name
}
