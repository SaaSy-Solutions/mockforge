#!/bin/bash
# Query production pillar usage from Prometheus metrics
#
# This script queries Prometheus metrics with pillar labels to show which pillars
# are most used in production.
#
# Usage:
#   ./scripts/pillar-usage.sh [options]
#
# Options:
#   --prometheus-url URL    Prometheus server URL (default: http://localhost:9090)
#   --time-range RANGE      Time range for query (default: 1h)
#   --format FORMAT         Output format: text, json (default: text)
#   --pillar PILLAR         Filter by specific pillar (reality, contracts, devx, cloud, ai)
#   --help                  Show this help message

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PROMETHEUS_URL="${PROMETHEUS_URL:-http://localhost:9090}"
TIME_RANGE="${TIME_RANGE:-1h}"
FORMAT="text"
FILTER_PILLAR=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --prometheus-url)
            PROMETHEUS_URL="$2"
            shift 2
            ;;
        --time-range)
            TIME_RANGE="$2"
            shift 2
            ;;
        --format)
            FORMAT="$2"
            shift 2
            ;;
        --pillar)
            FILTER_PILLAR="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --prometheus-url URL    Prometheus server URL (default: http://localhost:9090)"
            echo "  --time-range RANGE      Time range for query (default: 1h)"
            echo "  --format FORMAT         Output format: text, json (default: text)"
            echo "  --pillar PILLAR         Filter by specific pillar (reality, contracts, devx, cloud, ai)"
            echo "  --help                  Show this help message"
            echo ""
            echo "Environment variables:"
            echo "  PROMETHEUS_URL          Prometheus server URL"
            echo "  TIME_RANGE              Time range for queries"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Querying pillar usage from Prometheus...${NC}"

# Check if curl is available
if ! command -v curl &> /dev/null; then
    echo -e "${RED}Error: curl is required but not installed${NC}"
    exit 1
fi

# Function to query Prometheus
query_prometheus() {
    local query="$1"
    local encoded_query=$(printf '%s' "$query" | jq -sRr @uri)
    local url="${PROMETHEUS_URL}/api/v1/query?query=${encoded_query}"

    curl -s "$url" | jq -r '.data.result[] | "\(.metric.pillar // "unknown")|\(.value[1])"'
}

# Function to query Prometheus range
query_prometheus_range() {
    local query="$1"
    local start=$(date -u -d "-${TIME_RANGE}" +%s 2>/dev/null || date -u -v-${TIME_RANGE} +%s 2>/dev/null || echo "")
    local end=$(date -u +%s)
    local step="15s"

    if [[ -z "$start" ]]; then
        echo -e "${YELLOW}Warning: Could not parse time range. Using instant query.${NC}"
        query_prometheus "$query"
        return
    fi

    local encoded_query=$(printf '%s' "$query" | jq -sRr @uri)
    local url="${PROMETHEUS_URL}/api/v1/query_range?query=${encoded_query}&start=${start}&end=${end}&step=${step}"

    curl -s "$url" | jq -r '.data.result[] | "\(.metric.pillar // "unknown")|\(.values[-1][1])"'
}

# Check Prometheus connectivity
echo -e "${BLUE}Checking Prometheus connectivity...${NC}"
if ! curl -s "${PROMETHEUS_URL}/api/v1/status/config" > /dev/null 2>&1; then
    echo -e "${RED}Error: Cannot connect to Prometheus at ${PROMETHEUS_URL}${NC}"
    echo -e "${YELLOW}Make sure Prometheus is running and accessible.${NC}"
    exit 1
fi

echo -e "${GREEN}Connected to Prometheus${NC}"

# Query requests by pillar
echo -e "${BLUE}Querying requests by pillar...${NC}"

# Build pillar filter
PILLAR_FILTER=""
if [[ -n "$FILTER_PILLAR" ]]; then
    PILLAR_FILTER="pillar=\"${FILTER_PILLAR}\""
fi

