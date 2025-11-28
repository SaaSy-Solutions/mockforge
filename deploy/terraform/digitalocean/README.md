# MockForge DigitalOcean Deployment

Deploy MockForge on DigitalOcean using App Platform for managed container deployment.

## Features

- ✅ App Platform (managed containers)
- ✅ Auto-scaling
- ✅ Built-in load balancing
- ✅ HTTPS by default
- ✅ Custom domain support
- ✅ Simple pricing

## Prerequisites

1. DigitalOcean account
2. DigitalOcean API token
3. Terraform 1.0+ installed

## Quick Start

```bash
# Set your API token
export TF_VAR_do_token="your-do-token"

# Initialize Terraform
terraform init

# Review the plan
terraform plan

# Apply configuration
terraform apply
```

## Configuration

Create `terraform.tfvars`:

```hcl
do_token = "your-do-token"
project_name = "mockforge"
environment = "dev"
region = "nyc1"
```

## Cost Estimation

App Platform pricing is fixed per instance:

### Instance Sizes
- **basic-xxs**: $5/month (0.25 vCPU, 512MB RAM)
- **basic-xs**: $12/month (0.5 vCPU, 1GB RAM)
- **basic-s**: $24/month (1 vCPU, 2GB RAM)
- **basic-m**: $48/month (2 vCPU, 4GB RAM)

### Example Monthly Costs

**Development (1x basic-xs):**
- **Total: ~$12/month**

**Production (2-5x basic-s with auto-scaling):**
- **Total: ~$48-120/month** (varies with traffic)

## Custom Domain

To use a custom domain:

1. Update `terraform.tfvars`:
```hcl
custom_domain = "api.example.com"
```

2. Apply changes:
```bash
terraform apply
```

3. Update DNS records as shown in outputs

## Monitoring

View logs in DigitalOcean dashboard:
- Navigate to Apps → Your App → Runtime Logs

View metrics:
- Navigate to Apps → Your App → Metrics

## Scaling

Auto-scaling is configured via `min_instances` and `max_instances`.

Adjust in `terraform.tfvars`:
```hcl
min_instances = 2
max_instances = 10
```

## Cleanup

```bash
terraform destroy
```
