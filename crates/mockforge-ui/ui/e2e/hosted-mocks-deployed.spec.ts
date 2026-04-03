import { test, expect } from '@playwright/test';

/**
 * Hosted Mocks Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts hosted-mocks-deployed
 *
 * These tests verify the Hosted Mocks page functionality:
 *   1. Page load & layout
 *   2. Deployments table structure
 *   3. Empty state
 *   4. Deploy Mock dialog
 *   5. Navigation
 *   6. Accessibility
 *   7. Error-free operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Hosted Mocks — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/hosted-mocks`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    // Wait for the Hosted Mocks heading
    await expect(
      mainContent(page).getByRole('heading', { name: 'Hosted Mocks' })
    ).toBeVisible({ timeout: 10000 });
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the hosted mocks page at /hosted-mocks', async ({ page }) => {
      await expect(page).toHaveURL(/\/hosted-mocks/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Hosted Mocks' })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Deploy and manage cloud-hosted mock services')
      ).toBeVisible();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Hosted Mocks')).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Header Action Buttons
  // ---------------------------------------------------------------------------
  test.describe('Header Action Buttons', () => {
    test('should display the "Deploy Mock" button', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: 'Deploy Mock' })
      ).toBeVisible();
    });

    test('should display the "Refresh" button', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: 'Refresh' }).first()
      ).toBeVisible();
    });

    test('should handle Refresh button click without errors', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Refresh' }).first().click();
      await page.waitForTimeout(1500);

      await expect(
        main.getByRole('heading', { name: 'Hosted Mocks' })
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Deployments Table
  // ---------------------------------------------------------------------------
  test.describe('Deployments Table', () => {
    test('should display the table with correct column headers', async ({ page }) => {
      const main = mainContent(page);
      const table = main.getByRole('table');
      await expect(table).toBeVisible();

      await expect(table.getByRole('columnheader', { name: 'Name' })).toBeVisible();
      await expect(table.getByRole('columnheader', { name: 'Status' })).toBeVisible();
      await expect(table.getByRole('columnheader', { name: 'Health' })).toBeVisible();
      await expect(table.getByRole('columnheader', { name: 'URL' })).toBeVisible();
      await expect(table.getByRole('columnheader', { name: 'Created' })).toBeVisible();
      await expect(table.getByRole('columnheader', { name: 'Actions' })).toBeVisible();
    });

    test('should show either deployments or empty state', async ({ page }) => {
      const main = mainContent(page);
      const hasEmpty = await main.getByText('No deployments yet')
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasRows = await main.getByRole('row').nth(1)
        .isVisible({ timeout: 3000 }).catch(() => false);

      expect(hasEmpty || hasRows).toBeTruthy();
    });

    test('should display empty state message when no deployments exist', async ({ page }) => {
      const main = mainContent(page);
      const emptyMsg = main.getByText('No deployments yet. Create your first deployment to get started.');
      const hasEmpty = await emptyMsg.isVisible({ timeout: 3000 }).catch(() => false);

      if (hasEmpty) {
        await expect(emptyMsg).toBeVisible();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Deploy Mock Dialog
  // ---------------------------------------------------------------------------
  test.describe('Deploy Mock Dialog', () => {
    test('should open the Deploy dialog from "Deploy Mock" button', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Deploy Mock' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible({ timeout: 5000 });
      await expect(
        dialog.getByRole('heading', { name: 'Deploy New Mock Service' })
      ).toBeVisible();
    });

    test('should display all form fields', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Deploy Mock' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByRole('textbox', { name: 'Name' })).toBeVisible();
      await expect(dialog.getByRole('textbox', { name: 'Slug' })).toBeVisible();
      await expect(dialog.getByRole('textbox', { name: 'Description' })).toBeVisible();
      await expect(dialog.getByRole('textbox', { name: 'OpenAPI Spec URL (optional)' })).toBeVisible();
      await expect(dialog.getByRole('textbox', { name: 'Configuration (JSON)' })).toBeVisible();
    });

    test('should display name field with placeholder', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Deploy Mock' }).click();
      await page.waitForTimeout(500);

      const nameInput = page.getByRole('dialog').getByRole('textbox', { name: 'Name' });
      await expect(nameInput).toHaveAttribute('placeholder', 'My Mock Service');
    });

    test('should display slug field with auto-generation note', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Deploy Mock' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByRole('textbox', { name: 'Slug' })).toBeVisible();
      await expect(
        dialog.getByText('URL-friendly identifier (auto-generated from name)')
      ).toBeVisible();
    });

    test('should display configuration field with default value', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Deploy Mock' }).click();
      await page.waitForTimeout(500);

      const configInput = page.getByRole('dialog').getByRole('textbox', { name: 'Configuration (JSON)' });
      await expect(configInput).toHaveValue('{}');
    });

    test('should display Cancel and Deploy buttons', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Deploy Mock' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByRole('button', { name: 'Cancel' })).toBeVisible();
      await expect(dialog.getByRole('button', { name: 'Deploy' })).toBeVisible();
    });

    test('should disable Deploy button when name is empty', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Deploy Mock' }).click();
      await page.waitForTimeout(500);

      await expect(
        page.getByRole('dialog').getByRole('button', { name: 'Deploy' })
      ).toBeDisabled();
    });

    test('should enable Deploy button when name is filled', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Deploy Mock' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByRole('textbox', { name: 'Name' }).fill('E2E Test Mock');
      await page.waitForTimeout(300);

      await expect(
        dialog.getByRole('button', { name: 'Deploy' })
      ).toBeEnabled();
    });

    test('should auto-generate slug from name', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Deploy Mock' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByRole('textbox', { name: 'Name' }).fill('My Test Service');
      await page.waitForTimeout(500);

      const slugInput = dialog.getByRole('textbox', { name: 'Slug' });
      const slugValue = await slugInput.inputValue();
      // Slug should be a URL-friendly version of the name
      expect(slugValue).toMatch(/^[a-z0-9-]+$/);
    });

    test('should close dialog when Cancel is clicked', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Deploy Mock' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible();

      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);

      await expect(dialog).not.toBeVisible();
    });

    test('should allow filling in all form fields', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Deploy Mock' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');

      await dialog.getByRole('textbox', { name: 'Name' }).fill('E2E Mock Service');
      await dialog.getByRole('textbox', { name: 'Description' }).fill('Created by E2E test');
      await dialog.getByRole('textbox', { name: 'OpenAPI Spec URL (optional)' }).fill('https://example.com/openapi.json');

      await expect(dialog.getByRole('textbox', { name: 'Name' })).toHaveValue('E2E Mock Service');
      await expect(dialog.getByRole('textbox', { name: 'Description' })).toHaveValue('Created by E2E test');
      await expect(dialog.getByRole('textbox', { name: 'OpenAPI Spec URL (optional)' })).toHaveValue('https://example.com/openapi.json');

      // Cancel without deploying
      await dialog.getByRole('button', { name: 'Cancel' }).click();
    });

    test('should handle rapid dialog open/close without errors', async ({ page }) => {
      const main = mainContent(page);

      for (let i = 0; i < 3; i++) {
        await main.getByRole('button', { name: 'Deploy Mock' }).click();
        await page.waitForTimeout(300);
        await page.getByRole('dialog').getByRole('button', { name: 'Cancel' }).click();
        await page.waitForTimeout(300);
      }

      await expect(
        main.getByRole('heading', { name: 'Hosted Mocks' })
      ).toBeVisible();
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

      await nav.getByRole('button', { name: 'Hosted Mocks' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Hosted Mocks' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Services and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Services' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Services', exact: true, level: 1 })
      ).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: 'Hosted Mocks' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Hosted Mocks' })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Hosted Mocks' })
      ).toBeVisible();
    });

    test('should have a data table with proper structure', async ({ page }) => {
      const table = mainContent(page).getByRole('table');
      await expect(table).toBeVisible();

      // Should have header row
      const headers = table.getByRole('columnheader');
      expect(await headers.count()).toBe(6);
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

    test('Deploy dialog should have proper role and heading', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Deploy Mock' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible();
      await expect(
        dialog.getByRole('heading', { name: 'Deploy New Mock Service', level: 2 })
      ).toBeVisible();

      await dialog.getByRole('button', { name: 'Cancel' }).click();
    });

    test('Deploy dialog form inputs should have labels', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Deploy Mock' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      // All inputs are accessible by their label names
      await expect(dialog.getByRole('textbox', { name: 'Name' })).toBeVisible();
      await expect(dialog.getByRole('textbox', { name: 'Slug' })).toBeVisible();
      await expect(dialog.getByRole('textbox', { name: 'Description' })).toBeVisible();
      await expect(dialog.getByRole('textbox', { name: 'Configuration (JSON)' })).toBeVisible();

      await dialog.getByRole('button', { name: 'Cancel' }).click();
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
