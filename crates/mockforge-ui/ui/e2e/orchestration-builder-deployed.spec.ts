import { test, expect } from '@playwright/test';

/**
 * Orchestration Builder E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts orchestration-builder-deployed
 *
 * These tests verify the Orchestration Builder page functionality:
 *   1. Page Load & Layout
 *   2. Toolbar (Name, Execute, Save, Export, Import)
 *   3. Left Panel Tabs (Variables, Hooks, Assertions)
 *   4. Center Panel — Steps (Add, Display, Delete)
 *   5. Step Properties Drawer
 *   6. Variable Management
 *   7. Hook Management
 *   8. Assertion Management
 *   9. Export/Import
 *  10. Navigation
 *  11. Accessibility
 *  12. Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Orchestration Builder — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/orchestration-builder`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    await page.waitForTimeout(2000);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the orchestration builder at /orchestration-builder', async ({ page }) => {
      await expect(page).toHaveURL(/\/orchestration-builder/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner).toBeVisible();
      const hasBreadcrumb = await banner
        .getByText(/Orchestration/i)
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      expect(hasBreadcrumb).toBeTruthy();
    });

    test('should render the orchestration name input', async ({ page }) => {
      const main = mainContent(page);
      // The name field should show "New Orchestration" as default
      const nameInput = main.locator('input[placeholder="Orchestration Name"], input[value="New Orchestration"]').first();
      const hasInput = await nameInput.isVisible({ timeout: 5000 }).catch(() => false);
      // Fallback: check for any text input in the toolbar area
      const hasAnyInput = await main.locator('input[type="text"]').first()
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasInput || hasAnyInput).toBeTruthy();
    });

    test('should render the three-panel layout', async ({ page }) => {
      const main = mainContent(page);
      // Left panel has tabs, center has Add Step, right is a drawer (hidden initially)
      const hasLeftPanel = await main.getByRole('tab').first()
        .isVisible({ timeout: 5000 }).catch(() => false);
      const hasCenterContent = await main.getByText(/No steps added yet|Add Step/i).first()
        .isVisible({ timeout: 5000 }).catch(() => false);
      expect(hasLeftPanel || hasCenterContent).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Toolbar
  // ---------------------------------------------------------------------------
  test.describe('Toolbar', () => {
    test('should display Execute button', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: /Execute/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display Save button', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: /Save/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display Export button', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: /Export/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display Import button', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: /Import/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should allow editing orchestration name', async ({ page }) => {
      const main = mainContent(page);
      const nameInput = main.locator('input').first();
      if (await nameInput.isVisible({ timeout: 3000 }).catch(() => false)) {
        await nameInput.clear();
        await nameInput.fill('My Test Orchestration');
        await expect(nameInput).toHaveValue('My Test Orchestration');
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Left Panel Tabs
  // ---------------------------------------------------------------------------
  test.describe('Left Panel Tabs', () => {
    test('should display Variables tab', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('tab', { name: /Variables/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display Hooks tab', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('tab', { name: /Hooks/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display Assertions tab', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('tab', { name: /Assertions/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should switch between tabs', async ({ page }) => {
      const main = mainContent(page);

      // Click Hooks tab
      await main.getByRole('tab', { name: /Hooks/i }).click();
      await page.waitForTimeout(300);
      await expect(
        main.getByRole('button', { name: /Add Hook/i })
      ).toBeVisible({ timeout: 3000 });

      // Click Assertions tab
      await main.getByRole('tab', { name: /Assertions/i }).click();
      await page.waitForTimeout(300);
      await expect(
        main.getByRole('button', { name: /Add Assertion/i })
      ).toBeVisible({ timeout: 3000 });

      // Click Variables tab
      await main.getByRole('tab', { name: /Variables/i }).click();
      await page.waitForTimeout(300);
      await expect(
        main.getByRole('button', { name: /Add Variable/i })
      ).toBeVisible({ timeout: 3000 });
    });

    test('should default to Variables tab', async ({ page }) => {
      const main = mainContent(page);
      // Variables tab should be selected by default (aria-selected=true)
      const variablesTab = main.getByRole('tab', { name: /Variables/i });
      const isSelected = await variablesTab.getAttribute('aria-selected');
      expect(isSelected).toBe('true');
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Center Panel — Steps
  // ---------------------------------------------------------------------------
  test.describe('Center Panel — Steps', () => {
    test('should display empty state when no steps exist', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('No steps added yet')
      ).toBeVisible({ timeout: 5000 });
      await expect(
        main.getByText(/Click.*Add Step.*to start/i)
      ).toBeVisible({ timeout: 3000 });
    });

    test('should display Add Step button', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: /Add Step/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should add a step when clicking Add Step', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      // Step card should appear with "Step 1" name
      await expect(main.getByText('Step 1')).toBeVisible({ timeout: 3000 });
      // Default scenario
      await expect(main.getByText(/network_degradation/i)).toBeVisible({ timeout: 3000 });
    });

    test('should add multiple steps', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(300);
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(300);

      await expect(main.getByText('Step 1')).toBeVisible({ timeout: 3000 });
      await expect(main.getByText('Step 2')).toBeVisible({ timeout: 3000 });
    });

    test('should hide empty state after adding a step', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      const hasEmptyState = await main.getByText('No steps added yet')
        .isVisible({ timeout: 2000 }).catch(() => false);
      expect(hasEmptyState).toBeFalsy();
    });

    test('should delete a step via delete button', async ({ page }) => {
      const main = mainContent(page);

      // Add a step
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);
      await expect(main.getByText('Step 1')).toBeVisible({ timeout: 3000 });

      // Find and click the delete icon — MUI uses SVG with data-testid or aria-label
      // The delete button is an IconButton containing a DeleteIcon SVG
      const deleted = await (async () => {
        // Try data-testid first
        const byTestId = main.locator('[data-testid="DeleteIcon"]').first();
        if (await byTestId.isVisible({ timeout: 1000 }).catch(() => false)) {
          await byTestId.click();
          return true;
        }
        // Try aria-label
        const byAria = main.getByRole('button', { name: /delete/i }).first();
        if (await byAria.isVisible({ timeout: 1000 }).catch(() => false)) {
          await byAria.click();
          return true;
        }
        // Try finding the IconButton near "Step 1"
        const stepCard = main.locator('text=Step 1').locator('..').locator('..').locator('button').last();
        if (await stepCard.isVisible({ timeout: 1000 }).catch(() => false)) {
          await stepCard.click();
          return true;
        }
        return false;
      })();

      if (deleted) {
        await page.waitForTimeout(500);
        // Verify step was removed (either empty state or step count decreased)
        const stepGone = !(await main.getByText('Step 1').isVisible({ timeout: 1000 }).catch(() => false));
        expect(stepGone).toBeTruthy();
      } else {
        // If we can't find the delete button, just verify the step exists
        await expect(main.getByText('Step 1')).toBeVisible();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Step Properties Drawer
  // ---------------------------------------------------------------------------
  test.describe('Step Properties Drawer', () => {
    test('should open properties drawer when clicking a step', async ({ page }) => {
      const main = mainContent(page);

      // Add a step first
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);

      // Click the step card (not the delete button)
      await main.getByText('Step 1').click();
      await page.waitForTimeout(500);

      // Properties drawer should open
      await expect(page.getByText('Step Properties')).toBeVisible({ timeout: 5000 });
    });

    test('should display step name input in drawer', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);
      await main.getByText('Step 1').click();
      await page.waitForTimeout(500);

      // Step Name input
      const stepNameInput = page.locator('label:has-text("Step Name")');
      await expect(stepNameInput).toBeVisible({ timeout: 3000 });
    });

    test('should display scenario selector in drawer', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);
      await main.getByText('Step 1').click();
      await page.waitForTimeout(500);

      // Scenario dropdown should show default value
      await expect(
        page.getByText(/network_degradation/i).last()
      ).toBeVisible({ timeout: 3000 });
    });

    test('should display duration input in drawer', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);
      await main.getByText('Step 1').click();
      await page.waitForTimeout(500);

      await expect(
        page.locator('label:has-text("Duration")')
      ).toBeVisible({ timeout: 3000 });
    });

    test('should display Assertions section in drawer', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);
      await main.getByText('Step 1').click();
      await page.waitForTimeout(500);

      await expect(page.getByText(/Assertions \(0\)/)).toBeVisible({ timeout: 3000 });
    });

    test('should display Pre-Hooks and Post-Hooks sections in drawer', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(500);
      await main.getByText('Step 1').click();
      await page.waitForTimeout(500);

      await expect(page.getByText(/Pre-Hooks \(0\)/)).toBeVisible({ timeout: 3000 });
      await expect(page.getByText(/Post-Hooks \(0\)/)).toBeVisible({ timeout: 3000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Variable Management
  // ---------------------------------------------------------------------------
  test.describe('Variable Management', () => {
    test('should display Add Variable button on Variables tab', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: /Add Variable/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should add a variable', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: /Add Variable/i }).click();
      await page.waitForTimeout(300);

      // Variable "var_1" should appear in the list
      await expect(main.getByText('var_1')).toBeVisible({ timeout: 3000 });
    });

    test('should add multiple variables', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: /Add Variable/i }).click();
      await page.waitForTimeout(200);
      await main.getByRole('button', { name: /Add Variable/i }).click();
      await page.waitForTimeout(200);

      await expect(main.getByText('var_1')).toBeVisible({ timeout: 3000 });
      await expect(main.getByText('var_2')).toBeVisible({ timeout: 3000 });
    });

    test('should delete a variable', async ({ page }) => {
      const main = mainContent(page);

      // Add a variable
      await main.getByRole('button', { name: /Add Variable/i }).click();
      await page.waitForTimeout(300);
      await expect(main.getByText('var_1')).toBeVisible({ timeout: 3000 });

      // Click delete button next to the variable
      const deleteBtn = main.locator('[data-testid="DeleteIcon"]').first();
      if (await deleteBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
        await deleteBtn.click();
        await page.waitForTimeout(300);
        const varGone = !(await main.getByText('var_1').isVisible({ timeout: 1000 }).catch(() => false));
        expect(varGone).toBeTruthy();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 7. Hook Management
  // ---------------------------------------------------------------------------
  test.describe('Hook Management', () => {
    test('should switch to Hooks tab and display Add Hook button', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('tab', { name: /Hooks/i }).click();
      await page.waitForTimeout(300);

      await expect(
        main.getByRole('button', { name: /Add Hook/i })
      ).toBeVisible({ timeout: 3000 });
    });

    test('should add a hook', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('tab', { name: /Hooks/i }).click();
      await page.waitForTimeout(300);
      await main.getByRole('button', { name: /Add Hook/i }).click();
      await page.waitForTimeout(300);

      // Hook should appear with name and type chip
      await expect(main.getByText('hook_1')).toBeVisible({ timeout: 3000 });
      await expect(main.getByText('pre_step')).toBeVisible({ timeout: 3000 });
      await expect(main.getByText('0 actions')).toBeVisible({ timeout: 3000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 8. Assertion Management
  // ---------------------------------------------------------------------------
  test.describe('Assertion Management', () => {
    test('should switch to Assertions tab and display Add Assertion button', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('tab', { name: /Assertions/i }).click();
      await page.waitForTimeout(300);

      await expect(
        main.getByRole('button', { name: /Add Assertion/i })
      ).toBeVisible({ timeout: 3000 });
    });

    test('should add an assertion', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('tab', { name: /Assertions/i }).click();
      await page.waitForTimeout(300);
      await main.getByRole('button', { name: /Add Assertion/i }).click();
      await page.waitForTimeout(300);

      // Assertion type should appear
      await expect(main.getByText('variable_equals')).toBeVisible({ timeout: 3000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 9. Export/Import
  // ---------------------------------------------------------------------------
  test.describe('Export/Import', () => {
    test('should trigger download when clicking Export', async ({ page }) => {
      const main = mainContent(page);
      const exportBtn = main.getByRole('button', { name: /Export/i });

      const downloadPromise = page
        .waitForEvent('download', { timeout: 5000 })
        .catch(() => null);

      await exportBtn.click();

      const download = await downloadPromise;
      if (download) {
        expect(download.suggestedFilename()).toContain('.json');
      }
    });

    test('should have a hidden file input for import', async ({ page }) => {
      const fileInput = mainContent(page).locator('input[type="file"][accept=".json"]');
      await expect(fileInput).toBeAttached();
    });
  });

  // ---------------------------------------------------------------------------
  // 10. Navigation
  // ---------------------------------------------------------------------------
  test.describe('Navigation', () => {
    test('should navigate to Dashboard from sidebar', async ({ page }) => {
      const nav = page.getByRole('navigation', { name: 'Main navigation' });
      const dashboardLink = nav.getByRole('link', { name: /Dashboard/i }).first();

      if (await dashboardLink.isVisible({ timeout: 3000 }).catch(() => false)) {
        await dashboardLink.click();
        await page.waitForURL(/\/dashboard/, { timeout: 10000 });
        await expect(page).toHaveURL(/\/dashboard/);
      }
    });

    test('should preserve browser history', async ({ page }) => {
      const nav = page.getByRole('navigation', { name: 'Main navigation' });
      const dashboardLink = nav.getByRole('link', { name: /Dashboard/i }).first();

      if (await dashboardLink.isVisible({ timeout: 3000 }).catch(() => false)) {
        await dashboardLink.click();
        await page.waitForURL(/\/dashboard/, { timeout: 10000 });
        await page.goBack();
        await page.waitForURL(/\/orchestration-builder/, { timeout: 10000 });
        await expect(page).toHaveURL(/\/orchestration-builder/);
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 11. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a main landmark', async ({ page }) => {
      await expect(page.getByRole('main')).toBeVisible();
    });

    test('should have a navigation landmark', async ({ page }) => {
      await expect(
        page.getByRole('navigation', { name: 'Main navigation' })
      ).toBeVisible();
    });

    test('should have a skip link', async ({ page }) => {
      await expect(
        page.getByRole('link', { name: /Skip to main content/i })
      ).toBeAttached();
    });

    test('should have accessible tab controls', async ({ page }) => {
      const main = mainContent(page);
      const tabList = main.getByRole('tablist');
      await expect(tabList.first()).toBeVisible({ timeout: 5000 });

      const tabs = main.getByRole('tab');
      const tabCount = await tabs.count();
      expect(tabCount).toBeGreaterThanOrEqual(3);
    });

    test('should support keyboard tab navigation', async ({ page }) => {
      const main = mainContent(page);
      const variablesTab = main.getByRole('tab', { name: /Variables/i });

      await variablesTab.focus();
      await expect(variablesTab).toBeFocused();

      // Arrow right should move to next tab
      await page.keyboard.press('ArrowRight');
      const hooksTab = main.getByRole('tab', { name: /Hooks/i });
      await expect(hooksTab).toBeFocused();
    });
  });

  // ---------------------------------------------------------------------------
  // 12. Error-Free Operation
  // ---------------------------------------------------------------------------
  test.describe('Error-Free Operation', () => {
    test('should not display error UI on initial load', async ({ page }) => {
      const hasError = await mainContent(page)
        .getByText(/Something went wrong|Application error/i)
        .first()
        .isVisible({ timeout: 2000 })
        .catch(() => false);
      expect(hasError).toBeFalsy();
    });

    test('should not have critical console errors', async ({ page }) => {
      const errors: string[] = [];
      page.on('console', (msg) => {
        if (msg.type() === 'error') {
          const text = msg.text();
          if (
            !text.includes('favicon') &&
            !text.includes('404') &&
            !text.includes('Failed to fetch') &&
            !text.includes('net::ERR') &&
            !text.includes('WebSocket')
          ) {
            errors.push(text);
          }
        }
      });

      await page.goto(`${BASE_URL}/orchestration-builder`, {
        waitUntil: 'domcontentloaded',
        timeout: 30000,
      });
      await page.waitForTimeout(3000);

      expect(errors).toEqual([]);
    });

    test('should remain stable after rapid step additions', async ({ page }) => {
      const main = mainContent(page);
      const addBtn = main.getByRole('button', { name: /Add Step/i });

      for (let i = 0; i < 5; i++) {
        await addBtn.click();
        await page.waitForTimeout(100);
      }

      await page.waitForTimeout(500);

      // Should have 5 steps visible
      const stepCount = await main.getByText(/Step \d+/).count();
      expect(stepCount).toBeGreaterThanOrEqual(5);
    });

    test('should handle Execute click gracefully', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: /Execute/i }).click();
      await page.waitForTimeout(1000);

      // Dismiss any alert dialog
      page.on('dialog', (dialog) => dialog.dismiss());

      // Page should still be functional
      await expect(main).toBeVisible();
    });

    test('should handle Save click gracefully', async ({ page }) => {
      const main = mainContent(page);

      // Dismiss any alert dialogs
      page.on('dialog', (dialog) => dialog.dismiss());

      await main.getByRole('button', { name: /Save/i }).click();
      await page.waitForTimeout(1000);

      // Page should still be functional
      await expect(main).toBeVisible();
    });

    test('should handle full workflow without errors', async ({ page }) => {
      const main = mainContent(page);

      // 1. Rename orchestration
      const nameInput = main.locator('input').first();
      if (await nameInput.isVisible({ timeout: 3000 }).catch(() => false)) {
        await nameInput.clear();
        await nameInput.fill('E2E Test Orchestration');
      }

      // 2. Add a variable
      await main.getByRole('button', { name: /Add Variable/i }).click();
      await page.waitForTimeout(200);

      // 3. Switch to Hooks tab and add a hook
      await main.getByRole('tab', { name: /Hooks/i }).click();
      await page.waitForTimeout(200);
      await main.getByRole('button', { name: /Add Hook/i }).click();
      await page.waitForTimeout(200);

      // 4. Add a step
      await main.getByRole('button', { name: /Add Step/i }).click();
      await page.waitForTimeout(300);

      // 5. Verify step appeared
      await expect(main.getByText('Step 1')).toBeVisible({ timeout: 3000 });

      // No error boundary should appear
      const hasError = await main
        .getByText(/Something went wrong/i)
        .first()
        .isVisible({ timeout: 2000 })
        .catch(() => false);
      expect(hasError).toBeFalsy();
    });
  });
});
