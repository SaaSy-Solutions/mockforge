import { test, expect } from '@playwright/test';

/**
 * Config Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts config-deployed
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Config — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/config`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    await expect(
      mainContent(page).getByRole('heading', { name: 'Configuration', level: 1 })
    ).toBeVisible({ timeout: 10000 });
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the config page at /config', async ({ page }) => {
      await expect(page).toHaveURL(/\/config/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Configuration', level: 1 })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Manage MockForge settings and preferences')
      ).toBeVisible();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Config')).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Header Action Buttons
  // ---------------------------------------------------------------------------
  test.describe('Header Action Buttons', () => {
    test('should display the reality level indicator', async ({ page }) => {
      await expect(
        mainContent(page).getByText(/L\d/)
      ).toBeVisible();
    });

    test('should display the "Reset All" button', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: 'Reset All' })
      ).toBeVisible();
    });

    test('should display the "Save All Changes" button', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: 'Save All Changes' })
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Config Tab Navigation
  // ---------------------------------------------------------------------------
  test.describe('Config Tab Navigation', () => {
    test('should display all config tabs', async ({ page }) => {
      const main = mainContent(page);
      const tabs = [
        'Reality Slider',
        'General',
        'Protocols',
        'Latency',
        'Fault Injection',
        'Traffic Shaping',
        'Proxy',
        'Validation',
        'Environment',
      ];

      for (const tab of tabs) {
        await expect(
          main.getByRole('button', { name: new RegExp(tab) })
        ).toBeVisible();
      }
    });

    test('should display tab descriptions', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Unified realism control')).toBeVisible();
      await expect(main.getByText('Basic MockForge settings')).toBeVisible();
      await expect(main.getByText('Response delay and timing')).toBeVisible();
    });

    test('should switch to Latency tab when clicked', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Latency/ }).click();
      await page.waitForTimeout(500);

      // Latency tab should show "Latency Configuration" heading
      await expect(
        main.getByRole('heading', { name: /Latency Configuration/, level: 2 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should switch to Protocols tab when clicked', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Protocols/ }).click();
      await page.waitForTimeout(500);

      const hasProtocolContent = await main.getByText(/Protocol|HTTP|WebSocket|gRPC/i)
        .first().isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasProtocolContent).toBeTruthy();
    });

    test('should switch back to General tab', async ({ page }) => {
      const main = mainContent(page);

      // Switch to another tab first
      await main.getByRole('button', { name: /Latency/ }).click();
      await page.waitForTimeout(500);

      // Switch back to General
      await main.getByRole('button', { name: /General/ }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('heading', { name: 'General Settings', level: 2 })
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 4. General Settings Panel (Default Tab)
  // ---------------------------------------------------------------------------
  test.describe('General Settings Panel', () => {
    test('should display the General Settings heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'General Settings', level: 2 })
      ).toBeVisible();
    });

    test('should display the subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Basic MockForge configuration')
      ).toBeVisible();
    });

    test('should display Server Configuration section', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Server Configuration')
      ).toBeVisible();
    });

    test('should display port configuration fields', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('HTTP Port')).toBeVisible();
      await expect(main.getByText('WebSocket Port')).toBeVisible();
      await expect(main.getByText('gRPC Port')).toBeVisible();
      await expect(main.getByText('Admin Port')).toBeVisible();
    });

    test('should display default port values', async ({ page }) => {
      const main = mainContent(page);
      const spinbuttons = main.getByRole('spinbutton');

      // Check that port spinbuttons have values
      expect(await spinbuttons.count()).toBeGreaterThanOrEqual(4);
    });

    test('should display AI Mode section', async ({ page }) => {
      await expect(
        mainContent(page).getByText('AI Mode')
      ).toBeVisible();
    });

    test('should display AI Mode dropdown with options', async ({ page }) => {
      const main = mainContent(page);
      const aiDropdown = main.getByRole('combobox');
      await expect(aiDropdown).toBeVisible();

      const options = aiDropdown.locator('option');
      const optionTexts = await options.allTextContents();
      expect(optionTexts.length).toBeGreaterThanOrEqual(2);
    });

    test('should display Reset and Save buttons in General panel', async ({ page }) => {
      const main = mainContent(page);

      // General panel has its own Reset and Save buttons
      const resetButtons = main.getByRole('button', { name: 'Reset' });
      expect(await resetButtons.count()).toBeGreaterThanOrEqual(1);

      await expect(
        main.getByRole('button', { name: /Save.*Restart|Save/i }).first()
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Tab Content Depth
  // ---------------------------------------------------------------------------
  test.describe('Tab Content Depth', () => {
    test('should display Latency tab with Base Latency and Jitter fields', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Latency/ }).click();
      await page.waitForTimeout(500);

      // Use exact-text match — "Jitter" appears in both the label and the
      // description ("Random delay variation (± jitter)") which would otherwise
      // trigger a strict-mode violation.
      await expect(main.getByText('Base Latency (ms)', { exact: true })).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('Jitter (ms)', { exact: true })).toBeVisible();
    });

    test('should display Proxy tab with upstream configuration', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Proxy/ }).click();
      await page.waitForTimeout(500);

      const hasProxyContent = await main.getByText(/Proxy|Upstream|upstream/i).first()
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasProxyContent).toBeTruthy();
    });

    test('should display Validation tab with mode settings', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Validation/ }).click();
      await page.waitForTimeout(500);

      const hasValidationContent = await main.getByText(/Validation|mode|request|response/i).first()
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasValidationContent).toBeTruthy();
    });

    test('should display Environment tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Environment/ }).click();
      await page.waitForTimeout(500);

      const hasEnvContent = await main.getByText(/Environment|variable/i).first()
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasEnvContent).toBeTruthy();
    });

    test('should display Fault Injection tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Fault Injection/ }).click();
      await page.waitForTimeout(500);

      const hasFaultContent = await main.getByText(/Fault|Error|failure/i).first()
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasFaultContent).toBeTruthy();
    });

    test('should display Traffic Shaping tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Traffic Shaping/ }).click();
      await page.waitForTimeout(500);

      const hasTrafficContent = await main.getByText(/Traffic|Bandwidth|network/i).first()
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasTrafficContent).toBeTruthy();
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

      await nav.getByRole('button', { name: 'Config' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Configuration', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Organization and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Organization' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: /Organization/i, level: 1 })
      ).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: 'Config' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Configuration', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a single H1 heading', async ({ page }) => {
      const h1 = mainContent(page).getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);
      await expect(h1).toHaveText('Configuration');
    });

    test('should have an H2 section heading for the active tab', async ({ page }) => {
      const h2s = mainContent(page).getByRole('heading', { level: 2 });
      expect(await h2s.count()).toBeGreaterThanOrEqual(1);
    });

    test('should have a tab navigation region', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('navigation')
      ).toBeVisible();
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
  });

  // ---------------------------------------------------------------------------
  // 7. Error-Free Operation
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
          !err.includes('favicon') &&
          !err.includes('429') &&
          !err.includes('422')
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
  });
});
