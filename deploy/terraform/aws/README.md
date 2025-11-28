# MockForge AWS Deployment

Deploy MockForge on AWS using ECS Fargate with Application Load Balancer, auto-scaling, and CloudWatch monitoring.

## Features

- ✅ ECS Fargate (serverless containers)
- ✅ Application Load Balancer
- ✅ Auto-scaling based on CPU and memory
- ✅ CloudWatch logging and monitoring
- ✅ VPC with public/private subnets
- ✅ Health checks and zero-downtime deployments

## Prerequisites

1. AWS CLI configured with credentials
2. Terraform 1.0+ installed
3. AWS account with appropriate permissions

## Quick Start

```bash
# Initialize Terraform
terraform init

# Review the plan
terraform plan

# Apply configuration
terraform apply
```

## Configuration

### Basic Configuration

Create `terraform.tfvars`:

```hcl
project_name = "mockforge"
environment  = "dev"
region       = "us-east-1"
```

### Production Configuration

```hcl
project_name     = "mockforge-prod"
environment      = "prod"
region           = "us-east-1"
min_instances    = 2
max_instances    = 20
cpu              = 1024
memory           = 2048
enable_ssl       = true
certificate_arn  = "arn:aws:acm:us-east-1:123456789012:certificate/abc123"
log_retention_days = 30
```

## Cost Estimation

### Development Environment
- ECS Fargate (1 task, 0.5 vCPU, 1GB): ~$15/month
- ALB: ~$16/month
- CloudWatch Logs: ~$2/month
- **Total: ~$33/month**

### Production Environment
- ECS Fargate (2-10 tasks, 1 vCPU, 2GB): ~$60-300/month
- ALB: ~$16/month
- CloudWatch Logs: ~$10/month
- **Total: ~$86-326/month** (varies with traffic)

## Outputs

After deployment, Terraform outputs:
- `service_url` - HTTP API endpoint
- `admin_url` - Admin UI endpoint
- `health_check_url` - Health check endpoint

## SSL/TLS Setup

To enable HTTPS:

1. Request an ACM certificate:
```bash
aws acm request-certificate \
  --domain-name api.example.com \
  --validation-method DNS \
  --region us-east-1
```

2. Validate the certificate (add DNS records)

3. Update `terraform.tfvars`:
```hcl
enable_ssl      = true
certificate_arn = "arn:aws:acm:us-east-1:123456789012:certificate/abc123"
```

4. Apply changes:
```bash
terraform apply
```

## Monitoring

CloudWatch Container Insights provides:
- CPU and memory utilization
- Task count and health
- Request metrics
- Log aggregation

Access logs:
```bash
aws logs tail /ecs/mockforge --follow
```

## Scaling

Auto-scaling is enabled by default and scales based on:
- CPU utilization (target: 70%)
- Memory utilization (target: 80%)

Adjust scaling in `variables.tf` or via `terraform.tfvars`.

## Troubleshooting

### Service Not Starting
- Check ECS task logs in CloudWatch
- Verify security group rules
- Check task definition and container image

### Health Checks Failing
- Verify health check path: `/health/live`
- Check security group allows ALB → ECS traffic
- Review container logs for errors

### High Costs
- Reduce `max_instances`
- Lower CPU/memory allocation
- Disable Container Insights if not needed

## Cleanup

To destroy all resources:
```bash
terraform destroy
```

**Warning:** This will delete all resources including data!
