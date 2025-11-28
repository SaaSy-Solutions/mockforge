# Playwright E2E Test Coverage Setup

This document explains how to collect and view code coverage for Playwright E2E tests.

## Overview

Code coverage collection for Playwright tests works by:
1. **Instrumenting** your source code using `vite-plugin-istanbul`
2. **Running** Playwright tests against the instrumented code
3. **Collecting** coverage data from the browser during test execution
4. **Generating** coverage reports using `nyc`

## Quick Start

### Run Tests with Coverage

```bash
npm run test:e2e:coverage
```

This will:
- Start a Vite dev server with coverage instrumentation
- Run all Playwright E2E tests
- Collect coverage data
- Generate coverage reports

### View Coverage Report

After running tests with coverage, open the HTML report:

```bash
open coverage/e2e/index.html
# or
xdg-open coverage/e2e/index.html  # Linux
```

## Manual Setup

If you need to run coverage collection manually:

### 1. Start Dev Server with Coverage

```bash
VITE_CONFIG=vite.config.coverage.ts npm run dev
```

The `vite.config.coverage.ts` file extends the base config and adds the Istanbul plugin to instrument your code.

### 2. Run Playwright Tests

In another terminal:

```bash
PLAYWRIGHT_BASE_URL=http://localhost:5173 npx playwright test
```

### 3. Collect Coverage Data

Coverage data is automatically collected during test execution. You can also use the coverage helpers:

```typescript
import { collectCoverage, saveCoverage } from './e2e/coverage-helpers';

test('my test', async ({ page }) => {
  await page.goto('/');
  // ... your test code ...
  
  // Collect coverage at the end
  const coverage = await collectCoverage(page);
  await saveCoverage(page, 'my-test');
});
```

### 4. Generate Reports

```bash
npm run coverage:report
```

## Configuration

### Coverage Includes/Excludes

Edit `.nycrc.json` to control what files are included in coverage:

```json
{
  "include": ["src/**/*.{ts,tsx}"],
  "exclude": [
    "**/*.test.{ts,tsx}",
    "**/*.spec.{ts,tsx}",
    "**/__tests__/**"
  ]
}
```

### Coverage Thresholds

Set minimum coverage thresholds in `.nycrc.json`:

```json
{
  "check-coverage": true,
  "statements": 80,
  "branches": 75,
  "functions": 80,
  "lines": 80
}
```

## Understanding Coverage Reports

### Coverage Metrics

- **Statements**: Percentage of statements executed
- **Branches**: Percentage of branches (if/else, switch) executed
- **Functions**: Percentage of functions called
- **Lines**: Percentage of lines executed

### Coverage Types

- **File Coverage**: Shows which files were executed
- **Function Coverage**: Shows which functions were called
- **Branch Coverage**: Shows which code paths were taken
- **Line Coverage**: Shows which lines were executed

## Troubleshooting

### No Coverage Data Collected

1. **Check that dev server is using coverage config:**
   ```bash
   VITE_CONFIG=vite.config.coverage.ts npm run dev
   ```

2. **Verify Istanbul plugin is loaded:**
   Check browser console for `__coverage__` object

3. **Check coverage helpers:**
   Make sure `collectCoverage()` is called after page interactions

### Coverage Data Not Merging

If you have multiple coverage files, merge them:

```bash
npm run coverage:merge
```

### Performance Impact

Coverage instrumentation adds overhead. For faster test runs without coverage:

```bash
npm run test:e2e  # No coverage
```

## Integration with CI/CD

To collect coverage in CI:

```yaml
# Example GitHub Actions
- name: Run E2E tests with coverage
  run: |
    npm run test:e2e:coverage
    
- name: Upload coverage
  uses: codecov/codecov-action@v3
  with:
    files: ./coverage/e2e/lcov.info
```

## Best Practices

1. **Run coverage periodically**, not on every test run (performance)
2. **Set reasonable thresholds** based on your codebase maturity
3. **Focus on critical paths** - 100% coverage isn't always necessary
4. **Review uncovered code** to identify untested features
5. **Track coverage trends** over time to ensure it doesn't degrade

## Current Coverage Status

See `COVERAGE_ANALYSIS.md` for detailed coverage breakdown by page and feature.

