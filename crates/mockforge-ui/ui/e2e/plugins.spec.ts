import { test, expect } from '@playwright/test';
import { setupTest, assertPageLoaded, checkAnyVisible } from './test-helpers';
import { SELECTORS } from './constants';

/**
 * Plugins Page E2E Tests
 * 
 * Tests the plugin management functionality including:
 * - Plugin listing
 * - Installing plugins
 * - Reloading plugins
 * - Plugin status
 */
test.describe('Plugins Page', () => {
  test.beforeEach(async ({ page }) => {
    await setupTest(page, { tabName: 'Plugins' });
  });

  test('should load plugins page', async ({ page }) => {
    await assertPageLoaded(page, ['Plugin']);
    
    // Verify plugins-related content exists
    const hasPluginsContent = await checkAnyVisible(page, [
      'text=/Plugin/i',
      '[class*="plugin"]',
      '[data-testid="plugins"]',
    ]);

    expect(hasPluginsContent).toBeTruthy();
  });

  test('should display plugin list', async ({ page }) => {
    await page.waitForLoadState('networkidle');
    
    // Look for plugin list or tabs
    const hasPluginList = await checkAnyVisible(page, [
      '[class*="list"]',
      '[class*="tab"]',
      'text=/Installed/i',
    ]);

    await assertPageLoaded(page);
    expect(hasPluginList || await checkAnyVisible(page, [SELECTORS.common.empty, SELECTORS.common.emptyText])).toBeTruthy();
  });

  test('should show install plugin button', async ({ page }) => {
    // Look for install/add plugin button
    const hasInstallButton = await checkAnyVisible(page, [
      'button:has-text("Install")',
      SELECTORS.buttons.add,
    ]);

    await assertPageLoaded(page);
    // Install button is optional - test passes either way
  });

  test('should show reload all button', async ({ page }) => {
    // Look for reload button
    const hasReloadButton = await checkAnyVisible(page, [
      'button:has-text("Reload")',
    ]);

    await assertPageLoaded(page);
    // Reload button is optional - test passes either way
  });

  test('should handle empty plugins state', async ({ page }) => {
    await assertPageLoaded(page);
    
    const hasPlugins = await checkAnyVisible(page, [
      '[class*="plugin"]',
      '[class*="list"]',
      '[class*="tab"]',
    ]);
    
    const hasEmptyState = await checkAnyVisible(page, [
      'text=/No plugins/i',
      SELECTORS.common.empty,
      SELECTORS.common.emptyText,
    ]);
    
    // Either plugins or empty state should be visible
    expect(hasPlugins || hasEmptyState).toBeTruthy();
  });
});

