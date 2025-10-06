#!/bin/bash
# Script to run MockForge performance benchmarks
# Usage: ./scripts/run-benchmarks.sh [options]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== MockForge Benchmark Runner ===${NC}\n"

# Check if criterion is available
if ! grep -q "criterion" Cargo.toml; then
    echo -e "${RED}Error: Criterion not found in Cargo.toml${NC}"
    exit 1
fi

# Parse command line arguments
BASELINE=""
SAVE_BASELINE=""
BENCH_NAME="core_benchmarks"
GROUP=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --baseline)
            BASELINE="$2"
            shift 2
            ;;
        --save-baseline)
            SAVE_BASELINE="$2"
            shift 2
            ;;
        --bench)
            BENCH_NAME="$2"
            shift 2
            ;;
        --group)
            GROUP="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --baseline NAME          Compare against saved baseline"
            echo "  --save-baseline NAME     Save results as a baseline"
            echo "  --bench NAME             Run specific benchmark (default: core_benchmarks)"
            echo "  --group NAME             Run specific benchmark group"
            echo "  --help                   Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                                    # Run all benchmarks"
            echo "  $0 --save-baseline main               # Save baseline for main branch"
            echo "  $0 --baseline main                    # Compare against main baseline"
            echo "  $0 --group template_rendering         # Run only template benchmarks"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Build benchmark command
CMD="cargo bench --bench ${BENCH_NAME}"

if [ -n "$GROUP" ]; then
    CMD="$CMD -- $GROUP"
fi

if [ -n "$BASELINE" ]; then
    CMD="$CMD --baseline $BASELINE"
    echo -e "${YELLOW}Comparing against baseline: ${BASELINE}${NC}\n"
fi

if [ -n "$SAVE_BASELINE" ]; then
    CMD="$CMD --save-baseline $SAVE_BASELINE"
    echo -e "${YELLOW}Saving results as baseline: ${SAVE_BASELINE}${NC}\n"
fi

# Run benchmarks
echo -e "${GREEN}Running: ${CMD}${NC}\n"
eval $CMD

# Show results location
echo ""
echo -e "${GREEN}=== Benchmark Complete ===${NC}"
echo -e "HTML reports saved to: ${YELLOW}target/criterion/${NC}"
echo ""
echo "To view results:"
echo "  - Open target/criterion/report/index.html in a browser"
echo "  - Or run: firefox target/criterion/report/index.html"
