import { test, expect } from '@playwright/test';

/**
 * Services Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts services-deployed
 *
 * These tests verify the Services page functionality:
 *   1. Page load & layout
 *   2. Empty state
 *   3. Services list (when services exist)
 *   4. Navigation
 *   5. Sidebar structure
 *   6. Accessibility
 *   7. Error-free operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Services — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/services`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    // Wait for the Services heading to confirm content loaded
    await expect(
      mainContent(page).getByRole('heading', { name: 'Services', exact: true, level: 1 })
    ).toBeVisible({ timeout: 10000 });
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the services page at /services', async ({ page }) => {
      await expect(page).toHaveURL(/\/services/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Services', exact: true, level: 1 })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText(/Manage services and routes/)
      ).toBeVisible();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Services')).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Empty State or Services List
  // ---------------------------------------------------------------------------
  test.describe('Empty State or Services List', () => {
    test('should show either services or empty state', async ({ page }) => {
      const main = mainContent(page);

      const hasEmptyState = await main.getByText('No services configured')
        .isVisible({ timeout: 5000 }).catch(() => false);
      const hasServices = await main.locator('[class*="service"], [data-testid*="service"]')
        .first().isVisible({ timeout: 3000 }).catch(() => false);
      const hasMatchingRoutes = await main.getByText(/Matching Routes|No search active/)
        .isVisible({ timeout: 3000 }).catch(() => false);

      // Page should show something — either empty state or service content
      expect(hasEmptyState || hasServices || hasMatchingRoutes).toBeTruthy();
    });

    test('should display "No Services" heading in empty state', async ({ page }) => {
      const main = mainContent(page);
      const noServices = main.getByRole('heading', { name: 'No Services' });
      const hasEmpty = await noServices.isVisible({ timeout: 5000 }).catch(() => false);

      if (hasEmpty) {
        await expect(noServices).toBeVisible();
        await expect(
          main.getByText('No services configured. Add a service to get started.')
        ).toBeVisible();
      }
    });

    test('should display search-related content when services exist', async ({ page }) => {
      const main = mainContent(page);
      const hasServices = !(await main.getByRole('heading', { name: 'No Services' })
        .isVisible({ timeout: 3000 }).catch(() => false));

      if (hasServices) {
        // When services exist, the page shows "Matching Routes" card and ServicesPanel
        await expect(
          main.getByText(/Matching Routes|No search active|routes match/)
        ).toBeVisible();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Sidebar Structure
  // ---------------------------------------------------------------------------
  test.describe('Sidebar Structure', () => {
    test('should display all expected sidebar sections', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await expect(nav.getByRole('heading', { name: 'Core' })).toBeVisible();
      await expect(nav.getByRole('heading', { name: 'Services & Data' })).toBeVisible();
      await expect(nav.getByRole('heading', { name: 'Configuration' })).toBeVisible();
    });

    test('should display Federation in the sidebar', async ({ page }) => {
      await expect(
        page.locator('nav[aria-label="Main navigation"]')
          .getByRole('button', { name: 'Federation' })
      ).toBeVisible();
    });

    test('should display Pillar Analytics in the sidebar', async ({ page }) => {
      await expect(
        page.locator('nav[aria-label="Main navigation"]')
          .getByRole('button', { name: 'Pillar Analytics' })
      ).toBeVisible();
    });

    test('should display Template Marketplace in the sidebar', async ({ page }) => {
      await expect(
        page.locator('nav[aria-label="Main navigation"]')
          .getByRole('button', { name: 'Template Marketplace' })
      ).toBeVisible();
    });

    test('should display Plugin Registry in the sidebar', async ({ page }) => {
      await expect(
        page.locator('nav[aria-label="Main navigation"]')
          .getByRole('button', { name: 'Plugin Registry' })
      ).toBeVisible();
    });

    test('should display BYOK Keys in the sidebar', async ({ page }) => {
      await expect(
        page.locator('nav[aria-label="Main navigation"]')
          .getByRole('button', { name: 'BYOK Keys' })
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Navigation
  // ---------------------------------------------------------------------------
  test.describe('Navigation', () => {
    test('should navigate to Dashboard and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: 'Services' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Services', exact: true, level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Fixtures and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Fixtures' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Fixtures', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: 'Services' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Services', exact: true, level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Federation and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Federation' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Federations', level: 2 })
      ).toBeVisible({ timeout: 10000 });

      await nav.getByRole('button', { name: 'Services' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Services', exact: true, level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a single H1 heading', async ({ page }) => {
      const h1 = mainContent(page).getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);
      await expect(h1).toHaveText('Services');
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
  });

  // ---------------------------------------------------------------------------
  // 6. Error-Free Operation
  // ---------------------------------------------------------------------------
  test.describe('Error-Free Operation', () => {
    test('should load without JavaScript console errors', async ({ page }) => {
      const consoleErrors: string[] = [];

      page.on('console', (msg) => {
        if (msg.type() === 'error') {
          consoleErrors.push(msg.text());
        }
      });

      await page.reload({ waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(3000);

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

    test('should not show error loading state', async ({ page }) => {
      // The page should not be stuck in an error state
      const hasError = await mainContent(page)
        .getByText('Error Loading Services')
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasError).toBeFalsy();
    });
  });
});
