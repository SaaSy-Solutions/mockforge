#!/bin/bash
# High-scale load test runner for MockForge
# Tests with 10,000+ concurrent connections

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     MockForge High-Scale Load Test (10,000+ VUs)     ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════╝${NC}"
echo ""

# Configuration
BASE_URL="${BASE_URL:-http://localhost:3000}"
WS_URL="${WS_URL:-ws://localhost:3001/ws}"
GRPC_ADDR="${GRPC_ADDR:-localhost:50051}"
PROTOCOL="${PROTOCOL:-all}"
DURATION="${DURATION:-16m}"  # Total test duration

# Create results directory
mkdir -p tests/load/results
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_DIR="tests/load/results/high_scale_$TIMESTAMP"
mkdir -p "$RESULTS_DIR"

echo -e "${GREEN}Results will be saved to: $RESULTS_DIR${NC}"
echo ""

# Check if k6 is installed
if ! command -v k6 &> /dev/null; then
    echo -e "${RED}❌ k6 is not installed${NC}"
    echo "Install k6: https://k6.io/docs/getting-started/installation/"
    exit 1
fi

# Check if MockForge is running
echo -e "${YELLOW}Checking if MockForge is running...${NC}"
if ! curl -s -f "${BASE_URL}/health" > /dev/null 2>&1; then
    echo -e "${RED}❌ MockForge is not running at ${BASE_URL}${NC}"
    echo "Please start MockForge before running load tests:"
    echo "  cargo run --release -- serve"
    exit 1
fi
echo -e "${GREEN}✅ MockForge is running${NC}"
echo ""

# System resource check
echo -e "${YELLOW}Checking system resources...${NC}"
echo "Available memory: $(free -h | awk '/^Mem:/ {print $7}')"
echo "CPU cores: $(nproc)"
echo "File descriptor limit: $(ulimit -n)"
echo ""

# Warn if file descriptor limit is low
FD_LIMIT=$(ulimit -n)
if [ "$FD_LIMIT" -lt 20000 ]; then
    echo -e "${YELLOW}⚠️  File descriptor limit ($FD_LIMIT) may be too low for 10,000+ connections${NC}"
    echo "Increase with: ulimit -n 65536"
    echo ""
fi

# Run tests based on protocol
if [ "$PROTOCOL" == "all" ] || [ "$PROTOCOL" == "http" ]; then
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}Running High-Scale HTTP Load Test${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    k6 run \
        --out json="${RESULTS_DIR}/http_high_scale.json" \
        --summary-export="${RESULTS_DIR}/http_high_scale_summary.json" \
        --env BASE_URL="${BASE_URL}" \
        tests/load/http_load_high_scale.js

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ HTTP load test completed${NC}"
    else
        echo -e "${RED}❌ HTTP load test failed${NC}"
        exit 1
    fi
    echo ""
fi

if [ "$PROTOCOL" == "all" ] || [ "$PROTOCOL" == "websocket" ]; then
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}Running High-Scale WebSocket Load Test${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    k6 run \
        --out json="${RESULTS_DIR}/websocket_high_scale.json" \
        --summary-export="${RESULTS_DIR}/websocket_high_scale_summary.json" \
        --env WS_URL="${WS_URL}" \
        tests/load/websocket_load_high_scale.js

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ WebSocket load test completed${NC}"
    else
        echo -e "${RED}❌ WebSocket load test failed${NC}"
        exit 1
    fi
    echo ""
fi

# Summary
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}High-Scale Load Test Complete!${NC}"
echo ""
echo "Results saved to: $RESULTS_DIR"
echo ""
echo "To view results:"
echo "  cat $RESULTS_DIR/*_summary.json | jq"
echo ""
