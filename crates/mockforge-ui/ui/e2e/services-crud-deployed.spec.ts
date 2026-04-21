import { test, expect, type Page } from '@playwright/test';

/**
 * Services CRUD E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * These tests cover the cloud-mode Services page surface introduced by the
 * services wiring pass: workspace picker, Add Service dialog, Edit Service
 * dialog, Delete row action, and the unassign-from-workspace flow.
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts services-crud-deployed
 *
 * These tests exercise real writes against the caller's org. Each created
 * service is suffixed with a random token and is always cleaned up in an
 * `afterEach` — if a test fails before the cleanup runs, the service may be
 * left behind and can be deleted from the UI.
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';
const UNIQUE = () => `e2e-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`;

function mainContent(page: Page) {
  return page.getByRole('main');
}

async function gotoServices(page: Page) {
  await page.goto(`${BASE_URL}/services`, { waitUntil: 'domcontentloaded', timeout: 30_000 });
  await page.waitForSelector('nav[aria-label="Main navigation"]', { state: 'visible', timeout: 15_000 });
  await expect(
    mainContent(page).getByRole('heading', { name: 'Services', exact: true, level: 1 })
  ).toBeVisible({ timeout: 10_000 });
}

async function waitForServicesToLoad(page: Page) {
  // Either a service row, the empty-state card, or the Add Service button
  // should appear within a reasonable window.
  await Promise.race([
    mainContent(page).getByRole('button', { name: /Add Service/i }).waitFor({ timeout: 15_000 }),
    mainContent(page).getByRole('heading', { name: 'No Services' }).waitFor({ timeout: 15_000 }),
  ]);
}

async function deleteServiceIfPresent(page: Page, name: string) {
  // Best-effort cleanup. We swallow errors so one failed cleanup can't fail
  // the suite when the service was never created.
  try {
    page.once('dialog', (d) => void d.accept());
    const row = mainContent(page).locator('div.rounded-lg.border.bg-card', {
      has: page.getByRole('heading', { name, exact: true }),
    });
    const deleteBtn = row.getByRole('button', { name: new RegExp(`^Delete ${name}$`) });
    if (await deleteBtn.isVisible({ timeout: 1_500 }).catch(() => false)) {
      await deleteBtn.click();
      await expect(row).toHaveCount(0, { timeout: 10_000 });
    }
  } catch {
    // ignore
  }
}

test.describe('Services CRUD — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await gotoServices(page);
    await waitForServicesToLoad(page);
  });

  // ---------------------------------------------------------------------------
  // 1. Workspace picker
  // ---------------------------------------------------------------------------
  test.describe('Workspace picker', () => {
    test('renders a workspace filter with "All workspaces" default', async ({ page }) => {
      const select = page.getByLabel('Workspace', { exact: true });
      await expect(select).toBeVisible();
      await expect(select.locator('option', { hasText: 'All workspaces' })).toHaveCount(1);
    });

    test('changing the filter updates the services list', async ({ page }) => {
      const select = page.getByLabel('Workspace', { exact: true });
      const options = await select.locator('option').allTextContents();
      // Need at least one real workspace to exercise the filter.
      test.skip(options.length < 2, 'No workspaces configured — cannot exercise filter.');

      // Pick the first non-"All" option.
      const [, firstWorkspace] = options;
      await select.selectOption({ label: firstWorkspace });
      // Either a scoped list renders or the empty-state text flips to the
      // workspace-scoped copy; both are valid outcomes.
      const scopedEmpty = mainContent(page).getByText(
        /No services in this workspace yet/i
      );
      const anyServiceHeading = mainContent(page)
        .locator('div.rounded-lg.border.bg-card h3')
        .first();
      await Promise.race([
        scopedEmpty.waitFor({ timeout: 10_000 }),
        anyServiceHeading.waitFor({ timeout: 10_000 }),
      ]);
      await select.selectOption({ label: 'All workspaces' });
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Create / Edit / Delete
  // ---------------------------------------------------------------------------
  test.describe('Create / Edit / Delete', () => {
    test('creates, edits, then deletes a service', async ({ page }) => {
      const baseName = `svc-${UNIQUE()}`;
      const renamed = `${baseName}-renamed`;

      try {
        // --- Create ---
        await mainContent(page).getByRole('button', { name: /Add Service/i }).click();
        const createDialog = page.getByRole('dialog');
        await expect(createDialog.getByRole('heading', { name: 'Add Service' })).toBeVisible();
        await createDialog.getByLabel('Name').fill(baseName);
        await createDialog.getByLabel('Description').fill('created by e2e');
        await createDialog.getByLabel('Base URL').fill('https://example.invalid');
        await createDialog.getByRole('button', { name: 'Create Service' }).click();
        await expect(createDialog).toBeHidden({ timeout: 10_000 });

        const createdRow = mainContent(page).locator('div.rounded-lg.border.bg-card', {
          has: page.getByRole('heading', { name: baseName, exact: true }),
        });
        await expect(createdRow).toBeVisible({ timeout: 15_000 });

        // --- Edit ---
        await createdRow.getByRole('button', { name: new RegExp(`^Edit ${baseName}$`) }).click();
        const editDialog = page.getByRole('dialog');
        await expect(editDialog.getByRole('heading', { name: 'Edit Service' })).toBeVisible();
        const nameInput = editDialog.getByLabel('Name');
        await nameInput.fill('');
        await nameInput.fill(renamed);
        await editDialog.getByRole('button', { name: 'Save Changes' }).click();
        await expect(editDialog).toBeHidden({ timeout: 10_000 });

        const renamedRow = mainContent(page).locator('div.rounded-lg.border.bg-card', {
          has: page.getByRole('heading', { name: renamed, exact: true }),
        });
        await expect(renamedRow).toBeVisible({ timeout: 15_000 });

        // --- Delete ---
        page.once('dialog', (d) => void d.accept());
        await renamedRow.getByRole('button', { name: new RegExp(`^Delete ${renamed}$`) }).click();
        await expect(renamedRow).toHaveCount(0, { timeout: 10_000 });
      } finally {
        // Defensive cleanup in case any assertion failed mid-flow.
        await deleteServiceIfPresent(page, baseName);
        await deleteServiceIfPresent(page, renamed);
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Workspace assign / unassign
  // ---------------------------------------------------------------------------
  test.describe('Workspace assignment', () => {
    test('assigning then unassigning a workspace persists across reload', async ({ page }) => {
      const select = page.getByLabel('Workspace', { exact: true });
      const options = await select.locator('option').allTextContents();
      test.skip(options.length < 2, 'No workspaces configured — cannot exercise assignment.');
      const [, firstWorkspace] = options;

      const name = `svc-${UNIQUE()}`;
      try {
        // Create with explicit workspace.
        await mainContent(page).getByRole('button', { name: /Add Service/i }).click();
        const createDialog = page.getByRole('dialog');
        await createDialog.getByLabel('Name').fill(name);
        await createDialog.getByLabel('Workspace').selectOption({ label: firstWorkspace });
        await createDialog.getByRole('button', { name: 'Create Service' }).click();
        await expect(createDialog).toBeHidden({ timeout: 10_000 });

        const row = mainContent(page).locator('div.rounded-lg.border.bg-card', {
          has: page.getByRole('heading', { name, exact: true }),
        });
        await expect(row).toBeVisible({ timeout: 15_000 });

        // It should show up when filtering by that workspace.
        await select.selectOption({ label: firstWorkspace });
        await expect(row).toBeVisible({ timeout: 15_000 });

        // Unassign via Edit dialog.
        await row.getByRole('button', { name: new RegExp(`^Edit ${name}$`) }).click();
        const editDialog = page.getByRole('dialog');
        await editDialog.getByLabel('Workspace').selectOption('');
        await expect(
          editDialog.getByText(/Saving will unassign this service from its current workspace/i)
        ).toBeVisible();
        await editDialog.getByRole('button', { name: 'Save Changes' }).click();
        await expect(editDialog).toBeHidden({ timeout: 10_000 });

        // The filtered list should no longer contain the service.
        await expect(
          mainContent(page).locator('div.rounded-lg.border.bg-card', {
            has: page.getByRole('heading', { name, exact: true }),
          })
        ).toHaveCount(0, { timeout: 15_000 });

        // Switch back to "All workspaces" — it reappears.
        await select.selectOption({ label: 'All workspaces' });
        await expect(
          mainContent(page).locator('div.rounded-lg.border.bg-card', {
            has: page.getByRole('heading', { name, exact: true }),
          })
        ).toBeVisible({ timeout: 15_000 });

        // Reload — still unassigned (visible in "All", hidden in workspace).
        await page.reload({ waitUntil: 'domcontentloaded' });
        await waitForServicesToLoad(page);
        await select.selectOption({ label: firstWorkspace });
        await expect(
          mainContent(page).locator('div.rounded-lg.border.bg-card', {
            has: page.getByRole('heading', { name, exact: true }),
          })
        ).toHaveCount(0, { timeout: 10_000 });
      } finally {
        // Always clean up in "All workspaces" view.
        await page.getByLabel('Workspace', { exact: true })
          .selectOption({ label: 'All workspaces' })
          .catch(() => undefined);
        await deleteServiceIfPresent(page, name);
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Toggle persistence
  // ---------------------------------------------------------------------------
  test.describe('Toggles persist', () => {
    test('disabling a service survives a reload', async ({ page }) => {
      const name = `svc-${UNIQUE()}`;
      try {
        await mainContent(page).getByRole('button', { name: /Add Service/i }).click();
        const createDialog = page.getByRole('dialog');
        await createDialog.getByLabel('Name').fill(name);
        await createDialog.getByRole('button', { name: 'Create Service' }).click();
        await expect(createDialog).toBeHidden({ timeout: 10_000 });

        const row = mainContent(page).locator('div.rounded-lg.border.bg-card', {
          has: page.getByRole('heading', { name, exact: true }),
        });
        await expect(row).toBeVisible({ timeout: 15_000 });

        const toggle = row.getByRole('switch').first();
        await expect(toggle).toHaveAttribute('data-state', 'checked');
        await toggle.click();
        await expect(toggle).toHaveAttribute('data-state', 'unchecked');

        await page.reload({ waitUntil: 'domcontentloaded' });
        await waitForServicesToLoad(page);
        const refreshedToggle = mainContent(page)
          .locator('div.rounded-lg.border.bg-card', {
            has: page.getByRole('heading', { name, exact: true }),
          })
          .getByRole('switch')
          .first();
        await expect(refreshedToggle).toHaveAttribute('data-state', 'unchecked', { timeout: 10_000 });
      } finally {
        await deleteServiceIfPresent(page, name);
      }
    });
  });
});
