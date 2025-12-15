#!/bin/bash
# Generate per-crate test coverage baseline reports
#
# This script runs cargo llvm-cov for each crate individually, generates
# per-crate coverage reports, and creates a summary of coverage percentages.
#
# Usage:
#   ./scripts/coverage-baseline.sh [options]
#
# Options:
#   --output-dir DIR    Output directory for reports (default: ./coverage)
#   --format FORMAT     Output format: json, csv, text (default: json)
#   --threshold PERCENT Minimum coverage threshold (default: 80)
#   --html              Generate HTML reports for each crate
#   --parallel          Run coverage in parallel (faster but more resource intensive)
#   --crate NAME        Run coverage for a single crate only (e.g., mockforge-core)
#   --help              Show this help message

# Don't exit on error - we want to process all crates even if some fail
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT_DIR="${PROJECT_ROOT}/coverage"
FORMAT="json"
THRESHOLD=80
GENERATE_HTML=false
PARALLEL=false
TARGET_CRATE=""

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
        --threshold)
            THRESHOLD="$2"
            shift 2
            ;;
        --html)
            GENERATE_HTML=true
            shift
            ;;
        --parallel)
            PARALLEL=true
            shift
            ;;
        --crate)
            TARGET_CRATE="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --output-dir DIR    Output directory for reports (default: ./coverage)"
            echo "  --format FORMAT     Output format: json, csv, text (default: json)"
            echo "  --threshold PERCENT Minimum coverage threshold (default: 80)"
            echo "  --html              Generate HTML reports for each crate"
            echo "  --parallel          Run coverage in parallel"
            echo "  --crate NAME        Run coverage for a single crate only (e.g., mockforge-core)"
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
mkdir -p "$OUTPUT_DIR/crates"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Generating per-crate test coverage baseline...${NC}"
echo ""

# Check if cargo-llvm-cov is installed
if ! command -v cargo-llvm-cov &> /dev/null; then
    echo -e "${YELLOW}Warning: cargo-llvm-cov not found. Installing...${NC}"
    cargo install cargo-llvm-cov --locked
fi

# Function to get crate name from Cargo.toml
get_crate_name() {
    local cargo_toml="$1"
    if [[ -f "$cargo_toml" ]]; then
        grep -E '^name\s*=' "$cargo_toml" | head -1 | sed -E 's/^name\s*=\s*"([^"]+)".*/\1/' || echo ""
    fi
}

# Function to check if crate has tests
# Note: We'll attempt coverage anyway and let cargo-llvm-cov handle it
has_tests() {
    local crate_dir="$1"
    
    # Check if there's a tests directory
    if [[ -d "${crate_dir}/tests" ]]; then
        return 0
    fi
    
    # Check if there are test modules in src
    if [[ -d "${crate_dir}/src" ]] && \
       find "${crate_dir}/src" -name "*.rs" -type f -exec grep -l '#\[cfg(test)\]' {} \; 2>/dev/null | grep -q .; then
        return 0
    fi
    
    # Library crates implicitly have lib target, so they might have tests
    # We'll let cargo-llvm-cov determine if tests exist
    return 1
}

# Discover all crates in the workspace
echo -e "${BLUE}Discovering crates...${NC}"
CRATES=()
while IFS= read -r cargo_toml; do
    # Get absolute path and convert to relative
    crate_dir="$(cd "$(dirname "$cargo_toml")" && pwd)"
    crate_dir="${crate_dir#$PROJECT_ROOT/}"
    crate_name=$(get_crate_name "$cargo_toml")
    
    if [[ -n "$crate_name" ]] && [[ "$crate_name" =~ ^mockforge- ]] && [[ "$crate_dir" =~ ^crates/ ]]; then
        # Skip fuzz targets and examples
        if [[ "$crate_dir" =~ /fuzz$ ]] || [[ "$crate_dir" =~ /examples$ ]]; then
            continue
        fi
        
        CRATES+=("$crate_name|$crate_dir")
    fi
done < <(find "$PROJECT_ROOT/crates" -name "Cargo.toml" -type f)

