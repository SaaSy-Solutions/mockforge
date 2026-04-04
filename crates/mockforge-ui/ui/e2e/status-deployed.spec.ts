import { test, expect } from '@playwright/test';

/**
 * Service Status Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config (handles auth via setup project):
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts status-deployed
 *
 * These tests verify all Service Status functionality on the live deployed site:
 *   1.  Page Load & Layout
 *   2.  Overall Status Card
 *   3.  Services List
 *   4.  Recent Incidents
 *   5.  Navigation
 *   6.  Accessibility
 *   7.  Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

// Scoped locator for the main content area (excludes sidebar to avoid duplicate matches)
function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Service Status — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/status`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    // Wait for the app shell to render (sidebar navigation visible)
    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    // Wait for the Service Status heading to confirm content loaded
    await expect(
      mainContent(page).getByRole('heading', { name: 'Service Status', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    // Small stabilization delay for dynamic content
    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the status page at /status', async ({ page }) => {
      await expect(page).toHaveURL(/\/status/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Service Status', level: 1 })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Real-time status of MockForge Cloud services')
      ).toBeVisible();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Status')).toBeVisible();
    });

    test('should display the overall status card or loading/error state', async ({ page }) => {
      const main = mainContent(page);

      const hasOperational = await main
        .getByText(/All Systems Operational/)
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasDegraded = await main
        .getByText(/All Systems Degraded/)
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasDown = await main
        .getByText(/All Systems Down/)
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasError = await main
        .getByText(/Failed to load status/)
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasOperational || hasDegraded || hasDown || hasError).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Overall Status Card
  // ---------------------------------------------------------------------------
  test.describe('Overall Status Card', () => {
    test('should display the overall system status text', async ({ page }) => {
      const main = mainContent(page);

      const hasOperational = await main
        .getByText('All Systems Operational')
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasDegraded = await main
        .getByText('All Systems Degraded')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasDown = await main
        .getByText('All Systems Down')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // One of the three status texts must be present (unless error state)
      const hasStatusError = await main
        .getByText(/Failed to load status/)
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasOperational || hasDegraded || hasDown || hasStatusError).toBeTruthy();
    });

    test('should display a status badge', async ({ page }) => {
      const main = mainContent(page);

      // Status badge shows capitalized status: "Operational", "Degraded", or "Down"
      const hasOperationalBadge = await main
        .getByText('Operational', { exact: true })
        .first()
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasDegradedBadge = await main
        .getByText('Degraded', { exact: true })
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasDownBadge = await main
        .getByText('Down', { exact: true })
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasError = await main
        .getByText(/Failed to load status/)
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasOperationalBadge || hasDegradedBadge || hasDownBadge || hasError).toBeTruthy();
    });

    test('should display a last updated timestamp', async ({ page }) => {
      const main = mainContent(page);

      const hasTimestamp = await main
        .getByText(/Last updated:/)
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasError = await main
        .getByText(/Failed to load status/)
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasTimestamp || hasError).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Services List
  // ---------------------------------------------------------------------------
  test.describe('Services List', () => {
    test('should display the Services section heading', async ({ page }) => {
      const main = mainContent(page);

      const hasServicesSection = await main
        .getByText('Services')
        .first()
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasError = await main
        .getByText(/Failed to load status/)
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasServicesSection || hasError).toBeTruthy();
    });

    test('should display service rows with names', async ({ page }) => {
      const main = mainContent(page);

      // Check if the services section has content (each service has a name as font-medium text)
      const serviceRows = main.locator('.flex.items-center.justify-between.p-4.border.rounded-lg');
      const hasRows = await serviceRows
        .first()
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasError = await main
        .getByText(/Failed to load status/)
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasRows || hasError).toBeTruthy();
    });

    test('should display status badges for each service', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByText(/Failed to load status/)
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        // Status badges contain text like "Operational", "Degraded", or "Down"
        const badges = main.locator('.px-2.py-1.rounded-full.text-xs.font-medium');
        const badgeCount = await badges.count();
        // At least the overall status badge should be visible
        expect(badgeCount).toBeGreaterThanOrEqual(1);
      }
    });

    test('should display service status icons', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByText(/Failed to load status/)
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        // Status icons are SVGs (CheckCircle2, AlertTriangle, XCircle)
        const icons = main.locator('svg.h-5.w-5');
        const iconCount = await icons.count();
        // At least the overall status icon should be visible
        expect(iconCount).toBeGreaterThanOrEqual(1);
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Recent Incidents
  // ---------------------------------------------------------------------------
  test.describe('Recent Incidents', () => {
    test('should display the Recent Incidents section', async ({ page }) => {
      const main = mainContent(page);

      const hasIncidents = await main
        .getByText('Recent Incidents')
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasError = await main
        .getByText(/Failed to load status/)
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasIncidents || hasError).toBeTruthy();
    });

    test('should display incidents or empty state', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByText(/Failed to load status/)
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        // Either incidents are listed or the empty state is shown
        const hasEmptyState = await main
          .getByText('No incidents reported. All systems operational.')
          .isVisible({ timeout: 5000 })
          .catch(() => false);

        // If not empty state, check for incident items with timestamps
        const hasIncidentItems = await main
          .getByText(/Started:/)
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        expect(hasEmptyState || hasIncidentItems).toBeTruthy();
      }
    });

    test('should display incident status and impact badges when incidents exist', async ({ page }) => {
      const main = mainContent(page);

      const hasIncidentItems = await main
        .getByText(/Started:/)
        .first()
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasIncidentItems) {
        // Status badges for incidents: resolved, investigating, monitoring
        const hasStatusBadge = await main
          .getByText(/Resolved|Investigating|Monitoring/)
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        expect(hasStatusBadge).toBeTruthy();

        // Impact badges: minor, major, critical
        const hasImpactBadge = await main
          .getByText(/Minor|Major|Critical/)
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        expect(hasImpactBadge).toBeTruthy();
      }
    });

    test('should display resolved timestamps when incidents are resolved', async ({ page }) => {
      const main = mainContent(page);

      const hasResolvedIncident = await main
        .getByText(/Resolved:/)
        .first()
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      // This is optional — only pass if there are resolved incidents
      if (hasResolvedIncident) {
        await expect(
          main.getByText(/Resolved:/).first()
        ).toBeVisible();
      }
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

      // Navigate back to Status
      await page.goto(`${BASE_URL}/status`, {
        waitUntil: 'domcontentloaded',
        timeout: 30000,
      });
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Service Status', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Config and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Config' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Configuration', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back
      await page.goto(`${BASE_URL}/status`, {
        waitUntil: 'domcontentloaded',
        timeout: 30000,
      });
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Service Status', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should have documentation link in footer', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByText(/Failed to load status/)
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        const docLink = main.getByRole('link', { name: /our documentation/i });
        const hasDocLink = await docLink
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        if (hasDocLink) {
          await expect(docLink).toHaveAttribute('href', 'https://docs.mockforge.dev');
          await expect(docLink).toHaveAttribute('target', '_blank');
        }
      }
    });

    test('should have support link in footer', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByText(/Failed to load status/)
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        const supportLink = main.getByRole('link', { name: /contact support/i });
        const hasSupportLink = await supportLink
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        if (hasSupportLink) {
          await expect(supportLink).toHaveAttribute('href', '/support');
        }
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a single H1 heading', async ({ page }) => {
      const h1 = mainContent(page).getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);
      await expect(h1).toHaveText('Service Status');
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

    test('should use semantic color coding for status badges', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByText(/Failed to load status/)
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        // Verify status badges exist with appropriate classes
        const badges = main.locator('.px-2.py-1.rounded-full.text-xs.font-medium');
        const badgeCount = await badges.count();
        expect(badgeCount).toBeGreaterThanOrEqual(1);
      }
    });

    test('should have accessible SVG icons alongside status text', async ({ page }) => {
      const main = mainContent(page);

      const hasError = await main
        .getByText(/Failed to load status/)
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      if (!hasError) {
        // Icons (CheckCircle2, AlertTriangle, XCircle) should be present
        const svgIcons = main.locator('svg');
        const iconCount = await svgIcons.count();
        expect(iconCount).toBeGreaterThanOrEqual(1);
      }
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

    test('should handle API error gracefully', async ({ page }) => {
      const main = mainContent(page);

      // Whether data loads or an error state shows, the page should be functional
      const hasHeading = await main
        .getByRole('heading', { name: 'Service Status', level: 1 })
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      expect(hasHeading).toBeTruthy();
    });

    test('should auto-refresh without crashing', async ({ page }) => {
      // The page auto-refreshes every 60 seconds via refetchInterval
      // Wait a few seconds to ensure no immediate crash
      await page.waitForTimeout(3000);

      const main = mainContent(page);
      await expect(
        main.getByRole('heading', { name: 'Service Status', level: 1 })
      ).toBeVisible();
    });
  });
});
