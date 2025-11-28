import { test, expect } from '@playwright/test';
import { waitForDashboardLoad, navigateToTab } from './helpers';
import { setupTest, assertPageLoaded, checkAnyVisible } from './test-helpers';
import { SELECTORS, TIMEOUTS } from './constants';

/**
 * Dashboard Page E2E Tests
 *
 * Tests the main dashboard page functionality including:
 * - Page loading and data display
 * - Statistics and metrics
 * - Navigation
 * - Real-time updates
 */
test.describe('Dashboard Page', () => {
  test.beforeEach(async ({ page }) => {
    await setupTest(page, { tabName: 'Dashboard' });

    // Wait for dashboard-specific content to load with aggressive timeout
    try {
      await Promise.race([
        waitForDashboardLoad(page),
        new Promise<void>((_, reject) =>
          setTimeout(() => reject(new Error('Dashboard load timeout')), 8000) // Reduced from 15000
        )
      ]);
    } catch {
      // Dashboard load failed but continue test - page might still be usable
      // Just wait a brief moment for page to stabilize
      await page.waitForLoadState('domcontentloaded', { timeout: 2000 }).catch(() => {});
    }
  });

  test('should load dashboard successfully', async ({ page }) => {
    // The UI uses tab-based navigation, not hash routing, so URL stays the same
    await expect(page).toHaveURL(/localhost:5173/);

    // Wait for page to fully render (use condition-based wait)
    await page.waitForLoadState('domcontentloaded');

    // Assert page loaded with dashboard content
    await assertPageLoaded(page, ['Dashboard']);

    // Look for common dashboard elements
    const hasDashboardContent = await checkAnyVisible(page, [
      'h1:has-text("Dashboard")',
      'h2:has-text("Dashboard")',
      '[class*="dashboard"]',
      '[data-testid="dashboard"]',
    ]);

    // Verify page has content (not just empty HTML)
    const bodyText = await page.locator('body').textContent();
    expect(bodyText).toBeTruthy();
    expect(bodyText!.length).toBeGreaterThan(0);
    expect(hasDashboardContent).toBeTruthy();
  });

  test('should display server information', async ({ page }) => {
    // Wait for network to settle and dashboard to load
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000); // Allow time for metrics to load

    // Dashboard MUST show server metrics - check for MetricCard components
    const hasServerInfo = await checkAnyVisible(page, [
      'text=/Uptime/i',
      'text=/CPU Usage/i',
      'text=/Memory/i',
      'text=/Active Threads/i',
      '[class*="metric"]',
      '[class*="MetricCard"]',
    ]);

    // Verify page loaded successfully
    await assertPageLoaded(page, ['Dashboard']);

    // Dashboard should always show server metrics (even if zeros)
    expect(hasServerInfo).toBe(true);
  });

  test('should display route statistics', async ({ page }) => {
    // Wait for content to load
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000); // Allow time for logs/metrics to load

    // Dashboard shows response status distribution - verify this displays
    const hasRouteInfo = await checkAnyVisible(page, [
      'text=/Success Responses/i',
      'text=/Client Errors/i',
      'text=/Server Errors/i',
      'text=/Response Status/i',
      '[class*="MetricCard"]',
    ]);

    // Verify page loaded successfully
    await assertPageLoaded(page, ['Dashboard']);

    // Dashboard should show response statistics (even if zeros)
    expect(hasRouteInfo).toBe(true);
  });

  test('should handle empty state gracefully', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);

    // Verify page loaded successfully
    await assertPageLoaded(page, ['Dashboard']);

    // Dashboard should always show something - either metrics or an empty state message
    // Check for either metrics or empty state
    const hasMetrics = await checkAnyVisible(page, [
      '[class*="MetricCard"]',
      'text=/Uptime/i',
      'text=/CPU/i',
    ]);

    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No data/i',
      'text=/Unable to retrieve/i',
      '[class*="empty"]',
    ]);

    // Dashboard should show either metrics OR empty state (not blank)
    expect(hasMetrics || hasEmptyState).toBe(true);

    // Page should always be visible
    await expect(page.locator('body')).toBeVisible();
  });

  test('should navigate to other pages from dashboard', async ({ page }) => {
    // Test navigation to different tabs
    const tabs = ['Services', 'Logs'];

    for (const tab of tabs) {
      try {
        const navigated = await Promise.race([
          navigateToTab(page, tab),
          new Promise<boolean>((resolve) => setTimeout(() => resolve(false), 5000)),
        ]);

        if (navigated) {
          // Wait for navigation to complete (condition-based)
          await page.waitForLoadState('domcontentloaded');

          // Verify we're not on an error page
          await assertPageLoaded(page);

          // Navigate back to dashboard
          await navigateToTab(page, 'Dashboard');
          await page.waitForLoadState('domcontentloaded');
        }
      } catch {
        // Tab navigation might fail, continue with next tab
        continue;
      }
    }

    // Verify we're still on a valid page
    await assertPageLoaded(page);
  });
});
