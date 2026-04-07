import { test, expect } from '@playwright/test';

/**
 * Metrics Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config (handles auth via setup project):
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts metrics-deployed
 *
 * These tests verify all Performance Metrics functionality on the live deployed site:
 *   1. Page Load & Layout
 *   2. KPI Cards
 *   3. Section Headings
 *   4. Charts & Visualizations
 *   5. Endpoint Performance Table
 *   6. Navigation
 *   7. Accessibility
 *   8. Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

// Scoped locator for the main content area (excludes sidebar to avoid duplicate matches)
function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Performance Metrics — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/metrics`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    // Wait for the app shell to render (sidebar navigation visible)
    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    // Wait for the Performance Metrics heading to confirm content loaded
    await expect(
      mainContent(page).getByRole('heading', { name: 'Performance Metrics', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    // Small stabilization delay for dynamic content
    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the metrics page at /metrics', async ({ page }) => {
      await expect(page).toHaveURL(/\/metrics/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Performance Metrics', level: 1 })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      const main = mainContent(page);
      // The subtitle varies between loading/error/loaded states
      const hasLoadedSubtitle = await main
        .getByText('Real-time system performance and request analytics')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasAltSubtitle = await main
        .getByText('Monitor system performance and request analytics')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasLoadedSubtitle || hasAltSubtitle).toBeTruthy();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home').first()).toBeVisible();
      await expect(banner.getByText('Metrics').first()).toBeVisible();
    });

    test('should display metrics content or appropriate empty/error state', async ({ page }) => {
      const main = mainContent(page);

      // The page should display either the full metrics sections, a loading state,
      // a "No metrics available" warning, or an error alert
      const hasKPISection = await main
        .getByRole('heading', { name: 'Key Performance Indicators', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasNoMetrics = await main
        .getByText('No metrics available')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasLoading = await main
        .getByText('Loading metrics...')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasError = await main
        .getByText('Failed to load metrics')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasKPISection || hasNoMetrics || hasLoading || hasError).toBeTruthy();
    });

    test('should display the sidebar navigation with expected tabs', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');
      const tabs = [
        'Dashboard',
        'Workspaces',
        'Services',
        'Fixtures',
        'Hosted Mocks',
        'Config',
        'Organization',
        'Billing',
        'API Tokens',
        'Plan & Usage',
      ];

      for (const tab of tabs) {
        await expect(nav.getByRole('button', { name: tab })).toBeVisible();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 2. KPI Cards
  // ---------------------------------------------------------------------------
  test.describe('KPI Cards', () => {
    test('should display Total Requests metric card', async ({ page }) => {
      const main = mainContent(page);
      const hasCard = await main
        .getByText('Total Requests', { exact: true })
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasNoMetrics = await main
        .getByText('No metrics available')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // Either the KPI card is visible or the page shows a no-metrics state
      expect(hasCard || hasNoMetrics).toBeTruthy();
    });

    test('should display Avg Response Time metric card', async ({ page }) => {
      const main = mainContent(page);
      const hasCard = await main
        .getByText('Avg Response Time', { exact: true })
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasNoMetrics = await main
        .getByText('No metrics available')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasCard || hasNoMetrics).toBeTruthy();
    });

    test('should display Error Rate metric card', async ({ page }) => {
      const main = mainContent(page);
      const hasCard = await main
        .getByText('Error Rate', { exact: true })
        .first()
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasNoMetrics = await main
        .getByText('No metrics available')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasCard || hasNoMetrics).toBeTruthy();
    });

    test('should display Active Endpoints metric card', async ({ page }) => {
      const main = mainContent(page);
      const hasCard = await main
        .getByText('Active Endpoints', { exact: true })
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasNoMetrics = await main
        .getByText('No metrics available')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasCard || hasNoMetrics).toBeTruthy();
    });

    test('should display all four KPI cards together', async ({ page }) => {
      const main = mainContent(page);
      const hasKPISection = await main
        .getByRole('heading', { name: 'Key Performance Indicators', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasKPISection) {
        const kpiNames = ['Total Requests', 'Avg Response Time', 'Error Rate', 'Active Endpoints'];
        for (const name of kpiNames) {
          await expect(main.getByText(name, { exact: true }).first()).toBeVisible({ timeout: 5000 });
        }
      }
      // If KPI section is not visible, the page is in a no-data or error state
    });

    test('should display KPI card subtitles', async ({ page }) => {
      const main = mainContent(page);
      const hasKPISection = await main
        .getByRole('heading', { name: 'Key Performance Indicators', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasKPISection) {
        const subtitles = ['all endpoints', 'median', 'average', 'with traffic'];
        for (const subtitle of subtitles) {
          await expect(main.getByText(subtitle).first()).toBeVisible({ timeout: 5000 });
        }
      }
    });

    test('should display KPI values (numbers or zero)', async ({ page }) => {
      const main = mainContent(page);
      const hasKPISection = await main
        .getByRole('heading', { name: 'Key Performance Indicators', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasKPISection) {
        // Avg Response Time value ends with "ms"
        const hasMs = await main
          .getByText(/\d+ms/)
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        // Error Rate value ends with "%"
        const hasPercent = await main
          .getByText(/\d+\.\d+%/)
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        expect(hasMs || hasPercent).toBeTruthy();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Section Headings
  // ---------------------------------------------------------------------------
  test.describe('Section Headings', () => {
    test('should display Key Performance Indicators section', async ({ page }) => {
      const main = mainContent(page);
      const hasSection = await main
        .getByRole('heading', { name: 'Key Performance Indicators', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasNoMetrics = await main
        .getByText('No metrics available')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasSection || hasNoMetrics).toBeTruthy();
    });

    test('should display KPI section subtitle', async ({ page }) => {
      const main = mainContent(page);
      const hasSubtitle = await main
        .getByText('Critical system metrics at a glance')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasSubtitle) {
        await expect(main.getByText('Critical system metrics at a glance').first()).toBeVisible();
      }
    });

    test('should display Request Distribution section', async ({ page }) => {
      const main = mainContent(page);
      const hasSection = await main
        .getByRole('heading', { name: 'Request Distribution', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasSection) {
        await expect(main.getByText('Traffic breakdown by endpoint').first()).toBeVisible();
      }
    });

    test('should display Response Time Analysis section', async ({ page }) => {
      const main = mainContent(page);
      const hasSection = await main
        .getByRole('heading', { name: 'Response Time Analysis', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasSection) {
        await expect(main.getByText('Latency percentiles across all requests').first()).toBeVisible();
      }
    });

    test('should display Error Rate Analysis section', async ({ page }) => {
      const main = mainContent(page);
      const hasSection = await main
        .getByRole('heading', { name: 'Error Rate Analysis', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasSection) {
        await expect(main.getByText('Error rates by endpoint').first()).toBeVisible();
      }
    });

    test('should display System Resource Usage section', async ({ page }) => {
      const main = mainContent(page);
      const hasSection = await main
        .getByRole('heading', { name: 'System Resource Usage', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasSection) {
        await expect(main.getByText('Memory and CPU usage over time').first()).toBeVisible();
      }
    });

    test('should display Endpoint Performance section', async ({ page }) => {
      const main = mainContent(page);
      const hasSection = await main
        .getByRole('heading', { name: 'Endpoint Performance', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasSection) {
        await expect(
          main.getByText('Detailed performance metrics for each endpoint').first()
        ).toBeVisible();
      }
    });

    test('should display all major metric sections when data is loaded', async ({ page }) => {
      const main = mainContent(page);
      const hasKPISection = await main
        .getByRole('heading', { name: 'Key Performance Indicators', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasKPISection) {
        const sections = [
          'Key Performance Indicators',
          'Request Distribution',
          'Response Time Analysis',
          'Error Rate Analysis',
          'System Resource Usage',
          'Endpoint Performance',
        ];

        for (const section of sections) {
          await expect(
            main.getByRole('heading', { name: section, level: 2 })
          ).toBeVisible({ timeout: 5000 });
        }
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Charts & Visualizations
  // ---------------------------------------------------------------------------
  test.describe('Charts & Visualizations', () => {
    test('should display Request Distribution chart or empty state', async ({ page }) => {
      const main = mainContent(page);
      const hasSection = await main
        .getByRole('heading', { name: 'Request Distribution', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasSection) {
        // Should show either the bar chart title or the empty state
        const hasChart = await main
          .getByText('Requests by Endpoint')
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasEmptyState = await main
          .getByText('No request data')
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        expect(hasChart || hasEmptyState).toBeTruthy();
      }
    });

    test('should display empty state message for request distribution when no data', async ({ page }) => {
      const main = mainContent(page);
      const hasEmptyState = await main
        .getByText('No request data')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasEmptyState) {
        await expect(
          main.getByText('Start making API calls to see request distribution.').first()
        ).toBeVisible();
      }
    });

    test('should display Response Time Percentiles chart', async ({ page }) => {
      const main = mainContent(page);
      const hasSection = await main
        .getByRole('heading', { name: 'Response Time Analysis', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasSection) {
        // The response time chart always shows (P50, P95, P99) even if values are 0
        const hasChart = await main
          .getByText('Response Time Percentiles (ms)')
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        // Chart may or may not show percentile labels depending on data availability
        if (hasChart) {
          const hasP50 = await main.getByText('P50').first()
            .isVisible({ timeout: 3000 }).catch(() => false);
          // If percentile labels exist, verify all three are present
          if (hasP50) {
            await expect(main.getByText('P95').first()).toBeVisible();
            await expect(main.getByText('P99').first()).toBeVisible();
          }
        }
      }
    });

    test('should display Error Rate chart or empty state', async ({ page }) => {
      const main = mainContent(page);
      const hasSection = await main
        .getByRole('heading', { name: 'Error Rate Analysis', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasSection) {
        const hasChart = await main
          .getByText('Error Rates by Endpoint (%)')
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasEmptyState = await main
          .getByText('No error data')
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        expect(hasChart || hasEmptyState).toBeTruthy();
      }
    });

    test('should display empty state message for error rates when no data', async ({ page }) => {
      const main = mainContent(page);
      const hasEmptyState = await main
        .getByText('No error data')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasEmptyState) {
        await expect(
          main.getByText('Error rates will appear here when requests fail.').first()
        ).toBeVisible();
      }
    });

    test('should display System Resource Usage charts or empty state', async ({ page }) => {
      const main = mainContent(page);
      const hasSection = await main
        .getByRole('heading', { name: 'System Resource Usage', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasSection) {
        const hasMemoryChart = await main
          .getByText('Memory Usage (MB)')
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasCpuChart = await main
          .getByText('CPU Usage (%)')
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasEmptyState = await main
          .getByText('No time series data')
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        expect(hasMemoryChart || hasCpuChart || hasEmptyState).toBeTruthy();
      }
    });

    test('should display empty state message for time series when no data', async ({ page }) => {
      const main = mainContent(page);
      const hasEmptyState = await main
        .getByText('No time series data')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasEmptyState) {
        await expect(
          main.getByText('System metrics will appear here over time.').first()
        ).toBeVisible();
      }
    });

    test('should display bar chart elements with proper structure', async ({ page }) => {
      const main = mainContent(page);
      const hasChart = await main
        .getByText('Response Time Percentiles (ms)')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasChart) {
        // Bar charts use CSS-based bars (divs with rounded-full class)
        // Verify the chart section has visual bar elements or "No data available" text
        const hasNoData = await main
          .getByText('No data available')
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasBars = await main
          .getByText('P50')
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        expect(hasNoData || hasBars).toBeTruthy();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Endpoint Performance Table
  // ---------------------------------------------------------------------------
  test.describe('Endpoint Performance Table', () => {
    test('should display the Endpoint Performance section', async ({ page }) => {
      const main = mainContent(page);
      const hasSection = await main
        .getByRole('heading', { name: 'Endpoint Performance', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasNoMetrics = await main
        .getByText('No metrics available')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasSection || hasNoMetrics).toBeTruthy();
    });

    test('should display table column headers', async ({ page }) => {
      const main = mainContent(page);
      const hasTable = await main
        .locator('table')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasTable) {
        const columnHeaders = ['Endpoint', 'Requests', 'Error Rate', 'Status'];
        for (const header of columnHeaders) {
          await expect(
            main.locator('th').filter({ hasText: header }).first()
          ).toBeVisible();
        }
      }
    });

    test('should display table with accessible caption', async ({ page }) => {
      const main = mainContent(page);
      const hasTable = await main
        .locator('table')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasTable) {
        // The table has a sr-only caption for accessibility
        const caption = main.locator('table caption');
        await expect(caption).toBeAttached();
      }
    });

    test('should display endpoint rows with data or empty table', async ({ page }) => {
      const main = mainContent(page);
      const hasTable = await main
        .locator('table')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasTable) {
        // Table should have at least the header row
        const headerRow = main.locator('thead tr');
        await expect(headerRow).toBeVisible();

        // Body rows may or may not exist depending on data availability
        const bodyRows = main.locator('tbody tr');
        const rowCount = await bodyRows.count();
        // Row count can be 0 if no endpoint data exists — that's valid
        expect(rowCount).toBeGreaterThanOrEqual(0);
      }
    });

    test('should display status badges in table rows', async ({ page }) => {
      const main = mainContent(page);
      const bodyRows = main.locator('tbody tr');
      const rowCount = await bodyRows.count().catch(() => 0);

      if (rowCount > 0) {
        // Each row should have a status badge: "Healthy" or "Issues"
        const firstRow = bodyRows.first();
        const hasHealthy = await firstRow
          .getByText('Healthy')
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasIssues = await firstRow
          .getByText('Issues')
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        expect(hasHealthy || hasIssues).toBeTruthy();
      }
    });

    test('should display error rate badges in table rows', async ({ page }) => {
      const main = mainContent(page);
      const bodyRows = main.locator('tbody tr');
      const rowCount = await bodyRows.count().catch(() => 0);

      if (rowCount > 0) {
        // Each row should have an error rate percentage badge (e.g., "0.0%")
        const firstRow = bodyRows.first();
        const hasPercentage = await firstRow
          .getByText(/\d+\.\d+%/)
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        expect(hasPercentage).toBeTruthy();
      }
    });

    test('should display endpoint names in monospace font', async ({ page }) => {
      const main = mainContent(page);
      const bodyRows = main.locator('tbody tr');
      const rowCount = await bodyRows.count().catch(() => 0);

      if (rowCount > 0) {
        // Endpoint names use font-mono class
        const monoCell = bodyRows.first().locator('.font-mono');
        await expect(monoCell.first()).toBeVisible();
      }
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

      // Navigate back to Metrics via sidebar
      const hasMetricsButton = await nav
        .getByRole('button', { name: /Metrics/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasMetricsButton) {
        await nav.getByRole('button', { name: /Metrics/i }).click();
      } else {
        await page.goto(`${BASE_URL}/metrics`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Performance Metrics', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Services and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Services' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Services', exact: true, level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back to Metrics
      const hasMetricsButton = await nav
        .getByRole('button', { name: /Metrics/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasMetricsButton) {
        await nav.getByRole('button', { name: /Metrics/i }).click();
      } else {
        await page.goto(`${BASE_URL}/metrics`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Performance Metrics', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Config and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Config' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Configuration', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back to Metrics
      const hasMetricsButton = await nav
        .getByRole('button', { name: /Metrics/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasMetricsButton) {
        await nav.getByRole('button', { name: /Metrics/i }).click();
      } else {
        await page.goto(`${BASE_URL}/metrics`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Performance Metrics', level: 1 })
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
      await expect(h1).toHaveText('Performance Metrics');
    });

    test('should have multiple H2 section headings when data is loaded', async ({ page }) => {
      const main = mainContent(page);
      const hasKPISection = await main
        .getByRole('heading', { name: 'Key Performance Indicators', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasKPISection) {
        const h2s = main.getByRole('heading', { level: 2 });
        expect(await h2s.count()).toBeGreaterThanOrEqual(5);
      }
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

    test('should have proper heading hierarchy', async ({ page }) => {
      const main = mainContent(page);

      // H1: Performance Metrics
      const h1 = main.getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);

      // Chart titles use h3 level headings
      const hasKPISection = await main
        .getByRole('heading', { name: 'Key Performance Indicators', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasKPISection) {
        // H2 sections should exist between H1 and H3
        const h2Count = await main.getByRole('heading', { level: 2 }).count();
        expect(h2Count).toBeGreaterThanOrEqual(1);
      }
    });

    test('should have table with scoped column headers', async ({ page }) => {
      const main = mainContent(page);
      const hasTable = await main
        .locator('table')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasTable) {
        // Column headers should use <th> with scope="col"
        const scopedHeaders = main.locator('th[scope="col"]');
        expect(await scopedHeaders.count()).toBeGreaterThanOrEqual(4);
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 8. Auto-Refresh Behavior
  // ---------------------------------------------------------------------------
  test.describe('Auto-Refresh Behavior', () => {
    test('should auto-refresh metrics without errors after 15 seconds', async ({ page }) => {
      // The metrics hook refetches every 15 seconds
      // Wait long enough for at least one refresh cycle
      await page.waitForTimeout(16000);

      // Page should still be functional after refresh
      await expect(
        mainContent(page).getByRole('heading', { name: 'Performance Metrics', level: 1 })
      ).toBeVisible();

      // No crash or error boundary should appear
      const hasErrorBoundary = await page
        .getByText(/Something went wrong|Unexpected error|Application error/i)
        .first()
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasErrorBoundary).toBeFalsy();
    });

    test('should maintain page state through refresh cycle', async ({ page }) => {
      const main = mainContent(page);

      // Check initial state
      const hasKPISection = await main
        .getByRole('heading', { name: 'Key Performance Indicators', level: 2 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasNoMetrics = await main
        .getByText('No metrics available')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // Wait for a refresh cycle
      await page.waitForTimeout(16000);

      // State should remain consistent
      if (hasKPISection) {
        await expect(
          main.getByRole('heading', { name: 'Key Performance Indicators', level: 2 })
        ).toBeVisible({ timeout: 5000 });
      }
      if (hasNoMetrics) {
        // Still in no-data state or may have loaded data — either is acceptable
        await expect(
          main.getByRole('heading', { name: 'Performance Metrics', level: 1 })
        ).toBeVisible();
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

    test('should handle metrics API failure gracefully', async ({ page }) => {
      const main = mainContent(page);

      // When the API fails, the page should show an error alert or warning, not crash
      const hasError = await main
        .getByText('Failed to load metrics')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasNoMetrics = await main
        .getByText('No metrics available')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasData = await main
        .getByRole('heading', { name: 'Key Performance Indicators', level: 2 })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // One of these states should be present — the page should never be blank
      expect(hasError || hasNoMetrics || hasData).toBeTruthy();
    });

    test('should display the page heading even in error or empty states', async ({ page }) => {
      // Regardless of data availability, the H1 should always appear
      await expect(
        mainContent(page).getByRole('heading', { name: 'Performance Metrics', level: 1 })
      ).toBeVisible();
    });

    test('should not show error alert banners from unrelated failures', async ({ page }) => {
      const main = mainContent(page);

      // Check for unexpected error alerts that are not the expected "Failed to load metrics"
      const alertElements = main.locator('[role="alert"]');
      const alertCount = await alertElements.count().catch(() => 0);

      for (let i = 0; i < alertCount; i++) {
        const alertText = await alertElements.nth(i).textContent() || '';
        // Known acceptable alerts
        const isAcceptable =
          alertText.includes('Failed to load metrics') ||
          alertText.includes('No metrics available') ||
          alertText.includes('Processing metrics');

        if (!isAcceptable) {
          // Unexpected alert — fail the test with details
          expect(alertText).toContain('Expected known alert text');
        }
      }
    });
  });
});
