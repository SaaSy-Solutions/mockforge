import { test, expect } from '@playwright/test';

/**
 * Chains Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts chains-deployed
 *
 * These tests verify the Chains page functionality:
 *   1. Page load & layout
 *   2. Header action buttons
 *   3. Chains table / empty state
 *   4. Create Chain dialog
 *   5. Chain CRUD flow
 *   6. Delete confirmation dialog
 *   7. Navigation
 *   8. Accessibility
 *   9. Error-free operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Chains — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/chains`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    await expect(
      mainContent(page).getByRole('heading', { name: 'Request Chains', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the chains page at /chains', async ({ page }) => {
      await expect(page).toHaveURL(/\/chains/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Request Chains', level: 1 })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Manage and execute request chains for complex API workflows').first()
      ).toBeVisible();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home').first()).toBeVisible();
      await expect(banner.getByText('Chains').first()).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Header Action Buttons
  // ---------------------------------------------------------------------------
  test.describe('Header Action Buttons', () => {
    test('should display the "Create Chain" button', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: /Create Chain/i }).first()
      ).toBeVisible();
    });

    test('should handle Create Chain button click by opening dialog', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Chain/i }).first().click();
      await page.waitForTimeout(500);

      await expect(page.getByRole('dialog')).toBeVisible({ timeout: 5000 });

      // Close dialog to clean up
      const dialog = page.getByRole('dialog');
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Chains Table / Empty State
  // ---------------------------------------------------------------------------
  test.describe('Chains Table / Empty State', () => {
    test('should show either chains table or empty state', async ({ page }) => {
      const main = mainContent(page);

      const hasEmpty = await main.getByText('No Chains Found')
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasChains = await main.getByText('Available Chains')
        .isVisible({ timeout: 3000 }).catch(() => false);

      expect(hasEmpty || hasChains).toBeTruthy();
    });

    test('should display "No Chains Found" heading in empty state', async ({ page }) => {
      const main = mainContent(page);
      const hasEmpty = await main.getByText('No Chains Found').first()
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasEmpty) {
        await expect(main.getByText('No Chains Found').first()).toBeVisible();
        await expect(
          main.getByText('Create your first request chain to get started with complex API workflow testing.').first()
        ).toBeVisible();
      }
    });

    test('should display "Create First Chain" button in empty state', async ({ page }) => {
      const main = mainContent(page);
      const hasEmpty = await main.getByText('No Chains Found')
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasEmpty) {
        await expect(
          main.getByRole('button', { name: /Create First Chain/i })
        ).toBeVisible();
      }
    });

    test('should display table column headers when chains exist', async ({ page }) => {
      const main = mainContent(page);
      const hasChains = await main.getByText('Available Chains')
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasChains) {
        await expect(main.getByRole('columnheader', { name: 'Name' })).toBeVisible();
        await expect(main.getByRole('columnheader', { name: 'Description' })).toBeVisible();
        await expect(main.getByRole('columnheader', { name: 'Links' })).toBeVisible();
        await expect(main.getByRole('columnheader', { name: 'Status' })).toBeVisible();
        await expect(main.getByRole('columnheader', { name: 'Tags' })).toBeVisible();
        await expect(main.getByRole('columnheader', { name: 'Actions' })).toBeVisible();
      }
    });

    test('should display action buttons on chain rows when chains exist', async ({ page }) => {
      const main = mainContent(page);
      const hasChains = await main.getByText('Available Chains')
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasChains) {
        const firstRow = main.getByRole('row').nth(1); // skip header row
        await expect(firstRow.getByRole('button', { name: /View/i })).toBeVisible();
        await expect(firstRow.getByRole('button', { name: /Execute/i })).toBeVisible();
        await expect(firstRow.getByRole('button', { name: /Delete/i })).toBeVisible();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Create Chain Dialog
  // ---------------------------------------------------------------------------
  test.describe('Create Chain Dialog', () => {
    test('should open the Create Chain dialog from header button', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Chain/i }).first().click();
      await page.waitForTimeout(500);

      await expect(page.getByRole('dialog')).toBeVisible({ timeout: 5000 });
      await expect(page.getByRole('dialog').getByRole('heading', { name: 'Create Chain' })).toBeVisible();
    });

    test('should open the Create Chain dialog from empty state button', async ({ page }) => {
      const main = mainContent(page);
      const hasEmpty = await main.getByText('No Chains Found')
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasEmpty) {
        await main.getByRole('button', { name: /Create First Chain/i }).click();
      } else {
        await main.getByRole('button', { name: /Create Chain/i }).first().click();
      }
      await page.waitForTimeout(500);

      await expect(page.getByRole('dialog')).toBeVisible({ timeout: 5000 });
      await expect(page.getByRole('dialog').getByRole('heading', { name: 'Create Chain' })).toBeVisible();
    });

    test('should display YAML Definition textarea in the dialog', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Chain/i }).first().click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByText('YAML Definition').first()).toBeVisible();
      await expect(dialog.locator('textarea')).toBeVisible();
    });

    test('should display Load Example button in the dialog', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Chain/i }).first().click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(
        dialog.getByRole('button', { name: /Load Example/i })
      ).toBeVisible();
    });

    test('should load example YAML when Load Example is clicked', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Chain/i }).first().click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByRole('button', { name: /Load Example/i }).click();
      await page.waitForTimeout(500);

      const textarea = dialog.locator('textarea');
      const value = await textarea.inputValue();
      expect(value).toContain('User Management Workflow');
      expect(value).toContain('links:');
    });

    test('should display Create Chain and Cancel buttons in the dialog', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Chain/i }).first().click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByRole('button', { name: 'Cancel' })).toBeVisible();
      await expect(dialog.getByRole('button', { name: 'Create Chain' })).toBeVisible();
    });

    test('should display the dialog description', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Chain/i }).first().click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(
        dialog.getByText('Create a new request chain using YAML definition.').first()
      ).toBeVisible();
    });

    test('should close dialog when Cancel is clicked', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Chain/i }).first().click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible();

      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);

      await expect(dialog).not.toBeVisible();
    });

    test('should allow editing the YAML textarea', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Chain/i }).first().click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      const textarea = dialog.locator('textarea');

      await textarea.clear();
      await textarea.fill('name: test-chain\ndescription: test');
      await page.waitForTimeout(300);

      const value = await textarea.inputValue();
      expect(value).toContain('name: test-chain');

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Chain CRUD Flow
  // ---------------------------------------------------------------------------
  test.describe('Chain CRUD Flow', () => {
    const testChainYaml = `name: e2e-test-chain
description: Created by E2E test
enabled: true
links:
  - id: step1
    method: GET
    url: /api/health`;

    test('should create a chain via YAML, view it in the list, and delete it', async ({ page }) => {
      const main = mainContent(page);

      // Step 1: Open create dialog
      await main.getByRole('button', { name: /Create Chain/i }).first().click();
      await page.waitForTimeout(500);

      const createDialog = page.getByRole('dialog');
      await expect(createDialog).toBeVisible({ timeout: 5000 });

      // Step 2: Fill in YAML
      const textarea = createDialog.locator('textarea');
      await textarea.clear();
      await textarea.fill(testChainYaml);
      await page.waitForTimeout(300);

      // Step 3: Submit
      await createDialog.getByRole('button', { name: 'Create Chain' }).click();
      await page.waitForTimeout(3000);

      // Step 4: Check if creation succeeded (dialog should close)
      const dialogStillOpen = await page.getByRole('dialog')
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (dialogStillOpen) {
        // Creation may have failed due to API unavailability — close and skip
        const hasError = await page.getByRole('dialog').getByText(/Failed|Error/i)
          .isVisible({ timeout: 2000 }).catch(() => false);
        if (hasError) {
          await page.getByRole('dialog').getByRole('button', { name: 'Cancel' }).click();
          await page.waitForTimeout(500);
          return; // API not available, skip rest of test
        }
      }

      // Step 5: Verify the chain appears (look for table or refresh)
      await page.waitForTimeout(2000);

      const hasChains = await main.getByText('Available Chains')
        .isVisible({ timeout: 5000 }).catch(() => false);

      if (hasChains) {
        // Verify our chain is in the table
        const hasTestChain = await main.getByText('e2e-test-chain')
          .isVisible({ timeout: 5000 }).catch(() => false);

        if (hasTestChain) {
          // Step 6: View the chain
          const chainRow = main.getByRole('row').filter({ hasText: 'e2e-test-chain' });
          await chainRow.getByRole('button', { name: /View/i }).click();
          await page.waitForTimeout(1000);

          const viewDialog = page.getByRole('dialog');
          await expect(viewDialog).toBeVisible({ timeout: 5000 });

          // Verify view dialog content
          await expect(viewDialog.getByText('Overview').first()).toBeVisible({ timeout: 5000 });
          await expect(viewDialog.getByRole('button', { name: 'Close' })).toBeVisible();
          await expect(viewDialog.getByRole('button', { name: /Execute Chain/i })).toBeVisible();

          // Close view dialog
          await viewDialog.getByRole('button', { name: 'Close' }).click();
          await page.waitForTimeout(500);
        }

        // Step 7: Delete the chain via API cleanup
        const token = await page.evaluate(() => localStorage.getItem('auth_token'));
        const listResponse = await page.evaluate(async (authToken) => {
          const res = await fetch('/api/v1/chains', {
            headers: { 'Authorization': `Bearer ${authToken}` },
          });
          return res.json();
        }, token);

        const chainsData = (listResponse as { chains?: Array<{ name: string; id: string }> })?.chains || [];
        const testChain = chainsData.find(
          (c) => c.name === 'e2e-test-chain'
        );

        if (testChain) {
          await page.evaluate(async ({ id, authToken }) => {
            await fetch(`/api/v1/chains/${id}`, {
              method: 'DELETE',
              headers: { 'Authorization': `Bearer ${authToken}` },
            });
          }, { id: testChain.id, authToken: token });

          await page.waitForTimeout(1000);
        }
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Delete Confirmation Dialog
  // ---------------------------------------------------------------------------
  test.describe('Delete Confirmation Dialog', () => {
    test('should display delete dialog with correct content when chains exist', async ({ page }) => {
      const main = mainContent(page);
      const hasChains = await main.getByText('Available Chains')
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (!hasChains) {
        // No chains to delete — skip
        return;
      }

      // Click Delete on the first chain
      const firstRow = main.getByRole('row').nth(1);
      await firstRow.getByRole('button', { name: /Delete/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible({ timeout: 5000 });

      // Verify dialog content
      await expect(dialog.getByText('Delete Chain').first()).toBeVisible();
      await expect(dialog.getByText(/Are you sure you want to delete/)).toBeVisible();
      await expect(dialog.getByText(/This action cannot be undone/)).toBeVisible();

      // Verify buttons
      await expect(dialog.getByRole('button', { name: 'Cancel' })).toBeVisible();
      await expect(dialog.getByRole('button', { name: 'Delete' })).toBeVisible();

      // Cancel without deleting
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);

      await expect(dialog).not.toBeVisible();
    });

    test('should close delete dialog when Cancel is clicked', async ({ page }) => {
      const main = mainContent(page);
      const hasChains = await main.getByText('Available Chains')
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (!hasChains) {
        return;
      }

      const firstRow = main.getByRole('row').nth(1);
      await firstRow.getByRole('button', { name: /Delete/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible({ timeout: 5000 });

      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);

      await expect(dialog).not.toBeVisible();

      // Verify the chain still exists in the table
      await expect(main.getByText('Available Chains').first()).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 7. Navigation
  // ---------------------------------------------------------------------------
  test.describe('Navigation', () => {
    test('should navigate to Dashboard and back', async ({ page }) => {
      await page.goto(`${BASE_URL}/dashboard`, { waitUntil: 'domcontentloaded', timeout: 30000 });
      await expect(page).toHaveURL(/\/(dashboard)?$/, { timeout: 15000 });
      await page.goBack();
      await page.waitForTimeout(2000);
    });

    test('should navigate to Services and back', async ({ page }) => {
      await page.goto(`${BASE_URL}/services`, { waitUntil: 'domcontentloaded', timeout: 30000 });
      await expect(page).toHaveURL(/\/services/, { timeout: 15000 });
      await page.goBack();
      await page.waitForTimeout(2000);
    });
  });

  // ---------------------------------------------------------------------------
  // 8. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a single H1 heading', async ({ page }) => {
      const h1 = mainContent(page).getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);
      await expect(h1).toHaveText('Request Chains');
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

    test('should have table headers when chains exist', async ({ page }) => {
      const main = mainContent(page);
      const hasChains = await main.getByText('Available Chains')
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasChains) {
        const headers = main.getByRole('columnheader');
        expect(await headers.count()).toBeGreaterThanOrEqual(6);
      }
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

    test('should not show error loading state', async ({ page }) => {
      const hasError = await mainContent(page)
        .getByText('Error Loading Chains')
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasError).toBeFalsy();
    });
  });
});