# Query: Total requests by pillar
QUERY_REQUESTS="sum(rate(mockforge_requests_total{${PILLAR_FILTER}}[${TIME_RANGE}])) by (pillar)"
REQUESTS_DATA=$(query_prometheus_range "$QUERY_REQUESTS" || echo "")

# Query: Error rate by pillar
QUERY_ERRORS="sum(rate(mockforge_errors_total{${PILLAR_FILTER}}[${TIME_RANGE}])) by (pillar)"
ERRORS_DATA=$(query_prometheus_range "$QUERY_ERRORS" || echo "")

# Query: Average latency by pillar
QUERY_LATENCY="avg(rate(mockforge_request_duration_seconds_sum{${PILLAR_FILTER}}[${TIME_RANGE}])) by (pillar) / avg(rate(mockforge_request_duration_seconds_count{${PILLAR_FILTER}}[${TIME_RANGE}])) by (pillar)"
LATENCY_DATA=$(query_prometheus_range "$QUERY_LATENCY" || echo "")

# Generate text report
if [[ "$FORMAT" == "text" ]]; then
    {
        echo "MockForge Pillar Usage Report"
        echo "============================="
        echo ""
        echo "Prometheus URL: $PROMETHEUS_URL"
        echo "Time Range: $TIME_RANGE"
        echo "Generated: $(date)"
        echo ""

        echo "Requests by Pillar (requests/second)"
        echo "------------------------------------"
        if [[ -z "$REQUESTS_DATA" ]]; then
            echo "  No data available"
        else
            echo "$REQUESTS_DATA" | while IFS='|' read -r pillar value; do
                printf "  %-15s %s req/s\n" "$pillar:" "$value"
            done
        fi
        echo ""

        echo "Error Rate by Pillar (errors/second)"
        echo "------------------------------------"
        if [[ -z "$ERRORS_DATA" ]]; then
            echo "  No data available"
        else
            echo "$ERRORS_DATA" | while IFS='|' read -r pillar value; do
                printf "  %-15s %s err/s\n" "$pillar:" "$value"
            done
        fi
        echo ""

        echo "Average Latency by Pillar (seconds)"
        echo "-----------------------------------"
        if [[ -z "$LATENCY_DATA" ]]; then
            echo "  No data available"
        else
            echo "$LATENCY_DATA" | while IFS='|' read -r pillar value; do
                printf "  %-15s %s s\n" "$pillar:" "$value"
            done
        fi
        echo ""

        echo "Note: Metrics require pillar labels to be populated."
        echo "      Use the --pillar parameter when recording metrics to enable this analysis."
    }
fi

# Generate JSON report
if [[ "$FORMAT" == "json" ]]; then
    {
        echo "{"
        echo "  \"prometheus_url\": \"$PROMETHEUS_URL\","
        echo "  \"time_range\": \"$TIME_RANGE\","
        echo "  \"generated\": \"$(date -Iseconds)\","
        echo "  \"requests_by_pillar\": {"

        first=true
        echo "$REQUESTS_DATA" | while IFS='|' read -r pillar value; do
            [[ -z "$pillar" ]] && continue
            [[ "$first" == false ]] && echo ","
            first=false
            echo "    \"$pillar\": $value"
        done
        echo "  },"

        echo "  \"errors_by_pillar\": {"
        first=true
        echo "$ERRORS_DATA" | while IFS='|' read -r pillar value; do
            [[ -z "$pillar" ]] && continue
            [[ "$first" == false ]] && echo ","
            first=false
            echo "    \"$pillar\": $value"
        done
        echo "  },"

        echo "  \"latency_by_pillar\": {"
        first=true
        echo "$LATENCY_DATA" | while IFS='|' read -r pillar value; do
            [[ -z "$pillar" ]] && continue
            [[ "$first" == false ]] && echo ","
            first=false
            echo "    \"$pillar\": $value"
        done
        echo "  }"
        echo "}"
    } | jq .
fi

echo -e "${GREEN}Done!${NC}"
