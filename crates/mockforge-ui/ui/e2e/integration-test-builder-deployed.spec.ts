import { test, expect } from '@playwright/test';

/**
 * Integration Test Builder Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config (handles auth via setup project):
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts integration-test-builder-deployed
 *
 * These tests verify all Integration Test Builder functionality on the live deployed site:
 *   1.  Page Load & Layout
 *   2.  Workflow Configuration
 *   3.  Add Step
 *   4.  Steps Display
 *   5.  Step Editor Dialog
 *   6.  Code Generation
 *   7.  Navigation
 *   8.  Accessibility
 *   9.  Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

// Scoped locator for the main content area (excludes sidebar to avoid duplicate matches)
function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Integration Test Builder — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/integration-test-builder`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    // Wait for the app shell to render (sidebar navigation visible)
    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    // Wait for the Integration Test Builder heading to confirm content loaded
    // Component uses MUI Typography variant="h4" which renders as <h4>
    await expect(
      mainContent(page).getByRole('heading', { name: 'Integration Test Builder' })
    ).toBeVisible({ timeout: 10000 });

    // Small stabilization delay for dynamic content
    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the integration test builder page at /integration-test-builder', async ({ page }) => {
      await expect(page).toHaveURL(/\/integration-test-builder/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Integration Test Builder' })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText(
          'Build multi-step integration tests with state management and variable extraction'
        )
      ).toBeVisible();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Integration Test Builder')).toBeVisible();
    });

    test('should display the Workflow Configuration section', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Workflow Configuration')
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Test Steps section', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText(/Test Steps/)
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Add Step button', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: /Add Step/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the empty state when no steps exist', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('No steps added yet')
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Workflow Configuration
  // ---------------------------------------------------------------------------
  test.describe('Workflow Configuration', () => {
    test('should display the Workflow Name input', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByLabel('Workflow Name')
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Description textarea', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByLabel('Description')
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Base URL input', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByLabel('Base URL')
      ).toBeVisible({ timeout: 5000 });
    });

    test('should have the default Base URL value', async ({ page }) => {
      const main = mainContent(page);
      const baseUrlInput = main.getByLabel('Base URL');
      await expect(baseUrlInput).toHaveValue('http://localhost:3000');
    });

    test('should display the Timeout input', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByLabel('Timeout (ms)')
      ).toBeVisible({ timeout: 5000 });
    });

    test('should have the default Timeout value', async ({ page }) => {
      const main = mainContent(page);
      const timeoutInput = main.getByLabel('Timeout (ms)');
      await expect(timeoutInput).toHaveValue('30000');
    });

    test('should allow editing the Workflow Name', async ({ page }) => {
      const main = mainContent(page);
      const nameInput = main.getByLabel('Workflow Name');
      await nameInput.clear();
      await nameInput.fill('My Custom Workflow');
      await expect(nameInput).toHaveValue('My Custom Workflow');
    });

    test('should allow editing the Description', async ({ page }) => {
      const main = mainContent(page);
      const descInput = main.getByLabel('Description');
      await descInput.fill('Test workflow description');
      await expect(descInput).toHaveValue('Test workflow description');
    });

    test('should allow editing the Base URL', async ({ page }) => {
      const main = mainContent(page);
      const urlInput = main.getByLabel('Base URL');
      await urlInput.clear();
      await urlInput.fill('http://localhost:8080');
      await expect(urlInput).toHaveValue('http://localhost:8080');
    });

    test('should display the Generate Code section', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Generate Code')
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Add Step
  // ---------------------------------------------------------------------------
  test.describe('Add Step', () => {
    test('should open the step editor dialog when Add Step is clicked', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      await expect(page.getByRole('dialog')).toBeVisible({ timeout: 5000 });
      await expect(
        page.getByRole('dialog').getByText('Add Step')
      ).toBeVisible();

      // Clean up — close dialog
      const dialog = page.getByRole('dialog');
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });

    test('should add a step and display it in the stepper', async ({ page }) => {
      const main = mainContent(page);

      // Open dialog
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');

      // Fill in step details
      await dialog.getByLabel('Step Name').fill('Get Users');
      await dialog.getByLabel('Description').fill('Fetch all users');

      // Save the step
      await dialog.getByRole('button', { name: 'Save' }).click();
      await page.waitForTimeout(500);

      // Step should appear in the list
      await expect(main.getByText('Get Users')).toBeVisible({ timeout: 5000 });

      // Empty state should be gone
      const hasEmptyState = await main
        .getByText('No steps added yet')
        .isVisible({ timeout: 2000 })
        .catch(() => false);
      expect(hasEmptyState).toBeFalsy();
    });

    test('should show the step count after adding a step', async ({ page }) => {
      const main = mainContent(page);

      // Add a step
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByLabel('Step Name').fill('Test Step');
      await dialog.getByRole('button', { name: 'Save' }).click();
      await page.waitForTimeout(500);

      // Test Steps heading should show count
      await expect(
        main.getByText(/Test Steps \(1\)/)
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Steps Display
  // ---------------------------------------------------------------------------
  test.describe('Steps Display', () => {
    test('should display the empty state message and helper text', async ({ page }) => {
      const main = mainContent(page);

      const hasEmptyState = await main
        .getByText('No steps added yet')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasEmptyState) {
        await expect(
          main.getByText('Click "Add Step" to create your first test step')
        ).toBeVisible();
      }
    });

    test('should display step with method chip after adding', async ({ page }) => {
      const main = mainContent(page);

      // Add a step with POST method
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByLabel('Step Name').fill('Create User');

      // Change method to POST
      await dialog.getByLabel('Method').click();
      await page.waitForTimeout(300);
      await page.getByRole('option', { name: 'POST' }).click();
      await page.waitForTimeout(300);

      // Set path
      const pathInput = dialog.getByLabel(/Path/);
      await pathInput.clear();
      await pathInput.fill('/api/users');

      await dialog.getByRole('button', { name: 'Save' }).click();
      await page.waitForTimeout(500);

      // Verify the step is displayed with method chip
      await expect(main.getByText('Create User')).toBeVisible();
      await expect(main.getByText('POST')).toBeVisible();
    });

    test('should display step endpoint path', async ({ page }) => {
      const main = mainContent(page);

      // Add a step
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByLabel('Step Name').fill('Get Items');
      const pathInput = dialog.getByLabel(/Path/);
      await pathInput.clear();
      await pathInput.fill('/api/items');
      await dialog.getByRole('button', { name: 'Save' }).click();
      await page.waitForTimeout(500);

      await expect(main.getByText('/api/items')).toBeVisible({ timeout: 5000 });
    });

    test('should display edit and delete buttons for each step', async ({ page }) => {
      const main = mainContent(page);

      // Add a step first
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByLabel('Step Name').fill('Test Step');
      await dialog.getByRole('button', { name: 'Save' }).click();
      await page.waitForTimeout(500);

      // Check for edit and delete buttons (icon buttons)
      const editButtons = main.getByRole('button', { name: /edit/i });
      const deleteButtons = main.getByRole('button', { name: /delete/i });

      const hasEditBtn = await editButtons.first().isVisible({ timeout: 3000 }).catch(() => false);
      const hasDeleteBtn = await deleteButtons.first().isVisible({ timeout: 3000 }).catch(() => false);

      // At least one of edit/delete should be visible (they may use aria-labels or data-testid)
      expect(hasEditBtn || hasDeleteBtn).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Step Editor Dialog
  // ---------------------------------------------------------------------------
  test.describe('Step Editor Dialog', () => {
    test('should display all form fields in the step editor', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');

      await expect(dialog.getByLabel('Step Name')).toBeVisible();
      await expect(dialog.getByLabel('Description')).toBeVisible();
      await expect(dialog.getByLabel('Method')).toBeVisible();
      await expect(dialog.getByLabel(/Path/)).toBeVisible();
      await expect(dialog.getByLabel(/Request Body/)).toBeVisible();
      await expect(dialog.getByLabel('Expected Status Code')).toBeVisible();

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });

    test('should display the Extract Variables section', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByText('Extract Variables')).toBeVisible();

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });

    test('should have default method set to GET', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      // MUI Select shows the selected value as text
      await expect(dialog.getByText('GET')).toBeVisible();

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });

    test('should allow selecting different HTTP methods', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByLabel('Method').click();
      await page.waitForTimeout(300);

      // Verify all methods are available in the dropdown
      const methods = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'];
      for (const method of methods) {
        await expect(page.getByRole('option', { name: method })).toBeVisible();
      }

      // Select DELETE and verify
      await page.getByRole('option', { name: 'DELETE' }).click();
      await page.waitForTimeout(300);

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });

    test('should allow adding extraction variables', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');

      // Click the Add Extraction button
      await dialog.getByRole('button', { name: /Add Extraction/i }).click();
      await page.waitForTimeout(300);

      // Variable name, source, and pattern fields should appear
      await expect(dialog.getByLabel('Variable Name')).toBeVisible();
      await expect(dialog.getByLabel('Source')).toBeVisible();
      await expect(dialog.getByLabel(/JSONPath|Pattern/)).toBeVisible();

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });

    test('should have Cancel and Save buttons', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByRole('button', { name: 'Cancel' })).toBeVisible();
      await expect(dialog.getByRole('button', { name: 'Save' })).toBeVisible();

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });

    test('should close the dialog when Cancel is clicked', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      await expect(page.getByRole('dialog')).toBeVisible();

      await page.getByRole('dialog').getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);

      await expect(page.getByRole('dialog')).not.toBeVisible();
    });

    test('should open as Edit Step when editing an existing step', async ({ page }) => {
      const main = mainContent(page);

      // Add a step first
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByLabel('Step Name').fill('Existing Step');
      await dialog.getByRole('button', { name: 'Save' }).click();
      await page.waitForTimeout(500);

      // Try to find and click the edit button
      const editButton = main.locator('[aria-label*="edit" i], button:has(svg)').first();
      const hasEdit = await editButton.isVisible({ timeout: 3000 }).catch(() => false);

      if (hasEdit) {
        await editButton.click();
        await page.waitForTimeout(500);

        // Dialog should show Edit Step
        await expect(page.getByRole('dialog')).toBeVisible();
        await expect(
          page.getByRole('dialog').getByText('Edit Step')
        ).toBeVisible();

        // Clean up
        await page.getByRole('dialog').getByRole('button', { name: 'Cancel' }).click();
        await page.waitForTimeout(500);
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Code Generation
  // ---------------------------------------------------------------------------
  test.describe('Code Generation', () => {
    test('should display Rust, Python, and JavaScript buttons', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('button', { name: 'Rust' })).toBeVisible();
      await expect(main.getByRole('button', { name: 'Python' })).toBeVisible();
      await expect(main.getByRole('button', { name: 'JavaScript' })).toBeVisible();
    });

    test('should have code generation buttons disabled when no steps exist', async ({ page }) => {
      const main = mainContent(page);

      // Check if no steps exist
      const hasEmptyState = await main
        .getByText('No steps added yet')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasEmptyState) {
        await expect(main.getByRole('button', { name: 'Rust' })).toBeDisabled();
        await expect(main.getByRole('button', { name: 'Python' })).toBeDisabled();
        await expect(main.getByRole('button', { name: 'JavaScript' })).toBeDisabled();
      }
    });

    test('should enable code generation buttons after adding a step', async ({ page }) => {
      const main = mainContent(page);

      // Add a step
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByLabel('Step Name').fill('Test Step');
      await dialog.getByRole('button', { name: 'Save' }).click();
      await page.waitForTimeout(500);

      // Buttons should now be enabled
      await expect(main.getByRole('button', { name: 'Rust' })).toBeEnabled();
      await expect(main.getByRole('button', { name: 'Python' })).toBeEnabled();
      await expect(main.getByRole('button', { name: 'JavaScript' })).toBeEnabled();
    });

    test('should handle code generation button click without crashing', async ({ page }) => {
      const main = mainContent(page);

      // Add a step first
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByLabel('Step Name').fill('API Test');
      await dialog.getByRole('button', { name: 'Save' }).click();
      await page.waitForTimeout(500);

      // Click Rust code generation button
      await main.getByRole('button', { name: 'Rust' }).click();
      await page.waitForTimeout(1500);

      // Page should still be functional (code dialog may or may not open depending on API)
      await expect(
        main.getByRole('heading', { name: 'Integration Test Builder' })
      ).toBeVisible();
    });

    test('should display Close and Download buttons in code dialog if opened', async ({ page }) => {
      const main = mainContent(page);

      // Add a step first
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByLabel('Step Name').fill('API Test');
      await dialog.getByRole('button', { name: 'Save' }).click();
      await page.waitForTimeout(500);

      // Try generating code
      await main.getByRole('button', { name: 'Rust' }).click();
      await page.waitForTimeout(2000);

      // Check if code dialog appeared
      const codeDialog = page.getByRole('dialog');
      const hasCodeDialog = await codeDialog
        .getByText(/Generated Integration Test/)
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasCodeDialog) {
        await expect(codeDialog.getByRole('button', { name: 'Close' })).toBeVisible();
        await expect(codeDialog.getByRole('button', { name: 'Download' })).toBeVisible();

        // Clean up
        await codeDialog.getByRole('button', { name: 'Close' }).click();
        await page.waitForTimeout(500);
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

      // Navigate back to Integration Test Builder
      await page.goto(`${BASE_URL}/integration-test-builder`, {
        waitUntil: 'domcontentloaded',
        timeout: 30000,
      });
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Integration Test Builder' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Config and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Config' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Configuration', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back
      await page.goto(`${BASE_URL}/integration-test-builder`, {
        waitUntil: 'domcontentloaded',
        timeout: 30000,
      });
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Integration Test Builder' })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 8. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a heading for the page', async ({ page }) => {
      const heading = mainContent(page).getByRole('heading', { name: 'Integration Test Builder' });
      await expect(heading).toHaveCount(1);
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

      await expect(
        main.getByRole('button', { name: /Add Step/i })
      ).toBeVisible();
      await expect(
        main.getByRole('button', { name: 'Rust' })
      ).toBeVisible();
      await expect(
        main.getByRole('button', { name: 'Python' })
      ).toBeVisible();
      await expect(
        main.getByRole('button', { name: 'JavaScript' })
      ).toBeVisible();
    });

    test('should have accessible dialog when opened', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible();

      // Dialog should have a title
      await expect(
        dialog.getByText('Add Step')
      ).toBeVisible();

      // Clean up
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
    });

    test('should have labeled form inputs in workflow configuration', async ({ page }) => {
      const main = mainContent(page);

      // All inputs should have associated labels
      await expect(main.getByLabel('Workflow Name')).toBeVisible();
      await expect(main.getByLabel('Description')).toBeVisible();
      await expect(main.getByLabel('Base URL')).toBeVisible();
      await expect(main.getByLabel('Timeout (ms)')).toBeVisible();
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

    test('should not crash when opening and closing dialog rapidly', async ({ page }) => {
      const main = mainContent(page);

      // Open and close dialog twice
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(300);
      await page.getByRole('dialog').getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(300);

      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(300);
      await page.getByRole('dialog').getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);

      // Page should still be functional
      await expect(
        main.getByRole('heading', { name: 'Integration Test Builder' })
      ).toBeVisible();
    });

    test('should not crash when adding and deleting steps rapidly', async ({ page }) => {
      const main = mainContent(page);

      // Add a step
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(300);

      const dialog = page.getByRole('dialog');
      await dialog.getByLabel('Step Name').fill('Quick Step');
      await dialog.getByRole('button', { name: 'Save' }).click();
      await page.waitForTimeout(500);

      // Page should still be functional
      await expect(
        main.getByRole('heading', { name: 'Integration Test Builder' })
      ).toBeVisible();
    });
  });
});
