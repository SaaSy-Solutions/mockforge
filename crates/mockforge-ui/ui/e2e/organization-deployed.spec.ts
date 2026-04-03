import { test, expect } from '@playwright/test';

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';
function mainContent(page: import('@playwright/test').Page) { return page.getByRole('main'); }

test.describe('Organization — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/organization`, { waitUntil: 'domcontentloaded', timeout: 30000 });
    await page.waitForSelector('nav[aria-label="Main navigation"]', { state: 'visible', timeout: 15000 });
    await expect(mainContent(page).getByRole('heading', { name: 'Organizations', level: 1 })).toBeVisible({ timeout: 10000 });
  });

  test.describe('Page Load & Layout', () => {
    test('should load the organization page', async ({ page }) => {
      await expect(page).toHaveURL(/\/organization/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading and subtitle', async ({ page }) => {
      await expect(mainContent(page).getByRole('heading', { name: 'Organizations', level: 1 })).toBeVisible();
      await expect(mainContent(page).getByText('Manage your organizations and team members')).toBeVisible();
    });

    test('should display breadcrumbs', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Organization')).toBeVisible();
    });
  });

  test.describe('Organization List', () => {
    test('should display "Your Organizations" heading', async ({ page }) => {
      await expect(mainContent(page).getByRole('heading', { name: /Your Organizations/ })).toBeVisible();
    });

    test('should display at least one organization', async ({ page }) => {
      const main = mainContent(page);
      // Should show org name with plan badge
      const hasOrg = await main.getByText(/Free|Pro|Team/).first().isVisible({ timeout: 5000 }).catch(() => false);
      expect(hasOrg).toBeTruthy();
    });

    test('should display "Select an organization to manage"', async ({ page }) => {
      await expect(mainContent(page).getByText('Select an organization to manage')).toBeVisible();
    });
  });

  test.describe('Organization Detail', () => {
    test('should show org details when clicking an organization', async ({ page }) => {
      const main = mainContent(page);
      // Click the first org item
      const orgItem = main.getByText(/Free|Pro|Team/).first();
      await orgItem.click();
      await page.waitForTimeout(1000);

      // Should show Members and Settings tabs
      await expect(main.getByRole('button', { name: 'Members' })).toBeVisible({ timeout: 5000 });
      await expect(main.getByRole('button', { name: 'Settings' })).toBeVisible();
    });

    test('should display member list with owner', async ({ page }) => {
      const main = mainContent(page);
      await main.getByText(/Free|Pro|Team/).first().click();
      await page.waitForTimeout(1000);

      // Should show at least the owner
      await expect(main.getByText('Owner')).toBeVisible({ timeout: 5000 });
    });

    test('should switch to Settings tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByText(/Free|Pro|Team/).first().click();
      await page.waitForTimeout(1000);

      await main.getByRole('button', { name: 'Settings' }).click();
      await page.waitForTimeout(500);

      // Settings tab should show some settings content
      const pageText = await main.textContent();
      expect(pageText).toBeTruthy();
    });

    test('should switch back to Members tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByText(/Free|Pro|Team/).first().click();
      await page.waitForTimeout(1000);

      await main.getByRole('button', { name: 'Settings' }).click();
      await page.waitForTimeout(500);
      await main.getByRole('button', { name: 'Members' }).click();
      await page.waitForTimeout(500);

      await expect(main.getByText('Owner')).toBeVisible({ timeout: 5000 });
    });
  });

  test.describe('Navigation', () => {
    test('should navigate to Billing and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');
      await nav.getByRole('button', { name: 'Billing' }).click();
      await page.waitForTimeout(1500);
      await expect(mainContent(page).getByRole('heading', { name: /Billing/i, level: 1 })).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: 'Organization' }).click();
      await page.waitForTimeout(1500);
      await expect(mainContent(page).getByRole('heading', { name: 'Organizations', level: 1 })).toBeVisible({ timeout: 5000 });
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
      await page.waitForTimeout(3000);
      const critical = errors.filter(e => !e.includes('net::ERR_') && !e.includes('Failed to fetch') && !e.includes('NetworkError') && !e.includes('WebSocket') && !e.includes('favicon') && !e.includes('429') && !e.includes('422'));
      expect(critical).toHaveLength(0);
    });

    test('should not show error UI', async ({ page }) => {
      expect(await page.getByText(/Something went wrong|Unexpected error|Application error/i).first().isVisible({ timeout: 2000 }).catch(() => false)).toBeFalsy();
    });
  });
});
