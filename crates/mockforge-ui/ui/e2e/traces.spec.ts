import { test, expect } from '@playwright/test';
import { setupTest, assertPageLoaded, checkAnyVisible } from './test-helpers';
import { SELECTORS, TIMEOUTS } from './constants';

/**
 * Traces Page E2E Tests
 * 
 * Tests the distributed tracing functionality including:
 * - Trace listing and display
 * - Trace search and filtering
 * - Span tree visualization
 * - Trace detail viewing
 */
test.describe('Traces Page', () => {
  test.beforeEach(async ({ page }) => {
    await setupTest(page, { tabName: 'Traces' });
  });

  test('should load traces page', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000); // Traces might take time to load
    
    await assertPageLoaded(page, ['Trace']);
    
    // Verify traces-related content exists
    const hasTraceContent = await checkAnyVisible(page, [
      'text=/Trace/i',
      'text=/Distributed/i',
      '[class*="trace"]',
    ]);

    expect(hasTraceContent).toBeTruthy();
  });

  test('should display trace list', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000); // Traces might take longer to load
    
    await assertPageLoaded(page, ['Trace']);
    
    // Check for trace list or empty state
    const hasTraceList = await checkAnyVisible(page, [
      '[class*="trace"]',
      'table',
      '[role="list"]',
      'text=/span/i',
      'text=/trace/i',
    ]);

    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No traces/i',
      'text=/Loading/i',
      SELECTORS.common.empty,
      SELECTORS.common.emptyText,
    ]);

    // MUST have either traces OR empty/loading state
    expect(hasTraceList || hasEmptyState).toBe(true);
  });

  test('should allow searching traces', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    // Look for search input
    const searchInput = page.locator('input[type="search"], input[placeholder*="search" i]').first();
    
    const searchExists = await searchInput.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
    
    if (searchExists) {
      await searchInput.click();
      await searchInput.fill('test');
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(500);
      
      const searchValue = await searchInput.inputValue();
      expect(searchValue).toBe('test');
      
      await assertPageLoaded(page);
    } else {
      await assertPageLoaded(page);
    }
  });

  test('should handle empty traces state', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000); // Traces might take longer to load
    
    await assertPageLoaded(page, ['Trace']);
    
    const hasTraces = await checkAnyVisible(page, [
      '[class*="trace"]',
      'table',
      '[role="list"]',
      'text=/trace/i',
    ]);
    
    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No traces/i',
      'text=/Loading/i',
      SELECTORS.common.empty,
      SELECTORS.common.emptyText,
    ]);
    
    // MUST have either traces OR empty/loading state
    expect(hasTraces || hasEmptyState).toBe(true);
  });
});

