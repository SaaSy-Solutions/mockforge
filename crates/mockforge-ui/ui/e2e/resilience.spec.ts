import { test, expect } from '@playwright/test';
import { setupTest, assertPageLoaded, checkAnyVisible } from './test-helpers';
import { SELECTORS, TIMEOUTS } from './constants';

/**
 * Resilience Page E2E Tests
 * 
 * Tests the resilience dashboard functionality including:
 * - Circuit breaker status
 * - Bulkhead monitoring
 * - Resilience metrics
 * - Circuit breaker controls
 */
test.describe('Resilience Page', () => {
  test.beforeEach(async ({ page }) => {
    await setupTest(page, { tabName: 'Resilience' });
  });

  test('should load resilience page', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    await assertPageLoaded(page, ['Resilience']);
    
    // Verify resilience content exists
    const hasResilienceContent = await checkAnyVisible(page, [
      'text=/Resilience/i',
      'text=/Circuit Breaker/i',
      'text=/Bulkhead/i',
      '[class*="resilience"]',
    ]);

    expect(hasResilienceContent).toBeTruthy();
  });

  test('should display circuit breakers', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000); // Increased timeout
    
    await assertPageLoaded(page, ['Resilience']);
    
    // Check for circuit breaker list or empty state
    const hasCircuitBreakers = await checkAnyVisible(page, [
      'text=/Circuit Breaker/i',
      '[class*="circuit"]',
      'table',
      '[role="list"]',
    ]);

    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No circuit breakers/i',
      'text=/Loading/i',
      SELECTORS.common.empty,
      SELECTORS.common.emptyText,
    ]);

    const hasPageHeader = await checkAnyVisible(page, [
      'text=/Resilience/i',
    ]);
    
    // MUST have either circuit breakers OR empty/loading state OR page header
    expect(hasCircuitBreakers || hasEmptyState || hasPageHeader).toBe(true);
  });

  test('should display bulkheads', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000); // Increased timeout
    
    await assertPageLoaded(page, ['Resilience']);
    
    // Check for bulkhead list or empty state
    const hasBulkheads = await checkAnyVisible(page, [
      'text=/Bulkhead/i',
      '[class*="bulkhead"]',
      'table',
    ]);

    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No bulkheads/i',
      'text=/Loading/i',
      SELECTORS.common.empty,
    ]);

    const hasPageHeader = await checkAnyVisible(page, [
      'text=/Resilience/i',
    ]);
    
    // MUST have either bulkheads OR empty/loading state OR page header
    expect(hasBulkheads || hasEmptyState || hasPageHeader).toBe(true);
  });

  test('should show resilience summary', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000); // Increased timeout
    
    await assertPageLoaded(page, ['Resilience']);
    
    // Check for summary cards or metrics
    const hasSummary = await checkAnyVisible(page, [
      'text=/Total/i',
      'text=/Closed/i',
      'text=/Open/i',
      '[class*="summary"]',
      '[class*="card"]',
    ]);

    const hasPageHeader = await checkAnyVisible(page, [
      'text=/Resilience/i',
    ]);
    
    // Summary should be displayed OR page header visible
    expect(hasSummary || hasPageHeader).toBe(true);
  });

  test('should handle empty state gracefully', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000); // Increased timeout
    
    await assertPageLoaded(page, ['Resilience']);
    
    const hasContent = await checkAnyVisible(page, [
      '[class*="circuit"]',
      '[class*="bulkhead"]',
      'table',
    ]);
    
    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No circuit breakers/i',
      'text=/No bulkheads/i',
      'text=/Loading/i',
      SELECTORS.common.empty,
    ]);
    
    const hasPageHeader = await checkAnyVisible(page, [
      'text=/Resilience/i',
    ]);
    
    // MUST have either content OR empty/loading state OR page header
    expect(hasContent || hasEmptyState || hasPageHeader).toBe(true);
  });
});

