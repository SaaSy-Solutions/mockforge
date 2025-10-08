#!/bin/bash
# Measure MockForge startup latency with different configurations

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=== MockForge Startup Performance Benchmark ==="
echo "Project root: $PROJECT_ROOT"
echo ""

# Build the project
echo "Building MockForge..."
cd "$PROJECT_ROOT"
cargo build --release 2>&1 | tail -n 5
echo ""

# Function to measure startup time
measure_startup() {
    local test_name=$1
    local args=$2
    local timeout=10

    echo "--- Test: $test_name ---"

    # Start MockForge in background and capture logs
    local log_file=$(mktemp)
    timeout $timeout "$PROJECT_ROOT/target/release/mockforge" serve $args > "$log_file" 2>&1 &
    local pid=$!

    # Wait for startup to complete (look for "listening" in logs)
    local count=0
    while [ $count -lt 50 ]; do
        if grep -q "listening" "$log_file" 2>/dev/null; then
            break
        fi
        sleep 0.1
        count=$((count + 1))
    done

    # Kill the process
    kill $pid 2>/dev/null || true
    wait $pid 2>/dev/null || true

    # Extract timing information
    echo "Startup timing breakdown:"
    grep -E "(took|completed|time:)" "$log_file" | sed 's/^/  /'
    echo ""

    # Calculate total startup time (time from start to first "listening" message)
    local first_log_time=$(grep -m 1 "" "$log_file" | awk '{print $1}')
    local ready_time=$(grep -m 1 "listening" "$log_file" | awk '{print $1}')

    echo "Log file saved to: $log_file"
    echo "---"
    echo ""
}

# Test 1: Baseline (no OpenAPI spec)
echo ""
echo "TEST 1: Baseline - No OpenAPI spec"
measure_startup "Baseline" "--http-port 13000"

# Test 2: Large OpenAPI spec (100 endpoints)
echo ""
echo "TEST 2: Large OpenAPI spec (100 endpoints)"
# First, create a config file
cat > /tmp/mockforge_bench_config.yaml <<EOF
http:
  enabled: true
  port: 13001
  openapi_spec: $SCRIPT_DIR/large_api_100_endpoints.json
  validation:
    enabled: true
    mode: enforce

grpc:
  enabled: false

websocket:
  enabled: false
EOF

measure_startup "100 Endpoints" "--config /tmp/mockforge_bench_config.yaml --http-port 13001"

# Test 3: gRPC with proto files
echo ""
echo "TEST 3: gRPC server with multiple proto files"
cat > /tmp/mockforge_grpc_bench_config.yaml <<EOF
http:
  enabled: false

grpc:
  enabled: true
  port: 15001
  proto_dir: $SCRIPT_DIR/proto
  enable_reflection: true

websocket:
  enabled: false
EOF

measure_startup "gRPC Multi-Proto" "--config /tmp/mockforge_grpc_bench_config.yaml --grpc-port 15001"

# Test 4: Combined (HTTP + gRPC)
echo ""
echo "TEST 4: Combined HTTP (100 endpoints) + gRPC (3 services)"
cat > /tmp/mockforge_combined_bench_config.yaml <<EOF
http:
  enabled: true
  port: 13002
  openapi_spec: $SCRIPT_DIR/large_api_100_endpoints.json

grpc:
  enabled: true
  port: 15002
  proto_dir: $SCRIPT_DIR/proto

websocket:
  enabled: false
EOF

measure_startup "Combined" "--config /tmp/mockforge_combined_bench_config.yaml --http-port 13002 --grpc-port 15002"

echo ""
echo "=== Benchmark Complete ==="
echo ""
echo "Summary:"
echo "- Test 1 (Baseline): Minimal startup with no specs"
echo "- Test 2 (100 Endpoints): HTTP server with large OpenAPI spec"
echo "- Test 3 (gRPC): gRPC server with 3 proto files (~30 methods)"
echo "- Test 4 (Combined): Both HTTP and gRPC servers"
echo ""
echo "Check the timing breakdowns above for detailed performance metrics."
echo "Key metrics to review:"
echo "  - OpenAPI spec loading time"
echo "  - Route registry creation time"
echo "  - Proto file parsing time"
echo "  - Total startup time"
