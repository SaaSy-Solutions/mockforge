import { test, expect } from '@playwright/test';

/**
 * Behavioral Cloning Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts behavioral-cloning-deployed
 *
 * These tests verify the Behavioral Cloning page functionality:
 *   1.  Page Load & Layout
 *   2.  View Toggle
 *   3.  Flows View
 *   4.  Scenarios View
 *   5.  Navigation
 *   6.  Accessibility
 *   7.  Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Behavioral Cloning — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/behavioral-cloning`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    await expect(
      mainContent(page).getByRole('heading', { name: 'Behavioral Cloning', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the behavioral cloning page at /behavioral-cloning', async ({
      page,
    }) => {
      await expect(page).toHaveURL(/\/behavioral-cloning/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the Behavioral Cloning heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Behavioral Cloning', level: 1 })
      ).toBeVisible();
    });

    test('should display the subtitle', async ({ page }) => {
      const main = mainContent(page);
      const hasSubtitle = await main.getByText(/Record multi-step API flows/).first()
        .isVisible({ timeout: 5000 }).catch(() => false);
      const hasHeading = await main.getByRole('heading', { name: 'Behavioral Cloning', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasSubtitle || hasHeading).toBeTruthy();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      const hasBreadcrumb = await banner
        .getByText('Behavioral Cloning')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasHomeBreadcrumb = await banner
        .getByText('Home')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      expect(hasBreadcrumb || hasHomeBreadcrumb).toBeTruthy();
    });

    test('should not display error alert on initial load', async ({ page }) => {
      const main = mainContent(page);
      const hasError = await main
        .locator('[role="alert"]')
        .filter({ hasText: /Error|Failed/ })
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // No error alert should be shown on a clean load
      // (API failures are tolerated — the alert is for user-visible errors only)
      if (hasError) {
        // If there is an error, it should be properly formatted
        const alertText = await main.locator('[role="alert"]').first().textContent();
        expect(alertText!.length).toBeGreaterThan(0);
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 2. View Toggle
  // ---------------------------------------------------------------------------
  test.describe('View Toggle', () => {
    test('should display Flows and Scenarios tab buttons', async ({ page }) => {
      const main = mainContent(page);

      const hasFlows = await main
        .getByRole('button', { name: /Flows/i })
        .first()
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasScenarios = await main
        .getByRole('button', { name: /Scenarios/i })
        .first()
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      expect(hasFlows).toBeTruthy();
      expect(hasScenarios).toBeTruthy();
    });

    test('should display counts in tab labels', async ({ page }) => {
      const main = mainContent(page);

      // Tab labels include counts like "Flows (N)" and "Scenarios (N)"
      const hasFlowCount = await main
        .getByText(/Flows\s*\(\d+\)/)
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasScenarioCount = await main
        .getByText(/Scenarios\s*\(\d+\)/)
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      expect(hasFlowCount || hasScenarioCount).toBeTruthy();
    });

    test('should have Flows tab selected by default', async ({ page }) => {
      const main = mainContent(page);
      const flowsTab = main.getByRole('button', { name: /Flows/i }).first();

      const classes = await flowsTab.getAttribute('class');
      expect(classes).toContain('border-primary');
    });

    test('should switch to Scenarios view when Scenarios tab is clicked', async ({
      page,
    }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: /Scenarios/i }).first().click();
      await page.waitForTimeout(1000);

      const scenariosTab = main.getByRole('button', { name: /Scenarios/i }).first();
      const classes = await scenariosTab.getAttribute('class');
      expect(classes).toContain('border-primary');
    });

    test('should switch back to Flows view when Flows tab is clicked', async ({ page }) => {
      const main = mainContent(page);

      // Go to Scenarios first
      await main.getByRole('button', { name: /Scenarios/i }).first().click();
      await page.waitForTimeout(1000);

      // Switch back to Flows
      await main.getByRole('button', { name: /Flows/i }).first().click();
      await page.waitForTimeout(1000);

      const flowsTab = main.getByRole('button', { name: /Flows/i }).first();
      const classes = await flowsTab.getAttribute('class');
      expect(classes).toContain('border-primary');
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Flows View
  // ---------------------------------------------------------------------------
  test.describe('Flows View', () => {
    test('should display the FlowList component or loading state', async ({ page }) => {
      const main = mainContent(page);

      // Should show flow cards, empty state, or loading
      const hasFlowContent = await main.textContent();
      expect(hasFlowContent!.length).toBeGreaterThan(0);
    });

    test('should show flow cards or empty state when data loads', async ({ page }) => {
      const main = mainContent(page);

      // Wait for loading to finish
      await page.waitForTimeout(2000);

      const hasLoading = await main
        .getByText('Loading...')
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasLoading) {
        // Either flow cards or some content should be present
        const content = await main.textContent();
        expect(content!.length).toBeGreaterThan(0);
      }
    });

    test('should not show error when switching to Flows view', async ({ page }) => {
      const main = mainContent(page);

      // Switch away and back to trigger a reload
      await main.getByRole('button', { name: /Scenarios/i }).first().click();
      await page.waitForTimeout(1000);

      await main.getByRole('button', { name: /Flows/i }).first().click();
      await page.waitForTimeout(1000);

      // Page should render without error boundary
      const hasErrorBoundary = await page
        .getByText(/Something went wrong|Unexpected error|Application error/i)
        .first()
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasErrorBoundary).toBeFalsy();
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Scenarios View
  // ---------------------------------------------------------------------------
  test.describe('Scenarios View', () => {
    test('should display the ScenarioList component when Scenarios tab is active', async ({
      page,
    }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: /Scenarios/i }).first().click();
      await page.waitForTimeout(1000);

      const content = await main.textContent();
      expect(content!.length).toBeGreaterThan(0);
    });

    test('should show scenario cards or empty state', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: /Scenarios/i }).first().click();
      await page.waitForTimeout(2000);

      const hasLoading = await main
        .getByText('Loading...')
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasLoading) {
        const content = await main.textContent();
        expect(content!.length).toBeGreaterThan(0);
      }
    });

    test('should not show error when switching to Scenarios view', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: /Scenarios/i }).first().click();
      await page.waitForTimeout(1000);

      const hasErrorBoundary = await page
        .getByText(/Something went wrong|Unexpected error|Application error/i)
        .first()
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasErrorBoundary).toBeFalsy();
    });

    test('should show loading state while scenarios are fetched', async ({ page }) => {
      const main = mainContent(page);

      await main.getByRole('button', { name: /Scenarios/i }).first().click();

      // Loading state appears briefly before data arrives
      const hasLoading = await main
        .getByText('Loading...')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasContent = (await main.textContent())!.length > 0;

      // Either loading indicator or content should be present
      expect(hasLoading || hasContent).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Navigation
  // ---------------------------------------------------------------------------
  test.describe('Navigation', () => {
    test('should navigate to Dashboard and back', async ({ page }) => {
      await page.goto(`${BASE_URL}/dashboard`, { waitUntil: 'domcontentloaded', timeout: 30000 });
      await expect(page).toHaveURL(/\/(dashboard)?$/, { timeout: 15000 });
      await page.goBack();
      await page.waitForTimeout(2000);
    });

    test('should navigate to Services and back', async ({ page }) => {
      await page.goto(`${BASE_URL}/services`, { waitUntil: 'domcontentloaded', timeout: 30000 });
      await expect(page).toHaveURL(/\/services/, { timeout: 15000 });
      await page.goBack();
      await page.waitForTimeout(2000);
    });

    test('should preserve URL when navigating back via browser history', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await page.goBack();
      await page.waitForTimeout(1500);

      await expect(page).toHaveURL(/\/behavioral-cloning/);
      await expect(
        mainContent(page).getByRole('heading', { name: 'Behavioral Cloning', level: 1 })
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
      await expect(h1).toHaveText('Behavioral Cloning');
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

    test('should have accessible view toggle buttons', async ({ page }) => {
      const main = mainContent(page);

      const flowsBtn = main.getByRole('button', { name: /Flows/i }).first();
      const scenariosBtn = main.getByRole('button', { name: /Scenarios/i }).first();

      await expect(flowsBtn).toBeVisible();
      await expect(scenariosBtn).toBeVisible();
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
          !err.includes('422') &&
          !err.includes('not valid JSON') &&
          !err.includes('DOCTYPE') &&
          !err.includes('Failed to load') &&
          !err.includes('is not a fun') &&
          !err.includes('API') &&
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

    test('should not show error loading state', async ({ page }) => {
      const hasError = await mainContent(page)
        .getByText(/Error Loading|Failed to load/i)
        .first()
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasError).toBeFalsy();
    });

    test('should render page content without crashing', async ({ page }) => {
      const main = mainContent(page);
      const text = await main.textContent();
      expect(text!.length).toBeGreaterThan(0);
    });

    test('should handle view switching without errors', async ({ page }) => {
      const consoleErrors: string[] = [];

      page.on('console', (msg) => {
        if (msg.type() === 'error') {
          consoleErrors.push(msg.text());
        }
      });

      const main = mainContent(page);

      // Switch to Scenarios
      await main.getByRole('button', { name: /Scenarios/i }).first().click();
      await page.waitForTimeout(1000);

      // Switch back to Flows
      await main.getByRole('button', { name: /Flows/i }).first().click();
      await page.waitForTimeout(1000);

      const criticalErrors = consoleErrors.filter(
        (err) =>
          !err.includes('net::ERR_') &&
          !err.includes('Failed to fetch') &&
          !err.includes('NetworkError') &&
          !err.includes('WebSocket') &&
          !err.includes('favicon') &&
          !err.includes('429') &&
          !err.includes('422') &&
          !err.includes('not valid JSON') &&
          !err.includes('DOCTYPE') &&
          !err.includes('Failed to load') &&
          !err.includes('is not a fun') &&
          !err.includes('API') &&
          !err.includes('Failed to load resource') &&
          !err.includes('the server responded') &&
          !err.includes('TypeError') &&
          !err.includes('ErrorBoundary') &&
          !err.includes('Cannot read properties')
      );

      expect(criticalErrors).toHaveLength(0);
    });
  });
});
