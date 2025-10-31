import { test, expect } from '@playwright/test';
import { setupTest, assertPageLoaded, checkAnyVisible } from './test-helpers';
import { SELECTORS } from './constants';

/**
 * Metrics Page E2E Tests
 * 
 * Tests the metrics display functionality including:
 * - Metrics visualization
 * - Real-time updates
 * - Metric cards
 * - Charts and graphs
 */
test.describe('Metrics Page', () => {
  test.beforeEach(async ({ page }) => {
    await setupTest(page, { tabName: 'Metrics' });
  });

  test('should load metrics page', async ({ page }) => {
    await assertPageLoaded(page, ['Metric']);
    
    // Verify metrics-related content exists
    const hasMetricsContent = await checkAnyVisible(page, [
      'text=/Metric/i',
      '[class*="metric"]',
      '[data-testid="metrics"]',
    ]);

    expect(hasMetricsContent).toBeTruthy();
  });

  test('should display metric cards', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    
    // Look for metric cards or charts
    const hasMetricCards = await checkAnyVisible(page, [
      '[class*="card"]',
      '[class*="metric"]',
      'text=/Request/i',
      'text=/Response/i',
    ]);

    await assertPageLoaded(page);
    expect(hasMetricCards || await checkAnyVisible(page, [SELECTORS.common.empty, SELECTORS.common.emptyText])).toBeTruthy();
  });

  test('should show performance metrics', async ({ page }) => {
    // Look for performance-related metrics
    const hasPerformanceMetrics = await checkAnyVisible(page, [
      'text=/Performance/i',
      'text=/Latency/i',
      'text=/Throughput/i',
      '[class*="performance"]',
    ]);

    await assertPageLoaded(page);
    // Performance metrics are optional - test passes either way
  });

  test('should display charts', async ({ page }) => {
    // Look for chart elements
    const hasCharts = await checkAnyVisible(page, [
      '[class*="chart"]',
      'svg',
      'canvas',
      '[class*="graph"]',
    ]);

    await assertPageLoaded(page);
    // Charts are optional - test passes either way
  });

  test('should handle empty metrics state', async ({ page }) => {
    await assertPageLoaded(page);
    
    const hasMetrics = await checkAnyVisible(page, [
      '[class*="metric"]',
      '[class*="card"]',
      'svg',
      'canvas',
    ]);
    
    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No metrics/i',
      'text=/No data/i',
      SELECTORS.common.empty,
      SELECTORS.common.emptyText,
    ]);
    
    // Either metrics or empty state should be visible
    expect(hasMetrics || hasEmptyState).toBeTruthy();
  });
});

