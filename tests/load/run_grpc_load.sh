#!/bin/bash
# gRPC Load Testing Runner Script

set -e

# Configuration
GRPC_ADDR="${GRPC_ADDR:-localhost:50051}"
USE_TLS="${USE_TLS:-false}"
DURATION="${DURATION:-5m}"
VUS="${VUS:-100}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== MockForge gRPC Load Testing ===${NC}"
echo "gRPC Address: $GRPC_ADDR"
echo "Use TLS: $USE_TLS"
echo "Duration: $DURATION"
echo "Virtual Users: $VUS"
echo ""

# Check if k6 is installed
if ! command -v k6 &> /dev/null; then
    echo -e "${RED}Error: k6 is not installed${NC}"
    echo "Install k6: https://k6.io/docs/get-started/installation/"
    exit 1
fi

# Check if server is running
echo -e "${YELLOW}Checking if gRPC server is running...${NC}"
if command -v grpcurl &> /dev/null; then
    if grpcurl -plaintext "$GRPC_ADDR" list &> /dev/null; then
        echo -e "${GREEN}gRPC server is running${NC}"
    else
        echo -e "${YELLOW}Warning: Unable to verify gRPC server${NC}"
        echo "Attempting to proceed with load test..."
    fi
else
    echo -e "${YELLOW}grpcurl not installed, skipping server check${NC}"
fi

# Create results directory
mkdir -p tests/load/results

echo -e "${YELLOW}Running k6 gRPC load test...${NC}"

# Run k6 test
k6 run \
    --out json=tests/load/results/k6-grpc-results.json \
    --summary-export=tests/load/results/k6-grpc-summary.json \
    -e GRPC_ADDR="$GRPC_ADDR" \
    -e USE_TLS="$USE_TLS" \
    tests/load/grpc_load.js

echo ""
echo -e "${GREEN}=== gRPC Load Test Completed ===${NC}"
echo "Results saved in tests/load/results/"
