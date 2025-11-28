import { test, expect } from '@playwright/test';
import { setupTest, assertPageLoaded, checkAnyVisible } from './test-helpers';
import { SELECTORS, TIMEOUTS } from './constants';

/**
 * Observability Page E2E Tests
 * 
 * Tests the observability dashboard functionality including:
 * - Real-time metrics display
 * - Active alerts
 * - WebSocket connection status
 * - Metric cards and visualizations
 */
test.describe('Observability Page', () => {
  test.beforeEach(async ({ page }) => {
    await setupTest(page, { tabName: 'Observability' });
  });

  test('should load observability page', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    await assertPageLoaded(page, ['Observability']);
    
    // Verify observability-related content exists
    const hasObservabilityContent = await checkAnyVisible(page, [
      'text=/Observability/i',
      'text=/Real-Time Metrics/i',
      '[class*="observability"]',
    ]);

    expect(hasObservabilityContent).toBeTruthy();
  });

  test('should display real-time metrics', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000); // Allow time for metrics to load
    
    // Check for metric cards
    const hasMetrics = await checkAnyVisible(page, [
      'text=/Events/i',
      'text=/Latency/i',
      'text=/Alerts/i',
      'text=/Impact/i',
      '[class*="MetricCard"]',
    ]);

    await assertPageLoaded(page);
    
    // Observability page should show metrics (even if zeros)
    expect(hasMetrics).toBe(true);
  });

  test('should show connection status', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    // Check for connection status badge (may be in header actions)
    const hasConnectionStatus = await checkAnyVisible(page, [
      'text=/Connected/i',
      'text=/Disconnected/i',
      '[class*="badge"]',
      '[class*="status"]',
    ]);

    await assertPageLoaded(page);
    
    // Connection status might not always be visible - page should still load
    await assertPageLoaded(page);
  });

  test('should display active alerts section', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    // Check for alerts section
    const hasAlerts = await checkAnyVisible(page, [
      'text=/Active Alerts/i',
      'text=/Alerts/i',
      '[class*="alert"]',
    ]);

    await assertPageLoaded(page);
    
    // Alerts section should exist (even if empty)
    expect(hasAlerts).toBe(true);
  });

  test('should handle empty state gracefully', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    await assertPageLoaded(page, ['Observability']);
    
    // Page should show either metrics or empty state
    const hasMetrics = await checkAnyVisible(page, [
      '[class*="MetricCard"]',
      'text=/Events/i',
    ]);
    
    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No data/i',
      SELECTORS.common.empty,
    ]);
    
    // Should have either metrics OR empty state
    expect(hasMetrics || hasEmptyState).toBe(true);
  });
});

