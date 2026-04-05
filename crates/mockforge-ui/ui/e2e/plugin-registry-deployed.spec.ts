import { test, expect } from '@playwright/test';

/**
 * Plugin Registry Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts plugin-registry-deployed
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Plugin Registry — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/plugin-registry`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    await expect(
      mainContent(page).getByRole('heading', { name: 'Plugin Registry' })
    ).toBeVisible({ timeout: 10000 });
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the plugin registry page', async ({ page }) => {
      await expect(page).toHaveURL(/\/plugin-registry/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Plugin Registry' })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Discover and install plugins from the MockForge ecosystem')
      ).toBeVisible();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Plugin Registry')).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Search & Filters
  // ---------------------------------------------------------------------------
  test.describe('Search & Filters', () => {
    test('should display the search input', async ({ page }) => {
      await expect(
        mainContent(page).getByPlaceholder('Search plugins...')
      ).toBeVisible();
    });

    test('should display the Category filter', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Category', { exact: true }).first()
      ).toBeVisible();
    });

    test('should display the Language filter', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Language', { exact: true }).first()
      ).toBeVisible();
    });

    test('should display the Sort By filter', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Sort By', { exact: true }).first()
      ).toBeVisible();
    });

    test('should display the Min Rating filter with star buttons', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Min Rating', { exact: true }).first()).toBeVisible();
      await expect(main.getByRole('radio', { name: '1 Star' })).toBeVisible();
      await expect(main.getByRole('radio', { name: '5 Stars' })).toBeVisible();
    });

    test('should display the Min Security filter', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Min Security', { exact: true }).first()).toBeVisible();
      await expect(main.getByRole('spinbutton')).toBeVisible();
    });

    test('should default Min Security to 0', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('spinbutton')
      ).toHaveValue('0');
    });

    test('should allow typing in the search input', async ({ page }) => {
      const searchInput = mainContent(page).getByPlaceholder('Search plugins...');
      await searchInput.fill('auth-plugin');
      await page.waitForTimeout(300);
      await expect(searchInput).toHaveValue('auth-plugin');
      await searchInput.clear();
    });

    test('should allow clicking star rating filters', async ({ page }) => {
      const main = mainContent(page);
      await main.getByText('4 Stars').click({ force: true });
      await page.waitForTimeout(300);
      await main.getByText('Empty', { exact: true }).click({ force: true });
      await page.waitForTimeout(300);
    });

    test('should allow changing the Min Security value', async ({ page }) => {
      const securityInput = mainContent(page).getByRole('spinbutton');
      await securityInput.fill('5');
      await page.waitForTimeout(300);
      await expect(securityInput).toHaveValue('5');
      await securityInput.fill('0');
    });

    test('should display the results count', async ({ page }) => {
      await expect(
        mainContent(page).getByText(/\d+ plugins? found/)
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Filter Dropdowns
  // ---------------------------------------------------------------------------
  test.describe('Filter Dropdowns', () => {
    test('should open Category dropdown and show options', async ({ page }) => {
      const main = mainContent(page);
      // MUI Select: click the displayed value text to open the dropdown
      await main.getByText('All Categories').click();
      await page.waitForTimeout(300);

      const listbox = page.getByRole('listbox');
      await expect(listbox).toBeVisible({ timeout: 3000 });
      await expect(listbox.getByText('Authentication')).toBeVisible();
      await expect(listbox.getByText('Templates')).toBeVisible();
      await expect(listbox.getByText('Middleware')).toBeVisible();

      await page.keyboard.press('Escape');
    });

    test('should open Language dropdown and show options', async ({ page }) => {
      const main = mainContent(page);
      await main.getByText('All Languages').click();
      await page.waitForTimeout(300);

      const listbox = page.getByRole('listbox');
      await expect(listbox).toBeVisible({ timeout: 3000 });
      await expect(listbox.getByText('Rust')).toBeVisible();
      await expect(listbox.getByText('JavaScript')).toBeVisible();
      await expect(listbox.getByText('TypeScript')).toBeVisible();

      await page.keyboard.press('Escape');
    });

    test('should open Sort By dropdown and show options', async ({ page }) => {
      const main = mainContent(page);
      await main.getByText('Most Popular').click();
      await page.waitForTimeout(300);

      const listbox = page.getByRole('listbox');
      await expect(listbox).toBeVisible({ timeout: 3000 });
      await expect(listbox.getByText('Most Downloaded')).toBeVisible();
      await expect(listbox.getByText('Top Rated')).toBeVisible();
      await expect(listbox.getByText('Recently Updated')).toBeVisible();
      await expect(listbox.getByText('Best Security Score')).toBeVisible();

      await page.keyboard.press('Escape');
    });

    test('should select a category and update results count', async ({ page }) => {
      const main = mainContent(page);
      await main.getByText('All Categories').click();
      await page.waitForTimeout(300);
      await page.getByRole('listbox').getByText('Authentication').click();
      await page.waitForTimeout(500);

      await expect(main.getByText(/\d+ plugins? found/)).toBeVisible();
    });

    test('should select a language and update results count', async ({ page }) => {
      const main = mainContent(page);
      await main.getByText('All Languages').click();
      await page.waitForTimeout(300);
      await page.getByRole('listbox').getByText('Rust').click();
      await page.waitForTimeout(500);

      await expect(main.getByText(/\d+ plugins? found/)).toBeVisible();
    });

    test('should select a sort option and maintain results', async ({ page }) => {
      const main = mainContent(page);
      await main.getByText('Most Popular').click();
      await page.waitForTimeout(300);
      await page.getByRole('listbox').getByText('Top Rated').click();
      await page.waitForTimeout(500);

      await expect(main.getByText(/\d+ plugins? found/)).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Plugin List / Cards
  // ---------------------------------------------------------------------------
  test.describe('Plugin List', () => {
    test('should show plugins or "0 plugins found"', async ({ page }) => {
      const main = mainContent(page);
      const hasZero = await main.getByText('0 plugins found')
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasPlugins = await main.getByText(/[1-9]\d* plugins? found/)
        .isVisible({ timeout: 3000 }).catch(() => false);

      expect(hasZero || hasPlugins).toBeTruthy();
    });

    test('should display plugin cards with expected content when plugins exist', async ({ page }) => {
      const main = mainContent(page);
      const hasPlugins = await main.getByText(/[1-9]\d* plugins? found/)
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasPlugins) {
        // Each plugin card should have name, description, rating, and action buttons
        const firstCard = main.locator('.MuiCard-root').first();
        await expect(firstCard).toBeVisible({ timeout: 3000 });

        // Cards should show category chip, rating stars, and View Details / Install buttons
        const hasViewDetails = await firstCard.getByRole('button', { name: /View Details/i })
          .isVisible({ timeout: 2000 }).catch(() => false);
        const hasInstall = await firstCard.getByRole('button', { name: /Install/i })
          .isVisible({ timeout: 2000 }).catch(() => false);

        expect(hasViewDetails || hasInstall).toBeTruthy();
      }
    });

    test('should display security badge on plugin cards when plugins exist', async ({ page }) => {
      const main = mainContent(page);
      const hasPlugins = await main.getByText(/[1-9]\d* plugins? found/)
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasPlugins) {
        // Security badges use text: Excellent, Good, Fair, or Poor
        const hasBadge = await main.getByText(/Excellent|Good|Fair|Poor/).first()
          .isVisible({ timeout: 3000 }).catch(() => false);
        expect(hasBadge).toBeTruthy();
      }
    });

    test('should show View Details and Install buttons on cards when plugins exist', async ({ page }) => {
      const main = mainContent(page);
      const hasPlugins = await main.getByText(/[1-9]\d* plugins? found/)
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasPlugins) {
        const viewBtn = main.getByRole('button', { name: /View Details/i }).first();
        const installBtn = main.getByRole('button', { name: /Install/i }).first();

        const hasView = await viewBtn.isVisible({ timeout: 2000 }).catch(() => false);
        const hasInstall = await installBtn.isVisible({ timeout: 2000 }).catch(() => false);

        expect(hasView || hasInstall).toBeTruthy();
      }
    });

    test('should open plugin details dialog when clicking View Details', async ({ page }) => {
      const main = mainContent(page);
      const viewBtn = main.getByRole('button', { name: /View Details/i }).first();

      if (await viewBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
        await viewBtn.click();
        await page.waitForTimeout(500);

        // Details dialog should appear with tabs (Overview, Versions, Reviews, Security)
        const dialog = page.getByRole('dialog');
        const hasDialog = await dialog.isVisible({ timeout: 3000 }).catch(() => false);

        if (hasDialog) {
          const hasOverview = await dialog.getByText(/Overview/i).isVisible({ timeout: 2000 }).catch(() => false);
          const hasVersions = await dialog.getByText(/Versions/i).isVisible({ timeout: 2000 }).catch(() => false);
          expect(hasOverview || hasVersions).toBeTruthy();

          // Close dialog
          const closeBtn = dialog.getByRole('button', { name: /Close/i }).first();
          if (await closeBtn.isVisible({ timeout: 1000 }).catch(() => false)) {
            await closeBtn.click();
          } else {
            await page.keyboard.press('Escape');
          }
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

      await nav.getByRole('button', { name: 'Plugin Registry' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Plugin Registry' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Template Marketplace and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Template Marketplace' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Template Marketplace' })
      ).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: 'Plugin Registry' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Plugin Registry' })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Plugin Registry' })
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
      await expect(main.getByPlaceholder('Search plugins...')).toBeVisible();
      await expect(main.getByRole('radio', { name: '1 Star' })).toBeVisible();
      await expect(main.getByRole('spinbutton')).toBeVisible();
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
          !err.includes('favicon') &&
          !err.includes('429') &&
          !err.includes('422')
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
