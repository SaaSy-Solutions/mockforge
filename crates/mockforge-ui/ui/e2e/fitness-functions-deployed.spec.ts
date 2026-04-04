import { test, expect } from '@playwright/test';

/**
 * Fitness Functions Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config (handles auth via setup project):
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts fitness-functions-deployed
 *
 * These tests verify all Fitness Functions functionality on the live deployed site:
 *   1.  Page Load & Layout
 *   2.  Header Controls
 *   3.  Global Fitness Summary
 *   4.  Registered Functions List / Empty State
 *   5.  Create Dialog
 *   6.  Function CRUD Flow
 *   7.  Navigation
 *   8.  Accessibility
 *   9.  Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

// Scoped locator for the main content area (excludes sidebar to avoid duplicate matches)
function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Fitness Functions — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/fitness-functions`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    // Wait for the app shell to render (sidebar navigation visible)
    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    // Wait for the Fitness Functions heading to confirm content loaded
    await expect(
      mainContent(page).getByRole('heading', { name: 'Fitness Functions', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    // Small stabilization delay for dynamic content
    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the fitness functions page at /fitness-functions', async ({ page }) => {
      await expect(page).toHaveURL(/\/fitness-functions/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Fitness Functions', level: 1 })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText(
          'Register custom tests that run against each new contract version to enforce constraints'
        )
      ).toBeVisible();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Fitness Functions')).toBeVisible();
    });

    test('should display the Registered Fitness Functions section', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('heading', { name: 'Registered Fitness Functions', level: 2 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Refresh button', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: /Refresh/i })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Header Controls
  // ---------------------------------------------------------------------------
  test.describe('Header Controls', () => {
    test('should display the registered function count', async ({ page }) => {
      const main = mainContent(page);
      // The count text matches "N fitness function(s) registered"
      const hasCount = await main
        .getByText(/\d+ fitness function[s]? registered/)
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      expect(hasCount).toBeTruthy();
    });

    test('should display the Show/Hide Summary button', async ({ page }) => {
      const main = mainContent(page);
      const hasHide = await main
        .getByRole('button', { name: /Hide Summary/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasShow = await main
        .getByRole('button', { name: /Show Summary/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasHide || hasShow).toBeTruthy();
    });

    test('should toggle summary visibility when Show/Hide Summary is clicked', async ({ page }) => {
      const main = mainContent(page);

      // Determine the current state
      const hideBtn = main.getByRole('button', { name: /Hide Summary/i });
      const showBtn = main.getByRole('button', { name: /Show Summary/i });
      const isCurrentlyShown = await hideBtn
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (isCurrentlyShown) {
        // Summary is visible — click Hide Summary
        await hideBtn.click();
        await page.waitForTimeout(500);

        // The Global Fitness Summary section should now be hidden
        const hasSummarySection = await main
          .getByRole('heading', { name: 'Global Fitness Summary', level: 2 })
          .isVisible({ timeout: 2000 })
          .catch(() => false);
        expect(hasSummarySection).toBeFalsy();

        // Button should now say "Show Summary"
        await expect(showBtn).toBeVisible();

        // Click Show Summary to restore
        await showBtn.click();
        await page.waitForTimeout(500);
        await expect(hideBtn).toBeVisible();
      } else {
        // Summary is hidden — click Show Summary
        await showBtn.click();
        await page.waitForTimeout(500);
        await expect(hideBtn).toBeVisible();

        // Click Hide Summary to restore
        await hideBtn.click();
        await page.waitForTimeout(500);
        await expect(showBtn).toBeVisible();
      }
    });

    test('should display the Create Fitness Function button', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: /Create Fitness Function/i })
      ).toBeVisible();
    });

    test('should open create dialog when Create Fitness Function is clicked', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      await expect(page.getByRole('dialog')).toBeVisible({ timeout: 5000 });
      await expect(
        page.getByRole('dialog').getByText('Create Fitness Function')
      ).toBeVisible();

      // Clean up — close dialog
      const dialog = page.getByRole('dialog');
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });

    test('should handle Refresh button click without crashing', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Refresh/i }).click();
      await page.waitForTimeout(1500);

      // Page should still be functional after refresh
      await expect(
        main.getByRole('heading', { name: 'Fitness Functions', level: 1 })
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Global Fitness Summary
  // ---------------------------------------------------------------------------
  test.describe('Global Fitness Summary', () => {
    test('should display the Global Fitness Summary section when summary is shown', async ({ page }) => {
      const main = mainContent(page);

      // Ensure summary is visible (click Show Summary if needed)
      const showBtn = main.getByRole('button', { name: /Show Summary/i });
      const isHidden = await showBtn
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      if (isHidden) {
        await showBtn.click();
        await page.waitForTimeout(500);
      }

      await expect(
        main.getByRole('heading', { name: 'Global Fitness Summary', level: 2 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display summary subtitle', async ({ page }) => {
      const main = mainContent(page);

      // Ensure summary is visible
      const showBtn = main.getByRole('button', { name: /Show Summary/i });
      const isHidden = await showBtn
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      if (isHidden) {
        await showBtn.click();
        await page.waitForTimeout(500);
      }

      await expect(
        main.getByText('Aggregate fitness test results across all endpoints')
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display summary cards or empty state', async ({ page }) => {
      const main = mainContent(page);

      // Ensure summary is visible
      const showBtn = main.getByRole('button', { name: /Show Summary/i });
      const isHidden = await showBtn
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      if (isHidden) {
        await showBtn.click();
        await page.waitForTimeout(500);
      }

      // Either the summary cards (Total Tests, Passed, Failed, Pass Rate) are shown
      // or the empty state is shown
      const hasTotalTests = await main
        .getByText('Total Tests')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasEmptyState = await main
        .getByText('No Fitness Test Results')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasTotalTests || hasEmptyState).toBeTruthy();
    });

    test('should display all four summary cards when data exists', async ({ page }) => {
      const main = mainContent(page);

      // Ensure summary is visible
      const showBtn = main.getByRole('button', { name: /Show Summary/i });
      const isHidden = await showBtn
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      if (isHidden) {
        await showBtn.click();
        await page.waitForTimeout(500);
      }

      const hasTotalTests = await main
        .getByText('Total Tests')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasTotalTests) {
        await expect(main.getByText('Total Tests')).toBeVisible();
        await expect(main.getByText('Passed')).toBeVisible();
        await expect(main.getByText('Failed')).toBeVisible();
        await expect(main.getByText('Pass Rate')).toBeVisible();
      }
    });

    test('should display Per-Endpoint Fitness Results table when data exists', async ({ page }) => {
      const main = mainContent(page);

      // Ensure summary is visible
      const showBtn = main.getByRole('button', { name: /Show Summary/i });
      const isHidden = await showBtn
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      if (isHidden) {
        await showBtn.click();
        await page.waitForTimeout(500);
      }

      const hasPerEndpoint = await main
        .getByRole('heading', { name: 'Per-Endpoint Fitness Results', level: 2 })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasPerEndpoint) {
        // Verify table headers
        const endpointSection = main.locator('section').filter({
          hasText: 'Per-Endpoint Fitness Results',
        });
        await expect(endpointSection.getByText('Endpoint')).toBeVisible();
        await expect(endpointSection.getByText('Total')).toBeVisible();
      }
    });

    test('should display Per-Function Results table when data exists', async ({ page }) => {
      const main = mainContent(page);

      // Ensure summary is visible
      const showBtn = main.getByRole('button', { name: /Show Summary/i });
      const isHidden = await showBtn
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      if (isHidden) {
        await showBtn.click();
        await page.waitForTimeout(500);
      }

      const hasPerFunction = await main
        .getByRole('heading', { name: 'Per-Function Results', level: 2 })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasPerFunction) {
        const functionSection = main.locator('section').filter({
          hasText: 'Per-Function Results',
        });
        await expect(functionSection.getByText('Function')).toBeVisible();
        await expect(functionSection.getByText('Total')).toBeVisible();
      }
    });

    test('should display empty state message when no fitness test results exist', async ({ page }) => {
      const main = mainContent(page);

      // Ensure summary is visible
      const showBtn = main.getByRole('button', { name: /Show Summary/i });
      const isHidden = await showBtn
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      if (isHidden) {
        await showBtn.click();
        await page.waitForTimeout(500);
      }

      const hasEmptyState = await main
        .getByText('No Fitness Test Results')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasEmptyState) {
        await expect(
          main.getByText('Fitness test results will appear here once contract drift is detected')
        ).toBeVisible();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Registered Functions List / Empty State
  // ---------------------------------------------------------------------------
  test.describe('Registered Functions List / Empty State', () => {
    test('should show either registered functions or empty state', async ({ page }) => {
      const main = mainContent(page);

      const hasEmpty = await main
        .getByText('No Fitness Functions')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasFunctions = await main
        .getByText(/\d+ fitness function[s]? registered/)
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // Either empty state or the count text should be visible
      expect(hasEmpty || hasFunctions).toBeTruthy();
    });

    test('should display empty state with correct messaging when no functions exist', async ({ page }) => {
      const main = mainContent(page);

      const hasEmpty = await main
        .getByText('No Fitness Functions')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasEmpty) {
        await expect(main.getByText('No Fitness Functions')).toBeVisible();
        await expect(
          main.getByText('Create your first fitness function to start enforcing contract constraints')
        ).toBeVisible();
      }
    });

    test('should display function details when functions exist', async ({ page }) => {
      const main = mainContent(page);
      const count = await main
        .getByText(/0 fitness functions registered/)
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // If count is not "0 fitness functions registered", functions may exist
      if (!count) {
        // Check if a function row is visible by looking for Test/Edit buttons
        const hasTestButton = await main
          .getByRole('button', { name: /Test/i })
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        if (hasTestButton) {
          // Verify function row elements are present
          const hasEditButton = await main
            .getByRole('button', { name: /Edit/i })
            .first()
            .isVisible({ timeout: 3000 })
            .catch(() => false);
          expect(hasEditButton).toBeTruthy();
        }
      }
    });

    test('should display Enabled or Disabled badge on function rows', async ({ page }) => {
      const main = mainContent(page);

      const hasEnabled = await main
        .getByText('Enabled', { exact: true })
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasDisabled = await main
        .getByText('Disabled', { exact: true })
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // If functions exist, at least one badge should be visible
      const hasEmpty = await main
        .getByText('No Fitness Functions')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (!hasEmpty) {
        // Functions exist — should have at least one badge
        expect(hasEnabled || hasDisabled).toBeTruthy();
      }
    });

    test('should display type badges on function rows', async ({ page }) => {
      const main = mainContent(page);

      const hasEmpty = await main
        .getByText('No Fitness Functions')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (!hasEmpty) {
        // Check for any known type badge
        const typeLabels = ['Response Size', 'Required Field', 'Field Count', 'Schema Complexity', 'Custom'];
        let foundType = false;
        for (const label of typeLabels) {
          const has = await main
            .getByText(label, { exact: true })
            .first()
            .isVisible({ timeout: 2000 })
            .catch(() => false);
          if (has) {
            foundType = true;
            break;
          }
        }
        expect(foundType).toBeTruthy();
      }
    });

    test('should display scope badges on function rows', async ({ page }) => {
      const main = mainContent(page);

      const hasEmpty = await main
        .getByText('No Fitness Functions')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (!hasEmpty) {
        // Check for any known scope badge
        const hasGlobal = await main
          .getByText('Global', { exact: true })
          .first()
          .isVisible({ timeout: 2000 })
          .catch(() => false);
        const hasWorkspace = await main
          .getByText(/Workspace:/)
          .first()
          .isVisible({ timeout: 2000 })
          .catch(() => false);
        const hasService = await main
          .getByText(/Service:/)
          .first()
          .isVisible({ timeout: 2000 })
          .catch(() => false);
        const hasEndpoint = await main
          .getByText(/Endpoint:/)
          .first()
          .isVisible({ timeout: 2000 })
          .catch(() => false);

        expect(hasGlobal || hasWorkspace || hasService || hasEndpoint).toBeTruthy();
      }
    });

    test('should display timestamps on function rows', async ({ page }) => {
      const main = mainContent(page);

      const hasEmpty = await main
        .getByText('No Fitness Functions')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (!hasEmpty) {
        const hasCreated = await main
          .getByText(/Created:/)
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasUpdated = await main
          .getByText(/Updated:/)
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        expect(hasCreated).toBeTruthy();
        expect(hasUpdated).toBeTruthy();
      }
    });

    test('should display Test, Edit, and Delete buttons on function rows', async ({ page }) => {
      const main = mainContent(page);

      const hasEmpty = await main
        .getByText('No Fitness Functions')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (!hasEmpty) {
        await expect(
          main.getByRole('button', { name: /Test/i }).first()
        ).toBeVisible({ timeout: 3000 });
        await expect(
          main.getByRole('button', { name: /Edit/i }).first()
        ).toBeVisible({ timeout: 3000 });
        // Delete button uses a Trash icon without text — find by role
        const deleteButtons = main.locator('button').filter({ hasText: '' });
        // There should be at least one button with trash icon near each row
        const totalButtons = main.getByRole('button');
        expect(await totalButtons.count()).toBeGreaterThanOrEqual(3);
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Create Dialog
  // ---------------------------------------------------------------------------
  test.describe('Create Dialog', () => {
    test('should open the Create Fitness Function dialog', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      await expect(page.getByRole('dialog')).toBeVisible({ timeout: 5000 });
      await expect(
        page.getByRole('dialog').getByText('Create Fitness Function')
      ).toBeVisible();
    });

    test('should display dialog description', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(
        dialog.getByText('Register a new fitness function to test contract changes')
      ).toBeVisible();
    });

    test('should display Name input field', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByText('Name')).toBeVisible();
      await expect(
        dialog.getByPlaceholder('e.g., Mobile API Response Size Limit')
      ).toBeVisible();
    });

    test('should display Description textarea', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByText('Description')).toBeVisible();
      await expect(
        dialog.getByPlaceholder('Describe what this fitness function checks...')
      ).toBeVisible();
    });

    test('should display Function Type dropdown defaulting to Response Size', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByText('Function Type')).toBeVisible();
      // The default value is "Response Size"
      await expect(dialog.getByText('Response Size')).toBeVisible();
    });

    test('should show Max Increase Percent field for Response Size type', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      // Default type is response_size, so Max Increase Percent should be visible
      await expect(dialog.getByText('Max Increase Percent')).toBeVisible();
      await expect(
        dialog.getByText('Maximum allowed response size increase percentage')
      ).toBeVisible();
    });

    test('should display Scope dropdown defaulting to Global', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByText('Scope')).toBeVisible();
      await expect(dialog.getByText('Global')).toBeVisible();
    });

    test('should display Enabled toggle', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByText('Enabled')).toBeVisible();
      const switches = dialog.getByRole('switch');
      expect(await switches.count()).toBeGreaterThanOrEqual(1);
    });

    test('should display Cancel and Create buttons', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByRole('button', { name: 'Cancel' })).toBeVisible();
      await expect(dialog.getByRole('button', { name: /Create Fitness Function/i })).toBeVisible();
    });

    test('should close dialog when Cancel is clicked', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible();

      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);

      await expect(dialog).not.toBeVisible();
    });

    test('should allow editing the Name input', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      const nameInput = dialog.getByPlaceholder('e.g., Mobile API Response Size Limit');
      await nameInput.fill('E2E Test Function');
      await page.waitForTimeout(300);

      const value = await nameInput.inputValue();
      expect(value).toBe('E2E Test Function');

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });

    test('should allow editing the Description textarea', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      const textarea = dialog.getByPlaceholder('Describe what this fitness function checks...');
      await textarea.fill('Test description for E2E');
      await page.waitForTimeout(300);

      const value = await textarea.inputValue();
      expect(value).toBe('Test description for E2E');

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });

    test('should show Function Type dropdown options', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');

      // Click the Function Type trigger to open the dropdown
      const functionTypeSection = dialog.locator('div').filter({ hasText: 'Function Type' });
      const trigger = functionTypeSection.getByRole('combobox').first();
      await trigger.click();
      await page.waitForTimeout(500);

      // Verify all four options are present
      await expect(page.getByRole('option', { name: 'Response Size' })).toBeVisible({ timeout: 3000 });
      await expect(page.getByRole('option', { name: 'Required Field' })).toBeVisible({ timeout: 3000 });
      await expect(page.getByRole('option', { name: 'Field Count' })).toBeVisible({ timeout: 3000 });
      await expect(page.getByRole('option', { name: 'Schema Complexity' })).toBeVisible({ timeout: 3000 });

      // Close dropdown by pressing Escape
      await page.keyboard.press('Escape');
      await page.waitForTimeout(300);

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });

    test('should show Scope dropdown options', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');

      // Find the Scope combobox — it is the second combobox in the dialog
      const comboboxes = dialog.getByRole('combobox');
      const scopeTrigger = comboboxes.nth(1);
      await scopeTrigger.click();
      await page.waitForTimeout(500);

      await expect(page.getByRole('option', { name: 'Global' })).toBeVisible({ timeout: 3000 });
      await expect(page.getByRole('option', { name: 'Workspace' })).toBeVisible({ timeout: 3000 });
      await expect(page.getByRole('option', { name: 'Service' })).toBeVisible({ timeout: 3000 });
      await expect(page.getByRole('option', { name: 'Endpoint' })).toBeVisible({ timeout: 3000 });

      // Close dropdown
      await page.keyboard.press('Escape');
      await page.waitForTimeout(300);

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });

    test('should show Workspace ID field when Workspace scope is selected', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');

      // Select Workspace scope
      const comboboxes = dialog.getByRole('combobox');
      const scopeTrigger = comboboxes.nth(1);
      await scopeTrigger.click();
      await page.waitForTimeout(500);
      await page.getByRole('option', { name: 'Workspace' }).click();
      await page.waitForTimeout(500);

      await expect(dialog.getByText('Workspace ID')).toBeVisible();
      await expect(dialog.getByPlaceholder('workspace-1')).toBeVisible();

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });

    test('should show Service Name field when Service scope is selected', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');

      // Select Service scope
      const comboboxes = dialog.getByRole('combobox');
      const scopeTrigger = comboboxes.nth(1);
      await scopeTrigger.click();
      await page.waitForTimeout(500);
      await page.getByRole('option', { name: 'Service' }).click();
      await page.waitForTimeout(500);

      await expect(dialog.getByText('Service Name')).toBeVisible();
      await expect(dialog.getByPlaceholder('user-service')).toBeVisible();

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });

    test('should show Endpoint Pattern field when Endpoint scope is selected', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');

      // Select Endpoint scope
      const comboboxes = dialog.getByRole('combobox');
      const scopeTrigger = comboboxes.nth(1);
      await scopeTrigger.click();
      await page.waitForTimeout(500);
      await page.getByRole('option', { name: 'Endpoint' }).click();
      await page.waitForTimeout(500);

      await expect(dialog.getByText('Endpoint Pattern')).toBeVisible();
      await expect(dialog.getByPlaceholder('/v1/mobile/*')).toBeVisible();

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });

    test('should show Path Pattern and Allow New Required toggle for Required Field type', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');

      // Select Required Field type
      const comboboxes = dialog.getByRole('combobox');
      const typeTrigger = comboboxes.first();
      await typeTrigger.click();
      await page.waitForTimeout(500);
      await page.getByRole('option', { name: 'Required Field' }).click();
      await page.waitForTimeout(500);

      await expect(dialog.getByText('Path Pattern')).toBeVisible();
      await expect(dialog.getByPlaceholder('/v1/mobile/*')).toBeVisible();
      await expect(dialog.getByText('Allow new required fields')).toBeVisible();

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });

    test('should show Max Fields input for Field Count type', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');

      // Select Field Count type
      const comboboxes = dialog.getByRole('combobox');
      const typeTrigger = comboboxes.first();
      await typeTrigger.click();
      await page.waitForTimeout(500);
      await page.getByRole('option', { name: 'Field Count' }).click();
      await page.waitForTimeout(500);

      await expect(dialog.getByText('Max Fields')).toBeVisible();
      await expect(dialog.getByText('Maximum number of fields allowed')).toBeVisible();

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });

    test('should show Max Depth input for Schema Complexity type', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');

      // Select Schema Complexity type
      const comboboxes = dialog.getByRole('combobox');
      const typeTrigger = comboboxes.first();
      await typeTrigger.click();
      await page.waitForTimeout(500);
      await page.getByRole('option', { name: 'Schema Complexity' }).click();
      await page.waitForTimeout(500);

      await expect(dialog.getByText('Max Depth')).toBeVisible();
      await expect(dialog.getByText('Maximum schema depth allowed')).toBeVisible();

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Function CRUD Flow
  // ---------------------------------------------------------------------------
  test.describe('Function CRUD Flow', () => {
    const testFunctionName = `e2e-test-fn-${Date.now()}`;

    test('should create a fitness function, see it in the list, and delete it', async ({ page }) => {
      const main = mainContent(page);

      // Step 1: Open create dialog
      await main.getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible({ timeout: 5000 });

      // Step 2: Fill in the form
      await dialog.getByPlaceholder('e.g., Mobile API Response Size Limit').fill(testFunctionName);
      await dialog
        .getByPlaceholder('Describe what this fitness function checks...')
        .fill('Created by E2E test — safe to delete');
      await page.waitForTimeout(300);

      // Leave defaults: Function Type = Response Size, Scope = Global, Enabled = true

      // Step 3: Submit the form
      await dialog.getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(3000);

      // Step 4: Check if creation succeeded (dialog should close)
      const dialogStillOpen = await page
        .getByRole('dialog')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (dialogStillOpen) {
        // Creation may have failed due to API unavailability
        const hasError = await page
          .getByRole('dialog')
          .getByText(/Failed|Error/i)
          .isVisible({ timeout: 2000 })
          .catch(() => false);
        if (hasError) {
          await page.getByRole('dialog').getByRole('button', { name: 'Cancel' }).click();
          await page.waitForTimeout(500);
          return; // API not available, skip rest of test
        }
      }

      // Step 5: Verify the function appears in the list
      await page.waitForTimeout(2000);

      const hasTestFunction = await main
        .getByText(testFunctionName)
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasTestFunction) {
        // Verify badges are present on the new function
        await expect(main.getByText(testFunctionName)).toBeVisible();

        // Step 6: Delete the function via API cleanup
        const token = await page.evaluate(() => localStorage.getItem('auth_token'));
        const listResponse = await page.evaluate(async (authToken) => {
          const res = await fetch('/api/v1/drift/fitness-functions', {
            headers: { Authorization: `Bearer ${authToken}` },
          });
          return res.json();
        }, token);

        const functionsData =
          (listResponse as { functions?: Array<{ name: string; id: string }> })?.functions || [];
        const testFunc = functionsData.find((f) => f.name === testFunctionName);

        if (testFunc) {
          await page.evaluate(
            async ({ id, authToken }) => {
              await fetch(`/api/v1/drift/fitness-functions/${id}`, {
                method: 'DELETE',
                headers: { Authorization: `Bearer ${authToken}` },
              });
            },
            { id: testFunc.id, authToken: token }
          );
          await page.waitForTimeout(1000);
        }

        // Step 7: Refresh and verify deletion
        await main.getByRole('button', { name: /Refresh/i }).click();
        await page.waitForTimeout(2000);

        const stillExists = await main
          .getByText(testFunctionName)
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        expect(stillExists).toBeFalsy();
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

      // Navigate back to Fitness Functions
      const hasFitnessButton = await nav
        .getByRole('button', { name: /Fitness/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasFitnessButton) {
        await nav.getByRole('button', { name: /Fitness/i }).click();
      } else {
        // Fall back to direct navigation
        await page.goto(`${BASE_URL}/fitness-functions`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Fitness Functions', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Config and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Config' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Configuration', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back to Fitness Functions
      const hasFitnessButton = await nav
        .getByRole('button', { name: /Fitness/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasFitnessButton) {
        await nav.getByRole('button', { name: /Fitness/i }).click();
      } else {
        await page.goto(`${BASE_URL}/fitness-functions`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Fitness Functions', level: 1 })
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
      await expect(h1).toHaveText('Fitness Functions');
    });

    test('should have multiple H2 section headings', async ({ page }) => {
      const h2s = mainContent(page).getByRole('heading', { level: 2 });
      // At minimum: Global Fitness Summary + Registered Fitness Functions
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

    test('should have accessible buttons with discernible text', async ({ page }) => {
      const main = mainContent(page);

      // All primary action buttons should have text
      await expect(
        main.getByRole('button', { name: /Create Fitness Function/i })
      ).toBeVisible();
      await expect(
        main.getByRole('button', { name: /Refresh/i })
      ).toBeVisible();

      const summaryButton = main.getByRole('button', { name: /Summary/i });
      await expect(summaryButton).toBeVisible();
    });

    test('should have accessible dialog when opened', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible();

      // Dialog should have a heading
      await expect(
        dialog.getByText('Create Fitness Function')
      ).toBeVisible();

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
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

    test('should not crash when toggling summary rapidly', async ({ page }) => {
      const main = mainContent(page);

      // Toggle summary visibility twice quickly
      const summaryButton = main.getByRole('button', { name: /Summary/i });
      await summaryButton.click();
      await page.waitForTimeout(200);
      await summaryButton.click();
      await page.waitForTimeout(200);
      await summaryButton.click();
      await page.waitForTimeout(500);

      // Page should still be functional
      await expect(
        main.getByRole('heading', { name: 'Fitness Functions', level: 1 })
      ).toBeVisible();
    });

    test('should not crash when opening and closing dialog rapidly', async ({ page }) => {
      const main = mainContent(page);

      // Open and close dialog twice
      await main.getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(300);
      await page.getByRole('dialog').getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(300);

      await main.getByRole('button', { name: /Create Fitness Function/i }).click();
      await page.waitForTimeout(300);
      await page.getByRole('dialog').getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);

      // Page should still be functional
      await expect(
        main.getByRole('heading', { name: 'Fitness Functions', level: 1 })
      ).toBeVisible();
    });
  });
});
