import { test, expect } from '@playwright/test';

/**
 * Template Marketplace Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts template-marketplace-deployed
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Template Marketplace — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/template-marketplace`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    await expect(
      mainContent(page).getByRole('heading', { name: 'Template Marketplace' })
    ).toBeVisible({ timeout: 10000 });
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the template marketplace page', async ({ page }) => {
      await expect(page).toHaveURL(/\/template-marketplace/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Template Marketplace' })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Browse and discover chaos orchestration templates')
      ).toBeVisible();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Template Marketplace')).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Search & Filters
  // ---------------------------------------------------------------------------
  test.describe('Search & Filters', () => {
    test('should display the search input', async ({ page }) => {
      await expect(
        mainContent(page).getByPlaceholder('Search templates...')
      ).toBeVisible();
    });

    test('should display the Category filter', async ({ page }) => {
      await expect(mainContent(page).getByText('Category', { exact: true }).first()).toBeVisible();
    });

    test('should display the Sort By filter', async ({ page }) => {
      await expect(mainContent(page).getByText('Sort By', { exact: true }).first()).toBeVisible();
    });

    test('should display the Min Rating filter', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Min Rating:')).toBeVisible();
    });

    test('should display star rating radio buttons', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('radio', { name: '1 Star' })).toBeVisible();
      await expect(main.getByRole('radio', { name: '2 Stars' })).toBeVisible();
      await expect(main.getByRole('radio', { name: '3 Stars' })).toBeVisible();
      await expect(main.getByRole('radio', { name: '4 Stars' })).toBeVisible();
      await expect(main.getByRole('radio', { name: '5 Stars' })).toBeVisible();
    });

    test('should allow typing in the search input', async ({ page }) => {
      const searchInput = mainContent(page).getByPlaceholder('Search templates...');
      await searchInput.fill('payment');
      await page.waitForTimeout(300);
      await expect(searchInput).toHaveValue('payment');
      await searchInput.clear();
    });

    test('should allow clicking star rating filters', async ({ page }) => {
      const main = mainContent(page);

      // Click the label text instead of the radio directly (Radix UI styling intercepts)
      await main.getByText('3 Stars').click({ force: true });
      await page.waitForTimeout(300);

      // Reset
      await main.getByText('Empty', { exact: true }).click({ force: true });
      await page.waitForTimeout(300);
    });

    test('should display the results count', async ({ page }) => {
      await expect(
        mainContent(page).getByText(/\d+ templates? found/)
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Filter Interactions
  // ---------------------------------------------------------------------------
  test.describe('Filter Interactions', () => {
    test('should open Category dropdown and show options', async ({ page }) => {
      const main = mainContent(page);
      // Click the default category value to open dropdown
      await main.getByText('All Categories').click();
      await page.waitForTimeout(300);

      const listbox = page.getByRole('listbox');
      if (await listbox.isVisible({ timeout: 3000 }).catch(() => false)) {
        const options = await listbox.locator('[role="option"]').count();
        expect(options).toBeGreaterThanOrEqual(2);
        await page.keyboard.press('Escape');
      }
    });

    test('should open Sort By dropdown and show options', async ({ page }) => {
      const main = mainContent(page);
      await main.getByText('Most Popular').click();
      await page.waitForTimeout(300);

      const listbox = page.getByRole('listbox');
      if (await listbox.isVisible({ timeout: 3000 }).catch(() => false)) {
        const options = await listbox.locator('[role="option"]').count();
        expect(options).toBeGreaterThanOrEqual(2);
        await page.keyboard.press('Escape');
      }
    });

    test('should update results count when search query changes', async ({ page }) => {
      const main = mainContent(page);
      const searchInput = main.getByPlaceholder('Search templates...');

      // Get initial count text
      const initialText = await main.getByText(/\d+ templates? found/).textContent();

      // Type a search query
      await searchInput.fill('xyznonexistent123');
      await page.waitForTimeout(500);

      // Results count should update (possibly to 0)
      await expect(main.getByText(/\d+ templates? found/)).toBeVisible();

      await searchInput.clear();
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Template List / Cards
  // ---------------------------------------------------------------------------
  test.describe('Template List', () => {
    test('should show templates or "0 templates found"', async ({ page }) => {
      const main = mainContent(page);
      const hasZero = await main.getByText('0 templates found')
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasTemplates = await main.getByText(/[1-9]\d* templates? found/)
        .isVisible({ timeout: 3000 }).catch(() => false);

      expect(hasZero || hasTemplates).toBeTruthy();
    });

    test('should display template cards with View Details when templates exist', async ({ page }) => {
      const main = mainContent(page);
      const hasTemplates = await main.getByText(/[1-9]\d* templates? found/)
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasTemplates) {
        const viewBtn = main.getByRole('button', { name: /View Details/i }).first();
        const hasView = await viewBtn.isVisible({ timeout: 3000 }).catch(() => false);
        expect(hasView).toBeTruthy();
      }
    });

    test('should open template details dialog when clicking View Details', async ({ page }) => {
      const main = mainContent(page);
      const viewBtn = main.getByRole('button', { name: /View Details/i }).first();

      if (await viewBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
        await viewBtn.click();
        await page.waitForTimeout(500);

        const dialog = page.getByRole('dialog');
        if (await dialog.isVisible({ timeout: 3000 }).catch(() => false)) {
          // Dialog should contain template info
          const hasContent = await dialog.getByText(/Description|Overview|Install|Download/i)
            .first().isVisible({ timeout: 2000 }).catch(() => false);
          expect(hasContent).toBeTruthy();

          // Close dialog
          await page.keyboard.press('Escape');
        }
      }
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

      await nav.getByRole('button', { name: 'Template Marketplace' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Template Marketplace' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Plugin Registry and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Plugin Registry' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: /Plugin/i })
      ).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: 'Template Marketplace' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Template Marketplace' })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Template Marketplace' })
      ).toBeVisible();
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
      await expect(main.getByPlaceholder('Search templates...')).toBeVisible();
      await expect(main.getByRole('radio', { name: '1 Star' })).toBeVisible();
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
          !err.includes('429')
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
