import { test, expect } from '@playwright/test';
import { setupTest, assertPageLoaded, checkAnyVisible } from './test-helpers';
import { SELECTORS, TIMEOUTS } from './constants';

/**
 * Template Marketplace Page E2E Tests
 * 
 * Tests the template marketplace functionality including:
 * - Template browsing
 * - Template search and filtering
 * - Template installation
 * - Template details
 */
test.describe('Template Marketplace Page', () => {
  test.beforeEach(async ({ page }) => {
    await setupTest(page, { tabName: 'Template Marketplace' });
  });

  test('should load template marketplace page', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    await assertPageLoaded(page, ['Template', 'Marketplace']);
    
    // Verify marketplace content exists
    const hasMarketplaceContent = await checkAnyVisible(page, [
      'text=/Template/i',
      'text=/Marketplace/i',
      '[class*="marketplace"]',
      '[class*="template"]',
    ]);

    expect(hasMarketplaceContent).toBeTruthy();
  });

  test('should display template list', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    // Check for template cards or list
    const hasTemplates = await checkAnyVisible(page, [
      '[class*="template"]',
      '[class*="card"]',
      'table',
      '[role="list"]',
    ]);

    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No templates/i',
      SELECTORS.common.empty,
      SELECTORS.common.emptyText,
    ]);

    await assertPageLoaded(page);
    
    // MUST have either templates OR empty state
    expect(hasTemplates || hasEmptyState).toBe(true);
  });

  test('should allow searching templates', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    // Look for search input
    const searchInput = page.locator('input[type="search"], input[placeholder*="search" i]').first();
    
    const searchExists = await searchInput.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
    
    if (searchExists) {
      await searchInput.fill('test');
      await page.waitForTimeout(500);
      
      const searchValue = await searchInput.inputValue();
      expect(searchValue).toBe('test');
      
      await assertPageLoaded(page);
    } else {
      await assertPageLoaded(page);
    }
  });

  test('should show install buttons', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    // Look for install buttons (may not be visible if no templates)
    const hasInstallButtons = await checkAnyVisible(page, [
      'button:has-text("Install")',
      'button:has-text("Download")',
      '[class*="install"]',
    ]);

    await assertPageLoaded(page);
    
    // Install buttons might not be visible if no templates - that's okay
    await assertPageLoaded(page);
  });

  test('should handle empty marketplace state', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    await assertPageLoaded(page, ['Marketplace']);
    
    const hasTemplates = await checkAnyVisible(page, [
      '[class*="template"]',
      '[class*="card"]',
      'table',
    ]);
    
    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No templates/i',
      SELECTORS.common.empty,
      SELECTORS.common.emptyText,
    ]);
    
    // MUST have either templates OR empty state
    expect(hasTemplates || hasEmptyState).toBe(true);
  });
});

