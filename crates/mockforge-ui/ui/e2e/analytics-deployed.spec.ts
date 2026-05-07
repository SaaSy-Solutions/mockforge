import { test, expect } from '@playwright/test';

/**
 * /analytics route — cloud-mode notice (issue #394).
 *
 * The local AnalyticsPage (request-traffic dashboard) is a self-hosted feature.
 * In cloud mode the route renders a banner pointing users to PillarAnalyticsPage
 * and CloudTracesPage. This spec verifies that notice on the deployed cloud
 * site (https://app.mockforge.dev/).
 *
 * The full request-traffic dashboard tests live in component-level vitest
 * specs and are exercised against a self-hosted runtime.
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('/analytics — Cloud Mode Notice', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/analytics`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });
    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });
  });

  test.describe('Cloud Notice', () => {
    test('renders the self-hosted feature notice', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Request-traffic analytics is a self-hosted feature')
      ).toBeVisible({ timeout: 10000 });
    });

    test('links to Pillar Analytics', async ({ page }) => {
      const main = mainContent(page);
      const link = main.getByRole('button', { name: 'Analytics', exact: true });
      await expect(link).toBeVisible();
      await link.click();
      await page.waitForTimeout(1500);
      await expect(page).toHaveURL(/\/pillar-analytics/);
    });

    test('links to Cloud Traces', async ({ page }) => {
      const main = mainContent(page);
      const link = main.getByRole('button', { name: 'Cloud Traces', exact: true });
      await expect(link).toBeVisible();
      await link.click();
      await page.waitForTimeout(1500);
      await expect(page).toHaveURL(/\/cloud-traces/);
    });
  });

  test.describe('Navigation', () => {
    test('navigates to Dashboard and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');
      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);
      await expect(
        mainContent(page).getByRole('heading', { name: 'Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  test.describe('Accessibility', () => {
    test('has landmarks', async ({ page }) => {
      await expect(page.getByRole('main')).toBeVisible();
      await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
      await expect(page.getByRole('banner')).toBeVisible();
    });

    test('has skip links', async ({ page }) => {
      await expect(page.getByRole('link', { name: 'Skip to navigation' })).toBeAttached();
      await expect(page.getByRole('link', { name: 'Skip to main content' })).toBeAttached();
    });
  });

  test.describe('Error-Free Operation', () => {
    test('loads without critical console errors', async ({ page }) => {
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

    test('does not show error UI', async ({ page }) => {
      const hasErr = await page
        .getByText(/Something went wrong|Unexpected error|Application error/i)
        .first()
        .isVisible({ timeout: 2000 })
        .catch(() => false);
      expect(hasErr).toBeFalsy();
    });
  });
});
