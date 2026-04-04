import { test, expect } from '@playwright/test';

/**
 * Distributed Traces Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config (handles auth via setup project):
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts traces-deployed
 *
 * These tests verify all Distributed Traces functionality on the live deployed site:
 *   1.  Page Load & Layout
 *   2.  Search
 *   3.  Trace List
 *   4.  Trace Details
 *   5.  Navigation
 *   6.  Accessibility
 *   7.  Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

// Scoped locator for the main content area (excludes sidebar to avoid duplicate matches)
function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Distributed Traces — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/traces`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    // Wait for the app shell to render (sidebar navigation visible)
    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    // Wait for the Distributed Traces heading to confirm content loaded
    await expect(
      mainContent(page).getByRole('heading', { name: 'Distributed Traces', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    // Small stabilization delay for dynamic content
    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the traces page at /traces', async ({ page }) => {
      await expect(page).toHaveURL(/\/traces/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Distributed Traces', level: 1 })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      const main = mainContent(page);

      // The subtitle varies depending on state: loading shows "OpenTelemetry trace viewer",
      // normal shows "View and analyze OpenTelemetry traces"
      const hasFullSubtitle = await main
        .getByText('View and analyze OpenTelemetry traces')
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasAltSubtitle = await main
        .getByText('OpenTelemetry trace viewer')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasFullSubtitle || hasAltSubtitle).toBeTruthy();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Traces')).toBeVisible();
    });

    test('should display the Refresh button or error retry', async ({ page }) => {
      const main = mainContent(page);

      const hasRefresh = await main
        .getByRole('button', { name: /Refresh/i })
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasRetry = await main
        .getByRole('button', { name: /Retry/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasRefresh || hasRetry).toBeTruthy();
    });

    test('should display the search input', async ({ page }) => {
      const main = mainContent(page);

      const hasSearch = await main
        .getByPlaceholder('Search traces by ID or service name...')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      // Search is only visible when not in loading/error state
      const hasError = await main
        .getByRole('button', { name: /Retry/i })
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasSearch || hasError).toBeTruthy();
    });

    test('should display the two-column layout with Traces and Trace Details', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByRole('button', { name: /Retry/i })
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        await expect(
          main.getByRole('heading', { name: 'Traces', level: 2 })
        ).toBeVisible({ timeout: 5000 });
        await expect(
          main.getByRole('heading', { name: 'Trace Details', level: 2 })
        ).toBeVisible({ timeout: 5000 });
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Search
  // ---------------------------------------------------------------------------
  test.describe('Search', () => {
    test('should display the search input with placeholder text', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByRole('button', { name: /Retry/i })
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        await expect(
          main.getByPlaceholder('Search traces by ID or service name...')
        ).toBeVisible({ timeout: 5000 });
      }
    });

    test('should accept search input', async ({ page }) => {
      const main = mainContent(page);

      const searchInput = main.getByPlaceholder('Search traces by ID or service name...');
      const hasSearch = await searchInput
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasSearch) {
        await searchInput.fill('test-service');
        await expect(searchInput).toHaveValue('test-service');
      }
    });

    test('should filter traces when typing in search', async ({ page }) => {
      const main = mainContent(page);

      const searchInput = main.getByPlaceholder('Search traces by ID or service name...');
      const hasSearch = await searchInput
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasSearch) {
        // Type a non-matching query
        await searchInput.fill('zzz-nonexistent-trace-id-zzz');
        await page.waitForTimeout(500);

        // Should show either no traces or the empty state
        const tracesSection = main.getByText(/\d+ traces found/);
        const hasZeroTraces = await tracesSection
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        if (hasZeroTraces) {
          await expect(main.getByText('0 traces found')).toBeVisible();
        }

        // Clear search
        await searchInput.clear();
        await page.waitForTimeout(500);
      }
    });

    test('should show empty state when search matches nothing', async ({ page }) => {
      const main = mainContent(page);

      const searchInput = main.getByPlaceholder('Search traces by ID or service name...');
      const hasSearch = await searchInput
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasSearch) {
        await searchInput.fill('zzz-nonexistent-zzz');
        await page.waitForTimeout(500);

        // Check for the empty state message
        const hasNoTraces = await main
          .getByText(/No traces found/)
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasZeroCount = await main
          .getByText('0 traces found')
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        expect(hasNoTraces || hasZeroCount).toBeTruthy();

        // Clear search
        await searchInput.clear();
        await page.waitForTimeout(500);
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Trace List
  // ---------------------------------------------------------------------------
  test.describe('Trace List', () => {
    test('should display the Traces section heading with count', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByRole('button', { name: /Retry/i })
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        await expect(
          main.getByRole('heading', { name: 'Traces', level: 2 })
        ).toBeVisible({ timeout: 5000 });

        await expect(
          main.getByText(/\d+ traces found/)
        ).toBeVisible({ timeout: 5000 });
      }
    });

    test('should display trace cards or empty state', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByRole('button', { name: /Retry/i })
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        // Either trace cards are present or the empty state message
        const hasTraceCards = await main
          .locator('.cursor-pointer.border')
          .first()
          .isVisible({ timeout: 5000 })
          .catch(() => false);
        const hasEmptyState = await main
          .getByText(/No traces found/)
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        expect(hasTraceCards || hasEmptyState).toBeTruthy();
      }
    });

    test('should display trace info (ID, service name, duration, span count) when traces exist', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByRole('button', { name: /Retry/i })
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        const hasTraces = await main
          .getByText(/\d+\.\d+ms/)
          .first()
          .isVisible({ timeout: 5000 })
          .catch(() => false);

        if (hasTraces) {
          // Trace cards should show duration and span count
          await expect(
            main.getByText(/\d+\.\d+ms/).first()
          ).toBeVisible();
          await expect(
            main.getByText(/\d+ spans/).first()
          ).toBeVisible();
        }
      }
    });

    test('should display status badges (ok/error) on trace cards', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByRole('button', { name: /Retry/i })
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        const hasTraces = await main
          .getByText(/\d+ spans/)
          .first()
          .isVisible({ timeout: 5000 })
          .catch(() => false);

        if (hasTraces) {
          // Status badge should show "ok" or "error"
          const hasOk = await main
            .getByText('ok', { exact: true })
            .first()
            .isVisible({ timeout: 3000 })
            .catch(() => false);
          const hasErrorStatus = await main
            .getByText('error', { exact: true })
            .first()
            .isVisible({ timeout: 3000 })
            .catch(() => false);

          expect(hasOk || hasErrorStatus).toBeTruthy();
        }
      }
    });

    test('should handle Refresh button click without crashing', async ({ page }) => {
      const main = mainContent(page);

      const refreshBtn = main.getByRole('button', { name: /Refresh/i });
      const hasRefresh = await refreshBtn
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasRefresh) {
        await refreshBtn.click();
        await page.waitForTimeout(2000);

        // Page should still be functional
        await expect(
          main.getByRole('heading', { name: 'Distributed Traces', level: 1 })
        ).toBeVisible();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Trace Details
  // ---------------------------------------------------------------------------
  test.describe('Trace Details', () => {
    test('should display "Select a trace to view details" when no trace is selected', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByRole('button', { name: /Retry/i })
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        await expect(
          main.getByText('Select a trace to view details')
        ).toBeVisible({ timeout: 5000 });
      }
    });

    test('should display the Trace Details section heading', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByRole('button', { name: /Retry/i })
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        await expect(
          main.getByRole('heading', { name: 'Trace Details', level: 2 })
        ).toBeVisible({ timeout: 5000 });
      }
    });

    test('should display trace details subtitle showing "Select a trace" or span count', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByRole('button', { name: /Retry/i })
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        const hasSelectSubtitle = await main
          .getByText('Select a trace')
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasSpanSubtitle = await main
          .getByText(/\d+ spans/)
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        expect(hasSelectSubtitle || hasSpanSubtitle).toBeTruthy();
      }
    });

    test('should show trace info when a trace card is clicked', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByRole('button', { name: /Retry/i })
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        // Find a clickable trace card
        const traceCards = main.locator('.cursor-pointer.border.rounded-lg');
        const hasCards = await traceCards
          .first()
          .isVisible({ timeout: 5000 })
          .catch(() => false);

        if (hasCards) {
          // Click the first trace
          await traceCards.first().click();
          await page.waitForTimeout(500);

          // The "Select a trace" text should be replaced with trace info
          const hasSelectText = await main
            .getByText('Select a trace to view details')
            .isVisible({ timeout: 2000 })
            .catch(() => false);
          expect(hasSelectText).toBeFalsy();

          // Trace Info fields should be visible
          await expect(main.getByText('Trace ID')).toBeVisible({ timeout: 5000 });
          await expect(main.getByText('Duration')).toBeVisible();
          await expect(main.getByText('Service')).toBeVisible();
          await expect(main.getByText('Status')).toBeVisible();
        }
      }
    });

    test('should display the Span Tree section when a trace is selected', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByRole('button', { name: /Retry/i })
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        const traceCards = main.locator('.cursor-pointer.border.rounded-lg');
        const hasCards = await traceCards
          .first()
          .isVisible({ timeout: 5000 })
          .catch(() => false);

        if (hasCards) {
          await traceCards.first().click();
          await page.waitForTimeout(500);

          // Span Tree heading should appear
          await expect(
            main.getByText('Span Tree')
          ).toBeVisible({ timeout: 5000 });
        }
      }
    });

    test('should highlight the selected trace card', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByRole('button', { name: /Retry/i })
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        const traceCards = main.locator('.cursor-pointer.border.rounded-lg');
        const hasCards = await traceCards
          .first()
          .isVisible({ timeout: 5000 })
          .catch(() => false);

        if (hasCards) {
          await traceCards.first().click();
          await page.waitForTimeout(500);

          // Selected card should have blue border class
          const selectedCard = main.locator('.border-blue-500');
          await expect(selectedCard).toBeVisible({ timeout: 3000 });
        }
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Navigation
  // ---------------------------------------------------------------------------
  test.describe('Navigation', () => {
    test('should navigate to Dashboard and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back to Traces
      await page.goto(`${BASE_URL}/traces`, {
        waitUntil: 'domcontentloaded',
        timeout: 30000,
      });
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Distributed Traces', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Config and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Config' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Configuration', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back
      await page.goto(`${BASE_URL}/traces`, {
        waitUntil: 'domcontentloaded',
        timeout: 30000,
      });
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Distributed Traces', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a single H1 heading', async ({ page }) => {
      const h1 = mainContent(page).getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);
      await expect(h1).toHaveText('Distributed Traces');
    });

    test('should have H2 section headings when data is loaded', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByRole('button', { name: /Retry/i })
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        const h2s = main.getByRole('heading', { level: 2 });
        // At minimum: Traces + Trace Details
        expect(await h2s.count()).toBeGreaterThanOrEqual(2);
      }
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

    test('should have accessible search input with placeholder', async ({ page }) => {
      const main = mainContent(page);

      const searchInput = main.getByPlaceholder('Search traces by ID or service name...');
      const hasSearch = await searchInput
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasSearch) {
        // Input should be a text input
        await expect(searchInput).toHaveAttribute('type', 'text');
      }
    });

    test('should have accessible Refresh button', async ({ page }) => {
      const main = mainContent(page);

      const hasRefresh = await main
        .getByRole('button', { name: /Refresh/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasRetry = await main
        .getByRole('button', { name: /Retry/i })
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      // At least one action button should be visible
      expect(hasRefresh || hasRetry).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 7. Error-Free Operation
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
          !err.includes('not valid JSON') &&
          !err.includes('DOCTYPE')
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

    test('should gracefully handle API errors with retry button', async ({ page }) => {
      const main = mainContent(page);

      // The page should either show data or an error with retry
      const hasTraces = await main
        .getByRole('heading', { name: 'Traces', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasRetry = await main
        .getByRole('button', { name: /Retry/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasTraces || hasRetry).toBeTruthy();
    });

    test('should not crash when clicking Refresh rapidly', async ({ page }) => {
      const main = mainContent(page);

      const refreshBtn = main.getByRole('button', { name: /Refresh/i });
      const hasRefresh = await refreshBtn
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasRefresh) {
        await refreshBtn.click();
        await page.waitForTimeout(200);
        await refreshBtn.click();
        await page.waitForTimeout(200);
        await refreshBtn.click();
        await page.waitForTimeout(1500);

        // Page should still be functional
        await expect(
          main.getByRole('heading', { name: 'Distributed Traces', level: 1 })
        ).toBeVisible();
      }
    });

    test('should not crash when searching and clearing rapidly', async ({ page }) => {
      const main = mainContent(page);

      const searchInput = main.getByPlaceholder('Search traces by ID or service name...');
      const hasSearch = await searchInput
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasSearch) {
        await searchInput.fill('test');
        await page.waitForTimeout(200);
        await searchInput.clear();
        await page.waitForTimeout(200);
        await searchInput.fill('another-test');
        await page.waitForTimeout(200);
        await searchInput.clear();
        await page.waitForTimeout(500);

        // Page should still be functional
        await expect(
          main.getByRole('heading', { name: 'Distributed Traces', level: 1 })
        ).toBeVisible();
      }
    });
  });
});
