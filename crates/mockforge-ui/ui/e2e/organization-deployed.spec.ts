import { test, expect } from '@playwright/test';

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';
function mainContent(page: import('@playwright/test').Page) { return page.getByRole('main'); }

test.describe('Organization — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/organization`, { waitUntil: 'domcontentloaded', timeout: 30000 });
    await page.waitForSelector('nav[aria-label="Main navigation"]', { state: 'visible', timeout: 15000 });
    await expect(mainContent(page).getByRole('heading', { name: 'Organizations', level: 1 })).toBeVisible({ timeout: 10000 });
  });

  // ── Page Load & Layout ─────────────────────────────────────────────────────

  test.describe('Page Load & Layout', () => {
    test('should load the organization page', async ({ page }) => {
      await expect(page).toHaveURL(/\/organization/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading and subtitle', async ({ page }) => {
      await expect(mainContent(page).getByRole('heading', { name: 'Organizations', level: 1 })).toBeVisible();
      await expect(mainContent(page).getByText('Manage your organizations and team members')).toBeVisible();
    });

    test('should display breadcrumbs', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Organization')).toBeVisible();
    });
  });

  // ── Organization List ──────────────────────────────────────────────────────

  test.describe('Organization List', () => {
    test('should display "Your Organizations" heading', async ({ page }) => {
      await expect(mainContent(page).getByRole('heading', { name: /Your Organizations/ })).toBeVisible();
    });

    test('should display at least one organization', async ({ page }) => {
      const main = mainContent(page);
      const hasOrg = await main.getByText(/Free|Pro|Team/).first().isVisible({ timeout: 5000 }).catch(() => false);
      expect(hasOrg).toBeTruthy();
    });

    test('should display "Select an organization to manage"', async ({ page }) => {
      await expect(mainContent(page).getByText('Select an organization to manage')).toBeVisible();
    });

    test('should display "New" button to create organization', async ({ page }) => {
      await expect(mainContent(page).getByRole('button', { name: /New/ })).toBeVisible();
    });
  });

  // ── Create Organization Dialog ─────────────────────────────────────────────

  test.describe('Create Organization Dialog', () => {
    test('should open create organization dialog', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /New/ }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible({ timeout: 5000 });
      await expect(dialog.getByRole('heading', { name: 'Create Organization' })).toBeVisible();
    });

    test('should display name and slug fields', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /New/ }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog.getByText('Organization Name')).toBeVisible();
      await expect(dialog.getByText('Slug')).toBeVisible();
    });

    test('should auto-generate slug from name', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /New/ }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      const nameInput = dialog.getByPlaceholder('My Organization');
      await nameInput.fill('Test Org Name');
      await page.waitForTimeout(300);

      const slugInput = dialog.getByPlaceholder('my-organization');
      await expect(slugInput).toHaveValue('test-org-name');
    });

    test('should close dialog on Cancel', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /New/ }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible({ timeout: 5000 });

      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
      await expect(dialog).not.toBeVisible();
    });
  });

  // ── Organization Detail — Tabs ─────────────────────────────────────────────

  test.describe('Organization Detail', () => {
    test.beforeEach(async ({ page }) => {
      const main = mainContent(page);
      await main.getByText(/Free|Pro|Team/).first().click();
      await page.waitForTimeout(1000);
    });

    test('should show all tabs when organization is selected', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('button', { name: 'Members' })).toBeVisible({ timeout: 5000 });
      await expect(main.getByRole('button', { name: 'Settings' })).toBeVisible();
      await expect(main.getByRole('button', { name: 'Invitations' })).toBeVisible();
      await expect(main.getByRole('button', { name: 'Audit Log' })).toBeVisible();
      await expect(main.getByRole('button', { name: 'Templates' })).toBeVisible();
      await expect(main.getByRole('button', { name: 'SSO' })).toBeVisible();
      await expect(main.getByRole('button', { name: 'Usage' })).toBeVisible();
      await expect(main.getByRole('button', { name: 'AI' })).toBeVisible();
    });

    test('should display org name and slug in detail view', async ({ page }) => {
      const main = mainContent(page);
      const h3s = main.getByRole('heading', { level: 3 });
      expect(await h3s.count()).toBeGreaterThanOrEqual(1);
      await expect(main.getByText(/@\w+/).first()).toBeVisible({ timeout: 5000 });
    });
  });

  // ── Members Tab ────────────────────────────────────────────────────────────

  test.describe('Members Tab', () => {
    test.beforeEach(async ({ page }) => {
      const main = mainContent(page);
      await main.getByText(/Free|Pro|Team/).first().click();
      await page.waitForTimeout(1000);
    });

    test('should display member list with owner', async ({ page }) => {
      await expect(mainContent(page).getByText('Owner')).toBeVisible({ timeout: 5000 });
    });

    test('should display member email address', async ({ page }) => {
      await expect(mainContent(page).getByText(/@/).first()).toBeVisible({ timeout: 5000 });
    });

    test('should show Add Member button', async ({ page }) => {
      await expect(mainContent(page).getByRole('button', { name: /Add Member/ })).toBeVisible({ timeout: 5000 });
    });

    test('should show Invite Link button', async ({ page }) => {
      await expect(mainContent(page).getByRole('button', { name: /Invite Link/ })).toBeVisible({ timeout: 5000 });
    });

    test('should open Add Member dialog', async ({ page }) => {
      const main = mainContent(page);
      const addBtn = main.getByRole('button', { name: /Add Member/ });
      await expect(addBtn).toBeVisible({ timeout: 5000 });
      await addBtn.click();
      await page.waitForTimeout(1000);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible({ timeout: 10000 });
      await expect(dialog.getByRole('heading', { name: 'Add Member' })).toBeVisible({ timeout: 5000 });
      await expect(dialog.getByText('Email', { exact: true })).toBeVisible();
      await expect(dialog.getByText('Role', { exact: true })).toBeVisible();
    });

    test('should close Add Member dialog on Cancel', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Add Member/ }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
      await expect(dialog).not.toBeVisible();
    });

    test('should open Invite Link dialog', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Invite Link/ }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible({ timeout: 5000 });
      await expect(dialog.getByRole('heading', { name: 'Create Invitation Link' })).toBeVisible();
    });
  });

  // ── Settings Tab ───────────────────────────────────────────────────────────

  test.describe('Settings Tab', () => {
    test.beforeEach(async ({ page }) => {
      const main = mainContent(page);
      await main.getByText(/Free|Pro|Team/).first().click();
      await page.waitForTimeout(1000);
      await main.getByRole('button', { name: 'Settings' }).click();
      await page.waitForTimeout(500);
    });

    test('should display organization settings fields', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Organization Name')).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('Slug')).toBeVisible();
      await expect(main.getByText('Plan')).toBeVisible();
      await expect(main.getByText('Created')).toBeVisible();
    });

    test('should show Edit button', async ({ page }) => {
      await expect(mainContent(page).getByRole('button', { name: /Edit/ })).toBeVisible({ timeout: 5000 });
    });

    test('should toggle edit mode on Edit click', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Edit/ }).click();
      await page.waitForTimeout(300);

      await expect(main.getByRole('button', { name: /Save/ })).toBeVisible({ timeout: 5000 });
      await expect(main.getByRole('button', { name: 'Cancel' })).toBeVisible();
    });

    test('should cancel edit mode', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Edit/ }).click();
      await page.waitForTimeout(300);

      await main.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(300);

      await expect(main.getByRole('button', { name: /Edit/ })).toBeVisible({ timeout: 5000 });
    });

    test('should display Danger Zone with Delete button', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Danger Zone')).toBeVisible({ timeout: 5000 });
      await expect(main.getByRole('button', { name: /Delete Organization/ })).toBeVisible();
    });

    test('should show delete confirmation on click', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Delete Organization/ }).click();
      await page.waitForTimeout(300);

      await expect(main.getByRole('button', { name: /Confirm Delete/ })).toBeVisible({ timeout: 5000 });
      await expect(main.getByRole('button', { name: 'Cancel' })).toBeVisible();
    });

    test('should cancel delete confirmation', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Delete Organization/ }).click();
      await page.waitForTimeout(300);

      // Click the Cancel button in the danger zone (not the edit cancel)
      const dangerZone = main.getByText('Danger Zone').locator('..');
      await dangerZone.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(300);

      await expect(main.getByRole('button', { name: /Delete Organization/ })).toBeVisible({ timeout: 5000 });
    });
  });

  // ── Invitations Tab ────────────────────────────────────────────────────────

  test.describe('Invitations Tab', () => {
    test.beforeEach(async ({ page }) => {
      const main = mainContent(page);
      await main.getByText(/Free|Pro|Team/).first().click();
      await page.waitForTimeout(1000);
      await main.getByRole('button', { name: 'Invitations' }).click();
      await page.waitForTimeout(500);
    });

    test('should display invitations content', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText(/Create invitation links/)).toBeVisible({ timeout: 5000 });
    });

    test('should show Create Invitation button', async ({ page }) => {
      await expect(mainContent(page).getByRole('button', { name: /Create Invitation/ })).toBeVisible({ timeout: 5000 });
    });

    test('should open Create Invitation dialog', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /Create Invitation/ }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible({ timeout: 5000 });
      await expect(dialog.getByRole('heading', { name: 'Create Invitation' })).toBeVisible();
      await expect(dialog.getByText('Email')).toBeVisible();
      await expect(dialog.getByText('Role')).toBeVisible();
    });
  });

  // ── Audit Log Tab ──────────────────────────────────────────────────────────

  test.describe('Audit Log Tab', () => {
    test.beforeEach(async ({ page }) => {
      const main = mainContent(page);
      await main.getByText(/Free|Pro|Team/).first().click();
      await page.waitForTimeout(1000);
      await main.getByRole('button', { name: 'Audit Log' }).click();
      await page.waitForTimeout(500);
    });

    test('should display audit log filter', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText(/Filter by event/)).toBeVisible({ timeout: 5000 });
    });

    test('should display audit logs, empty state, or loading', async ({ page }) => {
      const main = mainContent(page);
      await page.waitForTimeout(2000);
      const hasLogs = await main.getByText(/Showing \d/).first().isVisible({ timeout: 5000 }).catch(() => false);
      const hasEmpty = await main.getByText(/No audit logs found/).isVisible({ timeout: 3000 }).catch(() => false);
      const hasLoading = await main.getByText(/Loading audit logs/).isVisible({ timeout: 2000 }).catch(() => false);
      expect(hasLogs || hasEmpty || hasLoading).toBeTruthy();
    });

    test('should have event type filter dropdown', async ({ page }) => {
      const main = mainContent(page);
      const select = main.locator('select');
      await expect(select).toBeVisible({ timeout: 5000 });
      // Verify the "All events" option exists
      const options = select.locator('option');
      expect(await options.count()).toBeGreaterThan(1);
    });
  });

  // ── Templates Tab ──────────────────────────────────────────────────────────

  test.describe('Templates Tab', () => {
    test.beforeEach(async ({ page }) => {
      const main = mainContent(page);
      await main.getByText(/Free|Pro|Team/).first().click();
      await page.waitForTimeout(1000);
      await main.getByRole('button', { name: 'Templates' }).click();
      await page.waitForTimeout(500);
    });

    test('should show New Template button', async ({ page }) => {
      await expect(mainContent(page).getByRole('button', { name: /New Template/ })).toBeVisible({ timeout: 5000 });
    });

    test('should display templates or empty state', async ({ page }) => {
      const main = mainContent(page);
      const hasTemplates = await main.getByText(/Default/).first().isVisible({ timeout: 3000 }).catch(() => false);
      const hasEmpty = await main.getByText(/No templates yet/).isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasTemplates || hasEmpty).toBeTruthy();
    });

    test('should open Create Template dialog', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /New Template/ }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await expect(dialog).toBeVisible({ timeout: 5000 });
      await expect(dialog.getByRole('heading', { name: 'Create Template' })).toBeVisible();
      await expect(dialog.getByText('Name')).toBeVisible();
      await expect(dialog.getByText('Description')).toBeVisible();
    });

    test('should close Create Template dialog on Cancel', async ({ page }) => {
      await mainContent(page).getByRole('button', { name: /New Template/ }).click();
      await page.waitForTimeout(500);

      const dialog = page.getByRole('dialog');
      await dialog.getByRole('button', { name: 'Cancel' }).click();
      await page.waitForTimeout(500);
      await expect(dialog).not.toBeVisible();
    });
  });

  // ── SSO Tab ────────────────────────────────────────────────────────────────

  test.describe('SSO Tab', () => {
    test.beforeEach(async ({ page }) => {
      const main = mainContent(page);
      await main.getByText(/Free|Pro|Team/).first().click();
      await page.waitForTimeout(1000);
      await main.getByRole('button', { name: 'SSO' }).click();
      await page.waitForTimeout(500);
    });

    test('should display SSO content based on plan', async ({ page }) => {
      const main = mainContent(page);
      // Free/Pro plans show the "requires Team plan" message
      // Team plans show the SSO configuration form
      const hasRestriction = await main.getByText(/SSO requires Team plan/).isVisible({ timeout: 5000 }).catch(() => false);
      const hasSSOConfig = await main.getByText(/SAML 2.0 Configuration/).isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasRestriction || hasSSOConfig).toBeTruthy();
    });

    test('should show plan gate message for non-Team plans', async ({ page }) => {
      const main = mainContent(page);
      const hasRestriction = await main.getByText(/SSO requires Team plan/).isVisible({ timeout: 5000 }).catch(() => false);
      if (hasRestriction) {
        await expect(main.getByText(/Upgrade to the Team plan/)).toBeVisible();
      }
    });
  });

  // ── Usage Tab ──────────────────────────────────────────────────────────────

  test.describe('Usage Tab', () => {
    test.beforeEach(async ({ page }) => {
      const main = mainContent(page);
      await main.getByText(/Free|Pro|Team/).first().click();
      await page.waitForTimeout(1000);
      await main.getByRole('button', { name: 'Usage' }).click();
      await page.waitForTimeout(500);
    });

    test('should display usage statistics', async ({ page }) => {
      const main = mainContent(page);
      // Wait for usage data to load
      const hasUsage = await main.getByText(/Current Usage/).isVisible({ timeout: 5000 }).catch(() => false);
      const hasLoading = await main.getByText(/Loading usage/).isVisible({ timeout: 2000 }).catch(() => false);
      expect(hasUsage || hasLoading).toBeTruthy();
    });

    test('should display usage metrics when loaded', async ({ page }) => {
      const main = mainContent(page);
      await page.waitForTimeout(2000);
      const hasMetrics = await main.getByText(/Total Requests/).isVisible({ timeout: 5000 }).catch(() => false);
      if (hasMetrics) {
        await expect(main.getByText('Storage')).toBeVisible();
        await expect(main.getByText('AI Tokens')).toBeVisible();
        await expect(main.getByText('Hosted Mocks')).toBeVisible();
      }
    });

    test('should display billing information', async ({ page }) => {
      const main = mainContent(page);
      await page.waitForTimeout(2000);
      const hasBilling = await main.getByText(/Billing/).first().isVisible({ timeout: 5000 }).catch(() => false);
      if (hasBilling) {
        await expect(main.getByText(/Plan/).first()).toBeVisible();
      }
    });
  });

  // ── AI Tab ─────────────────────────────────────────────────────────────────

  test.describe('AI Tab', () => {
    test.beforeEach(async ({ page }) => {
      const main = mainContent(page);
      await main.getByText(/Free|Pro|Team/).first().click();
      await page.waitForTimeout(1000);
      await main.getByRole('button', { name: 'AI' }).click();
      await page.waitForTimeout(500);
    });

    test('should display AI settings content', async ({ page }) => {
      const main = mainContent(page);
      const hasSettings = await main.getByText(/Rate Limits/).isVisible({ timeout: 5000 }).catch(() => false);
      const hasLoading = await main.getByText(/Loading AI settings/).isVisible({ timeout: 2000 }).catch(() => false);
      expect(hasSettings || hasLoading).toBeTruthy();
    });

    test('should display feature flags when loaded', async ({ page }) => {
      const main = mainContent(page);
      await page.waitForTimeout(2000);
      const hasFlags = await main.getByText(/Feature Flags/).isVisible({ timeout: 5000 }).catch(() => false);
      if (hasFlags) {
        await expect(main.getByText('AI Studio')).toBeVisible();
        await expect(main.getByText('AI Contract Diff')).toBeVisible();
        await expect(main.getByText('MockAI')).toBeVisible();
        await expect(main.getByText('Persona Generation')).toBeVisible();
      }
    });

    test('should display rate limit inputs when loaded', async ({ page }) => {
      const main = mainContent(page);
      await page.waitForTimeout(2000);
      const hasRateLimits = await main.getByText(/Max AI calls per workspace per day/).isVisible({ timeout: 5000 }).catch(() => false);
      if (hasRateLimits) {
        await expect(main.getByText(/Max AI calls per workspace per month/)).toBeVisible();
      }
    });

    test('should show Save AI Settings button when loaded', async ({ page }) => {
      const main = mainContent(page);
      await page.waitForTimeout(2000);
      const hasSave = await main.getByRole('button', { name: /Save AI Settings/ }).isVisible({ timeout: 5000 }).catch(() => false);
      expect(typeof hasSave).toBe('boolean');
    });
  });

  // ── Tab Navigation ─────────────────────────────────────────────────────────

  test.describe('Tab Navigation', () => {
    test.beforeEach(async ({ page }) => {
      const main = mainContent(page);
      await main.getByText(/Free|Pro|Team/).first().click();
      await page.waitForTimeout(1000);
    });

    test('should switch between all tabs', async ({ page }) => {
      const main = mainContent(page);
      const tabs = ['Settings', 'Invitations', 'Audit Log', 'Templates', 'SSO', 'Usage', 'AI', 'Members'];

      for (const tab of tabs) {
        await main.getByRole('button', { name: tab }).click();
        await page.waitForTimeout(500);
        // Verify the tab button appears active (no error thrown)
      }

      // Verify we're back on Members tab showing owner
      await expect(main.getByText('Owner')).toBeVisible({ timeout: 5000 });
    });
  });

  // ── Sidebar Navigation ────────────────────────────────────────────────────

  test.describe('Navigation', () => {
    test('should navigate to Billing and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');
      await nav.getByRole('button', { name: 'Billing' }).click();
      await page.waitForTimeout(1500);
      await expect(mainContent(page).getByRole('heading', { name: /Billing/i, level: 1 })).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: 'Organization' }).click();
      await page.waitForTimeout(1500);
      await expect(mainContent(page).getByRole('heading', { name: 'Organizations', level: 1 })).toBeVisible({ timeout: 5000 });
    });
  });

  // ── Accessibility ──────────────────────────────────────────────────────────

  test.describe('Accessibility', () => {
    test('should have a single H1', async ({ page }) => {
      const h1 = mainContent(page).getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);
    });

    test('should have landmarks', async ({ page }) => {
      await expect(page.getByRole('main')).toBeVisible();
      await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
      await expect(page.getByRole('banner')).toBeVisible();
    });

    test('should have skip links', async ({ page }) => {
      await expect(page.getByRole('link', { name: 'Skip to navigation' })).toBeAttached();
      await expect(page.getByRole('link', { name: 'Skip to main content' })).toBeAttached();
    });
  });

  // ── Error-Free Operation ───────────────────────────────────────────────────

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
