# MockForge Enhanced Deployment Guide

**Last Updated:** 2025-01-27
**Status:** Phase 1.2 Complete - Enhanced Self-Hosting Guides

This guide provides comprehensive deployment options for MockForge, including one-click scripts, Terraform modules, and Ansible playbooks.

---

## Quick Start Options

### Option 1: One-Click Scripts (Easiest)

Deploy to any cloud platform with a single command:

```bash
# AWS
./deploy/scripts/deploy-aws.sh --project-name mockforge --environment dev

# GCP
./deploy/scripts/deploy-gcp.sh --project-id your-project-id

# Azure
./deploy/scripts/deploy-azure.sh --project-name mockforge

# DigitalOcean
export DIGITALOCEAN_TOKEN="your-token"
./deploy/scripts/deploy-digitalocean.sh --project-name mockforge
```

### Option 2: Terraform Modules (Infrastructure as Code)

Use Terraform for version-controlled, repeatable deployments:

```bash
cd deploy/terraform/aws
terraform init
terraform plan
terraform apply
```

### Option 3: Ansible Playbooks (Configuration Management)

Automate deployment across multiple servers:

```bash
ansible-playbook -i inventory.ini deploy/ansible/docker.yml
```

---

## Platform Comparison

| Platform | Service | Deployment Time | Cost/Month | Best For |
|----------|---------|----------------|------------|----------|
| **AWS** | ECS Fargate | 5-10 min | $33-326 | Enterprise, AWS ecosystem |
| **GCP** | Cloud Run | 3-5 min | $10-95 | Serverless, pay-per-use |
| **Azure** | Container Apps | 5-10 min | $15-120 | Azure ecosystem |
| **DigitalOcean** | App Platform | 3-5 min | $12-120 | Simple, predictable pricing |

---

## Detailed Guides

### AWS Deployment

**Files:** `deploy/terraform/aws/`

**Features:**
- ECS Fargate with auto-scaling
- Application Load Balancer
- CloudWatch logging and monitoring
- VPC with public/private subnets
- Health checks and zero-downtime deployments

**Quick Start:**
```bash
cd deploy/terraform/aws
terraform init
terraform apply
```

**Documentation:** See `deploy/terraform/aws/README.md`

---

### GCP Deployment

**Files:** `deploy/terraform/gcp/`

**Features:**
- Cloud Run serverless deployment
- Auto-scaling (min/max instances)
- Cloud Logging integration
- HTTPS by default
- Custom domain support

**Quick Start:**
```bash
cd deploy/terraform/gcp
export TF_VAR_project_id=$(gcloud config get-value project)
terraform init
terraform apply
```

**Documentation:** See `deploy/terraform/gcp/README.md`

---

### Azure Deployment

**Files:** `deploy/terraform/azure/`

**Features:**
- Container Apps serverless deployment
- Auto-scaling
- Log Analytics integration
- HTTPS by default
- Resource group organization

**Quick Start:**
```bash
cd deploy/terraform/azure
az login
terraform init
terraform apply
```

**Documentation:** See `deploy/terraform/azure/README.md`

---

### DigitalOcean Deployment

**Files:** `deploy/terraform/digitalocean/`

**Features:**
- App Platform managed deployment
- Built-in load balancing
- Auto-scaling
- Simple pricing
- Custom domain support

**Quick Start:**
```bash
cd deploy/terraform/digitalocean
export TF_VAR_do_token="your-token"
terraform init
terraform apply
```

**Documentation:** See `deploy/terraform/digitalocean/README.md`

---

## Ansible Playbooks

**Location:** `deploy/ansible/`

### Available Playbooks

1. **docker.yml** - Docker deployment on Linux servers
2. **kubernetes.yml** - Kubernetes deployment (coming soon)
3. **systemd.yml** - Systemd service deployment (coming soon)

### Usage

```bash
# Create inventory
cat > inventory.ini << EOF
[servers]
server1 ansible_host=1.2.3.4 ansible_user=ubuntu
EOF

# Run playbook
ansible-playbook -i inventory.ini deploy/ansible/docker.yml
```

**Documentation:** See `deploy/ansible/README.md`

---

## One-Click Deployment Scripts

**Location:** `deploy/scripts/`

### Features

- ✅ Automatic prerequisite checking
- ✅ Credential validation
- ✅ Terraform initialization
- ✅ Configuration file generation
- ✅ Deployment with confirmation
- ✅ Service URL output

### Available Scripts

- `deploy-aws.sh` - AWS ECS Fargate deployment
- `deploy-gcp.sh` - GCP Cloud Run deployment
- `deploy-azure.sh` - Azure Container Apps deployment
- `deploy-digitalocean.sh` - DigitalOcean App Platform deployment

### Usage Examples

```bash
# AWS with custom settings
./deploy/scripts/deploy-aws.sh \
  --project-name my-mockforge \
  --environment prod \
  --region us-west-2

# GCP with auto-approval
./deploy/scripts/deploy-gcp.sh \
  --project-id my-gcp-project \
  --auto-approve

# Get help
./deploy/scripts/deploy-aws.sh --help
```

**Documentation:** See `deploy/scripts/README.md`

---

## Deployment Time Estimates

| Method | Time | Complexity |
|--------|------|------------|
| **One-Click Scripts** | 3-10 min | Low |
| **Terraform Modules** | 5-15 min | Medium |
| **Ansible Playbooks** | 10-20 min | Medium |
| **Manual Deployment** | 30-60 min | High |

---

## Cost Optimization Tips

1. **Start Small**: Use minimum instance counts initially
2. **Monitor Usage**: Track actual resource consumption
3. **Right-Size**: Adjust CPU/memory based on metrics
4. **Use Spot/Preemptible**: For non-production environments
5. **Auto-Scaling**: Set appropriate min/max limits

---

## Troubleshooting

### Common Issues

**Terraform Errors:**
- Verify cloud provider credentials
- Check IAM permissions
- Ensure required APIs are enabled

**Deployment Failures:**
- Check service logs
- Verify health check endpoints
- Review security group/firewall rules

**Connection Issues:**
- Verify DNS resolution
- Check load balancer status
- Review network configuration

---

## Next Steps

After deployment:

1. **Access Admin UI**: Use the provided admin_url
2. **Configure Mocks**: Import OpenAPI specs or create mocks
3. **Set Up Monitoring**: Configure alerts and dashboards
4. **Enable SSL**: Set up custom domains with HTTPS
5. **Scale as Needed**: Adjust auto-scaling based on traffic

---

## Support

- **Documentation**: See `docs/deployment/` for platform-specific guides
- **Terraform Modules**: See `deploy/terraform/` for IaC modules
- **Ansible Playbooks**: See `deploy/ansible/` for automation
- **Scripts**: See `deploy/scripts/` for one-click deployment

---

**Last Updated:** 2025-01-27
**Phase:** 1.2 Complete - Enhanced Self-Hosting Guides
