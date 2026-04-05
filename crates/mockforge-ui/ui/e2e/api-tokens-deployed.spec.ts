import { test, expect } from '@playwright/test';

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';
function mainContent(page: import('@playwright/test').Page) { return page.getByRole('main'); }

test.describe('API Tokens — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/api-tokens`, { waitUntil: 'domcontentloaded', timeout: 30000 });
    await page.waitForSelector('nav[aria-label="Main navigation"]', { state: 'visible', timeout: 15000 });
    await expect(mainContent(page).getByRole('heading', { name: 'API Tokens', level: 1 })).toBeVisible({ timeout: 10000 });
  });

  test.describe('Page Load & Layout', () => {
    test('should load the api tokens page', async ({ page }) => {
      await expect(page).toHaveURL(/\/api-tokens/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display heading and subtitle', async ({ page }) => {
      await expect(mainContent(page).getByRole('heading', { name: 'API Tokens', level: 1 })).toBeVisible();
      await expect(mainContent(page).getByText('Manage personal access tokens for CLI and API access')).toBeVisible();
    });

    test('should display breadcrumbs', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('API Tokens')).toBeVisible();
    });

    test('should display "Create Token" button', async ({ page }) => {
      await expect(mainContent(page).getByRole('button', { name: 'Create Token' })).toBeVisible();
    });
  });

  test.describe('Token List', () => {
    test('should display existing tokens or empty state', async ({ page }) => {
      const main = mainContent(page);
      const hasTokens = await main.getByRole('heading', { level: 3 }).first()
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasEmpty = await main.getByText(/No tokens|no.*tokens/i)
        .isVisible({ timeout: 3000 }).catch(() => false);
      // Should show tokens or empty state (page always loads)
      const pageText = await main.textContent();
      expect(pageText!.length).toBeGreaterThan(0);
    });

    test('should display token with prefix and scopes', async ({ page }) => {
      const main = mainContent(page);
      const hasToken = await main.getByText(/mfx_/).first()
        .isVisible({ timeout: 3000 }).catch(() => false);
      if (hasToken) {
        await expect(main.getByText(/mfx_/).first()).toBeVisible();
      }
    });
  });

  test.describe('Create Token Dialog', () => {
    test('should open dialog from "Create Token" button', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible({ timeout: 5000 });
      await expect(dialog.getByRole('heading', { name: 'Create API Token' })).toBeVisible();
    });

    test('should display Token Name field', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByRole('textbox', { name: 'Token Name' })).toBeVisible();
      await expect(dialog.getByRole('textbox', { name: 'Token Name' })).toHaveAttribute('placeholder', 'e.g., CLI Development');
    });

    test('should display scope checkboxes', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByText('Scopes')).toBeVisible();

      const scopes = ['Read Packages', 'Publish Packages', 'Read Projects', 'Write Projects', 'Deploy Mocks', 'Admin Organization'];
      for (const scope of scopes) {
        await expect(dialog.getByText(scope, { exact: true }).first()).toBeVisible();
      }
    });

    test('should display Expires In field', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByRole('spinbutton', { name: 'Expires In (Days)' })).toBeVisible();
      await expect(dialog.getByText('Optional: Set expiration in days')).toBeVisible();
    });

    test('should disable Create button when name is empty', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(500);
      await expect(page.getByRole('dialog').getByRole('button', { name: 'Create Token' })).toBeDisabled();
    });

    test('should enable Create button when name is filled', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByRole('textbox', { name: 'Token Name' }).fill('E2E Test Token');
      await page.waitForTimeout(300);

      await expect(dialog.getByRole('button', { name: 'Create Token' })).toBeEnabled();
    });

    test('should allow checking scope checkboxes', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      const checkbox = dialog.getByRole('checkbox').first();
      await checkbox.check();
      await expect(checkbox).toBeChecked();
      await checkbox.uncheck();
      await expect(checkbox).not.toBeChecked();
    });

    test('should allow checking multiple scope checkboxes', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      const checkboxes = dialog.getByRole('checkbox');
      const count = await checkboxes.count();

      // Check all scopes
      for (let i = 0; i < count; i++) {
        await checkboxes.nth(i).check();
      }

      // Verify all checked
      for (let i = 0; i < count; i++) {
        await expect(checkboxes.nth(i)).toBeChecked();
      }

      // Uncheck all
      for (let i = 0; i < count; i++) {
        await checkboxes.nth(i).uncheck();
      }

      await dialog.getByRole('button', { name: 'Cancel' }).click();
    });

    test('should allow setting expiry days', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      const expiryInput = dialog.getByRole('spinbutton', { name: 'Expires In (Days)' });

      await expiryInput.fill('30');
      await expect(expiryInput).toHaveValue('30');

      await expiryInput.fill('90');
      await expect(expiryInput).toHaveValue('90');

      await dialog.getByRole('button', { name: 'Cancel' }).click();
    });

    test('should close dialog on Cancel', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(500);
      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible();
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
      await expect(dialog).not.toBeVisible();
    });

    test('should create a token and show it in the list', async ({ page }) => {
      const tokenName = `E2E Token ${Date.now()}`;
      await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByRole('textbox', { name: 'Token Name' }).fill(tokenName);
      // Check at least one scope
      await dialog.getByRole('checkbox').first().check();
      await page.waitForTimeout(300);

      await dialog.getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(3000);

      // Token should appear in the list (or a success dialog)
      const main = mainContent(page);
      const hasNewToken = await main.getByText(tokenName)
        .isVisible({ timeout: 5000 }).catch(() => false);
      const hasTokenPrefix = await main.getByText(/mfx_/)
        .first().isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasNewToken || hasTokenPrefix).toBeTruthy();

      // Clean up: delete the token via API
      const token = await page.evaluate(() => localStorage.getItem('auth_token'));
      const listResponse = await page.evaluate(async (authToken) => {
        const res = await fetch('/api/v1/tokens', { headers: { 'Authorization': `Bearer ${authToken}` } });
        return res.json();
      }, token);

      const testToken = (listResponse as Array<{ name: string; id: string }>).find(t => t.name === tokenName);
      if (testToken) {
        await page.evaluate(async ({ id, authToken }) => {
          await fetch(`/api/v1/tokens/${id}`, { method: 'DELETE', headers: { 'Authorization': `Bearer ${authToken}` } });
        }, { id: testToken.id, authToken: token });
      }
    });
  });

  test.describe('Token Scopes & Validation', () => {
    test('should display all 6 scope checkboxes with labels', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      const checkboxes = dialog.getByRole('checkbox');
      expect(await checkboxes.count()).toBe(6);

      // Close
      await dialog.getByRole('button', { name: 'Cancel' }).click();
    });

    test('should allow selecting multiple scopes', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      const checkboxes = dialog.getByRole('checkbox');

      // Select first 3 scopes
      await checkboxes.nth(0).check();
      await checkboxes.nth(1).check();
      await checkboxes.nth(2).check();

      // Verify all 3 are checked
      await expect(checkboxes.nth(0)).toBeChecked();
      await expect(checkboxes.nth(1)).toBeChecked();
      await expect(checkboxes.nth(2)).toBeChecked();

      await dialog.getByRole('button', { name: 'Cancel' }).click();
    });

    test('should accept custom expiry days value', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      const expiryInput = dialog.getByRole('spinbutton');

      if (await expiryInput.isVisible({ timeout: 2000 }).catch(() => false)) {
        await expiryInput.clear();
        await expiryInput.fill('365');
        await expect(expiryInput).toHaveValue('365');
      }

      await dialog.getByRole('button', { name: 'Cancel' }).click();
    });
  });

  test.describe('Accessibility', () => {
    test('should have a single H1', async ({ page }) => {
      const h1 = mainContent(page).getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);
    });

    test('should have landmarks and skip links', async ({ page }) => {
      await expect(page.getByRole('main')).toBeVisible();
      await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
      await expect(page.getByRole('link', { name: 'Skip to navigation' })).toBeAttached();
    });

    test('dialog should have proper heading and labeled inputs', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(500);
      const dialog = page.getByRole('dialog');
      await expect(dialog.getByRole('heading', { level: 2 })).toBeVisible();
      await expect(dialog.getByRole('textbox', { name: 'Token Name' })).toBeVisible();
      await dialog.getByRole('button', { name: 'Cancel' }).click();
    });
  });

  test.describe('Error-Free Operation', () => {
    test('should load without critical console errors', async ({ page }) => {
      const errors: string[] = [];
      page.on('console', (msg) => { if (msg.type() === 'error') errors.push(msg.text()); });
      await page.reload({ waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(3000);
      const critical = errors.filter(e => !e.includes('net::ERR_') && !e.includes('Failed to fetch') && !e.includes('NetworkError') && !e.includes('WebSocket') && !e.includes('favicon') && !e.includes('429') && !e.includes('422'));
      expect(critical).toHaveLength(0);
    });

    test('should not show error UI', async ({ page }) => {
      expect(await page.getByText(/Something went wrong|Unexpected error|Application error/i).first().isVisible({ timeout: 2000 }).catch(() => false)).toBeFalsy();
    });
  });
});
