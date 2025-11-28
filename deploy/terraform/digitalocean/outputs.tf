output "service_url" {
  description = "Public URL to access MockForge"
  value       = digitalocean_app.main.live_url
}

output "admin_url" {
  description = "Admin UI URL"
  value       = "${digitalocean_app.main.live_url}:9080"
}

output "health_check_url" {
  description = "Health check endpoint URL"
  value       = "${digitalocean_app.main.live_url}/health/live"
}

output "app_id" {
  description = "DigitalOcean App ID"
  value       = digitalocean_app.main.id
}
