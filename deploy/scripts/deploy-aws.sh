#!/bin/bash
# One-click deployment script for AWS using Terraform
#
# This script automates the deployment of MockForge on AWS ECS Fargate.
# It handles Terraform initialization, planning, and application.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TERRAFORM_DIR="${SCRIPT_DIR}/../terraform/aws"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default values
PROJECT_NAME="mockforge"
ENVIRONMENT="dev"
REGION="us-east-1"
AUTO_APPROVE=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
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
      echo "  --project-name NAME    Project name (default: mockforge)"
      echo "  --environment ENV     Environment: dev, staging, prod (default: dev)"
      echo "  --region REGION       AWS region (default: us-east-1)"
      echo "  --auto-approve        Skip confirmation prompts"
      echo "  --help                Show this help message"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      echo "Use --help for usage information"
      exit 1
      ;;
  esac
done

echo -e "${GREEN}MockForge AWS Deployment${NC}"
echo "================================"
echo "Project: $PROJECT_NAME"
echo "Environment: $ENVIRONMENT"
echo "Region: $REGION"
echo ""

# Check prerequisites
echo -e "${YELLOW}Checking prerequisites...${NC}"

if ! command -v terraform &> /dev/null; then
    echo -e "${RED}Error: Terraform is not installed${NC}"
    echo "Install from: https://www.terraform.io/downloads"
    exit 1
fi

if ! command -v aws &> /dev/null; then
    echo -e "${RED}Error: AWS CLI is not installed${NC}"
    echo "Install from: https://aws.amazon.com/cli/"
    exit 1
fi

# Check AWS credentials
if ! aws sts get-caller-identity &> /dev/null; then
    echo -e "${RED}Error: AWS credentials not configured${NC}"
    echo "Run: aws configure"
    exit 1
fi

echo -e "${GREEN}✓ Prerequisites check passed${NC}"
echo ""

# Navigate to Terraform directory
cd "$TERRAFORM_DIR"

# Initialize Terraform
echo -e "${YELLOW}Initializing Terraform...${NC}"
terraform init

# Create terraform.tfvars
cat > terraform.tfvars << EOF
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
echo "1. Wait for ECS service to become stable (2-3 minutes)"
echo "2. Access Admin UI at the admin_url above"
echo "3. Configure your application to use the service_url"
