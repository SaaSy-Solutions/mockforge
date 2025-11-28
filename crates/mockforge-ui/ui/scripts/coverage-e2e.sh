#!/bin/bash
# E2E Test Coverage Collection Script
#
# This script runs Playwright E2E tests with code coverage collection enabled.
# It uses vite-plugin-istanbul to instrument the code and collects coverage
# data during test execution.

set -e

echo "🔍 Starting E2E test coverage collection..."

# Clean previous coverage data
echo "🧹 Cleaning previous coverage data..."
rm -rf coverage/e2e
rm -rf coverage/.nyc_output
mkdir -p coverage/e2e

# Start dev server with coverage instrumentation
echo "🚀 Starting Vite dev server with coverage instrumentation..."
VITE_CONFIG=vite.config.coverage.ts npm run dev > /dev/null 2>&1 &
DEV_SERVER_PID=$!

# Wait for dev server to be ready
echo "⏳ Waiting for dev server to be ready..."
sleep 5

# Check if dev server is running
if ! kill -0 $DEV_SERVER_PID 2>/dev/null; then
  echo "❌ Failed to start dev server"
  exit 1
fi

# Run Playwright tests with coverage collection enabled
echo "🧪 Running Playwright E2E tests with coverage..."
COLLECT_COVERAGE=true PLAYWRIGHT_BASE_URL=http://localhost:5173 npx playwright test --config=playwright.config.ts || TEST_EXIT_CODE=$?

# Stop dev server
echo "🛑 Stopping dev server..."
kill $DEV_SERVER_PID 2>/dev/null || true

# Process coverage data if tests completed
if [ -z "$TEST_EXIT_CODE" ] || [ "$TEST_EXIT_CODE" -eq 0 ]; then
  echo "📊 Processing coverage data..."
  
  # Merge coverage files if they exist
  if [ -d "coverage/e2e" ] && [ "$(ls -A coverage/e2e/*.json 2>/dev/null)" ]; then
    echo "✅ Coverage data collected successfully"
    echo "📁 Coverage reports available in: coverage/e2e/"
    echo "🌐 Open coverage/e2e/index.html in your browser to view the report"
  else
    echo "⚠️  No coverage data found. Make sure vite-plugin-istanbul is working correctly."
  fi
fi

exit ${TEST_EXIT_CODE:-0}

