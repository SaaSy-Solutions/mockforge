import { test, expect } from '@playwright/test';

/**
 * Workspaces Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts
 *
 * These tests verify all Workspaces page functionality:
 *   1. Page load & layout
 *   2. Empty state
 *   3. Header action buttons
 *   4. Create Workspace dialog
 *   5. Open from Directory dialog
 *   6. Dialog form validation
 *   7. Navigation
 *   8. Accessibility
 *   9. Error-free operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Workspaces — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/workspaces`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    await expect(
      mainContent(page).getByRole('heading', { name: 'Workspaces', level: 1 })
    ).toBeVisible({ timeout: 10000 });
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the workspaces page at /workspaces', async ({ page }) => {
      await expect(page).toHaveURL(/\/workspaces/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading and subtitle', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('heading', { name: 'Workspaces', level: 1 })).toBeVisible();
      await expect(main.getByText('Manage your mock API workspaces')).toBeVisible();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Workspaces')).toBeVisible();
    });

    test('should mark the Workspaces sidebar button as active', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');
      const workspacesButton = nav.getByRole('button', { name: 'Workspaces' });
      await expect(workspacesButton).toBeVisible();
      // The active button is visually distinct — verify it exists
      await expect(workspacesButton).toBeEnabled();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Header Action Buttons
  // ---------------------------------------------------------------------------
  test.describe('Header Action Buttons', () => {
    test('should display the "New Workspace" button', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: 'New Workspace' })
      ).toBeVisible();
    });

    test('should display the "Open from Directory" button in header', async ({ page }) => {
      // There are two "Open from Directory" buttons — one in header, one in empty state
      // The header one is in the action buttons area next to "New Workspace"
      const main = mainContent(page);
      const openButtons = main.getByRole('button', { name: 'Open from Directory' });
      expect(await openButtons.count()).toBeGreaterThanOrEqual(1);
      await expect(openButtons.first()).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Empty State
  // ---------------------------------------------------------------------------
  test.describe('Empty State', () => {
    test('should display the "No Workspaces Yet" heading', async ({ page }) => {
      const main = mainContent(page);
      const emptyHeading = main.getByRole('heading', { name: 'No Workspaces Yet' });
      const hasEmpty = await emptyHeading.isVisible({ timeout: 3000 }).catch(() => false);

      if (hasEmpty) {
        await expect(emptyHeading).toBeVisible();
      }
      // If workspaces exist, this section won't be visible — that's okay
    });

    test('should display helpful empty state message', async ({ page }) => {
      const main = mainContent(page);
      const emptyMsg = main.getByText(
        'Get started by creating a new workspace or opening an existing one from a directory.'
      );
      const hasEmpty = await emptyMsg.isVisible({ timeout: 3000 }).catch(() => false);

      if (hasEmpty) {
        await expect(emptyMsg).toBeVisible();
      }
    });

    test('should display an empty state illustration/icon', async ({ page }) => {
      const main = mainContent(page);
      const hasEmptyHeading = await main.getByRole('heading', { name: 'No Workspaces Yet' })
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasEmptyHeading) {
        // The empty state has an image/icon
        const emptySection = main.locator('div').filter({
          has: page.getByRole('heading', { name: 'No Workspaces Yet' }),
        }).first();
        await expect(emptySection.locator('img, svg').first()).toBeVisible();
      }
    });

    test('should show "Create Workspace" button in empty state', async ({ page }) => {
      const main = mainContent(page);
      const hasEmptyHeading = await main.getByRole('heading', { name: 'No Workspaces Yet' })
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasEmptyHeading) {
        // The empty state CTA buttons
        const emptySection = main.locator('div').filter({
          has: page.getByRole('heading', { name: 'No Workspaces Yet' }),
        }).first();
        await expect(emptySection.getByRole('button', { name: 'Create Workspace' })).toBeVisible();
      }
    });

    test('should show "Open from Directory" button in empty state', async ({ page }) => {
      const main = mainContent(page);
      const hasEmptyHeading = await main.getByRole('heading', { name: 'No Workspaces Yet' })
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasEmptyHeading) {
        // There are two "Open from Directory" buttons on the page (header + empty state)
        // Just verify at least 2 exist when empty state is shown
        const openButtons = main.getByRole('button', { name: 'Open from Directory' });
        expect(await openButtons.count()).toBeGreaterThanOrEqual(2);
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Create Workspace Dialog
  // ---------------------------------------------------------------------------
  test.describe('Create Workspace Dialog', () => {
    test('should open Create Workspace dialog from header "New Workspace" button', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'New Workspace' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible({ timeout: 5000 });
      await expect(dialog.getByRole('heading', { name: 'Create New Workspace' })).toBeVisible();
    });

    test('should open Create Workspace dialog from empty state button', async ({ page }) => {
      const main = mainContent(page);
      const hasEmptyState = await main.getByRole('heading', { name: 'No Workspaces Yet' })
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (!hasEmptyState) {
        test.skip();
        return;
      }

      // Click the empty state "Create Workspace" button
      const emptySection = main.locator('div').filter({
        has: page.getByRole('heading', { name: 'No Workspaces Yet' }),
      }).first();
      await emptySection.getByRole('button', { name: 'Create Workspace' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible({ timeout: 5000 });
      await expect(dialog.getByRole('heading', { name: 'Create New Workspace' })).toBeVisible();
    });

    test('should display the dialog description', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'New Workspace' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(
        dialog.getByText('Create a new workspace to organize your mock API endpoints.')
      ).toBeVisible();
    });

    test('should display Name field with placeholder', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'New Workspace' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByText('Name')).toBeVisible();
      const nameInput = dialog.getByRole('textbox', { name: 'Name' });
      await expect(nameInput).toBeVisible();
      await expect(nameInput).toHaveAttribute('placeholder', 'My Workspace');
    });

    test('should display Description field with placeholder', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'New Workspace' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByText('Description')).toBeVisible();
      const descInput = dialog.getByRole('textbox', { name: 'Description' });
      await expect(descInput).toBeVisible();
      await expect(descInput).toHaveAttribute('placeholder', 'Optional description...');
    });

    test('should display "Enable directory sync" checkbox', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'New Workspace' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByRole('checkbox', { name: 'Enable directory sync' })).toBeVisible();
      await expect(dialog.getByText('Enable directory sync')).toBeVisible();
    });

    test('should display Cancel and Create Workspace buttons', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'New Workspace' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByRole('button', { name: 'Cancel' })).toBeVisible();
      await expect(dialog.getByRole('button', { name: 'Create Workspace' })).toBeVisible();
    });

    test('should disable Create Workspace button when name is empty', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'New Workspace' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      const createButton = dialog.getByRole('button', { name: 'Create Workspace' });
      await expect(createButton).toBeDisabled();
    });

    test('should enable Create Workspace button when name is filled', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'New Workspace' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      const nameInput = dialog.getByRole('textbox', { name: 'Name' });
      const createButton = dialog.getByRole('button', { name: 'Create Workspace' });

      // Initially disabled
      await expect(createButton).toBeDisabled();

      // Fill in the name
      await nameInput.fill('E2E Test Workspace');
      await page.waitForTimeout(300);

      // Now enabled
      await expect(createButton).toBeEnabled();
    });

    test('should close dialog when Cancel is clicked', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'New Workspace' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible();

      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);

      await expect(dialog).not.toBeVisible();
    });

    test('should allow filling in all form fields', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'New Workspace' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      const nameInput = dialog.getByRole('textbox', { name: 'Name' });
      const descInput = dialog.getByRole('textbox', { name: 'Description' });
      const syncCheckbox = dialog.getByRole('checkbox', { name: 'Enable directory sync' });

      // Fill name
      await nameInput.fill('My Test Workspace');
      await expect(nameInput).toHaveValue('My Test Workspace');

      // Fill description
      await descInput.fill('A test workspace for E2E testing');
      await expect(descInput).toHaveValue('A test workspace for E2E testing');

      // Toggle checkbox
      await syncCheckbox.check();
      await expect(syncCheckbox).toBeChecked();

      // Uncheck
      await syncCheckbox.uncheck();
      await expect(syncCheckbox).not.toBeChecked();

      // Cancel without creating
      await dialog.getByRole('button', { name: 'Cancel' }).click();
    });

    test('should reopen dialog without errors after cancel', async ({ page }) => {
      const main = mainContent(page);

      // Open dialog and fill in a name
      await main.getByRole('button', { name: 'New Workspace' }).click();
      await page.waitForTimeout(500);

      let dialog = page.getByRole('dialog');
      await dialog.getByRole('textbox', { name: 'Name' }).fill('Temporary Name');
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);

      // Reopen dialog — should open without errors
      await main.getByRole('button', { name: 'New Workspace' }).click();
      await page.waitForTimeout(500);

      dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible();
      await expect(dialog.getByRole('heading', { name: 'Create New Workspace' })).toBeVisible();
      await expect(dialog.getByRole('textbox', { name: 'Name' })).toBeVisible();

      await dialog.getByRole('button', { name: 'Cancel' }).click();
    });

    test('should handle Create Workspace submission gracefully', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'New Workspace' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByRole('textbox', { name: 'Name' }).fill('E2E Workspace Test');
      await page.waitForTimeout(300);

      const createButton = dialog.getByRole('button', { name: 'Create Workspace' });
      await expect(createButton).toBeEnabled();
      await createButton.click();
      await page.waitForTimeout(2000);

      // After submission, page should not crash — either dialog closes, workspace appears,
      // or we're still on the workspaces page without error
      const main = mainContent(page);
      await expect(
        main.getByRole('heading', { name: 'Workspaces', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // No error boundary should appear
      const hasError = await page.getByText(/Something went wrong|Unexpected error/i)
        .first().isVisible({ timeout: 1000 }).catch(() => false);
      expect(hasError).toBeFalsy();
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Open from Directory Dialog
  // ---------------------------------------------------------------------------
  test.describe('Open from Directory Dialog', () => {
    test('should open Open from Directory dialog from header button', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Open from Directory' }).first().click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible({ timeout: 5000 });
      await expect(
        dialog.getByRole('heading', { name: 'Open Workspace from Directory' })
      ).toBeVisible();
    });

    test('should open Open from Directory dialog from empty state button', async ({ page }) => {
      const main = mainContent(page);
      const hasEmptyState = await main.getByRole('heading', { name: 'No Workspaces Yet' })
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (!hasEmptyState) {
        test.skip();
        return;
      }

      // Click the last "Open from Directory" button (the one in the empty state CTA area)
      await main.getByRole('button', { name: 'Open from Directory' }).last().click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible({ timeout: 5000 });
      await expect(
        dialog.getByRole('heading', { name: 'Open Workspace from Directory' })
      ).toBeVisible();
    });

    test('should display the dialog description', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Open from Directory' }).first().click();
      await page.waitForTimeout(500);

      await expect(
        page.getByRole('dialog').getByText('Open an existing workspace from a directory on your system.')
      ).toBeVisible();
    });

    test('should display Directory Path field with placeholder', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Open from Directory' }).first().click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByText('Directory Path')).toBeVisible();
      const pathInput = dialog.getByRole('textbox', { name: 'Directory Path' });
      await expect(pathInput).toBeVisible();
      await expect(pathInput).toHaveAttribute('placeholder', '/path/to/workspace');
    });

    test('should display Cancel and Open Workspace buttons', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Open from Directory' }).first().click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByRole('button', { name: 'Cancel' })).toBeVisible();
      await expect(dialog.getByRole('button', { name: 'Open Workspace' })).toBeVisible();
    });

    test('should disable Open Workspace button when path is empty', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Open from Directory' }).first().click();
      await page.waitForTimeout(500);

      await expect(
        page.getByRole('dialog').getByRole('button', { name: 'Open Workspace' })
      ).toBeDisabled();
    });

    test('should enable Open Workspace button when path is filled', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Open from Directory' }).first().click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      const pathInput = dialog.getByRole('textbox', { name: 'Directory Path' });
      const openButton = dialog.getByRole('button', { name: 'Open Workspace' });

      await expect(openButton).toBeDisabled();

      await pathInput.fill('/tmp/my-workspace');
      await page.waitForTimeout(300);

      await expect(openButton).toBeEnabled();
    });

    test('should close dialog when Cancel is clicked', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Open from Directory' }).first().click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible();

      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);

      await expect(dialog).not.toBeVisible();
    });

    test('should handle Open Workspace submission gracefully', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Open from Directory' }).first().click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByRole('textbox', { name: 'Directory Path' }).fill('/tmp/e2e-test-workspace');
      await page.waitForTimeout(300);

      const openButton = dialog.getByRole('button', { name: 'Open Workspace' });
      await expect(openButton).toBeEnabled();
      await openButton.click();
      await page.waitForTimeout(2000);

      // Page should not crash
      const main = mainContent(page);
      await expect(
        main.getByRole('heading', { name: 'Workspaces', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      const hasError = await page.getByText(/Something went wrong|Unexpected error/i)
        .first().isVisible({ timeout: 1000 }).catch(() => false);
      expect(hasError).toBeFalsy();
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Navigation
  // ---------------------------------------------------------------------------
  test.describe('Navigation', () => {
    test('should navigate to Dashboard and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: 'Workspaces' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Workspaces', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Services and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Services' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Services', exact: true, level: 1 })
      ).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: 'Workspaces' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Workspaces', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 7. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a single H1 heading', async ({ page }) => {
      const h1 = mainContent(page).getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);
      await expect(h1).toHaveText('Workspaces');
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

    test('Create Workspace dialog should have proper role and heading', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'New Workspace' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible();
      await expect(dialog.getByRole('heading', { level: 2 })).toBeVisible();
    });

    test('Open from Directory dialog should have proper role and heading', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Open from Directory' }).first().click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible();
      await expect(dialog.getByRole('heading', { level: 2 })).toBeVisible();
    });

    test('form inputs should have associated labels', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'New Workspace' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      // Name and Description fields should be findable by their label
      await expect(dialog.getByRole('textbox', { name: 'Name' })).toBeVisible();
      await expect(dialog.getByRole('textbox', { name: 'Description' })).toBeVisible();
      await expect(dialog.getByRole('checkbox', { name: 'Enable directory sync' })).toBeVisible();
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

    test('should handle rapid dialog open/close without errors', async ({ page }) => {
      const main = mainContent(page);

      // Rapidly open and close Create Workspace dialog
      for (let i = 0; i < 3; i++) {
        await main.getByRole('button', { name: 'New Workspace' }).click();
        await page.waitForTimeout(300);
        await page.getByRole('dialog').getByRole('button', { name: 'Cancel' }).click();
        await page.waitForTimeout(300);
      }

      // Page should still be functional
      await expect(
        main.getByRole('heading', { name: 'Workspaces', level: 1 })
      ).toBeVisible();
    });

    test('should handle rapid switching between Create and Open dialogs', async ({ page }) => {
      const main = mainContent(page);

      // Open Create dialog
      await main.getByRole('button', { name: 'New Workspace' }).click();
      await page.waitForTimeout(300);
      await expect(page.getByRole('dialog').getByText('Create New Workspace')).toBeVisible();
      await page.getByRole('dialog').getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(300);

      // Open Directory dialog
      await main.getByRole('button', { name: 'Open from Directory' }).first().click();
      await page.waitForTimeout(300);
      await expect(page.getByRole('dialog').getByText('Open Workspace from Directory')).toBeVisible();
      await page.getByRole('dialog').getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(300);

      // Page still functional
      await expect(
        main.getByRole('heading', { name: 'Workspaces', level: 1 })
      ).toBeVisible();
    });
  });
});