# Filter to target crate if specified
if [[ -n "$TARGET_CRATE" ]]; then
    FILTERED_CRATES=()
    ALL_CRATES=("${CRATES[@]}")  # Keep original for error message
    for crate_info in "${CRATES[@]}"; do
        IFS='|' read -r crate_name crate_dir <<< "$crate_info"
        if [[ "$crate_name" == "$TARGET_CRATE" ]]; then
            FILTERED_CRATES+=("$crate_info")
        fi
    done
    CRATES=("${FILTERED_CRATES[@]}")
    
    if [[ ${#CRATES[@]} -eq 0 ]]; then
        echo -e "${RED}Error: Crate '${TARGET_CRATE}' not found${NC}"
        echo -e "${YELLOW}Available crates:${NC}"
        for crate_info in "${ALL_CRATES[@]}"; do
            IFS='|' read -r crate_name crate_dir <<< "$crate_info"
            echo "  - $crate_name"
        done
        exit 1
    fi
    echo -e "${GREEN}Filtered to crate: ${TARGET_CRATE}${NC}"
else
    echo -e "${GREEN}Found ${#CRATES[@]} crates${NC}"
fi
echo ""

# Function to generate coverage for a single crate
generate_crate_coverage() {
    local crate_info="$1"
    IFS='|' read -r crate_name crate_dir <<< "$crate_info"
    
    local crate_output_dir="${OUTPUT_DIR}/crates/${crate_name}"
    mkdir -p "$crate_output_dir"
    
    echo -e "${BLUE}Processing ${crate_name}...${NC}"
    
    # Check if crate is excluded from workspace (server applications)
    local excluded_crates=("mockforge-registry-server" "mockforge-k8s-operator" "mockforge-federation" "mockforge-runtime-daemon")
    if printf '%s\n' "${excluded_crates[@]}" | grep -q "^${crate_name}$"; then
        echo -e "${YELLOW}  ⚠️  ${crate_name} is excluded from workspace (server application)${NC}"
        echo "{\"name\":\"${crate_name}\",\"coverage_percent\":0.0,\"lines_found\":0,\"lines_hit\":0,\"status\":\"excluded\"}" > "${crate_output_dir}/coverage.json"
        return 0
    fi
    
    # Try to run coverage - cargo-llvm-cov will handle crates without tests
    # We check for tests as a hint, but still attempt coverage
    
    # Generate coverage for this crate
    local lcov_file="${crate_output_dir}/lcov.info"
    local json_file="${crate_output_dir}/coverage.json"
    local error_log="${crate_output_dir}/error.log"
    
    # Run coverage - use LCOV format which is easier to parse
    # Capture stderr to check for compilation errors vs no tests
    # We check success by verifying LCOV output exists and has data
    # Some crates may be outside workspace, so try with and without --all-features
    cargo llvm-cov --package "$crate_name" --all-features --lcov --output-path "$lcov_file" 2>"$error_log"
    local llvm_cov_exit=$?
    
    # If that failed with "outside of workspace" error, try without --all-features
    if [[ $llvm_cov_exit -ne 0 ]] && grep -q "cannot specify features for packages outside of workspace\|outside of workspace" "$error_log" 2>/dev/null; then
        cargo llvm-cov --package "$crate_name" --lcov --output-path "$lcov_file" 2>"$error_log"
        llvm_cov_exit=$?
    fi
    
    # Check if we got valid LCOV output (this is the real success indicator)
    if [[ -f "$lcov_file" ]] && [[ -s "$lcov_file" ]] && grep -q "^LF:" "$lcov_file" 2>/dev/null; then
        # Parse LCOV format: LF:total_lines, LH:covered_lines
        local lines_found=$(grep "^LF:" "$lcov_file" | awk -F: '{sum+=$2} END {print sum+0}' 2>/dev/null || echo "0")
        local lines_hit=$(grep "^LH:" "$lcov_file" | awk -F: '{sum+=$2} END {print sum+0}' 2>/dev/null || echo "0")
        
        # Calculate percentage
        local coverage_percent=0
        if [[ $lines_found -gt 0 ]]; then
            coverage_percent=$(echo "scale=2; $lines_hit * 100 / $lines_found" | bc -l 2>/dev/null || echo "0")
            coverage_percent=$(printf "%.2f" "$coverage_percent" 2>/dev/null || echo "0")
        fi
        
        # Generate JSON report for detailed analysis (optional)
        # Try with --all-features first, fall back without if needed
        if ! cargo llvm-cov --package "$crate_name" --all-features --json 2>>"$error_log" > "$json_file" 2>/dev/null; then
            cargo llvm-cov --package "$crate_name" --json 2>>"$error_log" > "$json_file" || true
        fi
            
            # Generate HTML report if requested
            if [[ "$GENERATE_HTML" == "true" ]]; then
                local html_dir="${crate_output_dir}/html"
                mkdir -p "$html_dir"
                if ! cargo llvm-cov --package "$crate_name" --all-features --html --output-dir "$html_dir" 2>>"$error_log" 2>/dev/null; then
                    cargo llvm-cov --package "$crate_name" --html --output-dir "$html_dir" 2>>"$error_log" || true
                fi
            fi
            
            # Determine status
            local status="good"
            if (( $(echo "$coverage_percent < $THRESHOLD" | bc -l 2>/dev/null || echo "1") )); then
                status="below_threshold"
            fi
            
            # Update JSON with calculated values
            jq --arg cov "$coverage_percent" --arg found "$lines_found" --arg hit "$lines_hit" --arg stat "$status" \
               '. + {coverage_percent: ($cov | tonumber), lines_found: ($found | tonumber), lines_hit: ($hit | tonumber), status: $stat}' \
               "$json_file" > "${json_file}.tmp" && mv "${json_file}.tmp" "$json_file" 2>/dev/null || true
            
        if (( $(echo "$coverage_percent >= $THRESHOLD" | bc -l 2>/dev/null || echo "0") )); then
            echo -e "${GREEN}  ✅ ${crate_name}: ${coverage_percent}%${NC}"
        else
            echo -e "${YELLOW}  ⚠️  ${crate_name}: ${coverage_percent}% (below ${THRESHOLD}%)${NC}"
        fi
    else
        # Check if it's a compilation error or just no tests
        # Look for actual errors, not warnings
        local error_type="error"
        if grep -qE "error\[E[0-9]+\]|error: could not compile|error: process didn't exit successfully" "$error_log" 2>/dev/null; then
            error_type="compilation_error"
            echo -e "${RED}  ❌ Compilation error for ${crate_name}${NC}"
        elif grep -q "no test targets found\|no tests to run\|no tests found" "$error_log" 2>/dev/null; then
            error_type="no_tests"
            echo -e "${YELLOW}  ⚠️  No tests found for ${crate_name}${NC}"
        elif [[ -f "$json_file" ]] && jq -e . > /dev/null 2>&1 < "$json_file"; then
            # JSON file exists and is valid, but command failed - might be a timeout or other issue
            # Try to extract coverage from existing JSON
            local coverage=$(jq -r '.percent_covered // 0' "$json_file" 2>/dev/null || echo "0")
            if [[ "$coverage" != "0" ]] && [[ "$coverage" != "null" ]]; then
                local coverage_percent=$(echo "$coverage * 100" | bc -l 2>/dev/null || echo "0")
                coverage_percent=$(printf "%.2f" "$coverage_percent" 2>/dev/null || echo "0")
                echo -e "${YELLOW}  ⚠️  ${crate_name}: ${coverage_percent}% (partial run)${NC}"
                return 0
            fi
            error_type="error"
            echo -e "${RED}  ❌ Failed to run coverage for ${crate_name}${NC}"
        else
            error_type="error"
            echo -e "${RED}  ❌ Failed to run coverage for ${crate_name}${NC}"
        fi
        # Always create valid JSON, even on error
        echo "{\"name\":\"${crate_name}\",\"coverage_percent\":0.0,\"lines_found\":0,\"lines_hit\":0,\"status\":\"${error_type}\"}" > "$json_file"
    fi
}

# Export function for parallel execution
export -f generate_crate_coverage
export OUTPUT_DIR THRESHOLD GENERATE_HTML PROJECT_ROOT
export RED GREEN YELLOW BLUE NC

# Generate coverage for all crates
if [[ "$PARALLEL" == "true" ]]; then
    echo -e "${BLUE}Running coverage in parallel...${NC}"
    echo ""
    printf '%s\n' "${CRATES[@]}" | xargs -P "$(nproc)" -I {} bash -c 'generate_crate_coverage "$@"' _ {}
else
    echo -e "${BLUE}Running coverage sequentially...${NC}"
    echo ""
    for crate_info in "${CRATES[@]}"; do
        generate_crate_coverage "$crate_info"
    done
fi

echo ""
echo -e "${BLUE}Generating summary...${NC}"

# Collect all coverage data
SUMMARY_JSON="${OUTPUT_DIR}/summary.json"
SUMMARY_CSV="${OUTPUT_DIR}/summary.csv"
SUMMARY_TEXT="${OUTPUT_DIR}/summary.txt"

# Build JSON summary
{
    echo "{"
    echo "  \"generated\": \"$(date -Iseconds)\","
    echo "  \"threshold\": $THRESHOLD,"
    echo "  \"total_crates\": ${#CRATES[@]},"
    echo "  \"crates\": ["
    
    first=true
    total_coverage=0
    crates_with_coverage=0
    crates_below_threshold=0
    crates_no_tests=0
    crates_errors=0
    
    for crate_info in "${CRATES[@]}"; do
        IFS='|' read -r crate_name crate_dir <<< "$crate_info"
        json_file="${OUTPUT_DIR}/crates/${crate_name}/coverage.json"
        
        if [[ -f "$json_file" ]]; then
            [[ "$first" == false ]] && echo ","
            first=false
            
            coverage_percent=$(jq -r '.coverage_percent // 0' "$json_file" 2>/dev/null || echo "0")
            lines_found=$(jq -r '.lines_found // 0' "$json_file" 2>/dev/null || echo "0")
            lines_hit=$(jq -r '.lines_hit // 0' "$json_file" 2>/dev/null || echo "0")
            status=$(jq -r '.status // "unknown"' "$json_file" 2>/dev/null || echo "unknown")
            
            if [[ "$status" == "no_tests" ]]; then
                crates_no_tests=$((crates_no_tests + 1))
            elif [[ "$status" == "error" ]]; then
                crates_errors=$((crates_errors + 1))
            elif (( $(echo "$coverage_percent > 0" | bc -l 2>/dev/null || echo "0") )); then
                total_coverage=$(echo "$total_coverage + $coverage_percent" | bc -l 2>/dev/null || echo "$total_coverage")
                crates_with_coverage=$((crates_with_coverage + 1))
                
                if (( $(echo "$coverage_percent < $THRESHOLD" | bc -l 2>/dev/null || echo "1") )); then
                    crates_below_threshold=$((crates_below_threshold + 1))
                fi
            fi
            
            echo -n "    $(jq -c . "$json_file" 2>/dev/null || echo "{}")"
        fi
    done
    
    echo ""
    echo "  ],"
    
    # Calculate average coverage
    avg_coverage=0
    if [[ $crates_with_coverage -gt 0 ]]; then
        avg_coverage=$(echo "scale=2; $total_coverage / $crates_with_coverage" | bc -l 2>/dev/null || echo "0")
    fi
    
    echo "  \"statistics\": {"
    echo "    \"crates_with_coverage\": $crates_with_coverage,"
    echo "    \"crates_below_threshold\": $crates_below_threshold,"
    echo "    \"crates_no_tests\": $crates_no_tests,"
    echo "    \"crates_errors\": $crates_errors,"
    echo "    \"average_coverage\": $avg_coverage"
    echo "  }"
    echo "}"
} > "$SUMMARY_JSON"

# Generate CSV summary
{
    echo "crate_name,coverage_percent,lines_found,lines_hit,status"
    for crate_info in "${CRATES[@]}"; do
        IFS='|' read -r crate_name crate_dir <<< "$crate_info"
        json_file="${OUTPUT_DIR}/crates/${crate_name}/coverage.json"
        
        if [[ -f "$json_file" ]]; then
            coverage_percent=$(jq -r '.coverage_percent // 0' "$json_file" 2>/dev/null || echo "0")
            lines_found=$(jq -r '.lines_found // 0' "$json_file" 2>/dev/null || echo "0")
            lines_hit=$(jq -r '.lines_hit // 0' "$json_file" 2>/dev/null || echo "0")
            status=$(jq -r '.status // "unknown"' "$json_file" 2>/dev/null || echo "unknown")
            
            echo "${crate_name},${coverage_percent},${lines_found},${lines_hit},${status}"
        fi
    done
} > "$SUMMARY_CSV"

# Generate text summary
{
    echo "MockForge Test Coverage Summary"
    echo "==============================="
    echo ""
    echo "Generated: $(date)"
    echo "Threshold: ${THRESHOLD}%"
    echo ""
    echo "Statistics:"
    echo "  Total crates: ${#CRATES[@]}"
    echo "  Crates with coverage: $crates_with_coverage"
    echo "  Crates below threshold: $crates_below_threshold"
    echo "  Crates with no tests: $crates_no_tests"
    echo "  Crates with errors: $crates_errors"
    echo "  Average coverage: ${avg_coverage}%"
    echo ""
    echo "Per-Crate Coverage:"
    echo "-------------------"
    echo ""
    
    # Sort by coverage (descending)
    for crate_info in "${CRATES[@]}"; do
        IFS='|' read -r crate_name crate_dir <<< "$crate_info"
        json_file="${OUTPUT_DIR}/crates/${crate_name}/coverage.json"
        
        if [[ -f "$json_file" ]]; then
            coverage_percent=$(jq -r '.coverage_percent // 0' "$json_file" 2>/dev/null || echo "0")
            lines_found=$(jq -r '.lines_found // 0' "$json_file" 2>/dev/null || echo "0")
            lines_hit=$(jq -r '.lines_hit // 0' "$json_file" 2>/dev/null || echo "0")
            status=$(jq -r '.status // "unknown"' "$json_file" 2>/dev/null || echo "unknown")
            
            printf "%-40s %6.2f%%  (%d/%d lines)" "$crate_name" "$coverage_percent" "$lines_hit" "$lines_found"
            
            if [[ "$status" == "no_tests" ]]; then
                echo " [NO TESTS]"
            elif [[ "$status" == "error" ]]; then
                echo " [ERROR]"
            elif (( $(echo "$coverage_percent < $THRESHOLD" | bc -l 2>/dev/null || echo "1") )); then
                echo " [BELOW THRESHOLD]"
            else
                echo " [OK]"
            fi
        fi
    done | sort -t'%' -k2 -nr
    
    echo ""
    echo "Crates Below Threshold:"
    echo "----------------------"
    below_found=false
    for crate_info in "${CRATES[@]}"; do
        IFS='|' read -r crate_name crate_dir <<< "$crate_info"
        json_file="${OUTPUT_DIR}/crates/${crate_name}/coverage.json"
        
        if [[ -f "$json_file" ]]; then
            coverage_percent=$(jq -r '.coverage_percent // 0' "$json_file" 2>/dev/null || echo "0")
            status=$(jq -r '.status // "unknown"' "$json_file" 2>/dev/null || echo "unknown")
            
            if [[ "$status" != "good" ]] || (( $(echo "$coverage_percent < $THRESHOLD" | bc -l 2>/dev/null || echo "1") )); then
                below_found=true
                printf "  - %-40s %6.2f%%" "$crate_name" "$coverage_percent"
                if [[ "$status" == "no_tests" ]]; then
                    echo " (no tests)"
                elif [[ "$status" == "error" ]]; then
                    echo " (error)"
                else
                    echo ""
                fi
            fi
        fi
    done
    
    if [[ "$below_found" == "false" ]]; then
        echo "  (none - all crates meet threshold!)"
    fi
    
    echo ""
    echo "Reports:"
    echo "  JSON: $SUMMARY_JSON"
    echo "  CSV:  $SUMMARY_CSV"
    echo "  Text: $SUMMARY_TEXT"
    if [[ "$GENERATE_HTML" == "true" ]]; then
        echo "  HTML: ${OUTPUT_DIR}/crates/*/index.html"
    fi
} > "$SUMMARY_TEXT"

# Display summary based on format
case "$FORMAT" in
    json)
        echo -e "${GREEN}Coverage summary generated: $SUMMARY_JSON${NC}"
        cat "$SUMMARY_JSON" | jq .
        ;;
    csv)
        echo -e "${GREEN}Coverage summary generated: $SUMMARY_CSV${NC}"
        cat "$SUMMARY_CSV"
        ;;
    text|*)
        echo -e "${GREEN}Coverage summary generated: $SUMMARY_TEXT${NC}"
        cat "$SUMMARY_TEXT"
        ;;
esac

echo ""
echo -e "${GREEN}Done!${NC}"

