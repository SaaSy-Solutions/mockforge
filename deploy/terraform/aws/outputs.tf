output "service_url" {
  description = "Public URL to access MockForge HTTP API"
  value       = "http://${aws_lb.main.dns_name}"
}

output "admin_url" {
  description = "Public URL to access MockForge Admin UI"
  value       = "http://${aws_lb.main.dns_name}:9080"
}

output "health_check_url" {
  description = "Health check endpoint URL"
  value       = "http://${aws_lb.main.dns_name}/health/live"
}

output "load_balancer_dns" {
  description = "DNS name of the load balancer"
  value       = aws_lb.main.dns_name
}

output "ecs_cluster_name" {
  description = "Name of the ECS cluster"
  value       = aws_ecs_cluster.main.name
}

output "ecs_service_name" {
  description = "Name of the ECS service"
  value       = aws_ecs_service.main.name
}

output "cloudwatch_log_group" {
  description = "CloudWatch log group name"
  value       = aws_cloudwatch_log_group.main.name
}
