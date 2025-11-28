#!/bin/bash
# One-click deployment script for Azure using Terraform

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TERRAFORM_DIR="${SCRIPT_DIR}/../terraform/azure"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

PROJECT_NAME="mockforge"
ENVIRONMENT="dev"
LOCATION="eastus"
AUTO_APPROVE=false

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
    --location)
      LOCATION="$2"
      shift 2
      ;;
    --auto-approve)
      AUTO_APPROVE=true
      shift
      ;;
    --help)
      echo "Usage: $0 [OPTIONS]"
      echo "Options: --project-name, --environment, --location, --auto-approve, --help"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

echo -e "${GREEN}MockForge Azure Deployment${NC}"
echo "Project: $PROJECT_NAME"
echo "Environment: $ENVIRONMENT"
echo "Location: $LOCATION"
echo ""

# Check prerequisites
if ! command -v terraform &> /dev/null; then
    echo -e "${RED}Error: Terraform is not installed${NC}"
    exit 1
fi

if ! command -v az &> /dev/null; then
    echo -e "${RED}Error: Azure CLI is not installed${NC}"
    exit 1
fi

# Check Azure login
if ! az account show &> /dev/null; then
    echo -e "${RED}Error: Not logged in to Azure${NC}"
    echo "Run: az login"
    exit 1
fi

cd "$TERRAFORM_DIR"
terraform init

cat > terraform.tfvars << EOF
project_name = "$PROJECT_NAME"
environment  = "$ENVIRONMENT"
location     = "$LOCATION"
EOF

terraform plan -out=tfplan

if [ "$AUTO_APPROVE" = false ]; then
    read -p "Apply this plan? (yes/no): " confirm
    [ "$confirm" = "yes" ] || exit 0
fi

terraform apply tfplan

echo -e "${GREEN}Deployment complete!${NC}"
terraform output
