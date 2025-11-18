#!/bin/bash
# Generate test coverage reports grouped by pillar
#
# This script parses pillar tags from source files, maps test files to modules to pillars,
# and generates coverage reports grouped by pillar.
#
# Usage:
#   ./scripts/pillar-coverage.sh [options]
#
# Options:
#   --output-dir DIR    Output directory for reports (default: ./coverage-by-pillar)
#   --format FORMAT     Output format: text, json, html (default: text)
#   --pillar PILLAR     Filter by specific pillar (reality, contracts, devx, cloud, ai)
#   --help              Show this help message

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT_DIR="${PROJECT_ROOT}/coverage-by-pillar"
FORMAT="text"
FILTER_PILLAR=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --output-dir)
            OUTPUT_DIR="$2"
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
            echo "  --output-dir DIR    Output directory for reports (default: ./coverage-by-pillar)"
            echo "  --format FORMAT     Output format: text, json, html (default: text)"
            echo "  --pillar PILLAR     Filter by specific pillar (reality, contracts, devx, cloud, ai)"
            echo "  --help              Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Generating pillar-based test coverage report...${NC}"

# Check if cargo-llvm-cov is installed
if ! command -v cargo-llvm-cov &> /dev/null; then
    echo -e "${YELLOW}Warning: cargo-llvm-cov not found. Installing...${NC}"
    cargo install cargo-llvm-cov
fi

# Function to extract pillar tags from a Rust file
extract_pillars() {
    local file="$1"
    if [[ ! -f "$file" ]]; then
        return
    fi

    # Look for "Pillars: [Reality][AI]" pattern in doc comments
    grep -h "Pillars:" "$file" 2>/dev/null | \
        sed -n 's/.*Pillars:\s*\(\[[^\]]*\]*\).*/\1/p' | \
        sed 's/\[\([^\]]*\)\]/\1\n/g' | \
        grep -v '^$' | \
        tr '[:upper:]' '[:lower:]' | \
        sort -u
}

# Function to find all Rust source files with pillar tags
find_tagged_files() {
    find "$PROJECT_ROOT/crates" -name "*.rs" -type f | while read -r file; do
        pillars=$(extract_pillars "$file")
        if [[ -n "$pillars" ]]; then
            echo "$file|$pillars"
        fi
    done
}

# Function to map test files to source files
map_test_to_source() {
    local test_file="$1"
    # Convert test file path to source file path
    # e.g., tests/integration/test_reality.rs -> crates/mockforge-core/src/reality.rs
    local source_file="${test_file}"
    source_file="${source_file#tests/}"
    source_file="${source_file#integration/}"
    source_file="${source_file#unit/}"
    source_file="${source_file#test_}"
    source_file="${source_file%.rs}"

    # Try to find the corresponding source file
    find "$PROJECT_ROOT/crates" -name "${source_file}.rs" -o -name "${source_file}/mod.rs" 2>/dev/null | head -1
}

# Collect pillar information
echo -e "${BLUE}Collecting pillar tags from source files...${NC}"
declare -A PILLAR_FILES
declare -A PILLAR_TESTS

while IFS='|' read -r file pillars; do
    if [[ -z "$FILTER_PILLAR" ]] || echo "$pillars" | grep -q "^${FILTER_PILLAR}$"; then
        for pillar in $pillars; do
            PILLAR_FILES["$pillar"]+="$file"$'\n'
        done
    fi
done < <(find_tagged_files)

# Find test files
echo -e "${BLUE}Finding test files...${NC}"
TEST_FILES=$(find "$PROJECT_ROOT/tests" "$PROJECT_ROOT/crates" -name "*test*.rs" -o -name "tests.rs" 2>/dev/null || true)

# Generate coverage report
echo -e "${BLUE}Generating coverage report...${NC}"

# Run coverage if not already done
if [[ ! -f "$PROJECT_ROOT/coverage/lcov.info" ]]; then
    echo -e "${YELLOW}Running test coverage...${NC}"
    cd "$PROJECT_ROOT"
    cargo llvm-cov --all-features --workspace --lcov --output-path coverage/lcov.info --tests || {
        echo -e "${RED}Error: Failed to generate coverage report${NC}"
        exit 1
    }
fi

# Parse coverage data by pillar
echo -e "${BLUE}Analyzing coverage by pillar...${NC}"

# Generate text report
if [[ "$FORMAT" == "text" ]]; then
    REPORT_FILE="$OUTPUT_DIR/coverage-by-pillar.txt"
    {
        echo "MockForge Test Coverage by Pillar"
        echo "=================================="
        echo ""
        echo "Generated: $(date)"
        echo ""

        for pillar in reality contracts devx cloud ai; do
            if [[ -n "$FILTER_PILLAR" ]] && [[ "$pillar" != "$FILTER_PILLAR" ]]; then
                continue
            fi

            echo "Pillar: $pillar"
            echo "----------------"

            files="${PILLAR_FILES[$pillar]:-}"
            if [[ -z "$files" ]]; then
                echo "  No files tagged with this pillar"
            else
                file_count=$(echo "$files" | grep -c '^' || echo "0")
                echo "  Files tagged: $file_count"
                echo ""
                echo "  Tagged files:"
                echo "$files" | while read -r file; do
                    [[ -n "$file" ]] && echo "    - ${file#$PROJECT_ROOT/}"
                done
            fi
            echo ""
        done

        echo "Note: This report shows files tagged with pillars."
        echo "      Actual test coverage analysis requires integration with coverage tools."
        echo "      Use 'cargo llvm-cov' to generate detailed coverage reports."
    } > "$REPORT_FILE"

    echo -e "${GREEN}Coverage report generated: $REPORT_FILE${NC}"
    cat "$REPORT_FILE"
fi

# Generate JSON report
if [[ "$FORMAT" == "json" ]]; then
    REPORT_FILE="$OUTPUT_DIR/coverage-by-pillar.json"
    {
        echo "{"
        echo "  \"generated\": \"$(date -Iseconds)\","
        echo "  \"pillars\": {"

        first=true
        for pillar in reality contracts devx cloud ai; do
            if [[ -n "$FILTER_PILLAR" ]] && [[ "$pillar" != "$FILTER_PILLAR" ]]; then
                continue
            fi

            [[ "$first" == false ]] && echo ","
            first=false

            echo "    \"$pillar\": {"
            files="${PILLAR_FILES[$pillar]:-}"
            if [[ -z "$files" ]]; then
                echo "      \"file_count\": 0,"
                echo "      \"files\": []"
            else
                file_count=$(echo "$files" | grep -c '^' || echo "0")
                echo "      \"file_count\": $file_count,"
                echo "      \"files\": ["
                first_file=true
                echo "$files" | while read -r file; do
                    [[ -z "$file" ]] && continue
                    [[ "$first_file" == false ]] && echo ","
                    first_file=false
                    echo "        \"${file#$PROJECT_ROOT/}\""
                done
                echo "      ]"
            fi
            echo -n "    }"
        done
        echo ""
        echo "  }"
        echo "}"
    } > "$REPORT_FILE"

    echo -e "${GREEN}JSON report generated: $REPORT_FILE${NC}"
fi

echo -e "${GREEN}Done!${NC}"
