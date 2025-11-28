# How to Collect E2E Test Coverage

## Quick Start

To collect coverage, you need to:

1. **Stop your current dev server** (if running)

2. **Start dev server with coverage instrumentation:**
   ```bash
   VITE_CONFIG=vite.config.coverage.ts npm run dev
   ```

3. **In another terminal, run the coverage collector test:**
   ```bash
   COLLECT_COVERAGE=true PLAYWRIGHT_BASE_URL=http://localhost:5173 npx playwright test e2e/coverage-collector.spec.ts
   ```

4. **Check coverage data:**
   ```bash
   ls -la coverage/e2e/
   ```

5. **Generate HTML report:**
   ```bash
   npm run coverage:report
   ```

6. **View report:**
   ```bash
   open coverage/e2e/index.html
   ```

## Automated Script

Or use the automated script (starts/stops dev server automatically):

```bash
npm run test:e2e:coverage
```

## Current Status

✅ **Coverage infrastructure is set up:**
- `vite-plugin-istanbul` installed and configured
- Coverage collection helpers created
- Coverage collector test created
- Report generation configured

⚠️ **To get coverage data:**
- Dev server must be running with `VITE_CONFIG=vite.config.coverage.ts`
- Tests must execute code that's been instrumented
- Coverage data is collected via `window.__coverage__` in the browser

## What's Covered

The coverage collector test (`coverage-collector.spec.ts`) visits all 12 accessible pages:
- Dashboard, Services, Chains, Logs, Metrics, Analytics
- Fixtures, Import, Workspaces, Testing, Plugins, Config

This gives you a comprehensive view of which code paths are executed during E2E tests.

