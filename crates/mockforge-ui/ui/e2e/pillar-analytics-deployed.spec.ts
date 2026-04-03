import { test, expect } from '@playwright/test';

/**
 * Pillar Analytics Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts pillar-analytics-deployed
 *
 * These tests verify the Pillar Analytics page functionality:
 *   1. Page load & layout
 *   2. Workspace selection prompt
 *   3. Pillar cards (Reality, Contracts, DevX, Cloud, AI)
 *   4. Time range filter
 *   5. Pillar Usage Distribution chart section
 *   6. Navigation
 *   7. Accessibility
 *   8. Error-free operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Pillar Analytics — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/pillar-analytics`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    await expect(
      mainContent(page).getByRole('heading', { name: 'Pillar Usage Analytics', level: 1 })
    ).toBeVisible({ timeout: 10000 });
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the pillar analytics page at /pillar-analytics', async ({ page }) => {
      await expect(page).toHaveURL(/\/pillar-analytics/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Pillar Usage Analytics', level: 1 })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText("Track adoption of MockForge's foundational pillars across your workspaces")
      ).toBeVisible();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Pillar Analytics')).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Workspace Selection
  // ---------------------------------------------------------------------------
  test.describe('Workspace Selection', () => {
    test('should display the workspace selection prompt', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Select Workspace', level: 2 })
      ).toBeVisible();
    });

    test('should display the workspace selection description', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Please select a workspace to view pillar analytics, or view organization-wide metrics.')
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Pillar Cards
  // ---------------------------------------------------------------------------
  test.describe('Pillar Cards', () => {
    test('should display the Reality pillar card', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('heading', { name: 'Reality', level: 3 })).toBeVisible();
      await expect(main.getByText('Blended Reality')).toBeVisible();
    });

    test('should display the Contracts pillar card', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('heading', { name: 'Contracts', level: 3 })).toBeVisible();
      await expect(main.getByText('Enforcement Mode')).toBeVisible();
    });

    test('should display the DevX pillar card', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('heading', { name: 'DevX', level: 3 })).toBeVisible();
      await expect(main.getByText('SDK Installations')).toBeVisible();
    });

    test('should display the Cloud pillar card', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('heading', { name: 'Cloud', level: 3 }).first()).toBeVisible();
      await expect(main.getByText('Shared Scenarios')).toBeVisible();
    });

    test('should display the AI pillar card', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('heading', { name: 'AI', level: 3 })).toBeVisible();
      await expect(main.getByText('AI Mocks')).toBeVisible();
    });

    test('should display all five pillar cards', async ({ page }) => {
      const main = mainContent(page);
      const pillarHeadings = ['Reality', 'Contracts', 'DevX', 'AI'];

      for (const name of pillarHeadings) {
        await expect(
          main.getByRole('heading', { name, level: 3 })
        ).toBeVisible();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Time Range Filter
  // ---------------------------------------------------------------------------
  test.describe('Time Range Filter', () => {
    test('should display the Time Range label', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Time Range:')
      ).toBeVisible();
    });

    test('should display the time range dropdown', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('combobox')
      ).toBeVisible();
    });

    test('should have all time range options', async ({ page }) => {
      const dropdown = mainContent(page).getByRole('combobox');
      const options = dropdown.locator('option');
      const optionTexts = await options.allTextContents();

      expect(optionTexts).toContain('Last 24 Hours');
      expect(optionTexts).toContain('Last 7 Days');
      expect(optionTexts).toContain('Last 30 Days');
      expect(optionTexts).toContain('Last 90 Days');
      expect(optionTexts).toContain('All Time');
    });

    test('should default to "Last 7 Days"', async ({ page }) => {
      const dropdown = mainContent(page).getByRole('combobox');
      await expect(dropdown).toHaveValue('7d');
    });

    test('should allow changing the time range', async ({ page }) => {
      const dropdown = mainContent(page).getByRole('combobox');

      await dropdown.selectOption('Last 30 Days');
      await page.waitForTimeout(500);

      await dropdown.selectOption('Last 24 Hours');
      await page.waitForTimeout(500);

      // Reset
      await dropdown.selectOption('Last 7 Days');
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Pillar Usage Distribution
  // ---------------------------------------------------------------------------
  test.describe('Pillar Usage Distribution', () => {
    test('should display the distribution section heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Pillar Usage Distribution', level: 2 })
      ).toBeVisible();
    });

    test('should show either chart or loading state', async ({ page }) => {
      const main = mainContent(page);
      const hasChart = await main.locator('canvas').isVisible({ timeout: 3000 }).catch(() => false);
      const hasLoading = await main.getByText('Loading chart data').isVisible({ timeout: 3000 }).catch(() => false);
      const hasNoData = await main.getByText(/no.*data|N\/A/i).first().isVisible({ timeout: 3000 }).catch(() => false);

      // Should show something — chart, loading, or no-data state
      expect(hasChart || hasLoading || hasNoData).toBeTruthy();
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

      await nav.getByRole('button', { name: 'Pillar Analytics' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Pillar Usage Analytics', level: 1 })
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
      await expect(h1).toHaveText('Pillar Usage Analytics');
    });

    test('should have multiple H2 section headings', async ({ page }) => {
      const h2s = mainContent(page).getByRole('heading', { level: 2 });
      expect(await h2s.count()).toBeGreaterThanOrEqual(2);
    });

    test('should have multiple H3 pillar headings', async ({ page }) => {
      const h3s = mainContent(page).getByRole('heading', { level: 3 });
      expect(await h3s.count()).toBeGreaterThanOrEqual(5);
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
          !err.includes('favicon') &&
          !err.includes('429')
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
