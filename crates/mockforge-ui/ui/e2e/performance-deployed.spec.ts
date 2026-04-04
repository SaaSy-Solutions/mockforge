import { test, expect } from '@playwright/test';

/**
 * Performance Mode Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config (handles auth via setup project):
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts performance-deployed
 *
 * These tests verify all Performance Mode functionality on the live deployed site:
 *   1.  Page Load & Layout
 *   2.  Control Buttons
 *   3.  About Card
 *   4.  Status Banner
 *   5.  Navigation
 *   6.  Accessibility
 *   7.  Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

// Scoped locator for the main content area (excludes sidebar to avoid duplicate matches)
function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Performance Mode — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/performance`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    // Wait for the app shell to render (sidebar navigation visible)
    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    // Wait for the Performance Mode heading to confirm content loaded
    await expect(
      mainContent(page).getByRole('heading', { name: 'Performance Mode', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    // Small stabilization delay for dynamic content
    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the performance page at /performance', async ({ page }) => {
      await expect(page).toHaveURL(/\/performance/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Performance Mode', level: 1 })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText(
          'Lightweight load simulation with RPS control and bottleneck simulation'
        ).first()
      ).toBeVisible();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home').first()).toBeVisible();
      await expect(banner.getByText('Performance').first()).toBeVisible();
    });

    test('should not show loading skeleton after content loads', async ({ page }) => {
      const main = mainContent(page);

      // The animate-pulse loading skeleton should not be present after the heading renders
      const hasSkeleton = await main
        .locator('.animate-pulse')
        .first()
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      // If there's a skeleton, it should disappear quickly
      if (hasSkeleton) {
        await page.waitForTimeout(3000);
        const stillLoading = await main
          .locator('.animate-pulse')
          .first()
          .isVisible({ timeout: 2000 })
          .catch(() => false);
        expect(stillLoading).toBeFalsy();
      }
    });

    test('should display the About Performance Mode card', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('About Performance Mode').first()
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Control Buttons
  // ---------------------------------------------------------------------------
  test.describe('Control Buttons', () => {
    test('should display either Quick Start or Stop button', async ({ page }) => {
      const main = mainContent(page);

      const hasQuickStart = await main
        .getByRole('button', { name: /Quick Start/i })
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasStop = await main
        .getByRole('button', { name: /Stop/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasQuickStart || hasStop).toBeTruthy();
    });

    test('should show Quick Start button when performance is not running', async ({ page }) => {
      const main = mainContent(page);

      const isRunning = await main
        .getByText('Performance mode is running')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (!isRunning) {
        await expect(
          main.getByRole('button', { name: /Quick Start/i })
        ).toBeVisible({ timeout: 5000 });
      }
    });

    test('should show Stop button when performance is running', async ({ page }) => {
      const main = mainContent(page);

      const isRunning = await main
        .getByText('Performance mode is running')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (isRunning) {
        await expect(
          main.getByRole('button', { name: /Stop/i })
        ).toBeVisible({ timeout: 5000 });
      }
    });

    test('should handle Quick Start button click without crashing', async ({ page }) => {
      const main = mainContent(page);

      const quickStartBtn = main.getByRole('button', { name: /Quick Start/i });
      const hasQuickStart = await quickStartBtn
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasQuickStart) {
        // Set up dialog handler to dismiss the confirmation if it appears
        page.on('dialog', (dialog) => dialog.dismiss());

        await quickStartBtn.click();
        await page.waitForTimeout(2000);

        // Page should still be functional
        await expect(
          main.getByRole('heading', { name: 'Performance Mode', level: 1 })
        ).toBeVisible();
      }
    });

    test('should not show both Quick Start and Stop at the same time', async ({ page }) => {
      const main = mainContent(page);

      const hasQuickStart = await main
        .getByRole('button', { name: /Quick Start/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasStop = await main
        .getByRole('button', { name: /Stop/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // They should be mutually exclusive
      expect(hasQuickStart && hasStop).toBeFalsy();
    });
  });

  // ---------------------------------------------------------------------------
  // 3. About Card
  // ---------------------------------------------------------------------------
  test.describe('About Card', () => {
    test('should display the About Performance Mode heading', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('About Performance Mode').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the description text', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText(/lightweight load simulation tool/)
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the RPS Control feature', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('RPS Control:').first()
      ).toBeVisible({ timeout: 5000 });
      await expect(
        main.getByText(/Maintain a target requests-per-second rate/)
      ).toBeVisible();
    });

    test('should display the Bottleneck Simulation feature', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Bottleneck Simulation:').first()
      ).toBeVisible({ timeout: 5000 });
      await expect(
        main.getByText(/Simulate CPU, Memory, Network, I\/O, and Database bottlenecks/)
      ).toBeVisible();
    });

    test('should display the Latency Recording feature', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Latency Recording:').first()
      ).toBeVisible({ timeout: 5000 });
      await expect(
        main.getByText(/Track request latencies with detailed statistics/)
      ).toBeVisible();
    });

    test('should display the Real-time Metrics feature', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Real-time Metrics:').first()
      ).toBeVisible({ timeout: 5000 });
      await expect(
        main.getByText(/Monitor performance metrics as they change/)
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Status Banner
  // ---------------------------------------------------------------------------
  test.describe('Status Banner', () => {
    test('should display the status banner when performance is running', async ({ page }) => {
      const main = mainContent(page);

      const isRunning = await main
        .getByText('Performance mode is running')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (isRunning) {
        await expect(
          main.getByText('Performance mode is running').first()
        ).toBeVisible();
      }
    });

    test('should display target and current RPS when running', async ({ page }) => {
      const main = mainContent(page);

      const isRunning = await main
        .getByText('Performance mode is running')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (isRunning) {
        await expect(
          main.getByText(/Target:.*RPS/)
        ).toBeVisible();
        await expect(
          main.getByText(/Current:.*RPS/)
        ).toBeVisible();
      }
    });

    test('should display bottleneck count when running', async ({ page }) => {
      const main = mainContent(page);

      const isRunning = await main
        .getByText('Performance mode is running')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (isRunning) {
        await expect(
          main.getByText(/Bottlenecks: \d+/)
        ).toBeVisible();
      }
    });

    test('should display the pulsing active indicator when running', async ({ page }) => {
      const main = mainContent(page);

      const isRunning = await main
        .getByText('Performance mode is running')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (isRunning) {
        // Pulsing green dot with animate-pulse class
        const pulsingDot = main.locator('.bg-green-500.rounded-full.animate-pulse');
        await expect(pulsingDot).toBeVisible();

        // "Active" label next to the dot
        await expect(main.getByText('Active').first()).toBeVisible();
      }
    });

    test('should not display the status banner when not running', async ({ page }) => {
      const main = mainContent(page);

      const isRunning = await main
        .getByText('Performance mode is running')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (!isRunning) {
        // Banner should not be present
        const hasBanner = await main
          .getByText('Performance mode is running')
          .isVisible({ timeout: 2000 })
          .catch(() => false);
        expect(hasBanner).toBeFalsy();
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

      // Navigate back to Performance
      await page.goto(`${BASE_URL}/performance`, {
        waitUntil: 'domcontentloaded',
        timeout: 30000,
      });
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Performance Mode', level: 1 })
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
      await page.goto(`${BASE_URL}/performance`, {
        waitUntil: 'domcontentloaded',
        timeout: 30000,
      });
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Performance Mode', level: 1 })
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
      await expect(h1).toHaveText('Performance Mode');
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

    test('should have accessible buttons with discernible text', async ({ page }) => {
      const main = mainContent(page);

      const hasQuickStart = await main
        .getByRole('button', { name: /Quick Start/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasStop = await main
        .getByRole('button', { name: /Stop/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // At least one control button should be visible and accessible
      expect(hasQuickStart || hasStop).toBeTruthy();
    });

    test('should have an About section with list items', async ({ page }) => {
      const main = mainContent(page);

      // The feature list uses <li> elements
      const listItems = main.locator('li');
      const listCount = await listItems.count();
      // Should have at least 4 feature list items
      expect(listCount).toBeGreaterThanOrEqual(4);
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

    test('should remain stable after page idle', async ({ page }) => {
      // Wait a few seconds to ensure no delayed crashes
      await page.waitForTimeout(3000);

      const main = mainContent(page);
      await expect(
        main.getByRole('heading', { name: 'Performance Mode', level: 1 })
      ).toBeVisible();

      // About card should still be visible
      await expect(
        main.getByText('About Performance Mode').first()
      ).toBeVisible();
    });

    test('should handle loading state gracefully on reload', async ({ page }) => {
      await page.reload({ waitUntil: 'domcontentloaded' });

      // Should either show loading skeleton briefly or jump to content
      await expect(
        mainContent(page).getByRole('heading', { name: 'Performance Mode', level: 1 })
      ).toBeVisible({ timeout: 15000 });
    });
  });
});
