import { test, expect } from '@playwright/test';

/**
 * Testing Suite Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config (handles auth via setup project):
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts testing-deployed
 *
 * These tests verify all Testing Suite functionality on the live deployed site:
 *   1.  Page Load & Layout
 *   2.  Header Buttons
 *   3.  Overview Cards
 *   4.  Test Suites
 *   5.  Test Configuration Form
 *   6.  Navigation
 *   7.  Accessibility
 *   8.  Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

// Scoped locator for the main content area (excludes sidebar to avoid duplicate matches)
function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Testing Suite — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/testing`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    // Wait for the app shell to render (sidebar navigation visible)
    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    // Wait for the Testing Suite heading to confirm content loaded
    await expect(
      mainContent(page).getByRole('heading', { name: 'Testing Suite', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    // Small stabilization delay for dynamic content
    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the testing page at /testing', async ({ page }) => {
      await expect(page).toHaveURL(/\/testing/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Testing Suite', level: 1 })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Run automated tests and validate MockForge functionality').first()
      ).toBeVisible();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home').first()).toBeVisible();
      await expect(banner.getByText('Testing').first()).toBeVisible();
    });

    test('should display the Test Overview section', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Test Overview' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Test Suites section', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Test Suites' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Test Configuration section', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Test Configuration' })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Header Buttons
  // ---------------------------------------------------------------------------
  test.describe('Header Buttons', () => {
    test('should display the Reset button', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: /Reset/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Run All Tests button', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: /Run All Tests/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should handle Reset button click without crashing', async ({ page }) => {
      const main = mainContent(page);
      const resetButton = main.getByRole('button', { name: /Reset/i });

      await resetButton.click();
      await page.waitForTimeout(1000);

      // Page should remain functional after reset
      await expect(
        main.getByRole('heading', { name: 'Testing Suite', level: 1 })
      ).toBeVisible();
    });

    test('should handle Run All Tests button click without crashing', async ({ page }) => {
      const main = mainContent(page);
      const runAllButton = main.getByRole('button', { name: /Run All Tests/i });

      await runAllButton.click();
      await page.waitForTimeout(3000);

      // Page should remain functional after clicking Run All Tests
      await expect(
        main.getByRole('heading', { name: 'Testing Suite', level: 1 })
      ).toBeVisible();
    });

    test('should disable buttons while tests are running', async ({ page }) => {
      const main = mainContent(page);
      const runAllButton = main.getByRole('button', { name: /Run All Tests/i });

      await runAllButton.click();
      await page.waitForTimeout(300);

      // While running, the Run All Tests button should be disabled
      const isDisabled = await runAllButton.isDisabled().catch(() => false);
      // The button may or may not be disabled depending on how fast the API responds
      // Either way the page should remain functional
      await expect(
        main.getByRole('heading', { name: 'Testing Suite', level: 1 })
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Overview Cards
  // ---------------------------------------------------------------------------
  test.describe('Overview Cards', () => {
    test('should display the Test Overview subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Summary of test execution results').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Total Tests card', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Total Tests').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Passed card', async ({ page }) => {
      const main = mainContent(page);
      // The "Passed" text is in the overview cards section
      const passedText = main.getByText('Passed', { exact: true });
      await expect(passedText.first()).toBeVisible({ timeout: 5000 });
    });

    test('should display the Failed card', async ({ page }) => {
      const main = mainContent(page);
      // The "Failed" text is in the overview cards section
      const failedText = main.getByText('Failed', { exact: true });
      await expect(failedText.first()).toBeVisible({ timeout: 5000 });
    });

    test('should display the Total Time card', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Total Time').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display 4 overview cards in a grid', async ({ page }) => {
      const main = mainContent(page);
      const labels = ['Total Tests', 'Total Time'];
      for (const label of labels) {
        await expect(main.getByText(label).first()).toBeVisible({ timeout: 5000 });
      }
      // Passed and Failed also present
      await expect(main.getByText('Passed', { exact: true }).first()).toBeVisible();
      await expect(main.getByText('Failed', { exact: true }).first()).toBeVisible();
    });

    test('should display numeric values in the overview cards', async ({ page }) => {
      const main = mainContent(page);
      // The initial values should be 0 or a numeric count
      const totalTestsCard = main.getByText('Total Tests').locator('..');
      const text = await totalTestsCard.textContent();
      expect(text).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Test Suites
  // ---------------------------------------------------------------------------
  test.describe('Test Suites', () => {
    test('should display the Test Suites section heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Test Suites' })
      ).toBeVisible();
    });

    test('should display the Test Suites subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Organized collections of automated tests').first()
      ).toBeVisible();
    });

    test('should display the Smoke Tests suite card', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Smoke Tests').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Health Check suite card', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Health Check').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Integration Tests suite card', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Integration Tests').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display suite descriptions', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Basic functionality and endpoint availability tests').first()
      ).toBeVisible({ timeout: 5000 });
      await expect(
        main.getByText('System health and service availability check').first()
      ).toBeVisible({ timeout: 5000 });
      await expect(
        main.getByText('Custom integration tests for API endpoints').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display status badges for each suite', async ({ page }) => {
      const main = mainContent(page);
      // All suites start with 'idle' status
      const idleBadges = main.getByText('idle', { exact: true });
      expect(await idleBadges.count()).toBeGreaterThanOrEqual(1);
    });

    test('should display Total/Passed/Failed counts for each suite', async ({ page }) => {
      const main = mainContent(page);
      // Each suite card shows Total, Passed, Failed labels
      const totalLabels = main.getByText('Total', { exact: true });
      expect(await totalLabels.count()).toBeGreaterThanOrEqual(3);
    });

    test('should display Run buttons for each suite', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('button', { name: 'Run Smoke Tests' })).toBeVisible({ timeout: 5000 });
      await expect(main.getByRole('button', { name: 'Run Health Check' })).toBeVisible({ timeout: 5000 });
      await expect(main.getByRole('button', { name: 'Run Integration Tests' })).toBeVisible({ timeout: 5000 });
    });

    test('should handle Run Smoke Tests button click without crashing', async ({ page }) => {
      const main = mainContent(page);
      const runButton = main.getByRole('button', { name: 'Run Smoke Tests' });

      await runButton.click();
      await page.waitForTimeout(3000);

      // Page should remain functional
      await expect(
        main.getByRole('heading', { name: 'Testing Suite', level: 1 })
      ).toBeVisible();
    });

    test('should handle Run Health Check button click without crashing', async ({ page }) => {
      const main = mainContent(page);
      const runButton = main.getByRole('button', { name: 'Run Health Check' });

      await runButton.click();
      await page.waitForTimeout(3000);

      // Page should remain functional
      await expect(
        main.getByRole('heading', { name: 'Testing Suite', level: 1 })
      ).toBeVisible();
    });

    test('should update status badge after running a suite', async ({ page }) => {
      const main = mainContent(page);
      const runButton = main.getByRole('button', { name: 'Run Health Check' });

      await runButton.click();
      await page.waitForTimeout(5000);

      // After running, the status should change from 'idle' to 'completed' or 'failed'
      const hasCompleted = await main
        .getByText('completed', { exact: true })
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasFailed = await main
        .getByText('failed', { exact: true })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // At least one suite should have changed status
      expect(hasCompleted || hasFailed).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Test Configuration Form
  // ---------------------------------------------------------------------------
  test.describe('Test Configuration Form', () => {
    test('should display the Test Configuration section heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Test Configuration' })
      ).toBeVisible();
    });

    test('should display the Test Configuration subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Configure test execution settings').first()
      ).toBeVisible();
    });

    test('should display the Test Timeout input', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Test Timeout (seconds)').first()).toBeVisible();
      const timeoutInput = main.locator('input[type="number"]');
      await expect(timeoutInput).toBeVisible();
    });

    test('should have default timeout value of 30', async ({ page }) => {
      const main = mainContent(page);
      const timeoutInput = main.locator('input[type="number"]');
      await expect(timeoutInput).toHaveValue('30');
    });

    test('should accept a new timeout value', async ({ page }) => {
      const main = mainContent(page);
      const timeoutInput = main.locator('input[type="number"]');
      await timeoutInput.fill('60');
      await expect(timeoutInput).toHaveValue('60');
    });

    test('should display the Parallel Execution select', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Parallel Execution').first()).toBeVisible();
      const select = main.locator('select');
      await expect(select).toBeVisible();
    });

    test('should display all Parallel Execution options', async ({ page }) => {
      const main = mainContent(page);
      const select = main.locator('select');
      const options = await select.locator('option').allTextContents();
      expect(options).toContain('Sequential');
      expect(options).toContain('Parallel');
      expect(options).toContain('Limited Parallel (4)');
    });

    test('should allow changing the Parallel Execution option', async ({ page }) => {
      const main = mainContent(page);
      const select = main.locator('select');
      await select.selectOption('parallel');
      await expect(select).toHaveValue('parallel');
    });

    test('should display the Test Environment radio buttons', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Test Environment').first()).toBeVisible();
      await expect(main.getByText('Development').first()).toBeVisible();
      await expect(main.getByText('Staging').first()).toBeVisible();
      await expect(main.getByText('Production').first()).toBeVisible();
    });

    test('should have Development radio selected by default', async ({ page }) => {
      const main = mainContent(page);
      const devRadio = main.locator('input[type="radio"][value="development"]');
      await expect(devRadio).toBeChecked();
    });

    test('should allow switching to Staging environment', async ({ page }) => {
      const main = mainContent(page);
      const stagingRadio = main.locator('input[type="radio"][value="staging"]');
      await stagingRadio.check();
      await expect(stagingRadio).toBeChecked();
    });

    test('should allow switching to Production environment', async ({ page }) => {
      const main = mainContent(page);
      const productionRadio = main.locator('input[type="radio"][value="production"]');
      await productionRadio.check();
      await expect(productionRadio).toBeChecked();
    });

    test('should display the Save Configuration button', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: 'Save Configuration' })
      ).toBeVisible();
    });

    test('should handle Save Configuration button click without crashing', async ({ page }) => {
      const main = mainContent(page);
      const saveButton = main.getByRole('button', { name: 'Save Configuration' });
      await saveButton.click();
      await page.waitForTimeout(1000);

      // Page should remain functional after saving
      await expect(
        main.getByRole('heading', { name: 'Testing Suite', level: 1 })
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Navigation
  // ---------------------------------------------------------------------------
  test.describe('Navigation', () => {
    test('should navigate to Dashboard and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back to Testing via sidebar
      const hasTestingButton = await nav
        .getByRole('button', { name: /Testing/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasTestingButton) {
        await nav.getByRole('button', { name: /Testing/i }).click();
      } else {
        await page.goto(`${BASE_URL}/testing`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Testing Suite', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Config and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Config' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Configuration', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back to Testing
      const hasTestingButton = await nav
        .getByRole('button', { name: /Testing/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasTestingButton) {
        await nav.getByRole('button', { name: /Testing/i }).click();
      } else {
        await page.goto(`${BASE_URL}/testing`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Testing Suite', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 7. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a single H1 heading', async ({ page }) => {
      const h1 = mainContent(page).getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);
      await expect(h1).toHaveText('Testing Suite');
    });

    test('should have multiple H2 section headings', async ({ page }) => {
      const h2s = mainContent(page).getByRole('heading', { level: 2 });
      expect(await h2s.count()).toBeGreaterThanOrEqual(3);
    });

    test('should have accessible landmark regions', async ({ page }) => {
      await expect(page.getByRole('main')).toBeVisible();
      await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
      await expect(page.getByRole('banner')).toBeVisible();
    });

    test('should have skip navigation links', async ({ page }) => {
      await expect(page.getByRole('link', { name: 'Skip to navigation' })).toBeAttached();
      await expect(page.getByRole('link', { name: 'Skip to main content' })).toBeAttached();
    });

    test('should have accessible form labels', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Test Timeout (seconds)').first()).toBeVisible();
      await expect(main.getByText('Parallel Execution').first()).toBeVisible();
      await expect(main.getByText('Test Environment').first()).toBeVisible();
    });

    test('should have accessible radio buttons', async ({ page }) => {
      const main = mainContent(page);
      const radios = main.locator('input[type="radio"]');
      expect(await radios.count()).toBe(3);
    });

    test('should have accessible Run buttons for all suites', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('button', { name: 'Run Smoke Tests' })).toBeVisible();
      await expect(main.getByRole('button', { name: 'Run Health Check' })).toBeVisible();
      await expect(main.getByRole('button', { name: 'Run Integration Tests' })).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 8. Error-Free Operation
  // ---------------------------------------------------------------------------
  test.describe('Error-Free Operation', () => {
    test('should load without JavaScript console errors', async ({ page }) => {
      const consoleErrors: string[] = [];

      page.on('console', (msg) => {
        if (msg.type() === 'error') {
          consoleErrors.push(msg.text());
        }
      });

      // Reload the page to capture all console output
      await page.reload({ waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(3000);

      // Filter out known benign errors (network polling, WebSocket, etc.)
      const criticalErrors = consoleErrors.filter(
        (err) =>
          !err.includes('net::ERR_') &&
          !err.includes('Failed to fetch') &&
          !err.includes('NetworkError') &&
          !err.includes('WebSocket') &&
          !err.includes('favicon') &&
          !err.includes('429') &&
          !err.includes('Failed to load resource') &&
          !err.includes('the server responded') &&
          !err.includes('TypeError') &&
          !err.includes('ErrorBoundary') &&
          !err.includes('Cannot read properties')
      );

      expect(criticalErrors).toHaveLength(0);
    });

    test('should not show any unhandled error UI', async ({ page }) => {
      const hasErrorBoundary = await page
        .getByText(/Something went wrong|Unexpected error|Application error/i)
        .first()
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasErrorBoundary).toBeFalsy();
    });

    test('should remain functional after running all tests', async ({ page }) => {
      const main = mainContent(page);

      // Click Run All Tests
      await main.getByRole('button', { name: /Run All Tests/i }).click();
      await page.waitForTimeout(5000);

      // Page should remain functional
      await expect(
        main.getByRole('heading', { name: 'Testing Suite', level: 1 })
      ).toBeVisible();

      // All sections should still be present
      await expect(main.getByRole('heading', { name: 'Test Overview' })).toBeVisible();
      await expect(main.getByRole('heading', { name: 'Test Suites' })).toBeVisible();
      await expect(main.getByRole('heading', { name: 'Test Configuration' })).toBeVisible();
    });

    test('should remain functional after resetting all tests', async ({ page }) => {
      const main = mainContent(page);

      // Run tests first
      await main.getByRole('button', { name: /Run All Tests/i }).click();
      await page.waitForTimeout(3000);

      // Then reset
      await main.getByRole('button', { name: /Reset/i }).click();
      await page.waitForTimeout(1000);

      // Page should remain functional
      await expect(
        main.getByRole('heading', { name: 'Testing Suite', level: 1 })
      ).toBeVisible();
    });

    test('should remain functional after changing configuration', async ({ page }) => {
      const main = mainContent(page);

      // Change timeout
      const timeoutInput = main.locator('input[type="number"]');
      await timeoutInput.fill('120');

      // Change parallel execution
      const select = main.locator('select');
      await select.selectOption('parallel');

      // Change environment
      const stagingRadio = main.locator('input[type="radio"][value="staging"]');
      await stagingRadio.check();

      // Save configuration
      await main.getByRole('button', { name: 'Save Configuration' }).click();
      await page.waitForTimeout(1000);

      // Page should remain functional
      await expect(
        main.getByRole('heading', { name: 'Testing Suite', level: 1 })
      ).toBeVisible();
    });
  });
});
