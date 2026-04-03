import { test, expect } from '@playwright/test';

/**
 * Fixtures Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts fixtures-deployed
 *
 * These tests verify the Fixtures page functionality:
 *   1. Page load & layout
 *   2. Filter & Search section
 *   3. Fixtures list / empty state
 *   4. Header action buttons
 *   5. Navigation
 *   6. Accessibility
 *   7. Error-free operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Fixtures — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/fixtures`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    await expect(
      mainContent(page).getByRole('heading', { name: 'Mock Fixtures', level: 1 })
    ).toBeVisible({ timeout: 10000 });
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the fixtures page at /fixtures', async ({ page }) => {
      await expect(page).toHaveURL(/\/fixtures/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Mock Fixtures', level: 1 })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Manage and organize your API response fixtures')
      ).toBeVisible();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Fixtures')).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Header Action Buttons
  // ---------------------------------------------------------------------------
  test.describe('Header Action Buttons', () => {
    test('should display the "New Fixture" button', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: 'New Fixture' })
      ).toBeVisible();
    });

    test('should display the "Refresh" button', async ({ page }) => {
      // There are two Refresh buttons — one in header bar, one in page content
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: 'Refresh' })
      ).toBeVisible();
    });

    test('should handle Refresh button click without errors', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Refresh' }).click();
      await page.waitForTimeout(1500);

      // Page should still be functional
      await expect(
        main.getByRole('heading', { name: 'Mock Fixtures', level: 1 })
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Filter & Search Section
  // ---------------------------------------------------------------------------
  test.describe('Filter & Search Section', () => {
    test('should display the Filter & Search heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Filter & Search', level: 2 })
      ).toBeVisible();
    });

    test('should display the search description', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Find and organize your fixtures')
      ).toBeVisible();
    });

    test('should display the search input', async ({ page }) => {
      await expect(
        mainContent(page).getByPlaceholder('Search by name, path, or route...')
      ).toBeVisible();
    });

    test('should display the HTTP Method filter dropdown', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('HTTP Method')).toBeVisible();
      await expect(main.getByRole('combobox')).toBeVisible();
    });

    test('should have all HTTP methods in the dropdown', async ({ page }) => {
      const dropdown = mainContent(page).getByRole('combobox');
      await expect(dropdown).toBeVisible();

      // Check the options exist
      const options = dropdown.locator('option');
      const optionTexts = await options.allTextContents();
      expect(optionTexts).toContain('All Methods');
      expect(optionTexts).toContain('GET');
      expect(optionTexts).toContain('POST');
      expect(optionTexts).toContain('PUT');
      expect(optionTexts).toContain('DELETE');
      expect(optionTexts).toContain('PATCH');
      expect(optionTexts).toContain('HEAD');
    });

    test('should display the summary section', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Summary')).toBeVisible();
      await expect(main.getByText('Fixtures', { exact: true }).first()).toBeVisible();
      await expect(main.getByText('Total Size')).toBeVisible();
    });

    test('should allow typing in the search input', async ({ page }) => {
      const searchInput = mainContent(page).getByPlaceholder('Search by name, path, or route...');
      await searchInput.fill('/api/users');
      await page.waitForTimeout(300);
      await expect(searchInput).toHaveValue('/api/users');

      // Clear
      await searchInput.clear();
      await expect(searchInput).toHaveValue('');
    });

    test('should allow changing the HTTP method filter', async ({ page }) => {
      const dropdown = mainContent(page).getByRole('combobox');

      await dropdown.selectOption('GET');
      await page.waitForTimeout(300);

      // Reset to All Methods
      await dropdown.selectOption('All Methods');
      await page.waitForTimeout(300);
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Fixtures List / Empty State
  // ---------------------------------------------------------------------------
  test.describe('Fixtures List / Empty State', () => {
    test('should display the fixtures count heading', async ({ page }) => {
      // Shows "Fixtures (N)" where N is the count
      await expect(
        mainContent(page).getByRole('heading', { name: /Fixtures \(\d+\)/, level: 2 })
      ).toBeVisible();
    });

    test('should display the fixtures section subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Your mock response fixtures and templates')
      ).toBeVisible();
    });

    test('should show either fixtures or empty state', async ({ page }) => {
      const main = mainContent(page);
      const hasEmpty = await main.getByText('No fixtures found')
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasFixtures = await main.getByText(/Fixtures \([1-9]/)
        .isVisible({ timeout: 3000 }).catch(() => false);

      expect(hasEmpty || hasFixtures).toBeTruthy();
    });

    test('should display empty state message when no fixtures exist', async ({ page }) => {
      const main = mainContent(page);
      const hasEmpty = await main.getByRole('heading', { name: 'No fixtures found' })
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasEmpty) {
        await expect(
          main.getByText('No fixtures have been created yet. Create your first fixture to get started.')
        ).toBeVisible();
      }
    });

    test('should display "Create First Fixture" button in empty state', async ({ page }) => {
      const main = mainContent(page);
      const hasEmpty = await main.getByRole('heading', { name: 'No fixtures found' })
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasEmpty) {
        await expect(
          main.getByRole('button', { name: 'Create First Fixture' })
        ).toBeVisible();
      }
    });

    test('should display an empty state illustration', async ({ page }) => {
      const main = mainContent(page);
      const hasEmpty = await main.getByRole('heading', { name: 'No fixtures found' })
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasEmpty) {
        const emptySection = main.locator('div').filter({
          has: page.getByRole('heading', { name: 'No fixtures found' }),
        }).first();
        await expect(emptySection.locator('img, svg').first()).toBeVisible();
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

      await nav.getByRole('button', { name: 'Fixtures' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Mock Fixtures', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Services and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Services' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Services', exact: true, level: 1 })
      ).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: 'Fixtures' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Mock Fixtures', level: 1 })
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
      await expect(h1).toHaveText('Mock Fixtures');
    });

    test('should have multiple H2 section headings', async ({ page }) => {
      const h2s = mainContent(page).getByRole('heading', { level: 2 });
      expect(await h2s.count()).toBeGreaterThanOrEqual(2);
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

    test('should have labeled form controls', async ({ page }) => {
      const main = mainContent(page);
      // Search input has label
      await expect(main.getByText('Search Fixtures')).toBeVisible();
      await expect(main.getByPlaceholder('Search by name, path, or route...')).toBeVisible();
      // Method filter has label
      await expect(main.getByText('HTTP Method')).toBeVisible();
      await expect(main.getByRole('combobox')).toBeVisible();
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

      await page.reload({ waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(3000);

      const criticalErrors = consoleErrors.filter(
        (err) =>
          !err.includes('net::ERR_') &&
          !err.includes('Failed to fetch') &&
          !err.includes('NetworkError') &&
          !err.includes('WebSocket') &&
          !err.includes('favicon')
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
  });
});
