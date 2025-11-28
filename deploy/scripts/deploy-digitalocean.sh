#!/bin/bash
# One-click deployment script for DigitalOcean using Terraform

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TERRAFORM_DIR="${SCRIPT_DIR}/../terraform/digitalocean"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

DO_TOKEN=""
PROJECT_NAME="mockforge"
ENVIRONMENT="dev"
REGION="nyc1"
AUTO_APPROVE=false

while [[ $# -gt 0 ]]; do
  case $1 in
    --token)
      DO_TOKEN="$2"
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
      echo "Options: --token, --project-name, --environment, --region, --auto-approve, --help"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

# Get token from environment if not provided
if [ -z "$DO_TOKEN" ]; then
    DO_TOKEN="${DIGITALOCEAN_TOKEN:-}"
    if [ -z "$DO_TOKEN" ]; then
        echo -e "${RED}Error: DigitalOcean token required${NC}"
        echo "Set via: --token TOKEN or DIGITALOCEAN_TOKEN environment variable"
        exit 1
    fi
fi

echo -e "${GREEN}MockForge DigitalOcean Deployment${NC}"
echo "Project: $PROJECT_NAME"
echo "Environment: $ENVIRONMENT"
echo "Region: $REGION"
echo ""

if ! command -v terraform &> /dev/null; then
    echo -e "${RED}Error: Terraform is not installed${NC}"
    exit 1
fi

cd "$TERRAFORM_DIR"
terraform init

cat > terraform.tfvars << EOF
do_token     = "$DO_TOKEN"
project_name = "$PROJECT_NAME"
environment  = "$ENVIRONMENT"
region       = "$REGION"
EOF

terraform plan -out=tfplan

if [ "$AUTO_APPROVE" = false ]; then
    read -p "Apply this plan? (yes/no): " confirm
    [ "$confirm" = "yes" ] || exit 0
fi

terraform apply tfplan

echo -e "${GREEN}Deployment complete!${NC}"
terraform output
