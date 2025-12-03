#!/bin/bash
# HTTP Load Testing Runner Script

set -e

# Configuration
BASE_URL="${BASE_URL:-http://localhost:8080}"
DURATION="${DURATION:-60s}"
CONNECTIONS="${CONNECTIONS:-100}"
THREADS="${THREADS:-4}"
TOOL="${TOOL:-k6}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== MockForge HTTP Load Testing ===${NC}"
echo "Base URL: $BASE_URL"
echo "Duration: $DURATION"
echo "Connections: $CONNECTIONS"
echo "Threads: $THREADS"
echo "Tool: $TOOL"
echo ""

# Check if server is running
echo -e "${YELLOW}Checking if server is running...${NC}"
if ! curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/health" | grep -q "200"; then
    echo -e "${RED}Error: Server is not running at $BASE_URL${NC}"
    echo "Please start MockForge server first"
    exit 1
fi
echo -e "${GREEN}Server is running${NC}"
echo ""

# Run load test based on selected tool
if [ "$TOOL" == "k6" ]; then
    echo -e "${YELLOW}Running k6 load test...${NC}"

    # Check if k6 is installed
    if ! command -v k6 &> /dev/null; then
        echo -e "${RED}Error: k6 is not installed${NC}"
        echo "Install k6: https://k6.io/docs/get-started/installation/"
        exit 1
    fi

    # Run k6 test
    k6 run \
        --out json=tests/load/results/k6-http-results.json \
        --summary-export=tests/load/results/k6-http-summary.json \
        -e BASE_URL="$BASE_URL" \
        tests/load/http_load.js

elif [ "$TOOL" == "work" ]; then
    echo -e "${YELLOW}Running work load test...${NC}"

    # Check if work is installed
    if ! command -v work &> /dev/null; then
        echo -e "${RED}Error: work is not installed${NC}"
        echo "Install work: https://github.com/wg/wrk"
        exit 1
    fi

    # Create results directory
    mkdir -p tests/load/results

    # Run work test
    work -t"$THREADS" \
        -c"$CONNECTIONS" \
        -d"$DURATION" \
        -s tests/load/wrk_http.lua \
        --latency \
        "$BASE_URL" \
        | tee tests/load/results/wrk-http-results.txt

else
    echo -e "${RED}Error: Unknown tool '$TOOL'${NC}"
    echo "Supported tools: k6, work"
    exit 1
fi

echo ""
echo -e "${GREEN}=== Load Test Completed ===${NC}"
echo "Results saved in tests/load/results/"
