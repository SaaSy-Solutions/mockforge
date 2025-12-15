#!/bin/bash
# Prioritize crates for coverage improvement based on user impact and coverage gaps
#
# This script analyzes coverage baseline results and prioritizes crates
# for improvement based on:
# 1. User-facing impact (High/Medium/Low)
# 2. Current coverage percentage
# 3. Coverage gap (threshold - current)
#
# Usage:
#   ./scripts/prioritize-crates.sh [options]
#
# Options:
#   --baseline FILE    Path to coverage baseline JSON (default: coverage/summary.json)
#   --output FILE      Output file for prioritized list (default: coverage/prioritized-crates.json)
#   --help             Show this help message

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BASELINE_FILE="${PROJECT_ROOT}/coverage/summary.json"
OUTPUT_FILE="${PROJECT_ROOT}/coverage/prioritized-crates.json"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --baseline)
            BASELINE_FILE="$2"
            shift 2
            ;;
        --output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --baseline FILE    Path to coverage baseline JSON (default: coverage/summary.json)"
            echo "  --output FILE      Output file for prioritized list (default: coverage/prioritized-crates.json)"
            echo "  --help             Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Check if baseline file exists
if [[ ! -f "$BASELINE_FILE" ]]; then
    echo "Error: Baseline file not found: $BASELINE_FILE"
    echo "Run './scripts/coverage-baseline.sh' first to generate baseline"
    exit 1
fi

# Check if jq is installed
if ! command -v jq &> /dev/null; then
    echo "Error: jq is required but not installed"
    echo "Install with: sudo apt-get install jq"
    exit 1
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Prioritizing crates for coverage improvement...${NC}"
echo ""

# Crate priority mapping (user-facing impact)
declare -A CRATE_PRIORITY
# High priority: User-facing crates
CRATE_PRIORITY["mockforge-core"]="high"
CRATE_PRIORITY["mockforge-http"]="high"
CRATE_PRIORITY["mockforge-cli"]="high"
CRATE_PRIORITY["mockforge-sdk"]="high"

# Medium priority: Protocol crates, UI, scenarios
CRATE_PRIORITY["mockforge-grpc"]="medium"
CRATE_PRIORITY["mockforge-ws"]="medium"
CRATE_PRIORITY["mockforge-graphql"]="medium"
CRATE_PRIORITY["mockforge-ui"]="medium"
CRATE_PRIORITY["mockforge-scenarios"]="medium"
CRATE_PRIORITY["mockforge-recorder"]="medium"
CRATE_PRIORITY["mockforge-collab"]="medium"

# Low priority: Infrastructure, optional features
CRATE_PRIORITY["mockforge-kafka"]="low"
CRATE_PRIORITY["mockforge-mqtt"]="low"
CRATE_PRIORITY["mockforge-amqp"]="low"
CRATE_PRIORITY["mockforge-smtp"]="low"
CRATE_PRIORITY["mockforge-ftp"]="low"
CRATE_PRIORITY["mockforge-tcp"]="low"
CRATE_PRIORITY["mockforge-observability"]="low"
CRATE_PRIORITY["mockforge-tracing"]="low"
CRATE_PRIORITY["mockforge-analytics"]="low"

# Function to get crate priority
get_crate_priority() {
    local crate_name="$1"
    echo "${CRATE_PRIORITY[$crate_name]:-low}"
}

# Function to calculate priority score
# Higher score = higher priority for improvement
calculate_priority_score() {
    local priority="$1"
    local coverage="$2"
    local threshold="$3"
    
    # Priority weight
    local priority_weight=0
    case "$priority" in
        high) priority_weight=100 ;;
        medium) priority_weight=50 ;;
        low) priority_weight=25 ;;
    esac
    
    # Coverage gap weight (larger gap = higher priority)
    local coverage_gap=$(echo "$threshold - $coverage" | bc -l 2>/dev/null || echo "0")
    if (( $(echo "$coverage_gap < 0" | bc -l 2>/dev/null || echo "1") )); then
        coverage_gap=0
    fi
    
    # Calculate score: priority_weight * (1 + coverage_gap/10)
    local score=$(echo "$priority_weight * (1 + $coverage_gap / 10)" | bc -l 2>/dev/null || echo "$priority_weight")
    echo "$score"
}

