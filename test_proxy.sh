#!/bin/bash

# Test script for MockForge proxy configuration
# This script tests the proxy functionality by making requests to the configured external APIs

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🚀 Starting MockForge Proxy Tests..."

# Function to test an endpoint
test_proxy_endpoint() {
    local endpoint=$1
    local description=$2
    local expected_content=$3

    echo -n "Testing $description: "

    # Make request with timeout
    response=$(curl -s --max-time 10 "$endpoint" 2>/dev/null)

    if [[ $? -eq 0 ]] && [[ "$response" =~ "$expected_content" ]]; then
        echo -e "${GREEN}✅ PASS${NC}"
        return 0
    else
        echo -e "${RED}❌ FAIL${NC} (Status: $?, Response: $response)"
        return 1
    fi
}

# Wait for MockForge to start
echo "⏳ Checking if MockForge is running..."
max_attempts=30
attempt=1

while [[ $attempt -le $max_attempts ]]; do
    if curl -s --max-time 5 http://localhost:3000/health > /dev/null 2>&1; then
        echo -e "${GREEN}✅ MockForge is running!${NC}"
        break
    fi

    if [[ $attempt -eq 1 ]]; then
        echo "Starting MockForge in background..."
        cargo run --config config.dev.yaml > mockforge.log 2>&1 &
        MOCKFORGE_PID=$!
        echo "MockForge PID: $MOCKFORGE_PID"
    fi

    sleep 2
    ((attempt++))
    echo -n "."
done

if [[ $attempt -gt $max_attempts ]]; then
    echo -e "${RED}❌ Timeout: MockForge failed to start${NC}"
    exit 1
fi

echo "🧪 Running proxy tests..."

# Test 1: Weather API proxy
test_proxy_endpoint \
    "http://localhost:3000/external-api/weather/data/2.5/weather?q=London&appid=dummy_key" \
    "Weather API proxy" \
    "coord"

# Test 2: Maps API proxy
test_proxy_endpoint \
    "http://localhost:3000/external-api/maps/api/directions/json?origin=Paris&destination=London&key=dummy_key" \
    "Maps API proxy" \
    "routes"

# Test 3: Social API proxy
test_proxy_endpoint \
    "http://localhost:3000/external-api/social/v18.0/me?fields=id,name&access_token=dummy_token" \
    "Social API proxy" \
    "error"

# Test 4: Regular non-proxy request (should still work)
test_proxy_endpoint \
    "http://localhost:3000/api/users" \
    "Regular API endpoint" \
    "Not Found"

echo "📋 Test completed!"

# Cleanup
if [[ -n "$MOCKFORGE_PID" ]]; then
    echo "🧹 Cleaning up MockForge process (PID: $MOCKFORGE_PID)..."
    kill $MOCKFORGE_PID 2>/dev/null || true
    wait $MOCKFORGE_PID 2>/dev/null || true
fi

echo "✅ All tests finished."
