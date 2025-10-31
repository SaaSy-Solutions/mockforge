#!/bin/bash
# Simple Coverage Collection Script
#
# This script collects coverage by:
# 1. Starting dev server with coverage instrumentation
# 2. Running a single test that collects all coverage data
# 3. Generating coverage reports

set -e

echo "🔍 Collecting E2E test coverage..."

# Clean previous coverage
rm -rf coverage/e2e
mkdir -p coverage/e2e

# Check if dev server is already running
if curl -s http://localhost:5173 > /dev/null 2>&1; then
  echo "⚠️  Dev server already running on port 5173"
  echo "   Please stop it and restart with: VITE_CONFIG=vite.config.coverage.ts npm run dev"
  echo "   Or run this script when no dev server is running"
  exit 1
fi

# Start dev server with coverage
echo "🚀 Starting Vite dev server with coverage..."
VITE_CONFIG=vite.config.coverage.ts npm run dev > /tmp/vite-coverage.log 2>&1 &
DEV_PID=$!

# Wait for server
echo "⏳ Waiting for dev server..."
for i in {1..30}; do
  if curl -s http://localhost:5173 > /dev/null 2>&1; then
    echo "✅ Dev server ready"
    break
  fi
  if [ $i -eq 30 ]; then
    echo "❌ Dev server failed to start"
    kill $DEV_PID 2>/dev/null || true
    exit 1
  fi
  sleep 1
done

# Run a simple script to collect coverage from all pages
echo "🧪 Collecting coverage data..."
node -e "
const { chromium } = require('playwright');
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  await page.goto('http://localhost:5173');
  
  // Wait for app to load
  await page.waitForTimeout(5000);
  
  // Navigate through all pages to collect coverage
  const pages = ['dashboard', 'services', 'chains', 'logs', 'metrics', 'analytics', 
                 'fixtures', 'import', 'workspaces', 'testing', 'plugins', 'config'];
  
  for (const pageName of pages) {
    try {
      await page.evaluate((name) => {
        // Trigger navigation if possible
        const event = new CustomEvent('navigate', { detail: name });
        window.dispatchEvent(event);
      }, pageName);
      await page.waitForTimeout(1000);
    } catch (e) {
      console.warn('Failed to navigate to', pageName);
    }
  }
  
  // Collect final coverage
  const coverage = await page.evaluate(() => {
    return window.__coverage__ || null;
  });
  
  if (coverage) {
    const fs = require('fs');
    fs.writeFileSync('coverage/e2e/playwright-coverage.json', JSON.stringify(coverage, null, 2));
    console.log('✅ Coverage collected:', Object.keys(coverage).length, 'files');
  } else {
    console.warn('⚠️  No coverage data found');
  }
  
  await browser.close();
})();
" || echo "⚠️  Coverage collection script completed with warnings"

# Stop dev server
echo "🛑 Stopping dev server..."
kill $DEV_PID 2>/dev/null || true
wait $DEV_PID 2>/dev/null || true

# Generate report if coverage exists
if [ -f "coverage/e2e/playwright-coverage.json" ]; then
  echo "📊 Generating coverage report..."
  nyc report --reporter=html --reporter=text --temp-dir=coverage/e2e --report-dir=coverage/e2e || true
  echo "✅ Coverage report generated"
  echo "📁 View report: coverage/e2e/index.html"
else
  echo "⚠️  No coverage data collected"
fi

