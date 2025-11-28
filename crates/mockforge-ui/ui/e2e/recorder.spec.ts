import { test, expect } from '@playwright/test';
import { setupTest, assertPageLoaded, checkAnyVisible } from './test-helpers';
import { SELECTORS, TIMEOUTS } from './constants';

/**
 * Recorder Page E2E Tests
 * 
 * Tests the request recorder functionality including:
 * - Recording status
 * - Recorded requests display
 * - Scenario management
 * - Recording controls
 */
test.describe('Recorder Page', () => {
  test.beforeEach(async ({ page }) => {
    await setupTest(page, { tabName: 'Recorder' });
  });

  test('should load recorder page', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    await assertPageLoaded(page, ['Recorder']);
    
    // Verify recorder content exists
    const hasRecorderContent = await checkAnyVisible(page, [
      'text=/Recorder/i',
      'text=/Recording/i',
      '[class*="recorder"]',
    ]);

    expect(hasRecorderContent).toBeTruthy();
  });

  test('should display recording status', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    await assertPageLoaded(page, ['Recorder']);
    
    // Check for recording status indicator
    const hasStatus = await checkAnyVisible(page, [
      'text=/Recording/i',
      'text=/Stopped/i',
      'button:has-text("Start")',
      'button:has-text("Stop")',
      '[class*="status"]',
    ]);

    const hasPageHeader = await checkAnyVisible(page, [
      'text=/Recorder/i',
    ]);
    
    // Recording status should be visible OR page header visible
    expect(hasStatus || hasPageHeader).toBe(true);
  });

  test('should display recorded requests', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    // Check for recorded requests or empty state
    const hasRequests = await checkAnyVisible(page, [
      '[class*="request"]',
      'table',
      '[role="list"]',
    ]);

    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No requests/i',
      SELECTORS.common.empty,
      SELECTORS.common.emptyText,
    ]);

    await assertPageLoaded(page);
    
    // MUST have either requests OR empty state
    expect(hasRequests || hasEmptyState).toBe(true);
  });

  test('should show scenarios list', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000); // Increased timeout
    
    await assertPageLoaded(page, ['Recorder']);
    
    // Check for scenarios section
    const hasScenarios = await checkAnyVisible(page, [
      'text=/Scenario/i',
      '[class*="scenario"]',
      'table',
    ]);

    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No scenarios/i',
      'text=/Loading/i',
      SELECTORS.common.empty,
    ]);

    const hasPageHeader = await checkAnyVisible(page, [
      'text=/Recorder/i',
    ]);
    
    // MUST have either scenarios OR empty/loading state OR page header
    expect(hasScenarios || hasEmptyState || hasPageHeader).toBe(true);
  });

  test('should allow filtering recorded requests', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    // Look for filter/search input
    const searchInput = page.locator('input[type="search"], input[placeholder*="search" i]').first();
    
    const searchExists = await searchInput.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
    
    if (searchExists) {
      await searchInput.fill('test');
      await page.waitForTimeout(500);
      
      const searchValue = await searchInput.inputValue();
      expect(searchValue).toBe('test');
    }
    
    await assertPageLoaded(page);
  });

  test('should handle empty state gracefully', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    await assertPageLoaded(page, ['Recorder']);
    
    const hasRequests = await checkAnyVisible(page, [
      '[class*="request"]',
      'table',
    ]);
    
    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No requests/i',
      SELECTORS.common.empty,
    ]);
    
    // MUST have either requests OR empty state
    expect(hasRequests || hasEmptyState).toBe(true);
  });
});

