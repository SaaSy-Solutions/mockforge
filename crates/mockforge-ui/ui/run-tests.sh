#!/bin/bash
# Run Playwright E2E tests for MockForge Admin UI

set -e

# Navigate to the UI directory
cd "$(dirname "$0")"

# Set base URL if not already set
export PLAYWRIGHT_BASE_URL=${PLAYWRIGHT_BASE_URL:-http://localhost:5173}

# Run tests
echo "🧪 Running Playwright E2E tests..."
echo "📍 Base URL: $PLAYWRIGHT_BASE_URL"
echo ""

# Run with the chromium project
npx playwright test --project=chromium "$@"

