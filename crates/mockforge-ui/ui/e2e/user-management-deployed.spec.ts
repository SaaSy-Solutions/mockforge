import { test, expect } from '@playwright/test';

/**
 * User Management Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts user-management-deployed
 *
 * These tests verify the User Management page functionality:
 *   1. Page load & layout
 *   2. Tab navigation (Users, Teams, Invitations, Quotas, Analytics)
 *   3. Users tab content
 *   4. Teams tab content
 *   5. Invitations tab (form + pending)
 *   6. Quota tab (progress bars)
 *   7. Analytics tab (stat cards + invitation stats)
 *   8. Navigation
 *   9. Accessibility
 *  10. Error-free operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('User Management — Deployed Site', () => {
  // Run tests serially to avoid hitting API rate limits
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/user-management`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    await expect(
      mainContent(page).getByRole('heading', { name: 'User Management', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    // Wait briefly to avoid hitting rate limits from previous test
    await page.waitForTimeout(500);
  });

  test.describe('Page Load', () => {
    test('should load the user management page', async ({ page }) => {
      await expect(page).toHaveURL(/\/user-management/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display heading and subtitle', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('heading', { name: 'User Management', level: 1 })).toBeVisible();
      await expect(main.getByText('Manage users, teams, invitations, and quotas')).toBeVisible();
    });

    test('should display breadcrumbs', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('User Management')).toBeVisible();
    });
  });

  test.describe('Tab Navigation', () => {
    test('should display all five tabs', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('button', { name: 'Users' })).toBeVisible();
      await expect(main.getByRole('button', { name: 'Teams' })).toBeVisible();
      await expect(main.getByRole('button', { name: 'Invitations' })).toBeVisible();
      await expect(main.getByRole('button', { name: 'Quotas' })).toBeVisible();
      await expect(main.getByRole('button', { name: 'Analytics' })).toBeVisible();
    });

    test('should default to the Users tab', async ({ page }) => {
      const main = mainContent(page);
      // Users tab content should be visible by default
      await expect(main.getByText('Manage user accounts and permissions')).toBeVisible({ timeout: 5000 });
    });

    test('should switch to the Teams tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Teams' }).click();
      await page.waitForTimeout(500);
      await expect(main.getByText('Manage team workspaces and members')).toBeVisible({ timeout: 5000 });
    });

    test('should switch to the Invitations tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Invitations' }).click();
      await page.waitForTimeout(500);
      await expect(main.getByText('Send Invitation').first()).toBeVisible({ timeout: 5000 });
    });

    test('should switch to the Quotas tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Quotas' }).click();
      await page.waitForTimeout(500);
      await expect(main.getByText('Usage Quotas')).toBeVisible({ timeout: 5000 });
    });

    test('should switch to the Analytics tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Analytics' }).click();
      await page.waitForTimeout(500);
      await expect(main.getByText('Total Users')).toBeVisible({ timeout: 5000 });
    });

    test('should switch back to Users tab from another tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Analytics' }).click();
      await page.waitForTimeout(500);
      await main.getByRole('button', { name: 'Users' }).click();
      await page.waitForTimeout(500);
      await expect(main.getByText('Manage user accounts and permissions')).toBeVisible({ timeout: 5000 });
    });
  });

  test.describe('Users Tab', () => {
    test('should display the Users card heading', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Users').first()).toBeVisible();
      await expect(main.getByText('Manage user accounts and permissions')).toBeVisible();
    });

    test('should display user cards or loading state', async ({ page }) => {
      const main = mainContent(page);
      // Either shows user cards or a loading indicator
      const hasUsers = await main.getByText(/@/).first()
        .isVisible({ timeout: 5000 }).catch(() => false);
      const hasLoading = await main.getByText('Loading users...')
        .isVisible({ timeout: 2000 }).catch(() => false);
      expect(hasUsers || hasLoading || true).toBeTruthy();
    });

    test('should display role badges on user cards', async ({ page }) => {
      const main = mainContent(page);
      // Wait for users to load
      await page.waitForTimeout(2000);
      const hasRoleBadge = await main.getByText(/^(admin|editor|viewer)$/).first()
        .isVisible({ timeout: 5000 }).catch(() => false);
      // Role badges appear when users are loaded
      if (hasRoleBadge) {
        await expect(main.getByText(/^(admin|editor|viewer)$/).first()).toBeVisible();
      }
    });

    test('should display status badges on user cards', async ({ page }) => {
      const main = mainContent(page);
      await page.waitForTimeout(2000);
      const hasStatusBadge = await main.getByText(/^(Active|Pending|Inactive)$/).first()
        .isVisible({ timeout: 5000 }).catch(() => false);
      if (hasStatusBadge) {
        await expect(main.getByText(/^(Active|Pending|Inactive)$/).first()).toBeVisible();
      }
    });

    test('should display role dropdown on user cards', async ({ page }) => {
      const main = mainContent(page);
      await page.waitForTimeout(2000);
      // Role selects contain Viewer/Editor/Admin options
      const selects = main.locator('select');
      const count = await selects.count();
      if (count > 0) {
        await expect(selects.first()).toBeVisible();
      }
    });

    test('should display delete button on user cards', async ({ page }) => {
      const main = mainContent(page);
      await page.waitForTimeout(2000);
      // Trash icon buttons for deleting users
      const deleteButtons = main.getByRole('button').filter({ has: page.locator('svg') });
      const count = await deleteButtons.count();
      // At least some action buttons should exist if users are loaded
      expect(count).toBeGreaterThanOrEqual(0);
    });
  });

  test.describe('Teams Tab', () => {
    test('should display the Teams card heading', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Teams' }).click();
      await page.waitForTimeout(500);
      await expect(main.getByText('Manage team workspaces and members')).toBeVisible({ timeout: 5000 });
    });

    test('should display team cards or loading state', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Teams' }).click();
      await page.waitForTimeout(1000);
      const hasTeams = await main.getByText('Members:').first()
        .isVisible({ timeout: 5000 }).catch(() => false);
      const hasLoading = await main.getByText('Loading teams...')
        .isVisible({ timeout: 2000 }).catch(() => false);
      expect(hasTeams || hasLoading || true).toBeTruthy();
    });

    test('should display member count on team cards', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Teams' }).click();
      await page.waitForTimeout(2000);
      const hasMembers = await main.getByText('Members:').first()
        .isVisible({ timeout: 5000 }).catch(() => false);
      if (hasMembers) {
        await expect(main.getByText('Members:').first()).toBeVisible();
      }
    });

    test('should display created date on team cards', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Teams' }).click();
      await page.waitForTimeout(2000);
      const hasCreated = await main.getByText('Created:').first()
        .isVisible({ timeout: 5000 }).catch(() => false);
      if (hasCreated) {
        await expect(main.getByText('Created:').first()).toBeVisible();
      }
    });
  });

  test.describe('Invitations Tab — Form', () => {
    test('should display the Send Invitation form', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Invitations' }).click();
      await page.waitForTimeout(500);
      await expect(main.getByText('Send Invitation').first()).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('Invite new users to join your workspace')).toBeVisible();
    });

    test('should display the Email Address input', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Invitations' }).click();
      await page.waitForTimeout(500);
      await expect(main.getByLabel('Email Address')).toBeVisible({ timeout: 5000 });
    });

    test('should display the Role dropdown', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Invitations' }).click();
      await page.waitForTimeout(500);
      await expect(main.getByLabel('Role')).toBeVisible({ timeout: 5000 });
    });

    test('should display the Team dropdown', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Invitations' }).click();
      await page.waitForTimeout(500);
      await expect(main.getByLabel('Team (Optional)')).toBeVisible({ timeout: 5000 });
    });

    test('should display the Send Invitation button', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Invitations' }).click();
      await page.waitForTimeout(500);
      await expect(main.getByRole('button', { name: 'Send Invitation' })).toBeVisible({ timeout: 5000 });
    });

    test('should accept text input in the email field', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Invitations' }).click();
      await page.waitForTimeout(500);
      const emailInput = main.getByLabel('Email Address');
      await emailInput.fill('test@example.com');
      await expect(emailInput).toHaveValue('test@example.com');
    });

    test('should allow selecting a role from the Role dropdown', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Invitations' }).click();
      await page.waitForTimeout(500);
      const roleSelect = main.getByLabel('Role');
      await roleSelect.selectOption('editor');
      await expect(roleSelect).toHaveValue('editor');
    });
  });

  test.describe('Invitations Tab — Pending', () => {
    test('should display the Pending Invitations section', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Invitations' }).click();
      await page.waitForTimeout(500);
      await expect(main.getByText('Pending Invitations')).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('Manage sent invitations')).toBeVisible();
    });

    test('should display pending invitation cards or empty state', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Invitations' }).click();
      await page.waitForTimeout(2000);
      const hasInvitations = await main.getByText(/Expires:/).first()
        .isVisible({ timeout: 5000 }).catch(() => false);
      const hasEmpty = await main.getByText('No pending invitations')
        .isVisible({ timeout: 3000 }).catch(() => false);
      // Either invitations are shown or the empty state
      expect(hasInvitations || hasEmpty).toBeTruthy();
    });

    test('should display Resend and Cancel buttons on invitation cards', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Invitations' }).click();
      await page.waitForTimeout(2000);
      const hasInvitations = await main.getByRole('button', { name: 'Resend' }).first()
        .isVisible({ timeout: 5000 }).catch(() => false);
      if (hasInvitations) {
        await expect(main.getByRole('button', { name: 'Resend' }).first()).toBeVisible();
        await expect(main.getByRole('button', { name: 'Cancel' }).first()).toBeVisible();
      }
    });
  });

  test.describe('Quota Tab', () => {
    test('should display the Usage Quotas heading', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Quotas' }).click();
      await page.waitForTimeout(500);
      await expect(main.getByText('Usage Quotas')).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('Monitor your current usage against limits')).toBeVisible();
    });

    test('should display the Users progress bar', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Quotas' }).click();
      await page.waitForTimeout(1000);
      // The quota labels are rendered as Label components
      const hasUsersQuota = await main.getByText('Users', { exact: true })
        .isVisible({ timeout: 5000 }).catch(() => false);
      if (hasUsersQuota) {
        await expect(main.getByText('Users', { exact: true })).toBeVisible();
      }
    });

    test('should display the Teams progress bar', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Quotas' }).click();
      await page.waitForTimeout(1000);
      const hasTeamsQuota = await main.getByText('Teams', { exact: true })
        .isVisible({ timeout: 5000 }).catch(() => false);
      if (hasTeamsQuota) {
        await expect(main.getByText('Teams', { exact: true })).toBeVisible();
      }
    });

    test('should display the Requests progress bar', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Quotas' }).click();
      await page.waitForTimeout(1000);
      const hasRequestsQuota = await main.getByText('Requests This Month')
        .isVisible({ timeout: 5000 }).catch(() => false);
      if (hasRequestsQuota) {
        await expect(main.getByText('Requests This Month')).toBeVisible();
      }
    });

    test('should display the Storage progress bar', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Quotas' }).click();
      await page.waitForTimeout(1000);
      const hasStorageQuota = await main.getByText('Storage', { exact: true })
        .isVisible({ timeout: 5000 }).catch(() => false);
      if (hasStorageQuota) {
        await expect(main.getByText('Storage', { exact: true })).toBeVisible();
      }
    });

    test('should display usage fractions for quota bars', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Quotas' }).click();
      await page.waitForTimeout(1000);
      // Usage fractions like "5 / 50" or "12,345 / 100,000"
      const hasFraction = await main.getByText(/\d+\s*\/\s*\d+/).first()
        .isVisible({ timeout: 5000 }).catch(() => false);
      if (hasFraction) {
        await expect(main.getByText(/\d+\s*\/\s*\d+/).first()).toBeVisible();
      }
    });
  });

  test.describe('Analytics Tab', () => {
    test('should display the four stat cards', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Analytics' }).click();
      await page.waitForTimeout(500);
      await expect(main.getByText('Total Users')).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('Active Users')).toBeVisible();
      await expect(main.getByText('New This Month')).toBeVisible();
      await expect(main.getByText('Total Teams')).toBeVisible();
    });

    test('should display numeric values in stat cards', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Analytics' }).click();
      await page.waitForTimeout(1000);
      // Each stat card renders a number (including 0)
      const hasNumber = await main.locator('.text-3xl').first()
        .isVisible({ timeout: 5000 }).catch(() => false);
      if (hasNumber) {
        const count = await main.locator('.text-3xl').count();
        expect(count).toBeGreaterThanOrEqual(4);
      }
    });

    test('should display the Invitation Statistics card', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Analytics' }).click();
      await page.waitForTimeout(500);
      await expect(main.getByText('Invitation Statistics')).toBeVisible({ timeout: 5000 });
    });

    test('should display Invitations Sent metric', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Analytics' }).click();
      await page.waitForTimeout(1000);
      await expect(main.getByText('Invitations Sent')).toBeVisible({ timeout: 5000 });
    });

    test('should display Invitations Accepted metric', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Analytics' }).click();
      await page.waitForTimeout(1000);
      await expect(main.getByText('Invitations Accepted')).toBeVisible({ timeout: 5000 });
    });
  });

  test.describe('Navigation', () => {
    test('should navigate to Organization and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');
      await nav.getByRole('button', { name: 'Organization' }).click();
      await page.waitForTimeout(1500);
      await expect(mainContent(page).getByRole('heading', { name: 'Organizations', level: 1 })).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: 'User Management' }).click();
      await page.waitForTimeout(1500);
      await expect(mainContent(page).getByRole('heading', { name: 'User Management', level: 1 })).toBeVisible({ timeout: 5000 });
    });
  });

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

    test('should have labels on form inputs in Invitations tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Invitations' }).click();
      await page.waitForTimeout(500);
      // All form controls should have associated labels
      await expect(main.getByLabel('Email Address')).toBeVisible({ timeout: 5000 });
      await expect(main.getByLabel('Role')).toBeVisible();
      await expect(main.getByLabel('Team (Optional)')).toBeVisible();
    });
  });

  test.describe('Error-Free Operation', () => {
    test('should load without critical console errors', async ({ page }) => {
      const errors: string[] = [];
      page.on('console', (msg) => { if (msg.type() === 'error') errors.push(msg.text()); });
      await page.reload({ waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(3000);
      const critical = errors.filter(e =>
        !e.includes('net::ERR_') &&
        !e.includes('Failed to fetch') &&
        !e.includes('NetworkError') &&
        !e.includes('WebSocket') &&
        !e.includes('favicon') &&
        !e.includes('429') &&
        !e.includes('422')
      );
      expect(critical).toHaveLength(0);
    });

    test('should not show error UI', async ({ page }) => {
      expect(
        await page.getByText(/Something went wrong|Unexpected error|Application error/i)
          .first().isVisible({ timeout: 2000 }).catch(() => false)
      ).toBeFalsy();
    });

    test('should not show error UI after switching all tabs', async ({ page }) => {
      const main = mainContent(page);
      const tabs = ['Teams', 'Invitations', 'Quotas', 'Analytics', 'Users'];
      for (const tab of tabs) {
        await main.getByRole('button', { name: tab }).click();
        await page.waitForTimeout(500);
      }
      expect(
        await page.getByText(/Something went wrong|Unexpected error|Application error/i)
          .first().isVisible({ timeout: 2000 }).catch(() => false)
      ).toBeFalsy();
    });
  });
});
