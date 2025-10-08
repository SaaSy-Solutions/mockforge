#!/bin/bash
# Quick test to verify timing instrumentation

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=== Quick Startup Test ==="
echo ""

# Build just what we need (debug mode for faster build)
echo "Building MockForge (debug)..."
cd "$PROJECT_ROOT"
cargo build --bin mockforge 2>&1 | tail -n 3
echo ""

# Create a simple config with the large spec
cat > /tmp/mockforge_test_config.yaml <<EOF
http:
  enabled: true
  port: 13099
  openapi_spec: $SCRIPT_DIR/large_api_100_endpoints.json

grpc:
  enabled: false

websocket:
  enabled: false
EOF

echo "Starting MockForge with large OpenAPI spec (100 endpoints)..."
echo "Press Ctrl+C to stop"
echo ""

# Run with info level logging to see our timing messages
RUST_LOG=info "$PROJECT_ROOT/target/debug/mockforge" serve --config /tmp/mockforge_test_config.yaml --http-port 13099
