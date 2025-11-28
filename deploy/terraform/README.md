# MockForge Terraform Modules

Infrastructure as Code (IaC) modules for deploying MockForge on major cloud platforms.

## Available Modules

- **AWS** - ECS Fargate, EKS, EC2 deployments
- **GCP** - Cloud Run, GKE, Compute Engine deployments
- **Azure** - Container Apps, ASK, Virtual Machines deployments
- **DigitalOcean** - App Platform, DOKS, Droplets deployments

## Quick Start

### Prerequisites

1. Install Terraform (1.0+)
2. Configure cloud provider credentials
3. Clone this repository

### Basic Usage

```bash
# Navigate to desired platform module
cd deploy/terraform/aws

# Initialize Terraform
terraform init

# Review plan
terraform plan

# Apply configuration
terraform apply
```

## Module Structure

Each platform module includes:
- `main.tf` - Main resource definitions
- `variables.tf` - Input variables
- `outputs.tf` - Output values
- `versions.tf` - Provider version constraints
- `README.md` - Platform-specific documentation

## Common Variables

All modules support these common variables:

- `project_name` - Name prefix for resources
- `environment` - Environment name (dev, staging, prod)
- `region` - Cloud region
- `instance_type` - Compute instance type
- `min_instances` - Minimum number of instances
- `max_instances` - Maximum number of instances
- `enable_monitoring` - Enable monitoring stack
- `enable_ssl` - Enable SSL/TLS

## Outputs

All modules output:
- `service_url` - Public URL to access MockForge
- `admin_url` - Admin UI URL
- `health_check_url` - Health check endpoint

## Cost Estimation

See platform-specific README files for cost estimates.

## Support

For issues or questions, see:
- [Deployment Documentation](../../docs/deployment/)
- [Cloud Deployment Guide](../../docs/CLOUD_DEPLOYMENT.md)
