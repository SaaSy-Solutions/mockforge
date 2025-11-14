#!/bin/bash
# Run all load tests sequentially

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  MockForge Comprehensive Load Tests   ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"
echo ""

# Configuration
BASE_URL="${BASE_URL:-http://localhost:8080}"
WS_URL="${WS_URL:-ws://localhost:8080}"
GRPC_ADDR="${GRPC_ADDR:-localhost:50051}"
QUICK_MODE="${QUICK_MODE:-false}"

# Adjust duration based on mode
if [ "$QUICK_MODE" == "true" ]; then
    HTTP_DURATION="30s"
    WS_DURATION="1m"
    GRPC_DURATION="1m"
    echo -e "${YELLOW}Running in QUICK mode (shorter durations)${NC}"
else
    HTTP_DURATION="2m"
    WS_DURATION="5m"
    GRPC_DURATION="5m"
    echo -e "${GREEN}Running in FULL mode${NC}"
fi

echo ""

# Create results directory
mkdir -p tests/load/results
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_DIR="tests/load/results/run_$TIMESTAMP"
mkdir -p "$RESULTS_DIR"

echo -e "${GREEN}Results will be saved to: $RESULTS_DIR${NC}"
echo ""

# Test counter
TESTS_RUN=0
TESTS_FAILED=0

# Function to run a test
run_test() {
    local test_name=$1
    local test_script=$2
    shift 2
    local test_env=("$@")

    echo -e "${BLUE}╭────────────────────────────────────────╮${NC}"
    echo -e "${BLUE}│ Running: $test_name${NC}"
    echo -e "${BLUE}╰────────────────────────────────────────╯${NC}"
    echo ""

    TESTS_RUN=$((TESTS_RUN + 1))

    if env "${test_env[@]}" bash "$test_script"; then
        echo -e "${GREEN}✓ $test_name completed successfully${NC}"
        # Move results to timestamped directory
        mv tests/load/results/k6-* "$RESULTS_DIR/" 2>/dev/null || true
        mv tests/load/results/wrk-* "$RESULTS_DIR/" 2>/dev/null || true
    else
        echo -e "${RED}✗ $test_name failed${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi

    echo ""
    echo -e "${YELLOW}Waiting 10 seconds before next test...${NC}"
    sleep 10
    echo ""
}

# Run HTTP load tests
run_test "HTTP Load Test (k6)" \
    "tests/load/run_http_load.sh" \
    "BASE_URL=$BASE_URL" \
    "DURATION=$HTTP_DURATION" \
    "TOOL=k6"

run_test "HTTP Load Test (wrk)" \
    "tests/load/run_http_load.sh" \
    "BASE_URL=$BASE_URL" \
    "DURATION=$HTTP_DURATION" \
    "TOOL=wrk" \
    "CONNECTIONS=100" \
    "THREADS=4"

# Run WebSocket load tests
run_test "WebSocket Load Test" \
    "tests/load/run_websocket_load.sh" \
    "BASE_URL=$WS_URL" \
    "DURATION=$WS_DURATION"

# Run gRPC load tests
run_test "gRPC Load Test" \
    "tests/load/run_grpc_load.sh" \
    "GRPC_ADDR=$GRPC_ADDR" \
    "DURATION=$GRPC_DURATION"

# Run marketplace load tests (if registry server is available)
if curl -s -f "${BASE_URL}/health" > /dev/null 2>&1; then
    run_test "Marketplace Load Test" \
        "tests/load/run_marketplace_load.sh" \
        "REGISTRY_URL=$BASE_URL"
else
    echo -e "${YELLOW}⚠ Skipping marketplace load test (registry server not available)${NC}"
    echo ""
fi

# Summary
echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║         Load Test Summary              ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"
echo ""
echo -e "Tests Run:    ${GREEN}$TESTS_RUN${NC}"
echo -e "Tests Failed: ${RED}$TESTS_FAILED${NC}"
echo -e "Success Rate: $(( (TESTS_RUN - TESTS_FAILED) * 100 / TESTS_RUN ))%"
echo ""
echo -e "${GREEN}All results saved in: $RESULTS_DIR${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All load tests completed successfully!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some load tests failed${NC}"
    exit 1
fi
