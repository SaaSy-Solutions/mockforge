#!/bin/bash
# One-click deployment script for GCP using Terraform
#
# This script automates the deployment of MockForge on Google Cloud Run.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TERRAFORM_DIR="${SCRIPT_DIR}/../terraform/gcp"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Default values
PROJECT_ID=""
PROJECT_NAME="mockforge"
ENVIRONMENT="dev"
REGION="us-central1"
AUTO_APPROVE=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --project-id)
      PROJECT_ID="$2"
      shift 2
      ;;
    --project-name)
      PROJECT_NAME="$2"
      shift 2
      ;;
    --environment)
      ENVIRONMENT="$2"
      shift 2
      ;;
    --region)
      REGION="$2"
      shift 2
      ;;
    --auto-approve)
      AUTO_APPROVE=true
      shift
      ;;
    --help)
      echo "Usage: $0 [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  --project-id ID       GCP Project ID (required)"
      echo "  --project-name NAME   Project name (default: mockforge)"
      echo "  --environment ENV      Environment: dev, staging, prod (default: dev)"
      echo "  --region REGION       GCP region (default: us-central1)"
      echo "  --auto-approve        Skip confirmation prompts"
      echo "  --help                Show this help message"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

# Validate required parameters
if [ -z "$PROJECT_ID" ]; then
    PROJECT_ID=$(gcloud config get-value project 2>/dev/null || echo "")
    if [ -z "$PROJECT_ID" ]; then
        echo -e "${RED}Error: --project-id is required${NC}"
        echo "Set it via: --project-id YOUR_PROJECT_ID"
        echo "Or configure gcloud: gcloud config set project YOUR_PROJECT_ID"
        exit 1
    fi
fi

echo -e "${GREEN}MockForge GCP Deployment${NC}"
echo "================================"
echo "Project ID: $PROJECT_ID"
echo "Project Name: $PROJECT_NAME"
echo "Environment: $ENVIRONMENT"
echo "Region: $REGION"
echo ""

# Check prerequisites
echo -e "${YELLOW}Checking prerequisites...${NC}"

if ! command -v terraform &> /dev/null; then
    echo -e "${RED}Error: Terraform is not installed${NC}"
    exit 1
fi

if ! command -v gcloud &> /dev/null; then
    echo -e "${RED}Error: Google Cloud SDK is not installed${NC}"
    exit 1
fi

# Enable required APIs
echo -e "${YELLOW}Enabling required GCP APIs...${NC}"
gcloud services enable run.googleapis.com --project="$PROJECT_ID" || true
gcloud services enable cloudbuild.googleapis.com --project="$PROJECT_ID" || true

echo -e "${GREEN}✓ Prerequisites check passed${NC}"
echo ""

# Navigate to Terraform directory
cd "$TERRAFORM_DIR"

# Initialize Terraform
echo -e "${YELLOW}Initializing Terraform...${NC}"
terraform init

# Create terraform.tfvars
cat > terraform.tfvars << EOF
project_id   = "$PROJECT_ID"
project_name = "$PROJECT_NAME"
environment  = "$ENVIRONMENT"
region       = "$REGION"
EOF

# Plan deployment
echo -e "${YELLOW}Planning deployment...${NC}"
terraform plan -out=tfplan

# Confirm deployment
if [ "$AUTO_APPROVE" = false ]; then
    echo ""
    read -p "Apply this plan? (yes/no): " confirm
    if [ "$confirm" != "yes" ]; then
        echo "Deployment cancelled"
        exit 0
    fi
fi

# Apply deployment
echo -e "${YELLOW}Applying deployment...${NC}"
terraform apply tfplan

# Get outputs
echo ""
echo -e "${GREEN}Deployment complete!${NC}"
echo ""
echo "Service URLs:"
terraform output -json | jq -r '.service_url.value, .admin_url.value, .health_check_url.value' | while read url; do
    echo "  - $url"
done

echo ""
echo "Next steps:"
echo "1. Access Admin UI at the admin_url above"
echo "2. Configure your application to use the service_url"