# Read baseline and prioritize
# Check if baseline file exists and is valid JSON
if [[ ! -f "$BASELINE_FILE" ]]; then
    echo -e "${RED}Error: Baseline file not found: $BASELINE_FILE${NC}" >&2
    echo "[]" > "$OUTPUT_FILE"
    exit 1
fi

if ! jq -e . > /dev/null 2>&1 < "$BASELINE_FILE"; then
    echo -e "${RED}Error: Invalid JSON in baseline file: $BASELINE_FILE${NC}" >&2
    echo "[]" > "$OUTPUT_FILE"
    exit 1
fi

THRESHOLD=$(jq -r '.threshold // 80' "$BASELINE_FILE" 2>/dev/null || echo "80")

# Build prioritized list - handle missing fields gracefully
PRIORITIZED=$(jq -r --arg threshold "$THRESHOLD" '
    (.crates // [])[]? | 
    select(.status == "below_threshold" or .status == "no_tests" or .status == "error" or .status == "compilation_error") |
    {
        name: (.name // "unknown"),
        coverage_percent: ((.coverage_percent // 0) | tonumber),
        lines_found: ((.lines_found // 0) | tonumber),
        lines_hit: ((.lines_hit // 0) | tonumber),
        status: (.status // "error"),
        threshold: ($threshold | tonumber),
        coverage_gap: (($threshold | tonumber) - ((.coverage_percent // 0) | tonumber))
    }
' "$BASELINE_FILE" 2>/dev/null | jq -s '.' 2>/dev/null || echo "[]")

# Add priority and score to each crate
PRIORITIZED_WITH_SCORES=$(echo "$PRIORITIZED" | jq '
    map(. + {
        priority: (
            if .name == "mockforge-core" or .name == "mockforge-http" or .name == "mockforge-cli" or .name == "mockforge-sdk" then "high"
            elif .name | test("mockforge-(grpc|ws|graphql|ui|scenarios|recorder|collab)") then "medium"
            else "low"
            end
        ),
        priority_score: (
            (if .name == "mockforge-core" or .name == "mockforge-http" or .name == "mockforge-cli" or .name == "mockforge-sdk" then 100
            elif .name | test("mockforge-(grpc|ws|graphql|ui|scenarios|recorder|collab)") then 50
            else 25
            end) * (1 + (.coverage_gap // 80) / 10)
        )
    }) | sort_by(-.priority_score)
')

# Create output directory
mkdir -p "$(dirname "$OUTPUT_FILE")"

# Save prioritized list
echo "$PRIORITIZED_WITH_SCORES" > "$OUTPUT_FILE"

# Count prioritized crates
PRIORITIZED_COUNT=$(echo "$PRIORITIZED_WITH_SCORES" | jq 'length')

# Display summary
echo -e "${GREEN}Prioritized ${PRIORITIZED_COUNT} crates for improvement${NC}"
echo ""
echo "Top 10 Priority Crates:"
echo "======================="
echo ""

echo "$PRIORITIZED_WITH_SCORES" | jq -r '.[:10][] | 
    "\(.priority_score | floor)|\(.priority)|\(.name)|\(.coverage_percent | tostring)|\(.coverage_gap | tostring)"
' | while IFS='|' read -r score priority name coverage gap; do
    printf "  %6.0f  %-6s  %-40s  %6s%%  (gap: %6s%%)\n" "$score" "$priority" "$name" "$coverage" "$gap"
done

echo ""
echo -e "${GREEN}Prioritized list saved to: $OUTPUT_FILE${NC}"
echo ""
echo "Use this list to:"
echo "  1. Focus on high-priority crates first"
echo "  2. Track coverage improvement progress"
echo "  3. Plan coverage improvement sprints"

