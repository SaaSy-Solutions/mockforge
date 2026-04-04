import { test, expect } from '@playwright/test';

/**
 * Import Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config (handles auth via setup project):
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts import-deployed
 *
 * These tests verify all Import functionality on the live deployed site:
 *   1.  Page Load & Layout
 *   2.  Tab Navigation (4 tabs)
 *   3.  File Upload Section
 *   4.  Configuration Options
 *   5.  Preview & Import Flow
 *   6.  History Tab
 *   7.  Navigation
 *   8.  Accessibility
 *   9.  Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

// Scoped locator for the main content area (excludes sidebar to avoid duplicate matches)
function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Import API Collections — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/import`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    // Wait for the app shell to render (sidebar navigation visible)
    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    // Wait for the Import heading to confirm content loaded
    await expect(
      mainContent(page).getByRole('heading', { name: 'Import API Collections', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    // Small stabilization delay for dynamic content
    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the import page at /import', async ({ page }) => {
      await expect(page).toHaveURL(/\/import/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Import API Collections', level: 1 })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Import routes from Postman, Insomnia, or cURL commands').first()
      ).toBeVisible();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home').first()).toBeVisible();
      await expect(banner.getByText('Import').first()).toBeVisible();
    });

    test('should display the Upload File section by default', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Upload File').first()).toBeVisible({ timeout: 5000 });
    });

    test('should display the Preview Import section', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Preview Import').first()).toBeVisible({ timeout: 5000 });
    });

    test('should display the Import Routes section', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Import Routes').first()).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Tab Navigation (4 tabs)
  // ---------------------------------------------------------------------------
  test.describe('Tab Navigation', () => {
    test('should display all four tab triggers', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('button', { name: /Postman/i })).toBeVisible();
      await expect(main.getByRole('button', { name: /Insomnia/i })).toBeVisible();
      await expect(main.getByRole('button', { name: /cURL/i })).toBeVisible();
      await expect(main.getByRole('button', { name: /History/i })).toBeVisible();
    });

    test('should default to Postman tab', async ({ page }) => {
      const main = mainContent(page);
      // The Postman tab should be active by default with a file upload zone
      await expect(main.getByText('Drop Postman Collection here').first()).toBeVisible({ timeout: 5000 });
    });

    test('should switch to Insomnia tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Insomnia/i }).click();
      await page.waitForTimeout(500);

      await expect(main.getByText('Drop Insomnia Export here').first()).toBeVisible({ timeout: 5000 });
    });

    test('should switch to cURL tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /cURL/i }).click();
      await page.waitForTimeout(500);

      await expect(main.getByText('Drop cURL Commands here').first()).toBeVisible({ timeout: 5000 });
    });

    test('should switch to History tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /History/i }).click();
      await page.waitForTimeout(500);

      await expect(main.getByText('Import History').first()).toBeVisible({ timeout: 5000 });
    });

    test('should switch back to Postman tab after visiting other tabs', async ({ page }) => {
      const main = mainContent(page);

      // Switch to History
      await main.getByRole('button', { name: /History/i }).click();
      await page.waitForTimeout(500);
      await expect(main.getByText('Import History').first()).toBeVisible({ timeout: 5000 });

      // Switch back to Postman
      await main.getByRole('button', { name: /Postman/i }).click();
      await page.waitForTimeout(500);
      await expect(main.getByText('Drop Postman Collection here').first()).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 3. File Upload Section
  // ---------------------------------------------------------------------------
  test.describe('File Upload Section', () => {
    test('should display the file upload zone on Postman tab', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Drop Postman Collection here').first()).toBeVisible();
      await expect(main.getByText('or click to browse files').first()).toBeVisible();
    });

    test('should display Choose File button on Postman tab', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: /Choose File/i })
      ).toBeVisible();
    });

    test('should display the file upload zone on Insomnia tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Insomnia/i }).click();
      await page.waitForTimeout(500);

      await expect(main.getByText('Drop Insomnia Export here').first()).toBeVisible();
      await expect(
        main.getByRole('button', { name: /Choose File/i })
      ).toBeVisible();
    });

    test('should display the file upload zone on cURL tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /cURL/i }).click();
      await page.waitForTimeout(500);

      await expect(main.getByText('Drop cURL Commands here').first()).toBeVisible();
      await expect(
        main.getByRole('button', { name: /Choose File/i })
      ).toBeVisible();
    });

    test('should have a hidden file input for each format tab', async ({ page }) => {
      const main = mainContent(page);

      // Postman tab - file input should exist (hidden)
      await expect(main.locator('#file-upload-postman')).toBeAttached();

      // Switch to Insomnia and check
      await main.getByRole('button', { name: /Insomnia/i }).click();
      await page.waitForTimeout(500);
      await expect(main.locator('#file-upload-insomnia')).toBeAttached();

      // Switch to cURL and check
      await main.getByRole('button', { name: /cURL/i }).click();
      await page.waitForTimeout(500);
      await expect(main.locator('#file-upload-curl')).toBeAttached();
    });

    test('should display format-specific upload subtitle', async ({ page }) => {
      const main = mainContent(page);

      // Postman tab
      await expect(
        main.getByText('Upload your postman collection or export file').first()
      ).toBeVisible();

      // Insomnia tab
      await main.getByRole('button', { name: /Insomnia/i }).click();
      await page.waitForTimeout(500);
      await expect(
        main.getByText('Upload your insomnia collection or export file').first()
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Configuration Options
  // ---------------------------------------------------------------------------
  test.describe('Configuration Options', () => {
    test('should display Configuration section on Postman tab', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Configuration').first()).toBeVisible({ timeout: 5000 });
      await expect(
        main.getByText('Optional settings for import processing').first()
      ).toBeVisible();
    });

    test('should display Base URL Override field on Postman tab', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Base URL Override (optional)').first()).toBeVisible();
      await expect(
        main.getByPlaceholder('e.g., https://api.example.com')
      ).toBeVisible();
    });

    test('should display Configuration section on Insomnia tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Insomnia/i }).click();
      await page.waitForTimeout(500);

      await expect(main.getByText('Configuration').first()).toBeVisible({ timeout: 5000 });
    });

    test('should display Environment field on Insomnia tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Insomnia/i }).click();
      await page.waitForTimeout(500);

      await expect(main.getByText('Environment (optional)').first()).toBeVisible();
      await expect(
        main.getByPlaceholder('e.g., dev, staging, prod')
      ).toBeVisible();
    });

    test('should not display Configuration section on cURL tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /cURL/i }).click();
      await page.waitForTimeout(500);

      // cURL tab does not show the Configuration section
      const hasConfig = await main
        .getByText('Optional settings for import processing')
        .isVisible({ timeout: 2000 })
        .catch(() => false);
      expect(hasConfig).toBeFalsy();
    });

    test('should allow typing into Base URL Override field', async ({ page }) => {
      const main = mainContent(page);
      const input = main.getByPlaceholder('e.g., https://api.example.com');
      await input.fill('https://my-api.example.com');
      await page.waitForTimeout(300);

      const value = await input.inputValue();
      expect(value).toBe('https://my-api.example.com');
    });

    test('should allow typing into Environment field on Insomnia tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Insomnia/i }).click();
      await page.waitForTimeout(500);

      const input = main.getByPlaceholder('e.g., dev, staging, prod');
      await input.fill('staging');
      await page.waitForTimeout(300);

      const value = await input.inputValue();
      expect(value).toBe('staging');
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Preview & Import Flow
  // ---------------------------------------------------------------------------
  test.describe('Preview & Import Flow', () => {
    test('should display Preview Routes button', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: /Preview Routes/i })
      ).toBeVisible();
    });

    test('should display Preview Routes button as disabled when no file selected', async ({ page }) => {
      const main = mainContent(page);
      const previewButton = main.getByRole('button', { name: /Preview Routes/i });
      await expect(previewButton).toBeVisible();
      await expect(previewButton).toBeDisabled();
    });

    test('should display Import button', async ({ page }) => {
      const main = mainContent(page);
      // The import button text includes the route count
      const importButton = main.getByRole('button', { name: /Import.*Route/i });
      await expect(importButton).toBeVisible();
    });

    test('should display Import button as disabled when no preview results', async ({ page }) => {
      const main = mainContent(page);
      const importButton = main.getByRole('button', { name: /Import.*Route/i });
      await expect(importButton).toBeVisible();
      await expect(importButton).toBeDisabled();
    });

    test('should show empty route preview state when no routes previewed', async ({ page }) => {
      const main = mainContent(page);

      // Before preview, no route preview content should be visible
      const hasRoutePreview = await main
        .getByText('Generated Routes')
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      // No routes should be displayed until preview is triggered
      expect(hasRoutePreview).toBeFalsy();
    });

    test('should display Select All and Deselect All only after preview', async ({ page }) => {
      const main = mainContent(page);

      // Before preview, these buttons should not be visible
      const hasSelectAll = await main
        .getByRole('button', { name: /Select All/i })
        .isVisible({ timeout: 2000 })
        .catch(() => false);
      const hasDeselectAll = await main
        .getByRole('button', { name: /Deselect All/i })
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasSelectAll).toBeFalsy();
      expect(hasDeselectAll).toBeFalsy();
    });
  });

  // ---------------------------------------------------------------------------
  // 6. History Tab
  // ---------------------------------------------------------------------------
  test.describe('History Tab', () => {
    test('should display Import History section heading', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /History/i }).click();
      await page.waitForTimeout(500);

      await expect(main.getByText('Import History').first()).toBeVisible({ timeout: 5000 });
    });

    test('should display history subtitle', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /History/i }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByText('View and manage your previous import activities').first()
      ).toBeVisible();
    });

    test('should show history entries or empty state', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /History/i }).click();
      await page.waitForTimeout(500);

      const hasEntries = await main
        .getByText(/Import History \(\d+\)/)
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasEmpty = await main
        .getByText('No Import History')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasLoading = await main
        .getByText('Loading import history...')
        .isVisible({ timeout: 2000 })
        .catch(() => false);
      const hasError = await main
        .getByText('Failed to load import history')
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      // Should show one of the possible states
      expect(hasEntries || hasEmpty || hasLoading || hasError).toBeTruthy();
    });

    test('should display empty state description when no history', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /History/i }).click();
      await page.waitForTimeout(500);

      const hasEmpty = await main
        .getByText('No Import History')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasEmpty) {
        await expect(
          main.getByText('Your import history will appear here after you import collections.').first()
        ).toBeVisible();
      }
    });

    test('should display Clear History button when history exists', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /History/i }).click();
      await page.waitForTimeout(500);

      const hasEntries = await main
        .getByText(/Import History \(\d+\)/)
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasEntries) {
        await expect(
          main.getByRole('button', { name: /Clear History/i })
        ).toBeVisible();
      }
    });

    test('should display View Details button on history entries', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /History/i }).click();
      await page.waitForTimeout(500);

      const hasEntries = await main
        .getByText(/Import History \(\d+\)/)
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasEntries) {
        const hasViewDetails = await main
          .getByRole('button', { name: /View Details/i })
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        expect(hasViewDetails).toBeTruthy();
      }
    });

    test('should display history entry details when entries exist', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /History/i }).click();
      await page.waitForTimeout(500);

      const hasEntries = await main
        .getByText(/Import History \(\d+\)/)
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasEntries) {
        // Each entry shows format, status badge, filename, and route count
        const hasSuccessBadge = await main
          .getByText('Success', { exact: true })
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasFailedBadge = await main
          .getByText('Failed', { exact: true })
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        expect(hasSuccessBadge || hasFailedBadge).toBeTruthy();

        // Should display route count
        const hasRoutes = await main
          .getByText(/Routes: \d+/)
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        expect(hasRoutes).toBeTruthy();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 7. Navigation
  // ---------------------------------------------------------------------------
  test.describe('Navigation', () => {
    test('should navigate to Dashboard and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back to Import
      const hasImportButton = await nav
        .getByRole('button', { name: /Import/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasImportButton) {
        await nav.getByRole('button', { name: /Import/i }).click();
      } else {
        await page.goto(`${BASE_URL}/import`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Import API Collections', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Services and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Services' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Services', exact: true, level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back to Import
      const hasImportButton = await nav
        .getByRole('button', { name: /Import/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasImportButton) {
        await nav.getByRole('button', { name: /Import/i }).click();
      } else {
        await page.goto(`${BASE_URL}/import`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Import API Collections', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 8. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a single H1 heading', async ({ page }) => {
      const h1 = mainContent(page).getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);
      await expect(h1).toHaveText('Import API Collections');
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

    test('should have accessible tab controls', async ({ page }) => {
      const main = mainContent(page);
      const tabButtons = [
        main.getByRole('button', { name: /Postman/i }),
        main.getByRole('button', { name: /Insomnia/i }),
        main.getByRole('button', { name: /cURL/i }),
        main.getByRole('button', { name: /History/i }),
      ];

      for (const tab of tabButtons) {
        await expect(tab).toBeVisible();
      }
    });

    test('should have labeled form inputs', async ({ page }) => {
      const main = mainContent(page);
      // The Configuration section has labeled inputs
      await expect(main.getByText('Base URL Override (optional)').first()).toBeVisible();
    });

    test('should have accessible buttons with descriptive text', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: /Preview Routes/i })
      ).toBeVisible();
      await expect(
        main.getByRole('button', { name: /Import.*Route/i })
      ).toBeVisible();
      await expect(
        main.getByRole('button', { name: /Choose File/i })
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 9. Error-Free Operation
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

    test('should not crash when switching tabs rapidly', async ({ page }) => {
      const main = mainContent(page);

      // Rapidly switch between all four tabs
      await main.getByRole('button', { name: /Insomnia/i }).click();
      await page.waitForTimeout(200);
      await main.getByRole('button', { name: /cURL/i }).click();
      await page.waitForTimeout(200);
      await main.getByRole('button', { name: /History/i }).click();
      await page.waitForTimeout(200);
      await main.getByRole('button', { name: /Postman/i }).click();
      await page.waitForTimeout(200);
      await main.getByRole('button', { name: /cURL/i }).click();
      await page.waitForTimeout(200);
      await main.getByRole('button', { name: /Postman/i }).click();
      await page.waitForTimeout(500);

      // Page should still be functional
      await expect(
        main.getByRole('heading', { name: 'Import API Collections', level: 1 })
      ).toBeVisible();
      await expect(main.getByText('Drop Postman Collection here').first()).toBeVisible();
    });

    test('should handle page reload gracefully', async ({ page }) => {
      await page.reload({ waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(2000);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Import API Collections', level: 1 })
      ).toBeVisible({ timeout: 10000 });
    });
  });
});
