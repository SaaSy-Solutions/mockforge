import { test, expect } from '@playwright/test';

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';
function mainContent(page: import('@playwright/test').Page) { return page.getByRole('main'); }

test.describe('Plan & Usage — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/usage`, { waitUntil: 'domcontentloaded', timeout: 30000 });
    await page.waitForSelector('nav[aria-label="Main navigation"]', { state: 'visible', timeout: 15000 });
    await expect(mainContent(page).getByRole('heading', { name: 'Usage Dashboard', level: 1 })).toBeVisible({ timeout: 10000 });
  });

  test.describe('Page Load & Layout', () => {
    test('should load the usage page', async ({ page }) => {
      await expect(page).toHaveURL(/\/usage/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display heading and subtitle', async ({ page }) => {
      await expect(mainContent(page).getByRole('heading', { name: 'Usage Dashboard', level: 1 })).toBeVisible();
      await expect(mainContent(page).getByText("Monitor your organization's usage and limits")).toBeVisible();
    });

    test('should display breadcrumbs', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Plan & Usage')).toBeVisible();
    });
  });

  test.describe('Period & Plan Info', () => {
    test('should display current billing period', async ({ page }) => {
      await expect(
        mainContent(page).getByText(/Current Period:/)
      ).toBeVisible();
    });

    test('should display plan name', async ({ page }) => {
      await expect(
        mainContent(page).getByText(/free|Pro|Team/i).first()
      ).toBeVisible();
    });
  });

  test.describe('Tabs', () => {
    test('should display Current Usage and History tabs', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('button', { name: 'Current Usage' })).toBeVisible();
      await expect(main.getByRole('button', { name: 'History' })).toBeVisible();
    });

    test('should switch to History tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'History' }).click();
      await page.waitForTimeout(500);
      // History tab should show some history content or empty state
      const text = await main.textContent();
      expect(text).toBeTruthy();
    });

    test('should switch back to Current Usage tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'History' }).click();
      await page.waitForTimeout(500);
      await main.getByRole('button', { name: 'Current Usage' }).click();
      await page.waitForTimeout(500);
      await expect(main.getByRole('heading', { name: 'API Requests' })).toBeVisible({ timeout: 5000 });
    });
  });

  test.describe('Current Usage Tab (Default)', () => {
    test('should display API Requests usage card', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('heading', { name: 'API Requests' })).toBeVisible();
      await expect(main.getByText('Monthly request usage')).toBeVisible();
    });

    test('should display usage numbers', async ({ page }) => {
      const main = mainContent(page);
      // Shows "Used" label with numbers
      await expect(main.getByText('Used').first()).toBeVisible();
    });

    test('should display Storage usage card', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('heading', { name: 'Storage' })).toBeVisible();
      await expect(main.getByText('Storage usage')).toBeVisible();
    });

    test('should display Data Egress usage card', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('heading', { name: 'Data Egress' })).toBeVisible();
      await expect(main.getByText('Data transfer usage')).toBeVisible();
    });

    test('should display remaining amounts', async ({ page }) => {
      const main = mainContent(page);
      // Shows "remaining" or "Unlimited" for each metric
      const hasRemaining = await main.getByText(/remaining|Unlimited/).first()
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasRemaining).toBeTruthy();
    });
  });

  test.describe('Navigation', () => {
    test('should navigate to Billing and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');
      await nav.getByRole('button', { name: 'Billing' }).click();
      await page.waitForTimeout(1500);
      await expect(mainContent(page).getByRole('heading', { name: /Billing/i, level: 1 })).toBeVisible({ timeout: 10000 });

      await nav.getByRole('button', { name: 'Plan & Usage' }).click();
      await page.waitForTimeout(1500);
      await expect(mainContent(page).getByRole('heading', { name: 'Usage Dashboard', level: 1 })).toBeVisible({ timeout: 5000 });
    });
  });

  test.describe('Accessibility', () => {
    test('should have a single H1', async ({ page }) => {
      const h1 = mainContent(page).getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);
    });

    test('should have H3 usage card headings', async ({ page }) => {
      const h3s = mainContent(page).getByRole('heading', { level: 3 });
      expect(await h3s.count()).toBeGreaterThanOrEqual(3);
    });

    test('should have landmarks and skip links', async ({ page }) => {
      await expect(page.getByRole('main')).toBeVisible();
      await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
      await expect(page.getByRole('link', { name: 'Skip to navigation' })).toBeAttached();
    });
  });

  test.describe('Error-Free Operation', () => {
    test('should load without critical console errors', async ({ page }) => {
      const errors: string[] = [];
      page.on('console', (msg) => { if (msg.type() === 'error') errors.push(msg.text()); });
      await page.reload({ waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(3000);
      const critical = errors.filter(e => !e.includes('net::ERR_') && !e.includes('Failed to fetch') && !e.includes('NetworkError') && !e.includes('WebSocket') && !e.includes('favicon') && !e.includes('429') && !e.includes('422'));
      expect(critical).toHaveLength(0);
    });

    test('should not show error UI', async ({ page }) => {
      expect(await page.getByText(/Something went wrong|Unexpected error|Application error/i).first().isVisible({ timeout: 2000 }).catch(() => false)).toBeFalsy();
    });
  });
});
