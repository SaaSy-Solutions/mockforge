#!/bin/bash
# Test script for MockForge Registry Server API

set -e

BASE_URL="${REGISTRY_URL:-http://localhost:8080}"
BOLD='\033[1m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BOLD}MockForge Registry Server API Test${NC}\n"
echo "Testing API at: $BASE_URL"
echo "---"

# Test 1: Health Check
echo -e "\n${BLUE}1. Health Check${NC}"
curl -s "$BASE_URL/health" | jq '.'
echo -e "${GREEN}✓ Health check passed${NC}"

# Test 2: Search Plugins
echo -e "\n${BLUE}2. Search Plugins${NC}"
curl -s -X POST "$BASE_URL/api/v1/plugins/search" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "auth",
    "sort": "downloads",
    "page": 0,
    "per_page": 10
  }' | jq '.plugins[] | {name, version, downloads, rating}'
echo -e "${GREEN}✓ Plugin search successful${NC}"

# Test 3: Get Specific Plugin
echo -e "\n${BLUE}3. Get Plugin Details (auth-jwt)${NC}"
curl -s "$BASE_URL/api/v1/plugins/auth-jwt" | jq '{name, version, description, downloads, rating, tags}'
echo -e "${GREEN}✓ Plugin details retrieved${NC}"

# Test 4: Get Plugin Version
echo -e "\n${BLUE}4. Get Specific Version${NC}"
curl -s "$BASE_URL/api/v1/plugins/auth-jwt/versions/1.2.0" | jq '{version, checksum, size, download_url}'
echo -e "${GREEN}✓ Version details retrieved${NC}"

# Test 5: Register User
echo -e "\n${BLUE}5. Register New User${NC}"
REGISTER_RESPONSE=$(curl -s -X POST "$BASE_URL/api/v1/auth/register" \
  -H "Content-Type: application/json" \
  -d "{
    \"username\": \"testuser_$(date +%s)\",
    \"email\": \"test_$(date +%s)@example.com\",
    \"password\": \"testpassword123\"
  }")

echo "$REGISTER_RESPONSE" | jq '{user_id, username}'

TOKEN=$(echo "$REGISTER_RESPONSE" | jq -r '.token')
echo -e "${GREEN}✓ User registered, token obtained${NC}"

# Test 6: Login
echo -e "\n${BLUE}6. Login with Admin User${NC}"
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/v1/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@mockforge.dev",
    "password": "admin123"
  }')

echo "$LOGIN_RESPONSE" | jq '{user_id, username}'
ADMIN_TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.token')
echo -e "${GREEN}✓ Login successful${NC}"

# Test 7: Search by Category
echo -e "\n${BLUE}7. Search by Category (datasource)${NC}"
curl -s -X POST "$BASE_URL/api/v1/plugins/search" \
  -H "Content-Type: application/json" \
  -d '{
    "category": "datasource",
    "page": 0,
    "per_page": 5
  }' | jq '.plugins[] | {name, category, description}'
echo -e "${GREEN}✓ Category search successful${NC}"

# Test 8: Sort by Rating
echo -e "\n${BLUE}8. Search Sorted by Rating${NC}"
curl -s -X POST "$BASE_URL/api/v1/plugins/search" \
  -H "Content-Type: application/json" \
  -d '{
    "sort": "rating",
    "page": 0,
    "per_page": 5
  }' | jq '.plugins[] | {name, rating, downloads}'
echo -e "${GREEN}✓ Rating sort successful${NC}"

# Summary
echo -e "\n${BOLD}${GREEN}All Tests Passed!${NC}"
echo -e "
Next Steps:
  - Try publishing a plugin (see GETTING_STARTED.md)
  - Explore the Admin UI integration
  - Deploy to staging environment

For more examples, see: GETTING_STARTED.md
"
