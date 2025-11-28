# One-Click Deployment Scripts

Automated deployment scripts for MockForge on major cloud platforms.

## Available Scripts

- `deploy-aws.sh` - Deploy to AWS ECS Fargate
- `deploy-gcp.sh` - Deploy to Google Cloud Run
- `deploy-azure.sh` - Deploy to Azure Container Apps
- `deploy-digitalocean.sh` - Deploy to DigitalOcean App Platform

## Prerequisites

All scripts require:
- Terraform 1.0+ installed
- Cloud provider CLI tools configured
- Appropriate credentials/permissions

## Quick Start

### AWS Deployment

```bash
./deploy/scripts/deploy-aws.sh \
  --project-name mockforge \
  --environment dev \
  --region us-east-1
```

### GCP Deployment

```bash
./deploy/scripts/deploy-gcp.sh \
  --project-id your-gcp-project-id \
  --project-name mockforge \
  --environment dev \
  --region us-central1
```

### Azure Deployment

```bash
./deploy/scripts/deploy-azure.sh \
  --project-name mockforge \
  --environment dev \
  --location eastus
```

### DigitalOcean Deployment

```bash
export DIGITALOCEAN_TOKEN="your-do-token"
./deploy/scripts/deploy-digitalocean.sh \
  --project-name mockforge \
  --environment dev \
  --region nyc1
```

## Auto-Approval

Skip confirmation prompts:

```bash
./deploy/scripts/deploy-aws.sh --auto-approve
```

## Help

Get usage information:

```bash
./deploy/scripts/deploy-aws.sh --help
```

## What These Scripts Do

1. Check prerequisites (Terraform, cloud CLI)
2. Validate credentials
3. Initialize Terraform
4. Create configuration files
5. Plan deployment
6. Apply deployment (with confirmation)
7. Display service URLs

## Troubleshooting

### Terraform Not Found
- Install Terraform: https://www.terraform.io/downloads
- Ensure it's in your PATH

### Cloud CLI Not Configured
- **AWS**: Run `aws configure`
- **GCP**: Run `gcloud auth login` and `gcloud config set project PROJECT_ID`
- **Azure**: Run `az login`
- **DigitalOcean**: Set `DIGITALOCEAN_TOKEN` environment variable

### Permission Errors
- Verify cloud provider permissions
- Check IAM roles and policies
- Ensure billing is enabled (for GCP)

## Next Steps

After deployment:
1. Wait for services to become stable (2-5 minutes)
2. Access Admin UI at the provided URL
3. Configure your applications to use the service URL
4. Set up monitoring and alerts
