# MockForge GCP Deployment

Deploy MockForge on Google Cloud Platform using Cloud Run for serverless container deployment.

## Features

- ✅ Cloud Run (serverless containers)
- ✅ Auto-scaling (min/max instances)
- ✅ Cloud Logging integration
- ✅ Pay-per-use pricing
- ✅ HTTPS by default
- ✅ Custom domain support

## Prerequisites

1. Google Cloud SDK (`gcloud`) installed and configured
2. Terraform 1.0+ installed
3. GCP project with billing enabled
4. Required APIs enabled:
   ```bash
   gcloud services enable run.googleapis.com
   gcloud services enable cloudbuild.googleapis.com
   ```

## Quick Start

```bash
# Set your project ID
export TF_VAR_project_id=$(gcloud config get-value project)

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
project_id = "your-gcp-project-id"
project_name = "mockforge"
environment = "dev"
region = "us-central1"
```

## Cost Estimation

Cloud Run pricing is pay-per-use:
- **CPU**: $0.00002400 per vCPU-second
- **Memory**: $0.00000250 per GiB-second
- **Requests**: $0.40 per million requests

### Example Monthly Costs

**Low Traffic (1M requests, 100 vCPU-hours, 200 GiB-hours):**
- CPU: ~$8.64
- Memory: ~$0.50
- Requests: ~$0.40
- **Total: ~$10/month**

**Medium Traffic (10M requests, 1000 vCPU-hours, 2000 GiB-hours):**
- CPU: ~$86.40
- Memory: ~$5.00
- Requests: ~$4.00
- **Total: ~$95/month**

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

3. Verify domain ownership in Google Search Console
4. Update DNS records as instructed

## Monitoring

View logs:
```bash
gcloud logging read "resource.type=cloud_run_revision AND resource.labels.service_name=mockforge-service" --limit 50
```

View metrics in Cloud Console:
- Navigate to Cloud Run → Your Service → Metrics

## Scaling

Auto-scaling is automatic based on:
- Request volume
- CPU utilization
- Memory usage

Adjust `min_instances` and `max_instances` in `variables.tf`.

## Cleanup

```bash
terraform destroy
```
