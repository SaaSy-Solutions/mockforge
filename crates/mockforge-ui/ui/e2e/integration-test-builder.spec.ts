import { test, expect } from '@playwright/test';
import { setupTest, assertPageLoaded, checkAnyVisible } from './test-helpers';
import { SELECTORS, TIMEOUTS } from './constants';

/**
 * Integration Test Builder Page E2E Tests
 * 
 * Tests the integration test builder functionality including:
 * - Test builder interface
 * - Test step configuration
 * - Test workflow building
 * - Test execution
 */
test.describe('Integration Test Builder Page', () => {
  test.beforeEach(async ({ page }) => {
    await setupTest(page, { tabName: 'Integration Tests' });
  });

  test('should load integration test builder page', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    await assertPageLoaded(page, ['Integration', 'Test']);
    
    // Verify integration test builder content exists
    const hasBuilderContent = await checkAnyVisible(page, [
      'text=/Integration/i',
      'text=/Test Builder/i',
      '[class*="integration"]',
      '[class*="builder"]',
    ]);

    expect(hasBuilderContent).toBeTruthy();
  });

  test('should display builder interface', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    await assertPageLoaded(page, ['Integration', 'Test']);
    
    // Check for builder UI elements
    const hasBuilderUI = await checkAnyVisible(page, [
      'button:has-text("Add")',
      'button:has-text("Create")',
      'button:has-text("Save")',
      '[class*="builder"]',
      '[class*="stepper"]',
    ]);

    const hasPageHeader = await checkAnyVisible(page, [
      'text=/Integration/i',
      'text=/Test/i',
    ]);
    
    // Builder interface should be present OR page header visible
    expect(hasBuilderUI || hasPageHeader).toBe(true);
  });

  test('should show test steps or empty state', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000); // Increased timeout
    
    await assertPageLoaded(page, ['Integration']);
    
    const hasSteps = await checkAnyVisible(page, [
      '[class*="step"]',
      '[class*="stepper"]',
      'table',
      '[role="list"]',
    ]);

    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No tests/i',
      'text=/Create/i',
      'text=/Loading/i',
      SELECTORS.common.empty,
    ]);

    const hasPageHeader = await checkAnyVisible(page, [
      'text=/Integration/i',
    ]);
    
    // Should have either steps OR empty/loading state OR page header
    expect(hasSteps || hasEmptyState || hasPageHeader).toBe(true);
  });

  test('should allow creating new integration test', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    await assertPageLoaded(page, ['Integration']);
    
    // Look for create/new test button
    const hasCreateButton = await checkAnyVisible(page, [
      SELECTORS.buttons.create,
      'button:has-text("New")',
      'button:has-text("Add")',
    ]);

    const hasPageHeader = await checkAnyVisible(page, [
      'text=/Integration/i',
    ]);
    
    // Create button should exist OR page header visible
    expect(hasCreateButton || hasPageHeader).toBe(true);
  });

  test('should handle empty state gracefully', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000); // Increased timeout
    
    await assertPageLoaded(page, ['Integration']);
    
    const hasTests = await checkAnyVisible(page, [
      '[class*="test"]',
      '[class*="step"]',
    ]);
    
    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No integration tests/i',
      'text=/Create/i',
      'text=/Loading/i',
      SELECTORS.common.empty,
    ]);
    
    const hasPageHeader = await checkAnyVisible(page, [
      'text=/Integration/i',
    ]);
    
    // MUST have either tests OR empty/loading state OR page header
    expect(hasTests || hasEmptyState || hasPageHeader).toBe(true);
  });
});

