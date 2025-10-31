# MockForge Admin UI E2E Tests

End-to-end tests for the MockForge Admin UI using Playwright.

## Setup

### Prerequisites

- Node.js and npm installed
- MockForge backend server running (or configured to start automatically)

### Installation

```bash
cd crates/mockforge-ui/ui
npm install
```

This will install Playwright and all dependencies (already included in package.json).

## Running Tests

### Run all tests

```bash
npm run test:e2e
```

### Run tests in UI mode (interactive)

```bash
npm run test:e2e:ui
```

### Run specific test file

```bash
npx playwright test e2e/dashboard.spec.ts
```

### Run tests in headed mode (see browser)

```bash
npx playwright test --headed
```

### Run tests for specific browser

```bash
npx playwright test --project=chromium
npx playwright test --project=firefox
npx playwright test --project=webkit
```

### Run tests in debug mode

```bash
npx playwright test --debug
```

## Configuration

### Base URL

The tests use `http://localhost:5173` by default (Vite dev server). You can override this:

```bash
PLAYWRIGHT_BASE_URL=http://localhost:9080 npm run test:e2e
```

Or set it in your environment:

```bash
export PLAYWRIGHT_BASE_URL=http://localhost:9080
npm run test:e2e
```

### Starting the Backend

Before running tests, ensure the MockForge backend is running:

```bash
# Option 1: Use the dev script (includes UI)
./scripts/dev.sh

# Option 2: Start backend manually
cargo run -p mockforge-cli -- serve --admin-enabled

# Option 3: Start UI dev server separately
cd crates/mockforge-ui/ui
npm run dev
```

## Test Structure

```
e2e/
├── helpers.ts              # Test utilities and helpers
├── dashboard.spec.ts      # Dashboard page tests
├── services.spec.ts       # Services page tests
├── workspaces.spec.ts     # Workspaces page tests
├── logs.spec.ts           # Logs page tests
├── fixtures.spec.ts       # Fixtures page tests
├── import.spec.ts         # Import page tests
└── navigation.spec.ts     # Navigation and layout tests
```

## Writing Tests

### Using Helpers

The `helpers.ts` file provides utilities for common operations:

```typescript
import { waitForAppLoad, navigateToTab, fillField } from './helpers';

test('example', async ({ page }) => {
  await page.goto('/');
  await waitForAppLoad(page);
  await navigateToTab(page, 'Dashboard');
  await fillField(page, 'Name', 'Test Value');
});
```

### Best Practices

1. **Wait for page loads**: Always use `waitForAppLoad()` or `waitForDashboardLoad()` after navigation
2. **Use meaningful selectors**: Prefer data-testid attributes, but fall back to text/class selectors
3. **Handle empty states**: Tests should work even when there's no data
4. **Be resilient**: Tests should handle UI variations gracefully
5. **Clean up**: Don't leave test data in the system

## Debugging

### View Test Report

After running tests, view the HTML report:

```bash
npx playwright show-report
```

### Screenshots and Videos

Failed tests automatically capture screenshots and videos in `test-results/`.

### Debug Mode

Run tests in debug mode to step through:

```bash
npx playwright test --debug
```

### Chrome DevTools

Run tests with Chrome DevTools open:

```bash
npx playwright test --debug --project=chromium
```

## CI/CD

Tests are configured to run in CI environments:

- Retries: 2 retries on CI
- Workers: 1 worker on CI (sequential)
- Reports: HTML + GitHub actions reporter

## Environment Variables

- `PLAYWRIGHT_BASE_URL`: Base URL for the UI (default: `http://localhost:5173`)
- `CI`: Set to `true` in CI environments for appropriate retry/work strategies

## Troubleshooting

### Tests fail to connect

- Ensure the backend server is running
- Check that the base URL is correct
- Verify port 5173 (or your configured port) is accessible

### Tests are flaky

- Increase timeouts in `playwright.config.ts`
- Add more explicit waits
- Check for race conditions in test code

### Tests fail on CI

- Ensure backend is started before tests run
- Check network connectivity
- Review CI logs for detailed error messages

