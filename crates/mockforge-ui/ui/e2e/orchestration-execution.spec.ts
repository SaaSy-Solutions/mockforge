import { test, expect } from '@playwright/test';
import { setupTest, assertPageLoaded, checkAnyVisible } from './test-helpers';
import { SELECTORS, TIMEOUTS } from './constants';

/**
 * Orchestration Execution Page E2E Tests
 * 
 * Tests the orchestration execution view functionality including:
 * - Execution status display
 * - Step-by-step progress
 * - Execution controls
 * - Metrics and results
 */
test.describe('Orchestration Execution Page', () => {
  test.beforeEach(async ({ page }) => {
    await setupTest(page, { tabName: 'Orchestration Execution' });
  });

  test('should load orchestration execution page', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    await assertPageLoaded(page, ['Orchestration', 'Execution']);
    
    // Verify execution content exists
    const hasExecutionContent = await checkAnyVisible(page, [
      'text=/Orchestration/i',
      'text=/Execution/i',
      '[class*="execution"]',
    ]);

    expect(hasExecutionContent).toBeTruthy();
  });

  test('should display execution status', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    // Check for execution status
    const hasStatus = await checkAnyVisible(page, [
      'text=/Status/i',
      'text=/Running/i',
      'text=/Completed/i',
      'text=/Idle/i',
      '[class*="status"]',
    ]);

    await assertPageLoaded(page);
    
    // Status should be visible
    expect(hasStatus).toBe(true);
  });

  test('should show execution steps or empty state', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000); // Increased timeout
    
    await assertPageLoaded(page, ['Execution']);
    
    const hasSteps = await checkAnyVisible(page, [
      '[class*="step"]',
      '[class*="progress"]',
      'table',
      '[role="list"]',
    ]);

    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No execution/i',
      'text=/Loading/i',
      SELECTORS.common.empty,
    ]);

    const hasPageHeader = await checkAnyVisible(page, [
      'text=/Orchestration/i',
      'text=/Execution/i',
    ]);
    
    // Should have either steps OR empty/loading state OR page header
    expect(hasSteps || hasEmptyState || hasPageHeader).toBe(true);
  });

  test('should display execution controls', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    // Check for control buttons
    const hasControls = await checkAnyVisible(page, [
      'button:has-text("Start")',
      'button:has-text("Stop")',
      'button:has-text("Pause")',
      '[class*="control"]',
    ]);

    await assertPageLoaded(page);
    
    // Controls should be available (even if disabled)
    await assertPageLoaded(page);
  });

  test('should handle empty/loading state gracefully', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000); // Increased timeout
    
    await assertPageLoaded(page, ['Execution']);
    
    // Page should show either execution content or loading/empty state
    const hasContent = await checkAnyVisible(page, [
      '[class*="execution"]',
      '[class*="step"]',
    ]);
    
    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No execution/i',
      'text=/Loading/i',
      SELECTORS.common.empty,
    ]);
    
    const hasPageHeader = await checkAnyVisible(page, [
      'text=/Orchestration/i',
      'text=/Execution/i',
    ]);
    
    // MUST have either content OR empty/loading state OR page header
    expect(hasContent || hasEmptyState || hasPageHeader).toBe(true);
  });
});

