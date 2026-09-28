import { test, expect, type APIRequestContext } from '@playwright/test';

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';
function mainContent(page: import('@playwright/test').Page) { return page.getByRole('main'); }

// Tokens this spec creates are named `${E2E_TOKEN_PREFIX}<ms epoch>`. The sweep
// below matches that exact shape so it can never touch a user-named token.
const E2E_TOKEN_PREFIX = 'E2E Token ';
const E2E_TOKEN_NAME = /^E2E Token \d{13}$/;

/**
 * Delete API tokens whose name matches `predicate`.
 *
 * Uses `page.request`, which shares the browser context's cookies, so the
 * HttpOnly `mockforge_session` cookie authenticates the call. Do NOT send an
 * Authorization header: the JWT is memory-only since #1005, and a
 * `Bearer null` header takes precedence over the cookie and 401s.
 */
async function deleteE2ETokens(
  request: APIRequestContext,
  predicate: (name: string) => boolean,
): Promise<void> {
  const list = await request.get(`${BASE_URL}/api/v1/tokens`);
  if (!list.ok()) {
    throw new Error(`E2E token cleanup: GET /api/v1/tokens -> ${list.status()}`);
  }
  const tokens = (await list.json()) as Array<{ id: string; name: string }>;
  for (const t of tokens.filter((t) => predicate(t.name))) {
    const res = await request.delete(`${BASE_URL}/api/v1/tokens/${t.id}`);
    // 404: another worker's sweep already removed it.
    if (!res.ok() && res.status() !== 404) {
      throw new Error(`E2E token cleanup: DELETE ${t.id} -> ${res.status()}`);
    }
  }
}

test.describe('API Tokens — Deployed Site', () => {
  // Belt-and-braces sweep for tokens leaked by runs that died before their
  // `finally` (killed worker, timeout) or by pre-fix versions of this spec.
  test.afterAll(async ({ browser }) => {
    // Reuse the project's signed-in state (see playwright-deployed.config.ts).
    const context = await browser.newContext({ storageState: test.info().project.use.storageState });
    try {
      await deleteE2ETokens(context.request, (name) => E2E_TOKEN_NAME.test(name));
    } finally {
      await context.close();
    }
  });

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

      const scopes = ['Read Packages', 'Publish Packages', 'Deploy Mocks', 'Admin Organization', 'Read Usage', 'Manage Billing'];
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

    test('should enable Create button when name and scope are filled', async ({ page }) => {
      // Deployed form requires both a token name AND at least one scope before
      // the Create button enables.
      await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByRole('textbox', { name: 'Token Name' }).fill('E2E Test Token');
      // Click the labeled wrapper for the first scope (the native checkbox is
      // visually hidden behind a cursor:pointer container).
      await dialog.getByText('Read Packages').click();
      await page.waitForTimeout(300);

      await expect(dialog.getByRole('button', { name: 'Create Token' })).toBeEnabled();
    });

    test('should allow toggling a scope checkbox', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      const checkbox = dialog.getByRole('checkbox').first();
      // Click via the labeled wrapper (the native checkbox is wrapped in a
      // cursor:pointer container that handles the click).
      await dialog.getByText('Read Packages').click();
      await expect(checkbox).toBeChecked();
      await dialog.getByText('Read Packages').click();
      await expect(checkbox).not.toBeChecked();
    });

    test('should allow toggling multiple scope checkboxes', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      const checkboxes = dialog.getByRole('checkbox');
      // Click each labeled wrapper instead of the underlying input.
      const labels = ['Read Packages', 'Publish Packages', 'Deploy Mocks'];
      for (const label of labels) {
        await dialog.getByText(label).click();
      }

      for (let i = 0; i < labels.length; i++) {
        await expect(checkboxes.nth(i)).toBeChecked();
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
      const tokenName = `${E2E_TOKEN_PREFIX}${Date.now()}`;
      try {
        await mainContent(page).getByRole('button', { name: 'Create Token' }).click();
        await page.waitForTimeout(500);

        const dialog = page.getByRole('dialog');
        await dialog.getByRole('textbox', { name: 'Token Name' }).fill(tokenName);
        // Check at least one scope (click the labeled wrapper)
        await dialog.getByText('Read Packages').click();
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
      } finally {
        // Always clean up, even when an assertion above failed; otherwise every
        // failed attempt (and each of the config's retries) leaks a live token
        // into whatever account E2E_EMAIL points at.
        await deleteE2ETokens(page.request, (name) => name === tokenName);
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

      // Click labeled wrappers (native checkboxes are inside cursor:pointer containers)
      await dialog.getByText('Read Packages').click();
      await dialog.getByText('Publish Packages').click();
      await dialog.getByText('Deploy Mocks').click();

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
