#!/bin/bash
# Time Travel Demo Script
# This script demonstrates MockForge's time travel capabilities

set -e

BASE_URL="http://localhost:3000"
ADMIN_URL="http://localhost:9080"

echo "🕐 MockForge Time Travel Demo"
echo "=============================="
echo ""

# Helper function to print status
print_status() {
    echo "📊 Current Status:"
    curl -s "$ADMIN_URL/__mockforge/time-travel/status" | jq '.'
    echo ""
}

# 1. Check if MockForge is running
echo "1️⃣ Checking if MockForge is running..."
if ! curl -s -f "$ADMIN_URL/__mockforge/health" > /dev/null 2>&1; then
    echo "❌ MockForge is not running!"
    echo "Please start it with: mockforge serve --config examples/time-travel-demo.yaml --admin"
    exit 1
fi
echo "✅ MockForge is running"
echo ""

# 2. Enable time travel at a specific time
echo "2️⃣ Enabling time travel at 2025-01-01 00:00:00 UTC"
curl -s -X POST "$ADMIN_URL/__mockforge/time-travel/enable" \
    -H "Content-Type: application/json" \
    -d '{"time": "2025-01-01T00:00:00Z"}' | jq '.'
echo ""
print_status

# 3. Schedule some responses
echo "3️⃣ Scheduling responses for future times..."

# Schedule a token expiry notification for +30 minutes
echo "  - Scheduling token expiry warning for +30m"
RESPONSE_1=$(curl -s -X POST "$ADMIN_URL/__mockforge/time-travel/schedule" \
    -H "Content-Type: application/json" \
    -d '{
        "trigger_time": "+30m",
        "body": {"event": "token_expiry_warning", "message": "Your token will expire soon"},
        "status": 200,
        "name": "token_warning"
    }')
echo "$RESPONSE_1" | jq '.'

# Schedule a token expiry for +1 hour
echo "  - Scheduling token expiry for +1h"
RESPONSE_2=$(curl -s -X POST "$ADMIN_URL/__mockforge/time-travel/schedule" \
    -H "Content-Type: application/json" \
    -d '{
        "trigger_time": "+1h",
        "body": {"error": "token_expired", "message": "Your authentication token has expired"},
        "status": 401,
        "name": "token_expired"
    }')
echo "$RESPONSE_2" | jq '.'
echo ""

# 4. List scheduled responses
echo "4️⃣ Listing all scheduled responses:"
curl -s "$ADMIN_URL/__mockforge/time-travel/scheduled" | jq '.'
echo ""

# 5. Advance time by 35 minutes
echo "5️⃣ Advancing time by 35 minutes..."
curl -s -X POST "$ADMIN_URL/__mockforge/time-travel/advance" \
    -H "Content-Type: application/json" \
    -d '{"duration": "35m"}' | jq '.'
echo ""
print_status

# 6. Make a request (should trigger the first scheduled response)
echo "6️⃣ Making a request (should trigger token expiry warning)..."
echo "Response:"
curl -s "$BASE_URL/api/test" -w "\nStatus: %{http_code}\n" || echo "(Scheduled response would be returned here)"
echo ""

# 7. Advance time by another 30 minutes
echo "7️⃣ Advancing time by another 30 minutes..."
curl -s -X POST "$ADMIN_URL/__mockforge/time-travel/advance" \
    -H "Content-Type: application/json" \
    -d '{"duration": "30m"}' | jq '.'
echo ""
print_status

# 8. Demonstrate time scale
echo "8️⃣ Demonstrating time scale (2x speed)..."
curl -s -X POST "$ADMIN_URL/__mockforge/time-travel/scale" \
    -H "Content-Type: application/json" \
    -d '{"scale": 2.0}' | jq '.'
echo ""
echo "⏱️  Time is now running at 2x speed"
echo "   (Every real second = 2 virtual seconds)"
echo ""
sleep 3
print_status

# 9. Reset time travel
echo "9️⃣ Resetting time travel (back to real time)..."
curl -s -X POST "$ADMIN_URL/__mockforge/time-travel/reset" | jq '.'
echo ""
print_status

echo "✅ Demo complete!"
echo ""
echo "💡 Try these commands yourself:"
echo "   - Enable time travel:"
echo "     curl -X POST $ADMIN_URL/__mockforge/time-travel/enable -d '{\"time\": \"2025-01-01T00:00:00Z\"}'"
echo ""
echo "   - Advance time by 2 hours:"
echo "     curl -X POST $ADMIN_URL/__mockforge/time-travel/advance -d '{\"duration\": \"2h\"}'"
echo ""
echo "   - Schedule a response:"
echo "     curl -X POST $ADMIN_URL/__mockforge/time-travel/schedule -d '{\"trigger_time\": \"+1h\", \"body\": {\"message\": \"Hello from the future\"}}'"
echo ""
echo "   - Check status:"
echo "     curl $ADMIN_URL/__mockforge/time-travel/status"
