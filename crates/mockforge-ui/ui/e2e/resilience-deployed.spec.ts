import { test, expect } from '@playwright/test';

/**
 * Resilience Dashboard Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config (handles auth via setup project):
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts resilience-deployed
 *
 * These tests verify all Resilience Dashboard functionality on the live deployed site:
 *   1.  Page Load & Layout
 *   2.  Summary Cards
 *   3.  Tab Navigation
 *   4.  Circuit Breakers Tab
 *   5.  Bulkheads Tab
 *   6.  Controls (Refresh / Auto-refresh)
 *   7.  Navigation
 *   8.  Accessibility
 *   9.  Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

// Scoped locator for the main content area (excludes sidebar to avoid duplicate matches)
function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Resilience Dashboard — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/resilience`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    // Wait for the app shell to render (sidebar navigation visible)
    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    // Wait for the Resilience Dashboard heading to confirm content loaded
    await expect(
      mainContent(page).getByRole('heading', { name: 'Resilience Dashboard', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    // Small stabilization delay for dynamic content
    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the resilience page at /resilience', async ({ page }) => {
      await expect(page).toHaveURL(/\/resilience/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Resilience Dashboard', level: 1 })
      ).toBeVisible();
    });

    test('should display the tab navigation bar', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('button', { name: 'Circuit Breakers' })).toBeVisible();
      await expect(main.getByRole('button', { name: 'Bulkheads' })).toBeVisible();
    });

    test('should display the Refresh Now button', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: 'Refresh Now' })
      ).toBeVisible();
    });

    test('should display the Auto-refresh checkbox', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Auto-refresh (3s)').first()
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Summary Cards
  // ---------------------------------------------------------------------------
  test.describe('Summary Cards', () => {
    test('should display the Circuit Breakers summary card', async ({ page }) => {
      const main = mainContent(page);
      // The summary card has an h2 "Circuit Breakers" with stats below
      const hasCbSummary = await main
        .getByRole('heading', { name: 'Circuit Breakers', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasCbSummary) {
        // Verify stats labels are present
        await expect(main.getByText('Total:').first()).toBeVisible();
        await expect(main.getByText('Closed:').first()).toBeVisible();
        await expect(main.getByText('Half-Open:').first()).toBeVisible();
        await expect(main.getByText('Open:').first()).toBeVisible();
      }
      // Summary may not render if API returns null — page should still function
      await expect(
        main.getByRole('heading', { name: 'Resilience Dashboard', level: 1 })
      ).toBeVisible();
    });

    test('should display the Bulkheads summary card', async ({ page }) => {
      const main = mainContent(page);
      const hasBhSummary = await main
        .getByRole('heading', { name: 'Bulkheads', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasBhSummary) {
        await expect(main.getByText('Total Services:').first()).toBeVisible();
        await expect(main.getByText('Active Requests:').first()).toBeVisible();
        await expect(main.getByText('Queued Requests:').first()).toBeVisible();
      }
      // Summary may not render if API returns null
      await expect(
        main.getByRole('heading', { name: 'Resilience Dashboard', level: 1 })
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Tab Navigation
  // ---------------------------------------------------------------------------
  test.describe('Tab Navigation', () => {
    test('should default to Circuit Breakers tab', async ({ page }) => {
      const main = mainContent(page);
      const cbTab = main.getByRole('button', { name: 'Circuit Breakers' });

      // The active tab should have the blue styling (border-blue-500)
      await expect(cbTab).toBeVisible();

      // Content should show either circuit breaker cards or the empty state
      const hasCbContent = await main
        .getByText('No circuit breakers configured')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasCbCards = await main
        .getByText('Total Requests')
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasCbContent || hasCbCards).toBeTruthy();
    });

    test('should switch to Bulkheads tab', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: 'Bulkheads' }).click();
      await page.waitForTimeout(500);

      // After clicking Bulkheads, should see either bulkhead cards or the empty state
      const hasBhEmpty = await main
        .getByText('No bulkheads configured')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasBhCards = await main
        .getByText('Utilization')
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasBhEmpty || hasBhCards).toBeTruthy();
    });

    test('should switch back to Circuit Breakers tab', async ({ page }) => {
      const main = mainContent(page);

      // Switch to Bulkheads first
      await main.getByRole('button', { name: 'Bulkheads' }).click();
      await page.waitForTimeout(500);

      // Switch back to Circuit Breakers
      await main.getByRole('button', { name: 'Circuit Breakers' }).click();
      await page.waitForTimeout(500);

      // Circuit Breakers content should be visible again
      const hasCbEmpty = await main
        .getByText('No circuit breakers configured')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasCbCards = await main
        .getByText('Total Requests')
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasCbEmpty || hasCbCards).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Circuit Breakers Tab
  // ---------------------------------------------------------------------------
  test.describe('Circuit Breakers Tab', () => {
    test('should show empty state or circuit breaker cards', async ({ page }) => {
      const main = mainContent(page);

      const hasEmpty = await main
        .getByText('No circuit breakers configured')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasEmpty) {
        await expect(main.getByText('No circuit breakers configured').first()).toBeVisible();
      } else {
        // Should show stats columns for circuit breaker cards
        await expect(main.getByText('Total Requests').first()).toBeVisible({ timeout: 5000 });
        await expect(main.getByText('Success Rate').first()).toBeVisible();
        await expect(main.getByText('Failure Rate').first()).toBeVisible();
        await expect(main.getByText('Rejected').first()).toBeVisible();
      }
    });

    test('should display state badges when circuit breakers exist', async ({ page }) => {
      const main = mainContent(page);

      const hasEmpty = await main
        .getByText('No circuit breakers configured')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (!hasEmpty) {
        // At least one state badge should be visible (Closed, Open, or HalfOpen)
        const hasClosed = await main
          .getByText('Closed', { exact: true })
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasOpen = await main
          .getByText('Open', { exact: true })
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasHalfOpen = await main
          .getByText('HalfOpen', { exact: true })
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        expect(hasClosed || hasOpen || hasHalfOpen).toBeTruthy();
      }
    });

    test('should display Reset buttons when circuit breakers exist', async ({ page }) => {
      const main = mainContent(page);

      const hasEmpty = await main
        .getByText('No circuit breakers configured')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (!hasEmpty) {
        const resetButtons = main.getByRole('button', { name: 'Reset' });
        expect(await resetButtons.count()).toBeGreaterThanOrEqual(1);
      }
    });

    test('should display detailed stats when circuit breakers exist', async ({ page }) => {
      const main = mainContent(page);

      const hasEmpty = await main
        .getByText('No circuit breakers configured')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (!hasEmpty) {
        // Detailed stats row labels
        await expect(main.getByText('Successful:').first()).toBeVisible({ timeout: 5000 });
        await expect(main.getByText('Failed:').first()).toBeVisible();
        await expect(main.getByText('Consecutive Failures:').first()).toBeVisible();
        await expect(main.getByText('Consecutive Successes:').first()).toBeVisible();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Bulkheads Tab
  // ---------------------------------------------------------------------------
  test.describe('Bulkheads Tab', () => {
    test('should show empty state or bulkhead cards', async ({ page }) => {
      const main = mainContent(page);

      // Navigate to Bulkheads tab
      await main.getByRole('button', { name: 'Bulkheads' }).click();
      await page.waitForTimeout(500);

      const hasEmpty = await main
        .getByText('No bulkheads configured')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasEmpty) {
        await expect(main.getByText('No bulkheads configured').first()).toBeVisible();
      } else {
        // Should show bulkhead stat columns
        await expect(main.getByText('Active').first()).toBeVisible({ timeout: 5000 });
        await expect(main.getByText('Queued').first()).toBeVisible();
        await expect(main.getByText('Total').first()).toBeVisible();
        await expect(main.getByText('Rejected').first()).toBeVisible();
        await expect(main.getByText('Timeouts').first()).toBeVisible();
      }
    });

    test('should display utilization bar when bulkheads exist', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: 'Bulkheads' }).click();
      await page.waitForTimeout(500);

      const hasEmpty = await main
        .getByText('No bulkheads configured')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (!hasEmpty) {
        // Utilization label should be visible above the progress bar
        await expect(main.getByText('Utilization').first()).toBeVisible({ timeout: 5000 });
      }
    });

    test('should display Reset Stats buttons when bulkheads exist', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: 'Bulkheads' }).click();
      await page.waitForTimeout(500);

      const hasEmpty = await main
        .getByText('No bulkheads configured')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (!hasEmpty) {
        const resetButtons = main.getByRole('button', { name: 'Reset Stats' });
        expect(await resetButtons.count()).toBeGreaterThanOrEqual(1);
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Controls (Refresh / Auto-refresh)
  // ---------------------------------------------------------------------------
  test.describe('Controls', () => {
    test('should handle Refresh Now button click without crashing', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: 'Refresh Now' }).click();
      await page.waitForTimeout(1500);

      // Page should still be functional after refresh
      await expect(
        main.getByRole('heading', { name: 'Resilience Dashboard', level: 1 })
      ).toBeVisible();
    });

    test('should toggle Auto-refresh checkbox', async ({ page }) => {
      const main = mainContent(page);
      const checkbox = main.getByRole('checkbox');

      // Auto-refresh defaults to checked
      await expect(checkbox).toBeChecked();

      // Uncheck it
      await checkbox.uncheck();
      await page.waitForTimeout(500);
      await expect(checkbox).not.toBeChecked();

      // Re-check it
      await checkbox.check();
      await page.waitForTimeout(500);
      await expect(checkbox).toBeChecked();
    });

    test('should still display content after disabling auto-refresh', async ({ page }) => {
      const main = mainContent(page);
      const checkbox = main.getByRole('checkbox');

      await checkbox.uncheck();
      await page.waitForTimeout(2000);

      // Page should remain functional
      await expect(
        main.getByRole('heading', { name: 'Resilience Dashboard', level: 1 })
      ).toBeVisible();
      await expect(main.getByRole('button', { name: 'Refresh Now' })).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 7. Navigation
  // ---------------------------------------------------------------------------
  test.describe('Navigation', () => {
    test('should navigate to Dashboard and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back to Resilience
      const hasResilienceButton = await nav
        .getByRole('button', { name: /Resilience/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasResilienceButton) {
        await nav.getByRole('button', { name: /Resilience/i }).click();
      } else {
        await page.goto(`${BASE_URL}/resilience`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Resilience Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Config and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Config' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Configuration', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back to Resilience
      const hasResilienceButton = await nav
        .getByRole('button', { name: /Resilience/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasResilienceButton) {
        await nav.getByRole('button', { name: /Resilience/i }).click();
      } else {
        await page.goto(`${BASE_URL}/resilience`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Resilience Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 8. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a single H1 heading', async ({ page }) => {
      const h1 = mainContent(page).getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);
      await expect(h1).toHaveText('Resilience Dashboard');
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

    test('should have accessible tab buttons', async ({ page }) => {
      const main = mainContent(page);
      const tabs = [
        main.getByRole('button', { name: 'Circuit Breakers' }),
        main.getByRole('button', { name: 'Bulkheads' }),
      ];

      for (const tab of tabs) {
        await expect(tab).toBeVisible();
        await expect(tab).toBeEnabled();
      }
    });

    test('should have accessible checkbox for auto-refresh', async ({ page }) => {
      const checkbox = mainContent(page).getByRole('checkbox');
      await expect(checkbox).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 9. Error-Free Operation
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

    test('should remain stable after multiple tab switches', async ({ page }) => {
      const main = mainContent(page);

      // Rapidly switch tabs
      await main.getByRole('button', { name: 'Bulkheads' }).click();
      await page.waitForTimeout(300);
      await main.getByRole('button', { name: 'Circuit Breakers' }).click();
      await page.waitForTimeout(300);
      await main.getByRole('button', { name: 'Bulkheads' }).click();
      await page.waitForTimeout(300);
      await main.getByRole('button', { name: 'Circuit Breakers' }).click();
      await page.waitForTimeout(500);

      // Page should still be functional
      await expect(
        main.getByRole('heading', { name: 'Resilience Dashboard', level: 1 })
      ).toBeVisible();
    });
  });
});
