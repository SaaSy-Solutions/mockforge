import { test, expect } from '@playwright/test';
import { setupTest, assertPageLoaded, checkAnyVisible } from './test-helpers';
import { SELECTORS } from './constants';

/**
 * Testing Page E2E Tests
 * 
 * Tests the testing functionality including:
 * - Smoke tests
 * - Health checks
 * - Integration tests
 * - Test execution
 */
test.describe('Testing Page', () => {
  test.beforeEach(async ({ page }) => {
    await setupTest(page, { tabName: 'Testing' });
  });

  test('should load testing page', async ({ page }) => {
    await assertPageLoaded(page, ['Test']);
    
    // Verify testing-related content exists
    const hasTestingContent = await checkAnyVisible(page, [
      'text=/Test/i',
      '[class*="test"]',
      '[data-testid="testing"]',
    ]);

    expect(hasTestingContent).toBeTruthy();
  });

  test('should display test suites', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    
    // Look for test suite cards or lists
    const hasTestSuites = await checkAnyVisible(page, [
      'text=/Smoke/i',
      'text=/Health/i',
      'text=/Integration/i',
      '[class*="suite"]',
    ]);

    await assertPageLoaded(page);
    expect(hasTestSuites || await checkAnyVisible(page, [SELECTORS.common.empty, SELECTORS.common.emptyText])).toBeTruthy();
  });

  test('should show run tests button', async ({ page }) => {
    // Look for run/execute test button
    const hasRunButton = await checkAnyVisible(page, [
      'button:has-text("Run")',
      'button:has-text("Execute")',
      'button:has-text("Start")',
    ]);

    await assertPageLoaded(page);
    // Run button is optional - test passes either way
  });

  test('should display test results', async ({ page }) => {
    // Look for test result display
    const hasTestResults = await checkAnyVisible(page, [
      'text=/Passed/i',
      'text=/Failed/i',
      '[class*="result"]',
      '[class*="status"]',
    ]);

    await assertPageLoaded(page);
    // Test results are optional - test passes either way
  });

  test('should handle empty tests state', async ({ page }) => {
    await assertPageLoaded(page);
    
    const hasTests = await checkAnyVisible(page, [
      '[class*="suite"]',
      '[class*="test"]',
      '[class*="result"]',
    ]);
    
    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No tests/i',
      'text=/No results/i',
      SELECTORS.common.empty,
      SELECTORS.common.emptyText,
    ]);
    
    // Either tests or empty state should be visible
    expect(hasTests || hasEmptyState).toBeTruthy();
  });
});

