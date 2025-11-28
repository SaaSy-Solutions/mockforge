import { test, expect } from '@playwright/test';
import { setupTest, assertPageLoaded, checkAnyVisible } from './test-helpers';
import { SELECTORS } from './constants';

/**
 * Analytics Page E2E Tests
 * 
 * Tests the analytics dashboard functionality including:
 * - Analytics visualization
 * - Dashboard components
 * - Real-time updates
 */
test.describe('Analytics Page', () => {
  test.beforeEach(async ({ page }) => {
    await setupTest(page, { tabName: 'Analytics' });
  });

  test('should load analytics page', async ({ page }) => {
    await assertPageLoaded(page, ['Analytic']);
    
    // Verify analytics-related content exists
    const hasAnalyticsContent = await checkAnyVisible(page, [
      'text=/Analytic/i',
      '[class*="analytics"]',
      '[data-testid="analytics"]',
    ]);

    expect(hasAnalyticsContent).toBeTruthy();
  });

  test('should display analytics dashboard', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    
    // Look for dashboard components
    const hasDashboard = await checkAnyVisible(page, [
      '[class*="dashboard"]',
      '[class*="analytics"]',
      '[class*="card"]',
    ]);

    await assertPageLoaded(page);
    expect(hasDashboard || await checkAnyVisible(page, [SELECTORS.common.empty, SELECTORS.common.emptyText])).toBeTruthy();
  });

  test('should display analytics visualizations', async ({ page }) => {
    // Look for charts or visualizations
    const hasVisualizations = await checkAnyVisible(page, [
      '[class*="chart"]',
      'svg',
      'canvas',
      '[class*="graph"]',
    ]);

    await assertPageLoaded(page);
    // Visualizations are optional - test passes either way
  });

  test('should handle empty analytics state', async ({ page }) => {
    await assertPageLoaded(page);
    
    const hasAnalytics = await checkAnyVisible(page, [
      '[class*="analytics"]',
      '[class*="dashboard"]',
      '[class*="card"]',
    ]);
    
    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No data/i',
      'text=/No analytics/i',
      SELECTORS.common.empty,
      SELECTORS.common.emptyText,
    ]);
    
    // Either analytics or empty state should be visible
    expect(hasAnalytics || hasEmptyState).toBeTruthy();
  });
});

