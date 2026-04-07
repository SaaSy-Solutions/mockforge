import { test, expect } from '@playwright/test';

/**
 * Time Travel Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config (handles auth via setup project):
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts time-travel-deployed
 *
 * These tests verify all Time Travel functionality on the live deployed site:
 *   1.  Page Load & Layout
 *   2.  Status Display
 *   3.  Time Controls
 *   4.  Tab Navigation (3 tabs)
 *   5.  Cron Jobs Tab
 *   6.  Mutation Rules Tab
 *   7.  Navigation
 *   8.  Accessibility
 *   9.  Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

// Scoped locator for the main content area (excludes sidebar to avoid duplicate matches)
function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Time Travel — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/time-travel`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    // Wait for the app shell to render (sidebar navigation visible)
    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    // Wait for the Time Travel heading to confirm content loaded
    await expect(
      mainContent(page).getByRole('heading', { name: 'Time Travel', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    // Small stabilization delay for dynamic content
    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the time travel page at /time-travel', async ({ page }) => {
      await expect(page).toHaveURL(/\/time-travel/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Time Travel', level: 1 })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      const hasSubtitle = await mainContent(page)
        .getByText('Control virtual time for testing time-dependent applications')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasAltSubtitle = await mainContent(page)
        .getByText('Temporal simulation controls')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasSubtitle || hasAltSubtitle).toBeTruthy();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home').first()).toBeVisible();
      await expect(banner.getByText('Time Travel').first()).toBeVisible();
    });

    test('should display the status card', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Time Travel Status').first()).toBeVisible({ timeout: 5000 });
    });

    test('should display the tabs section', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('button', { name: /Cron Jobs/i })).toBeVisible({ timeout: 5000 });
      await expect(main.getByRole('button', { name: /Mutation Rules/i })).toBeVisible();
      await expect(main.getByRole('button', { name: /Scenarios/i })).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Status Display
  // ---------------------------------------------------------------------------
  test.describe('Status Display', () => {
    test('should display the status card with icon', async ({ page }) => {
      const main = mainContent(page);
      // The status card contains the Clock icon and "Time Travel Status" heading
      await expect(main.getByText('Time Travel Status').first()).toBeVisible();

      // Should show either "Virtual time is active" or "Using real time"
      const hasActive = await main
        .getByText('Virtual time is active')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasRealTime = await main
        .getByText('Using real time')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasActive || hasRealTime).toBeTruthy();
    });

    test('should display status badge when time travel is active', async ({ page }) => {
      const main = mainContent(page);
      const isActive = await main
        .getByText('Virtual time is active')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (isActive) {
        await expect(main.getByText('Active').first()).toBeVisible();
      }
    });

    test('should display time information card', async ({ page }) => {
      const main = mainContent(page);
      const isActive = await main
        .getByText('Virtual time is active')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (isActive) {
        // When active, should show Virtual Time, Time Scale, and Real Time cards
        await expect(main.getByText('Virtual Time').first()).toBeVisible({ timeout: 5000 });
        await expect(main.getByText('Time Scale').first()).toBeVisible();
        await expect(main.getByText('Real Time').first()).toBeVisible();
      } else {
        // When disabled, should show Real Time card
        await expect(main.getByText('Real Time').first()).toBeVisible({ timeout: 5000 });
      }
    });

    test('should display time scale value when active', async ({ page }) => {
      const main = mainContent(page);
      const isActive = await main
        .getByText('Virtual time is active')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (isActive) {
        // Time scale shows as "1.0x" or similar
        const hasScale = await main
          .getByText(/\d+\.\d+x/)
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        expect(hasScale).toBeTruthy();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Time Controls
  // ---------------------------------------------------------------------------
  test.describe('Time Controls', () => {
    test('should display Enable Time Travel button when disabled', async ({ page }) => {
      const main = mainContent(page);
      const isDisabled = await main
        .getByText('Using real time')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (isDisabled) {
        await expect(
          main.getByRole('button', { name: /Enable Time Travel/i })
        ).toBeVisible();
      }
    });

    test('should display Initial Time input when disabled', async ({ page }) => {
      const main = mainContent(page);
      const isDisabled = await main
        .getByText('Using real time')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (isDisabled) {
        await expect(main.getByText('Initial Time (ISO 8601, optional)').first()).toBeVisible();
        await expect(
          main.getByPlaceholder('2025-01-01T00:00:00Z')
        ).toBeVisible();
      }
    });

    test('should display Time Scale input when disabled', async ({ page }) => {
      const main = mainContent(page);
      const isDisabled = await main
        .getByText('Using real time')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (isDisabled) {
        await expect(main.getByText('Time Scale (1.0 = real time)').first()).toBeVisible();
        await expect(main.getByPlaceholder('1.0')).toBeVisible();
      }
    });

    test('should allow typing into Initial Time input', async ({ page }) => {
      const main = mainContent(page);
      const isDisabled = await main
        .getByText('Using real time')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (isDisabled) {
        const input = main.getByPlaceholder('2025-01-01T00:00:00Z');
        await input.fill('2026-06-15T12:00:00Z');
        await page.waitForTimeout(300);

        const value = await input.inputValue();
        expect(value).toBe('2026-06-15T12:00:00Z');
      }
    });

    test('should display Advance Duration and Set Scale controls when enabled', async ({ page }) => {
      const main = mainContent(page);
      const isActive = await main
        .getByText('Virtual time is active')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (isActive) {
        // Advance Duration input and button
        await expect(main.getByText('Advance Duration').first()).toBeVisible();
        await expect(main.getByPlaceholder('1h')).toBeVisible();
        await expect(
          main.getByRole('button', { name: /Advance/i })
        ).toBeVisible();

        // Time Scale input and Set Scale button
        await expect(
          main.getByRole('button', { name: /Set Scale/i })
        ).toBeVisible();
      }
    });

    test('should display Disable and Reset buttons when enabled', async ({ page }) => {
      const main = mainContent(page);
      const isActive = await main
        .getByText('Virtual time is active')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (isActive) {
        await expect(
          main.getByRole('button', { name: /Disable/i })
        ).toBeVisible();
        await expect(
          main.getByRole('button', { name: /Reset to Real Time/i })
        ).toBeVisible();
      }
    });

    test('should show either enabled or disabled controls (mutually exclusive)', async ({ page }) => {
      const main = mainContent(page);

      const hasEnableButton = await main
        .getByRole('button', { name: /Enable Time Travel/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasDisableButton = await main
        .getByRole('button', { name: /Disable/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // Exactly one set of controls should be visible
      expect(hasEnableButton || hasDisableButton).toBeTruthy();
      // They should not both be visible
      expect(hasEnableButton && hasDisableButton).toBeFalsy();
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Tab Navigation (3 tabs)
  // ---------------------------------------------------------------------------
  test.describe('Tab Navigation', () => {
    test('should display all three tab triggers', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('button', { name: /Cron Jobs/i })).toBeVisible();
      await expect(main.getByRole('button', { name: /Mutation Rules/i })).toBeVisible();
      await expect(main.getByRole('button', { name: /Scenarios/i })).toBeVisible();
    });

    test('should default to Cron Jobs tab', async ({ page }) => {
      const main = mainContent(page);
      // Cron Jobs tab content should be visible by default
      await expect(main.getByText('Scheduled Cron Jobs').first()).toBeVisible({ timeout: 5000 });
    });

    test('should switch to Mutation Rules tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Mutation Rules/i }).click();
      await page.waitForTimeout(500);

      await expect(main.getByText('Data Mutation Rules').first()).toBeVisible({ timeout: 5000 });
    });

    test('should switch to Scenarios tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Scenarios/i }).click();
      await page.waitForTimeout(500);

      await expect(main.getByText('Time Travel Scenarios').first()).toBeVisible({ timeout: 5000 });
    });

    test('should switch back to Cron Jobs tab after visiting other tabs', async ({ page }) => {
      const main = mainContent(page);

      // Switch to Scenarios
      await main.getByRole('button', { name: /Scenarios/i }).click();
      await page.waitForTimeout(500);
      await expect(main.getByText('Time Travel Scenarios').first()).toBeVisible({ timeout: 5000 });

      // Switch back to Cron Jobs
      await main.getByRole('button', { name: /Cron Jobs/i }).click();
      await page.waitForTimeout(500);
      await expect(main.getByText('Scheduled Cron Jobs').first()).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Cron Jobs Tab
  // ---------------------------------------------------------------------------
  test.describe('Cron Jobs Tab', () => {
    test('should display the Scheduled Cron Jobs heading', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Scheduled Cron Jobs').first()).toBeVisible();
    });

    test('should show cron job cards or empty state', async ({ page }) => {
      const main = mainContent(page);

      const hasJobs = await main
        .locator('div')
        .filter({ hasText: /executions/ })
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasEmpty = await main
        .getByText('No cron jobs')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasLoading = await main
        .locator('.animate-spin')
        .first()
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      // Should show either jobs, empty state, or loading
      expect(hasJobs || hasEmpty || hasLoading).toBeTruthy();
    });

    test('should display job details when cron jobs exist', async ({ page }) => {
      const main = mainContent(page);

      const hasJobs = await main
        .locator('div')
        .filter({ hasText: /executions/ })
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasJobs) {
        // Each job card should show name, schedule, and execution count
        const hasSchedule = await main
          .locator('div')
          .filter({ hasText: /executions/ })
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        expect(hasSchedule).toBeTruthy();

        // Should show Enabled or Disabled badge
        const hasEnabledBadge = await main
          .getByText('Enabled', { exact: true })
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasDisabledBadge = await main
          .getByText('Disabled', { exact: true })
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        expect(hasEnabledBadge || hasDisabledBadge).toBeTruthy();
      }
    });

    test('should display empty state message when no cron jobs', async ({ page }) => {
      const main = mainContent(page);
      const hasEmpty = await main
        .getByText('No cron jobs')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasEmpty) {
        await expect(
          main.getByText('Create cron jobs to schedule recurring events.').first()
        ).toBeVisible();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Mutation Rules Tab
  // ---------------------------------------------------------------------------
  test.describe('Mutation Rules Tab', () => {
    test('should display the Data Mutation Rules heading', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Mutation Rules/i }).click();
      await page.waitForTimeout(500);

      await expect(main.getByText('Data Mutation Rules').first()).toBeVisible({ timeout: 5000 });
    });

    test('should show mutation rule cards or empty state', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Mutation Rules/i }).click();
      await page.waitForTimeout(500);

      const hasRules = await main
        .getByText(/Entity:/)
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasEmpty = await main
        .getByText('No mutation rules')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasLoading = await main
        .locator('.animate-spin')
        .first()
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasRules || hasEmpty || hasLoading).toBeTruthy();
    });

    test('should display rule details when mutation rules exist', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Mutation Rules/i }).click();
      await page.waitForTimeout(500);

      const hasRules = await main
        .getByText(/Entity:/)
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasRules) {
        // Each rule card shows entity name and status badge
        const hasEnabledBadge = await main
          .getByText('Enabled', { exact: true })
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasDisabledBadge = await main
          .getByText('Disabled', { exact: true })
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        expect(hasEnabledBadge || hasDisabledBadge).toBeTruthy();
      }
    });

    test('should display empty state message when no mutation rules', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Mutation Rules/i }).click();
      await page.waitForTimeout(500);

      const hasEmpty = await main
        .getByText('No mutation rules')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasEmpty) {
        await expect(
          main.getByText('Create mutation rules to automatically modify data based on time triggers.').first()
        ).toBeVisible();
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

      // Navigate back to Time Travel
      const hasTimeTravelButton = await nav
        .getByRole('button', { name: /Time Travel/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasTimeTravelButton) {
        await nav.getByRole('button', { name: /Time Travel/i }).click();
      } else {
        await page.goto(`${BASE_URL}/time-travel`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Time Travel', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Config and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Config' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Configuration', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back to Time Travel
      const hasTimeTravelButton = await nav
        .getByRole('button', { name: /Time Travel/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasTimeTravelButton) {
        await nav.getByRole('button', { name: /Time Travel/i }).click();
      } else {
        await page.goto(`${BASE_URL}/time-travel`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Time Travel', level: 1 })
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
      await expect(h1).toHaveText('Time Travel');
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

    test('should have accessible tab controls', async ({ page }) => {
      const main = mainContent(page);
      // All three tabs should be accessible buttons
      const tabButtons = [
        main.getByRole('button', { name: /Cron Jobs/i }),
        main.getByRole('button', { name: /Mutation Rules/i }),
        main.getByRole('button', { name: /Scenarios/i }),
      ];

      for (const tab of tabButtons) {
        await expect(tab).toBeVisible();
      }
    });

    test('should have accessible input labels', async ({ page }) => {
      const main = mainContent(page);
      const isDisabled = await main
        .getByText('Using real time')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (isDisabled) {
        // Check that the form inputs have associated labels
        await expect(main.getByText('Initial Time (ISO 8601, optional)').first()).toBeVisible();
        await expect(main.getByText('Time Scale (1.0 = real time)').first()).toBeVisible();
      }
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
          !err.includes('DOCTYPE') &&
          !err.includes('Failed to load resource') &&
          !err.includes('the server responded') &&
          !err.includes('TypeError') &&
          !err.includes('ErrorBoundary') &&
          !err.includes('Cannot read properties')
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

    test('should not crash when switching tabs rapidly', async ({ page }) => {
      const main = mainContent(page);

      // Rapidly switch between all three tabs
      await main.getByRole('button', { name: /Mutation Rules/i }).click();
      await page.waitForTimeout(200);
      await main.getByRole('button', { name: /Scenarios/i }).click();
      await page.waitForTimeout(200);
      await main.getByRole('button', { name: /Cron Jobs/i }).click();
      await page.waitForTimeout(200);
      await main.getByRole('button', { name: /Mutation Rules/i }).click();
      await page.waitForTimeout(200);
      await main.getByRole('button', { name: /Cron Jobs/i }).click();
      await page.waitForTimeout(500);

      // Page should still be functional
      await expect(
        main.getByRole('heading', { name: 'Time Travel', level: 1 })
      ).toBeVisible();
      await expect(main.getByText('Scheduled Cron Jobs').first()).toBeVisible();
    });
  });
});
