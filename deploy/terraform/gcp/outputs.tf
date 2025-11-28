output "service_url" {
  description = "Public URL to access MockForge"
  value       = google_cloud_run_service.main.status[0].url
}

output "admin_url" {
  description = "Admin UI URL"
  value       = "${google_cloud_run_service.main.status[0].url}:9080"
}

output "health_check_url" {
  description = "Health check endpoint URL"
  value       = "${google_cloud_run_service.main.status[0].url}/health/live"
}

output "service_name" {
  description = "Cloud Run service name"
  value       = google_cloud_run_service.main.name
}

output "custom_domain_url" {
  description = "Custom domain URL (if configured)"
  value       = var.custom_domain != "" ? "https://${var.custom_domain}" : ""
}
