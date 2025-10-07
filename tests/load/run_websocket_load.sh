#!/bin/bash
# WebSocket Load Testing Runner Script

set -e

# Configuration
BASE_URL="${BASE_URL:-ws://localhost:8080}"
DURATION="${DURATION:-5m}"
VUS="${VUS:-100}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== MockForge WebSocket Load Testing ===${NC}"
echo "WebSocket URL: $BASE_URL"
echo "Duration: $DURATION"
echo "Virtual Users: $VUS"
echo ""

# Check if k6 is installed
if ! command -v k6 &> /dev/null; then
    echo -e "${RED}Error: k6 is not installed${NC}"
    echo "Install k6: https://k6.io/docs/get-started/installation/"
    exit 1
fi

# Check if server is running (convert ws:// to http:// for health check)
HEALTH_URL=$(echo "$BASE_URL" | sed 's/ws:/http:/')
echo -e "${YELLOW}Checking if server is running...${NC}"
if ! curl -s -o /dev/null -w "%{http_code}" "$HEALTH_URL/health" | grep -q "200"; then
    echo -e "${YELLOW}Warning: Unable to verify server health${NC}"
    echo "Attempting to proceed with load test..."
fi

# Create results directory
mkdir -p tests/load/results

echo -e "${YELLOW}Running k6 WebSocket load test...${NC}"

# Run k6 test
k6 run \
    --out json=tests/load/results/k6-websocket-results.json \
    --summary-export=tests/load/results/k6-websocket-summary.json \
    -e BASE_URL="$BASE_URL" \
    tests/load/websocket_load.js

echo ""
echo -e "${GREEN}=== WebSocket Load Test Completed ===${NC}"
echo "Results saved in tests/load/results/"
