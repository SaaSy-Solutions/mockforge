import { test, expect } from '@playwright/test';

/**
 * Federation Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts federation-deployed
 *
 * These tests verify Federation page functionality with REAL API data:
 *   1. Page load & layout
 *   2. Empty state
 *   3. Create federation (CRUD)
 *   4. Federation list display
 *   5. Delete federation (CRUD)
 *   6. Navigation
 *   7. Accessibility
 *   8. Error-free operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Federation — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/federation`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    // Wait for federation content to load (either the list heading or loading spinner to finish)
    await page.waitForTimeout(3000);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the federation page at /federation', async ({ page }) => {
      await expect(page).toHaveURL(/\/federation/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the Federations heading', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('heading', { name: 'Federations', level: 2 })
      ).toBeVisible({ timeout: 10000 });
    });

    test('should display the subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Compose multiple workspaces into federated virtual systems')
      ).toBeVisible({ timeout: 10000 });
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Federation')).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Create Federation Button
  // ---------------------------------------------------------------------------
  test.describe('Create Federation Button', () => {
    test('should display a "Create Federation" button', async ({ page }) => {
      const main = mainContent(page);
      // The Create Federation button appears either in the header or in the empty state
      const createBtn = main.getByRole('button', { name: /Create.*Federation/i });
      await expect(createBtn.first()).toBeVisible({ timeout: 10000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Empty State or Federation List
  // ---------------------------------------------------------------------------
  test.describe('Empty State or Federation List', () => {
    test('should show either federations or empty state', async ({ page }) => {
      const main = mainContent(page);

      const hasEmptyState = await main.getByText('No federations found')
        .isVisible({ timeout: 5000 }).catch(() => false);
      const hasFederations = await main.getByRole('heading', { level: 3 })
        .first().isVisible({ timeout: 3000 }).catch(() => false);

      expect(hasEmptyState || hasFederations).toBeTruthy();
    });

    test('should show "Create Your First Federation" in empty state if no federations exist', async ({ page }) => {
      const main = mainContent(page);
      const hasEmptyState = await main.getByText('No federations found')
        .isVisible({ timeout: 5000 }).catch(() => false);

      if (hasEmptyState) {
        await expect(
          main.getByRole('button', { name: /Create Your First Federation/i })
        ).toBeVisible();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 4. CRUD: Create → Verify → Delete Federation
  // ---------------------------------------------------------------------------
  test.describe('Federation CRUD Flow', () => {
    const testFederationName = `E2E Test Federation ${Date.now()}`;

    test('should create a federation, see it in the list, and delete it', async ({ page }) => {
      const main = mainContent(page);

      // Step 1: Click "Create Federation" button
      const createBtn = main.getByRole('button', { name: /Create.*Federation/i }).first();
      await expect(createBtn).toBeVisible({ timeout: 10000 });
      await createBtn.click();
      await page.waitForTimeout(1000);

      // Step 2: Fill in the Create Federation form
      // The FederationForm has "Federation Name" and "Description" fields
      await expect(
        main.getByRole('heading', { name: /Create Federation/i })
      ).toBeVisible({ timeout: 5000 });

      const nameInput = main.locator('input').first();
      await nameInput.fill(testFederationName);

      const descTextarea = main.locator('textarea').first();
      await descTextarea.fill('Created by E2E test');

      // Step 3: Submit the form
      await main.getByRole('button', { name: /Save Federation/i }).click();
      await page.waitForTimeout(3000);

      // Step 4: Verify the federation appears in the list
      // After save, the view should return to list mode
      await expect(
        main.getByRole('heading', { name: 'Federations', level: 2 })
      ).toBeVisible({ timeout: 10000 });

      await expect(
        main.getByText(testFederationName)
      ).toBeVisible({ timeout: 10000 });

      // Step 5: Verify description is shown
      await expect(
        main.getByText('Created by E2E test')
      ).toBeVisible();

      // Step 6: Delete the federation
      // The delete button has title "Delete Federation"
      const federationCard = main.locator('div').filter({
        hasText: testFederationName,
      }).first();

      const deleteBtn = federationCard.getByRole('button', { name: /Delete/i }).first();

      // Handle the confirm dialog
      page.on('dialog', (dialog) => dialog.accept());
      await deleteBtn.click();
      await page.waitForTimeout(3000);

      // Step 7: Verify the federation is gone
      const stillVisible = await main.getByText(testFederationName)
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(stillVisible).toBeFalsy();
    });

    test('should navigate to create form and back via cancel', async ({ page }) => {
      const main = mainContent(page);

      const createBtn = main.getByRole('button', { name: /Create.*Federation/i }).first();
      await expect(createBtn).toBeVisible({ timeout: 10000 });
      await createBtn.click();
      await page.waitForTimeout(1000);

      // Should show the Create form
      await expect(
        main.getByRole('heading', { name: /Create Federation/i })
      ).toBeVisible({ timeout: 5000 });

      // Cancel should go back to the list
      await main.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(1000);

      await expect(
        main.getByRole('heading', { name: 'Federations', level: 2 })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Create Form Fields
  // ---------------------------------------------------------------------------
  test.describe('Create Form Fields', () => {
    test.beforeEach(async ({ page }) => {
      const main = mainContent(page);
      const createBtn = main.getByRole('button', { name: /Create.*Federation/i }).first();
      await expect(createBtn).toBeVisible({ timeout: 10000 });
      await createBtn.click();
      await page.waitForTimeout(1000);
    });

    test('should display Federation Name input', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Federation Name')
      ).toBeVisible();
    });

    test('should display Description textarea', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Description')
      ).toBeVisible();
      await expect(
        mainContent(page).locator('textarea')
      ).toBeVisible();
    });

    test('should display Services section with Add Service button', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText(/Services/)).toBeVisible();
      await expect(
        main.getByRole('button', { name: /Add Service/i })
      ).toBeVisible();
    });

    test('should display Save Federation button', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: /Save Federation/i })
      ).toBeVisible();
    });

    test('should display Cancel button', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: 'Cancel' })
      ).toBeVisible();
    });

    test('should add a service when clicking Add Service', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Add Service/i }).click();
      await page.waitForTimeout(500);

      // Should show service fields: Service Name, Workspace ID, Base Path, Reality Level
      await expect(main.getByText('Service Name', { exact: true })).toBeVisible();
      await expect(main.getByText('Base Path')).toBeVisible();
      await expect(main.getByText('Reality Level')).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Navigation
  // ---------------------------------------------------------------------------
  test.describe('Navigation', () => {
    test('should navigate to Dashboard and back to Federation', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back via URL since Federation isn't in the sidebar
      await page.goto(`${BASE_URL}/federation`, { waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(3000);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Federations', level: 2 })
      ).toBeVisible({ timeout: 10000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 7. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
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
  // 8. Error-Free Operation
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
          !err.includes('federation') // API may not be deployed yet
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
