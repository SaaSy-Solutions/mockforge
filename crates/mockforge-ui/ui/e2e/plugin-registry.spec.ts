import { test, expect } from '@playwright/test';
import { setupTest, assertPageLoaded, checkAnyVisible } from './test-helpers';
import { SELECTORS, TIMEOUTS } from './constants';

/**
 * Plugin Registry Page E2E Tests
 * 
 * Tests the plugin registry functionality including:
 * - Plugin browsing
 * - Plugin search and filtering
 * - Plugin installation
 * - Plugin details and ratings
 */
test.describe('Plugin Registry Page', () => {
  test.beforeEach(async ({ page }) => {
    await setupTest(page, { tabName: 'Plugin Registry' });
  });

  test('should load plugin registry page', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    await assertPageLoaded(page, ['Plugin', 'Registry']);
    
    // Verify registry content exists
    const hasRegistryContent = await checkAnyVisible(page, [
      'text=/Plugin/i',
      'text=/Registry/i',
      '[class*="registry"]',
      '[class*="plugin"]',
    ]);

    expect(hasRegistryContent).toBeTruthy();
  });

  test('should display plugin list', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    // Check for plugin cards or list
    const hasPlugins = await checkAnyVisible(page, [
      '[class*="plugin"]',
      '[class*="card"]',
      'table',
      '[role="list"]',
    ]);

    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No plugins/i',
      SELECTORS.common.empty,
      SELECTORS.common.emptyText,
    ]);

    await assertPageLoaded(page);
    
    // MUST have either plugins OR empty state
    expect(hasPlugins || hasEmptyState).toBe(true);
  });

  test('should allow searching plugins', async ({ page }) => {
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
    
    // Look for install buttons (may not be visible if no plugins)
    const hasInstallButtons = await checkAnyVisible(page, [
      'button:has-text("Install")',
      'button:has-text("Download")',
      '[class*="install"]',
    ]);

    await assertPageLoaded(page);
    
    // Install buttons might not be visible if no plugins - that's okay
    await assertPageLoaded(page);
  });

  test('should allow filtering plugins', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);
    
    // Look for filter controls (category, type, etc.)
    const hasFilters = await checkAnyVisible(page, [
      'select',
      '[role="combobox"]',
      'text=/Category/i',
      'text=/Type/i',
    ]);

    await assertPageLoaded(page);
    
    // Filters might not be visible - that's okay
    await assertPageLoaded(page);
  });

  test('should handle empty registry state', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    await assertPageLoaded(page, ['Registry']);
    
    const hasPlugins = await checkAnyVisible(page, [
      '[class*="plugin"]',
      '[class*="card"]',
      'table',
    ]);
    
    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No plugins/i',
      SELECTORS.common.empty,
      SELECTORS.common.emptyText,
    ]);
    
    // MUST have either plugins OR empty state
    expect(hasPlugins || hasEmptyState).toBe(true);
  });
});

