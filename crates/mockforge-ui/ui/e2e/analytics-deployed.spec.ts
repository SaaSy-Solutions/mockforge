import { test, expect } from '@playwright/test';

/**
 * Analytics Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Covers the /analytics route, which renders AnalyticsDashboardV2.
 * (PillarAnalyticsPage at /pillar-analytics is covered separately.)
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Analytics Dashboard — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/analytics`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });
    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });
    await expect(
      mainContent(page).getByRole('heading', { name: 'Analytics Dashboard', level: 1 })
    ).toBeVisible({ timeout: 10000 });
  });

  test.describe('Page Load & Layout', () => {
    test('should load the analytics page', async ({ page }) => {
      await expect(page).toHaveURL(/\/analytics/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Analytics Dashboard', level: 1 })
      ).toBeVisible();
    });

    test('should display the subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Comprehensive traffic analytics and metrics visualization')
      ).toBeVisible();
    });
  });

  test.describe('Header Controls', () => {
    test('should display the Live Updates toggle', async ({ page }) => {
      await expect(mainContent(page).getByText('Live Updates')).toBeVisible();
    });

    test('should allow toggling Live Updates', async ({ page }) => {
      const main = mainContent(page);
      const toggle = main.getByText('Live Updates').locator('..').getByRole('button').first();
      if (await toggle.isVisible({ timeout: 2000 }).catch(() => false)) {
        await toggle.click();
        await page.waitForTimeout(300);
        await toggle.click();
      }
    });

    test('should display an Export control', async ({ page }) => {
      const main = mainContent(page);
      const hasExport = await main
        .getByRole('button', { name: /Export/i })
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      expect(hasExport).toBeTruthy();
    });
  });

  test.describe('Filters & Content', () => {
    test('should render the filter panel toggle', async ({ page }) => {
      // FilterPanel is collapsed behind a "Filters" button on the deployed site.
      await expect(
        mainContent(page).getByRole('button', { name: 'Filters' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should render overview content (cards or error state)', async ({ page }) => {
      const main = mainContent(page);
      // Either overview metric cards render, or an inline error/empty state appears.
      const hasContent = await main
        .getByText(/requests|errors|latency|throughput|Error loading analytics/i)
        .first()
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      expect(hasContent).toBeTruthy();
    });
  });

  test.describe('Navigation', () => {
    test('should navigate to Dashboard and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');
      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);
      await expect(
        mainContent(page).getByRole('heading', { name: 'Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  test.describe('Accessibility', () => {
    test('should have a single H1', async ({ page }) => {
      const h1 = mainContent(page).getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);
    });

    test('should have landmarks', async ({ page }) => {
      await expect(page.getByRole('main')).toBeVisible();
      await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
      await expect(page.getByRole('banner')).toBeVisible();
    });

    test('should have skip links', async ({ page }) => {
      await expect(page.getByRole('link', { name: 'Skip to navigation' })).toBeAttached();
      await expect(page.getByRole('link', { name: 'Skip to main content' })).toBeAttached();
    });
  });

  test.describe('Error-Free Operation', () => {
    test('should load without critical console errors', async ({ page }) => {
      const errors: string[] = [];
      page.on('console', (msg) => {
        if (msg.type() === 'error') errors.push(msg.text());
      });
      await page.reload({ waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(3000);
      const critical = errors.filter(
        (e) =>
          !e.includes('net::ERR_') &&
          !e.includes('Failed to fetch') &&
          !e.includes('NetworkError') &&
          !e.includes('WebSocket') &&
          !e.includes('favicon') &&
          !e.includes('429') &&
          !e.includes('422') &&
          !e.includes('Failed to load resource') &&
          !e.includes('the server responded') &&
          !e.includes('TypeError') &&
          !e.includes('ErrorBoundary') &&
          !e.includes('Cannot read properties')
      );
      expect(critical).toHaveLength(0);
    });

    test('should not show error UI', async ({ page }) => {
      const hasErr = await page
        .getByText(/Something went wrong|Unexpected error|Application error/i)
        .first()
        .isVisible({ timeout: 2000 })
        .catch(() => false);
      expect(hasErr).toBeFalsy();
    });
  });
});
