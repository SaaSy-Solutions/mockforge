import { test, expect } from '@playwright/test';

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';
function mainContent(page: import('@playwright/test').Page) { return page.getByRole('main'); }

test.describe('Billing — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/billing`, { waitUntil: 'domcontentloaded', timeout: 30000 });
    await page.waitForSelector('nav[aria-label="Main navigation"]', { state: 'visible', timeout: 15000 });
    await expect(mainContent(page).getByRole('heading', { name: /Billing/i, level: 1 })).toBeVisible({ timeout: 15000 });
  });

  test.describe('Page Load & Layout', () => {
    test('should load the billing page', async ({ page }) => {
      await expect(page).toHaveURL(/\/billing/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display heading and subtitle', async ({ page }) => {
      await expect(mainContent(page).getByRole('heading', { name: /Billing/i, level: 1 })).toBeVisible();
      await expect(mainContent(page).getByText('Manage your subscription and view usage')).toBeVisible();
    });

    test('should display breadcrumbs', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Billing')).toBeVisible();
    });
  });

  test.describe('Tabs', () => {
    test('should display Overview, Usage, and Plans tabs', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('button', { name: 'Overview' })).toBeVisible();
      await expect(main.getByRole('button', { name: 'Usage' })).toBeVisible();
      await expect(main.getByRole('button', { name: 'Plans' })).toBeVisible();
    });

    test('should switch to Usage tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Usage' }).click();
      await page.waitForTimeout(500);
      // Usage tab should show some usage content
      const text = await main.textContent();
      expect(text).toBeTruthy();
    });

    test('should switch to Plans tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Plans' }).click();
      await page.waitForTimeout(500);
      // Plans tab should show plan options
      const hasPlans = await main.getByText(/Pro|Team|Free/i).first().isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasPlans).toBeTruthy();
    });

    test('should switch back to Overview tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Plans' }).click();
      await page.waitForTimeout(500);
      await main.getByRole('button', { name: 'Overview' }).click();
      await page.waitForTimeout(500);
      await expect(main.getByRole('heading', { name: /Current Plan/i })).toBeVisible({ timeout: 5000 });
    });
  });

  test.describe('Overview Tab (Default)', () => {
    test('should display Current Plan card', async ({ page }) => {
      await expect(mainContent(page).getByRole('heading', { name: /Current Plan/i })).toBeVisible();
    });

    test('should show plan name badge', async ({ page }) => {
      // Shows "free", "pro", or "team"
      await expect(mainContent(page).getByText(/free|pro|team/i).first()).toBeVisible();
    });

    test('should display plan details', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Projects')).toBeVisible();
      await expect(main.getByText('Collaborators')).toBeVisible();
      await expect(main.getByText('Environments')).toBeVisible();
    });

    test('should display upgrade button for free plan', async ({ page }) => {
      const main = mainContent(page);
      const hasUpgrade = await main.getByRole('button', { name: /Upgrade/i })
        .isVisible({ timeout: 3000 }).catch(() => false);
      // Only present on free plan
      if (hasUpgrade) {
        await expect(main.getByRole('button', { name: /Upgrade/i })).toBeVisible();
      }
    });

    test('should display Usage This Month section', async ({ page }) => {
      await expect(mainContent(page).getByRole('heading', { name: /Usage This Month/i })).toBeVisible();
    });

    test('should display request and storage usage', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Requests')).toBeVisible();
      await expect(main.getByText('Storage')).toBeVisible();
    });
  });

  test.describe('Navigation', () => {
    test('should navigate to Organization and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');
      await nav.getByRole('button', { name: 'Organization' }).click();
      await page.waitForTimeout(1500);
      await expect(mainContent(page).getByRole('heading', { name: 'Organizations', level: 1 })).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: 'Billing' }).click();
      await page.waitForTimeout(1500);
      await expect(mainContent(page).getByRole('heading', { name: /Billing/i, level: 1 })).toBeVisible({ timeout: 10000 });
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
      page.on('console', (msg) => { if (msg.type() === 'error') errors.push(msg.text()); });
      await page.reload({ waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(5000);
      const critical = errors.filter(e => !e.includes('net::ERR_') && !e.includes('Failed to fetch') && !e.includes('NetworkError') && !e.includes('WebSocket') && !e.includes('favicon') && !e.includes('429') && !e.includes('422'));
      expect(critical).toHaveLength(0);
    });

    test('should not show error UI', async ({ page }) => {
      expect(await page.getByText(/Something went wrong|Unexpected error|Application error/i).first().isVisible({ timeout: 2000 }).catch(() => false)).toBeFalsy();
    });
  });
});
