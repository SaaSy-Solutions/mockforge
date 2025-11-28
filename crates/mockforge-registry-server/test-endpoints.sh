#!/bin/bash
# API Endpoint Test Script for MockForge Registry Server
# Usage: ./test-endpoints.sh [base_url]

BASE_URL="${1:-http://localhost:8080}"
echo "Testing MockForge Registry Server at $BASE_URL"
echo "=============================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
PASSED=0
FAILED=0

# Test function
test_endpoint() {
    local name="$1"
    local method="$2"
    local endpoint="$3"
    local data="$4"
    local expected_status="${5:-200}"

    echo -n "Testing $name... "

    if [ -n "$data" ]; then
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$BASE_URL$endpoint" \
            -H "Content-Type: application/json" \
            -d "$data" 2>&1)
    else
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$BASE_URL$endpoint" 2>&1)
    fi

    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | sed '$d')

    if [ "$http_code" = "$expected_status" ]; then
        echo -e "${GREEN}✓ PASS${NC} (HTTP $http_code)"
        ((PASSED++))
        return 0
    else
        echo -e "${RED}✗ FAIL${NC} (HTTP $http_code, expected $expected_status)"
        echo "  Response: $body" | head -c 200
        echo ""
        ((FAILED++))
        return 1
    fi
}

# Test authenticated endpoint
test_auth_endpoint() {
    local name="$1"
    local method="$2"
    local endpoint="$3"
    local token="$4"
    local data="$5"
    local expected_status="${6:-200}"

    echo -n "Testing $name... "

    if [ -n "$data" ]; then
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$BASE_URL$endpoint" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $token" \
            -d "$data" 2>&1)
    else
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$BASE_URL$endpoint" \
            -H "Authorization: Bearer $token" 2>&1)
    fi

    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | sed '$d')

    if [ "$http_code" = "$expected_status" ]; then
        echo -e "${GREEN}✓ PASS${NC} (HTTP $http_code)"
        ((PASSED++))
        return 0
    else
        echo -e "${RED}✗ FAIL${NC} (HTTP $http_code, expected $expected_status)"
        echo "  Response: $body" | head -c 200
        echo ""
        ((FAILED++))
        return 1
    fi
}

echo "=== Public Endpoints ==="
test_endpoint "Health Check" "GET" "/health"
test_endpoint "Stats" "GET" "/api/v1/stats"
test_endpoint "Plugin Search" "POST" "/api/v1/plugins/search" '{"query": "", "tags": [], "category": null, "sort": "downloads", "per_page": 10, "page": 0}'

echo ""
echo "=== Authentication ==="
# Register a test user
REGISTER_RESPONSE=$(curl -s -X POST "$BASE_URL/api/v1/auth/register" \
    -H "Content-Type: application/json" \
    -d '{"username": "testuser'$(date +%s)'", "email": "test'$(date +%s)'@example.com", "password": "TestPass123!"}')

TOKEN=$(echo "$REGISTER_RESPONSE" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)

if [ -n "$TOKEN" ]; then
    echo -e "${GREEN}✓ User registration successful${NC}"
    ((PASSED++))

    echo ""
    echo "=== Authenticated Endpoints ==="
    test_auth_endpoint "2FA Status" "GET" "/api/v1/auth/2fa/status" "$TOKEN"
    test_auth_endpoint "API Tokens List" "GET" "/api/v1/api-tokens" "$TOKEN"
    test_auth_endpoint "Usage Stats" "GET" "/api/v1/usage" "$TOKEN"
else
    echo -e "${RED}✗ User registration failed${NC}"
    ((FAILED++))
    TOKEN=""
fi

echo ""
echo "=============================================="
echo -e "Results: ${GREEN}$PASSED passed${NC}, ${RED}$FAILED failed${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    exit 0
else
    exit 1
fi
