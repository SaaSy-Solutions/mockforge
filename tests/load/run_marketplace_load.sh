#!/bin/bash
# Marketplace Load Test Runner
# Runs load tests for marketplace endpoints (plugins, templates, scenarios)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results"
MARKETPLACE_TEST="$SCRIPT_DIR/marketplace_load.js"

# Default values
REGISTRY_URL="${REGISTRY_URL:-http://localhost:8080}"
AUTH_TOKEN="${AUTH_TOKEN:-}"
ORG_ID="${ORG_ID:-}"
QUICK_MODE="${QUICK_MODE:-false}"

# Create results directory if it doesn't exist
mkdir -p "$RESULTS_DIR"

echo -e "${GREEN}=== Marketplace Load Test ===${NC}"
echo "Registry URL: $REGISTRY_URL"
echo "Results directory: $RESULTS_DIR"
echo ""

# Check if k6 is installed
if ! command -v k6 &> /dev/null; then
    echo -e "${RED}Error: k6 is not installed${NC}"
    echo "Install k6: https://k6.io/docs/getting-started/installation/"
    exit 1
fi

# Check if registry server is accessible
echo -e "${YELLOW}Checking registry server availability...${NC}"
if ! curl -s -f "$REGISTRY_URL/health" > /dev/null 2>&1; then
    echo -e "${RED}Error: Registry server is not accessible at $REGISTRY_URL${NC}"
    echo "Make sure the registry server is running and accessible"
    exit 1
fi
echo -e "${GREEN}Registry server is accessible${NC}"
echo ""

# Prepare k6 environment variables
export K6_REGISTRY_URL="$REGISTRY_URL"
if [ -n "$AUTH_TOKEN" ]; then
    export K6_AUTH_TOKEN="$AUTH_TOKEN"
fi
if [ -n "$ORG_ID" ]; then
    export K6_ORG_ID="$ORG_ID"
fi

# Generate timestamp for results
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
RESULTS_FILE="$RESULTS_DIR/marketplace_load_${TIMESTAMP}.json"
SUMMARY_FILE="$RESULTS_DIR/marketplace_load_${TIMESTAMP}.txt"

echo -e "${GREEN}Starting marketplace load test...${NC}"
echo "Results will be saved to: $RESULTS_FILE"
echo ""

# Run k6 load test
if k6 run \
    --out "json=$RESULTS_FILE" \
    --summary-export="$SUMMARY_FILE" \
    "$MARKETPLACE_TEST"; then
    echo ""
    echo -e "${GREEN}=== Load Test Completed Successfully ===${NC}"
    echo "Results: $RESULTS_FILE"
    echo "Summary: $SUMMARY_FILE"
    echo ""

    # Display summary if available
    if [ -f "$SUMMARY_FILE" ]; then
        echo -e "${GREEN}=== Test Summary ===${NC}"
        cat "$SUMMARY_FILE" | grep -E "(marketplace_|http_req)" | head -20
        echo ""
    fi
else
    echo ""
    echo -e "${RED}=== Load Test Failed ===${NC}"
    exit 1
fi
