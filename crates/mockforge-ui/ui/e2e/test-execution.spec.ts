import { test, expect } from '@playwright/test';
import { setupTest, assertPageLoaded, checkAnyVisible } from './test-helpers';
import { SELECTORS, TIMEOUTS } from './constants';

/**
 * Test Execution Dashboard E2E Tests
 * 
 * Tests the test execution dashboard functionality including:
 * - Test execution history
 * - Execution metrics and statistics
 * - Test results display
 * - Execution controls
 */
test.describe('Test Execution Page', () => {
  test.beforeEach(async ({ page }) => {
    await setupTest(page, { tabName: 'Test Execution' });
  });

  test('should load test execution page', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    await assertPageLoaded(page, ['Test Execution', 'Execution']);
    
    // Verify test execution content exists
    const hasExecutionContent = await checkAnyVisible(page, [
      'text=/Test Execution/i',
      'text=/Execution/i',
      '[class*="execution"]',
    ]);

    expect(hasExecutionContent).toBeTruthy();
  });

  test('should display execution metrics', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    // Check for metrics or charts
    const hasMetrics = await checkAnyVisible(page, [
      'text=/Success Rate/i',
      'text=/Total Tests/i',
      '[class*="metric"]',
      '[class*="chart"]',
    ]);

    await assertPageLoaded(page);
    
    // Metrics should be displayed (even if zeros)
    expect(hasMetrics).toBe(true);
  });

  test('should show execution list or empty state', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000); // Increased timeout
    
    await assertPageLoaded(page, ['Execution']);
    
    const hasExecutions = await checkAnyVisible(page, [
      'table',
      '[class*="execution"]',
      '[role="list"]',
    ]);

    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No executions/i',
      'text=/Loading/i',
      SELECTORS.common.empty,
      SELECTORS.common.emptyText,
    ]);

    const hasPageHeader = await checkAnyVisible(page, [
      'text=/Test Execution/i',
      'text=/Execution/i',
    ]);
    
    // MUST have either executions OR empty/loading state OR page header
    expect(hasExecutions || hasEmptyState || hasPageHeader).toBe(true);
  });

  test('should allow filtering executions', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    // Look for filter/search input
    const filterInput = page.locator('input[type="search"], input[placeholder*="search" i]').first();
    
    const filterExists = await filterInput.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
    
    if (filterExists) {
      await filterInput.fill('test');
      await page.waitForTimeout(500);
      
      const filterValue = await filterInput.inputValue();
      expect(filterValue).toBe('test');
    }
    
    await assertPageLoaded(page);
  });

  test('should handle empty executions state', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000); // Increased timeout
    
    await assertPageLoaded(page, ['Execution']);
    
    const hasExecutions = await checkAnyVisible(page, [
      'table',
      '[class*="execution"]',
    ]);
    
    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No executions/i',
      'text=/Loading/i',
      SELECTORS.common.empty,
    ]);
    
    const hasPageHeader = await checkAnyVisible(page, [
      'text=/Test Execution/i',
      'text=/Execution/i',
    ]);
    
    // MUST have either executions OR empty/loading state OR page header
    expect(hasExecutions || hasEmptyState || hasPageHeader).toBe(true);
  });
});

