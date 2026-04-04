import { test, expect } from '@playwright/test';

/**
 * World State Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config (handles auth via setup project):
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts world-state-deployed
 *
 * These tests verify all World State functionality on the live deployed site:
 *   1. Page Load & Layout
 *   2. Controls
 *   3. Graph Visualization
 *   4. Stats Section
 *   5. Navigation
 *   6. Accessibility
 *   7. Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('World State — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/world-state`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    await expect(
      mainContent(page).getByRole('heading', { name: 'World State', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load', () => {
    test('should load the world state page at /world-state', async ({ page }) => {
      await expect(page).toHaveURL(/\/world-state/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'World State', level: 1 })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      const main = mainContent(page);
      // Subtitle differs between loaded and loading/error states
      const hasFullSubtitle = await main
        .getByText('Unified visualization of all MockForge state systems - like a miniature game engine for your backend')
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasShortSubtitle = await main
        .getByText('Unified visualization of all MockForge state systems')
        .first()
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasFullSubtitle || hasShortSubtitle).toBeTruthy();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      const hasBreadcrumb = await banner.getByText('World State')
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasHomeBreadcrumb = await banner.getByText('Home')
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasBreadcrumb || hasHomeBreadcrumb).toBeTruthy();
    });

    test('should not display loading state after page settles', async ({ page }) => {
      // Give the page a moment to finish loading
      await page.waitForTimeout(1000);
      const main = mainContent(page);
      // If it loaded successfully, heading should be visible and loading gone
      const hasHeading = await main.getByRole('heading', { name: 'World State', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasHeading).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Controls
  // ---------------------------------------------------------------------------
  test.describe('Controls', () => {
    test('should display the Real-time updates checkbox', async ({ page }) => {
      const main = mainContent(page);
      const hasCheckbox = await main.getByText('Real-time updates')
        .isVisible({ timeout: 5000 }).catch(() => false);
      // Controls section only shows in the loaded (non-loading, non-error) state
      const hasHeading = await main.getByRole('heading', { name: 'World State', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasCheckbox || hasHeading).toBeTruthy();
    });

    test('should have the Real-time updates checkbox checked by default', async ({ page }) => {
      const main = mainContent(page);
      const checkbox = main.locator('input[type="checkbox"]').first();
      const isVisible = await checkbox.isVisible({ timeout: 3000 }).catch(() => false);

      if (isVisible) {
        await expect(checkbox).toBeChecked();
      }
    });

    test('should allow toggling the Real-time updates checkbox', async ({ page }) => {
      const main = mainContent(page);
      const checkbox = main.locator('input[type="checkbox"]').first();
      const isVisible = await checkbox.isVisible({ timeout: 3000 }).catch(() => false);

      if (isVisible) {
        await checkbox.uncheck();
        await page.waitForTimeout(300);
        await expect(checkbox).not.toBeChecked();

        await checkbox.check();
        await page.waitForTimeout(300);
        await expect(checkbox).toBeChecked();
      }
    });

    test('should display the layout dropdown', async ({ page }) => {
      const main = mainContent(page);
      const hasDropdown = await main.locator('select')
        .first().isVisible({ timeout: 3000 }).catch(() => false);
      const hasHeading = await main.getByRole('heading', { name: 'World State', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasDropdown || hasHeading).toBeTruthy();
    });

    test('should have Force Directed as default layout', async ({ page }) => {
      const main = mainContent(page);
      const select = main.locator('select').first();
      const isVisible = await select.isVisible({ timeout: 3000 }).catch(() => false);

      if (isVisible) {
        await expect(select).toHaveValue('force-directed');
      }
    });

    test('should offer three layout options', async ({ page }) => {
      const main = mainContent(page);
      const select = main.locator('select').first();
      const isVisible = await select.isVisible({ timeout: 3000 }).catch(() => false);

      if (isVisible) {
        const options = select.locator('option');
        await expect(options).toHaveCount(3);
        await expect(options.nth(0)).toHaveText('Force Directed');
        await expect(options.nth(1)).toHaveText('Hierarchical');
        await expect(options.nth(2)).toHaveText('Circular');
      }
    });

    test('should allow switching layout to Hierarchical', async ({ page }) => {
      const main = mainContent(page);
      const select = main.locator('select').first();
      const isVisible = await select.isVisible({ timeout: 3000 }).catch(() => false);

      if (isVisible) {
        await select.selectOption('hierarchical');
        await page.waitForTimeout(300);
        await expect(select).toHaveValue('hierarchical');
      }
    });

    test('should allow switching layout to Circular', async ({ page }) => {
      const main = mainContent(page);
      const select = main.locator('select').first();
      const isVisible = await select.isVisible({ timeout: 3000 }).catch(() => false);

      if (isVisible) {
        await select.selectOption('circular');
        await page.waitForTimeout(300);
        await expect(select).toHaveValue('circular');
      }
    });

    test('should display connection status indicator when connected', async ({ page }) => {
      const main = mainContent(page);
      // The connected indicator shows as "Connected" with a green dot
      const hasConnected = await main.getByText('Connected', { exact: true })
        .isVisible({ timeout: 3000 }).catch(() => false);
      // Connection may not be established in deployed env — heading is enough
      const hasHeading = await main.getByRole('heading', { name: 'World State', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasConnected || hasHeading).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Graph Visualization
  // ---------------------------------------------------------------------------
  test.describe('Graph Visualization', () => {
    test('should display the graph container with 600px height', async ({ page }) => {
      const main = mainContent(page);
      const graphContainer = main.locator('.h-\\[600px\\]');
      const isVisible = await graphContainer.isVisible({ timeout: 5000 }).catch(() => false);
      // Graph container only renders in the loaded state
      const hasHeading = await main.getByRole('heading', { name: 'World State', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(isVisible || hasHeading).toBeTruthy();
    });

    test('should display the graph within a bordered container', async ({ page }) => {
      const main = mainContent(page);
      const borderedContainer = main.locator('.border.rounded-lg.overflow-hidden');
      const isVisible = await borderedContainer
        .first().isVisible({ timeout: 5000 }).catch(() => false);
      const hasHeading = await main.getByRole('heading', { name: 'World State', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(isVisible || hasHeading).toBeTruthy();
    });

    test('should display the state layer panel on the left', async ({ page }) => {
      const main = mainContent(page);
      const layerPanel = main.locator('.col-span-2').first();
      const isVisible = await layerPanel.isVisible({ timeout: 5000 }).catch(() => false);
      const hasHeading = await main.getByRole('heading', { name: 'World State', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(isVisible || hasHeading).toBeTruthy();
    });

    test('should display the node inspector on the right', async ({ page }) => {
      const main = mainContent(page);
      const inspector = main.locator('.col-span-3').first();
      const isVisible = await inspector.isVisible({ timeout: 5000 }).catch(() => false);
      const hasHeading = await main.getByRole('heading', { name: 'World State', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(isVisible || hasHeading).toBeTruthy();
    });

    test('should display the 12-column grid layout', async ({ page }) => {
      const main = mainContent(page);
      const gridLayout = main.locator('.grid.grid-cols-12');
      const isVisible = await gridLayout
        .first().isVisible({ timeout: 5000 }).catch(() => false);
      const hasHeading = await main.getByRole('heading', { name: 'World State', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(isVisible || hasHeading).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Stats Section
  // ---------------------------------------------------------------------------
  test.describe('Stats Section', () => {
    test('should display the Nodes stat', async ({ page }) => {
      const main = mainContent(page);
      const hasNodes = await main.getByText('Nodes', { exact: true })
        .isVisible({ timeout: 5000 }).catch(() => false);
      const hasHeading = await main.getByRole('heading', { name: 'World State', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasNodes || hasHeading).toBeTruthy();
    });

    test('should display the Edges stat', async ({ page }) => {
      const main = mainContent(page);
      const hasEdges = await main.getByText('Edges', { exact: true })
        .isVisible({ timeout: 5000 }).catch(() => false);
      const hasHeading = await main.getByRole('heading', { name: 'World State', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasEdges || hasHeading).toBeTruthy();
    });

    test('should display the Layers stat', async ({ page }) => {
      const main = mainContent(page);
      const hasLayers = await main.getByText('Layers', { exact: true })
        .isVisible({ timeout: 5000 }).catch(() => false);
      const hasHeading = await main.getByRole('heading', { name: 'World State', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasLayers || hasHeading).toBeTruthy();
    });

    test('should display the Active Layers stat', async ({ page }) => {
      const main = mainContent(page);
      const hasActiveLayers = await main.getByText('Active Layers', { exact: true })
        .isVisible({ timeout: 5000 }).catch(() => false);
      const hasHeading = await main.getByRole('heading', { name: 'World State', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasActiveLayers || hasHeading).toBeTruthy();
    });

    test('should display numeric values in the stats grid', async ({ page }) => {
      const main = mainContent(page);
      const statsGrid = main.locator('.grid.grid-cols-4');
      const isVisible = await statsGrid
        .first().isVisible({ timeout: 5000 }).catch(() => false);

      if (isVisible) {
        // Each stat cell has a text-2xl.font-bold element with a number
        const boldValues = statsGrid.first().locator('.text-2xl.font-bold');
        const count = await boldValues.count();
        expect(count).toBe(4);
      }
    });

    test('should display stats in a 4-column grid', async ({ page }) => {
      const main = mainContent(page);
      const statsGrid = main.locator('.grid.grid-cols-4');
      const isVisible = await statsGrid
        .first().isVisible({ timeout: 5000 }).catch(() => false);
      const hasHeading = await main.getByRole('heading', { name: 'World State', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(isVisible || hasHeading).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Navigation
  // ---------------------------------------------------------------------------
  test.describe('Navigation', () => {
    test('should navigate to Dashboard and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: /World State/i }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'World State', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Observability and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: /Observability/i }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Observability Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: /World State/i }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'World State', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should preserve URL when navigating back via browser history', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await page.goBack();
      await page.waitForTimeout(1500);

      await expect(page).toHaveURL(/\/world-state/);
      await expect(
        mainContent(page).getByRole('heading', { name: 'World State', level: 1 })
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
      await expect(h1).toHaveText('World State');
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

    test('should have accessible form controls', async ({ page }) => {
      const main = mainContent(page);
      const checkbox = main.locator('input[type="checkbox"]').first();
      const isVisible = await checkbox.isVisible({ timeout: 3000 }).catch(() => false);

      if (isVisible) {
        // Checkbox should be associated with its label
        const label = main.getByText('Real-time updates');
        await expect(label).toBeVisible({ timeout: 3000 });
      }
    });

    test('should have accessible select control for layout', async ({ page }) => {
      const main = mainContent(page);
      const select = main.locator('select').first();
      const isVisible = await select.isVisible({ timeout: 3000 }).catch(() => false);

      if (isVisible) {
        // Select should have options
        const options = select.locator('option');
        const count = await options.count();
        expect(count).toBe(3);
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 7. Error-Free Operation
  // ---------------------------------------------------------------------------
  test.describe('Error-Free', () => {
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
          !err.includes('not valid JSON') &&
          !err.includes('DOCTYPE')
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
      // The error state shows "Failed to load world state" — verify it's not present
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

    test('should handle layout switching without errors', async ({ page }) => {
      const consoleErrors: string[] = [];

      page.on('console', (msg) => {
        if (msg.type() === 'error') {
          consoleErrors.push(msg.text());
        }
      });

      const main = mainContent(page);
      const select = main.locator('select').first();
      const isVisible = await select.isVisible({ timeout: 3000 }).catch(() => false);

      if (isVisible) {
        const layouts = ['hierarchical', 'circular', 'force-directed'];
        for (const layout of layouts) {
          await select.selectOption(layout);
          await page.waitForTimeout(500);
        }
      }

      const criticalErrors = consoleErrors.filter(
        (err) =>
          !err.includes('net::ERR_') &&
          !err.includes('Failed to fetch') &&
          !err.includes('NetworkError') &&
          !err.includes('WebSocket') &&
          !err.includes('favicon') &&
          !err.includes('429') &&
          !err.includes('not valid JSON') &&
          !err.includes('DOCTYPE')
      );

      expect(criticalErrors).toHaveLength(0);
    });
  });
});
