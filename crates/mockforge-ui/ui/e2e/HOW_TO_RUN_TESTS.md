# How to Run E2E Tests

## Prerequisites

1. **Start the dev server** (in one terminal):
   ```bash
   cd crates/mockforge-ui/ui
   npm run dev
   ```

2. **Ensure the backend API is running** on `http://localhost:9080` (or configure your proxy accordingly)

## Running Tests

### Option 1: Using the Helper Script (Recommended)

```bash
cd crates/mockforge-ui/ui
./run-tests.sh
```

Or with specific files:
```bash
./run-tests.sh e2e/dashboard.spec.ts e2e/services.spec.ts
```

### Option 2: Direct Playwright Command

**IMPORTANT: You must be in the `crates/mockforge-ui/ui` directory!**

```bash
cd crates/mockforge-ui/ui
PLAYWRIGHT_BASE_URL=http://localhost:5173 npx playwright test --project=chromium
```

### Option 3: Using npm Script

```bash
cd crates/mockforge-ui/ui
npm run test:e2e
```

(You may need to add this script to `package.json` if it doesn't exist)

## Common Issues

### Error: "Project(s) 'chromium' not found"

**Solution**: Make sure you're in the correct directory:
```bash
cd crates/mockforge-ui/ui
pwd  # Should show: .../mockforge/crates/mockforge-ui/ui
```

### Error: "Cannot connect to http://localhost:5173"

**Solution**: Start the dev server:
```bash
cd crates/mockforge-ui/ui
npm run dev
```

### Tests timing out

The tests have a 60-second timeout per test. If tests consistently timeout:
1. Check that the dev server is responsive
2. Check that the backend API is running
3. Verify network connectivity

## Test Options

### List all tests without running:
```bash
cd crates/mockforge-ui/ui
npx playwright test --list
```

### Run specific test file:
```bash
cd crates/mockforge-ui/ui
npx playwright test --project=chromium e2e/dashboard.spec.ts
```

### Run in headed mode (see browser):
```bash
cd crates/mockforge-ui/ui
npx playwright test --project=chromium --headed
```

### Run with dot reporter (faster output):
```bash
cd crates/mockforge-ui/ui
npx playwright test --project=chromium --reporter=dot
```

### View HTML report:
```bash
cd crates/mockforge-ui/ui
npx playwright show-report
```

## Test Statistics

- **Total Tests**: 238
- **Browser**: Chromium (Firefox/WebKit disabled until navigation is fully stable)
- **Timeout**: 60 seconds per test
- **Expected Duration**: ~15-20 minutes for full suite

## Coverage Collection

To collect code coverage, you need to run the dev server with coverage instrumentation:

```bash
cd crates/mockforge-ui/ui
VITE_CONFIG=vite.config.coverage.ts npm run dev
```

Then in another terminal:
```bash
cd crates/mockforge-ui/ui
COLLECT_COVERAGE=true npx playwright test --project=chromium e2e/coverage-collector.spec.ts
```

