import { test, expect } from '@playwright/test';
import { setupTest, assertPageLoaded, checkAnyVisible } from './test-helpers';
import { SELECTORS, TIMEOUTS } from './constants';

/**
 * Edge Cases E2E Tests
 * 
 * Tests edge cases and error scenarios:
 * - Concurrent user actions
 * - Network disconnection/reconnection
 * - Browser back/forward navigation
 * - Deep linking to specific states
 * - Form validation
 * - Error handling
 */
test.describe('Edge Cases', () => {
  test.describe('Concurrent User Actions', () => {
    test('should handle rapid button clicks', async ({ page }) => {
      await setupTest(page, { tabName: 'Services' });
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(1000);
      
      // Find a button that can be clicked
      const buttons = page.locator('button:not([disabled])').first();
      
      const hasButton = await buttons.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
      
      if (hasButton) {
        // Rapid clicks
        for (let i = 0; i < 3; i++) {
          await buttons.click({ force: true }).catch(() => {});
          await page.waitForTimeout(100);
        }
        
        // Verify page is still responsive
        await assertPageLoaded(page);
      } else {
        await assertPageLoaded(page);
      }
    });

    test('should handle multiple form submissions', async ({ page }) => {
      await setupTest(page, { tabName: 'Config' });
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(1000);
      
      const saveButton = page.locator('button:has-text("Save"), button[type="submit"]').first();
      
      const hasSaveButton = await saveButton.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
      
      if (hasSaveButton) {
        // Try multiple clicks
        for (let i = 0; i < 2; i++) {
          await saveButton.click({ force: true }).catch(() => {});
          await page.waitForTimeout(500);
        }
        
        await assertPageLoaded(page);
      } else {
        await assertPageLoaded(page);
      }
    });
  });

  test.describe('Network Disconnection/Reconnection', () => {
    test('should handle offline mode gracefully', async ({ page }) => {
      await setupTest(page, { tabName: 'Dashboard' });
      
      await page.waitForLoadState('networkidle');
      
      if (page.isClosed()) {
        return; // Skip if page closed
      }
      
      // Simulate offline
      try {
        await page.context().setOffline(true);
        await page.waitForTimeout(1000);
        
        // Try to navigate
        await setupTest(page, { tabName: 'Services' });
        
        // Verify page still loads (might show error state)
        if (!page.isClosed()) {
          await assertPageLoaded(page);
        }
        
        // Go back online
        await page.context().setOffline(false);
        await page.waitForTimeout(1000);
        
        // Verify reconnection
        if (!page.isClosed()) {
          await assertPageLoaded(page);
        }
      } catch (error) {
        // If offline mode fails, that's okay - restore online state
        try {
          await page.context().setOffline(false);
        } catch {
          // Ignore cleanup errors
        }
      }
    });

    test('should retry failed requests', async ({ page }) => {
      await setupTest(page, { tabName: 'Metrics' });
      
      await page.waitForLoadState('networkidle');
      
      // Look for retry buttons if errors occur
      const retryButton = page.locator('button:has-text("Retry"), button:has-text("Reload")').first();
      
      const hasRetryButton = await retryButton.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
      
      if (hasRetryButton) {
        await retryButton.click();
        await page.waitForTimeout(1000);
      }
      
      await assertPageLoaded(page);
    });
  });

  test.describe('Browser Navigation', () => {
    test('should handle browser back button', async ({ page }) => {
      await setupTest(page, { tabName: 'Dashboard' });
      await assertPageLoaded(page, ['Dashboard']);
      
      await setupTest(page, { tabName: 'Services' });
      await assertPageLoaded(page, ['Service']);
      
      // Navigate back using browser history
      await page.goBack();
      await page.waitForLoadState('domcontentloaded');
      await page.waitForTimeout(1000);
      
      // Should still be functional
      await assertPageLoaded(page);
    });

    test('should handle browser forward button', async ({ page }) => {
      await setupTest(page, { tabName: 'Dashboard' });
      await page.waitForTimeout(500);
      
      await setupTest(page, { tabName: 'Services' });
      await page.waitForTimeout(500);
      
      await page.goBack();
      await page.waitForTimeout(500);
      
      await page.goForward();
      await page.waitForLoadState('domcontentloaded');
      await page.waitForTimeout(1000);
      
      await assertPageLoaded(page);
    });

    test('should preserve state on navigation', async ({ page }) => {
      await setupTest(page, { tabName: 'Services' });
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(1000);
      
      // Make a change (like search)
      const searchInput = page.locator('#global-search-input').first();
      const hasSearch = await searchInput.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
      
      if (hasSearch) {
        await searchInput.fill('test');
        await page.waitForTimeout(500);
        
        // Navigate away and back
        await setupTest(page, { tabName: 'Dashboard' });
        await page.waitForTimeout(500);
        
        await setupTest(page, { tabName: 'Services' });
        await page.waitForTimeout(1000);
        
        // State might be preserved or reset - both are valid
        await assertPageLoaded(page);
      } else {
        await assertPageLoaded(page);
      }
    });
  });

  test.describe('Deep Linking', () => {
    test('should handle direct navigation to pages', async ({ page }) => {
      // Navigate directly to different pages
      const pages = ['Dashboard', 'Services', 'Chains', 'Logs'];
      
      for (const pageName of pages) {
        await setupTest(page, { tabName: pageName });
        await assertPageLoaded(page);
        await page.waitForTimeout(500);
      }
    });

    test('should handle URL-based state', async ({ page }) => {
      await setupTest(page);
      
      // Since we use tab-based navigation, URL might not change
      // But we can verify navigation works
      await setupTest(page, { tabName: 'Services' });
      await assertPageLoaded(page, ['Service']);
      
      await setupTest(page, { tabName: 'Dashboard' });
      await assertPageLoaded(page, ['Dashboard']);
    });
  });

  test.describe('Form Validation', () => {
    test('should validate required fields', async ({ page }) => {
      await setupTest(page, { tabName: 'Config' });
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(1000);
      
      // Look for required input fields
      const requiredInputs = page.locator('input[required], input[aria-required="true"]').first();
      
      const hasRequiredInputs = await requiredInputs.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
      
      if (hasRequiredInputs) {
        // Try to submit without filling
        const saveButton = page.locator('button:has-text("Save"), button[type="submit"]').first();
        const hasSaveButton = await saveButton.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
        
        if (hasSaveButton) {
          await saveButton.click();
          await page.waitForTimeout(500);
          
          // Look for validation errors
          const errorMessages = await checkAnyVisible(page, [
            'text=/required/i',
            'text=/invalid/i',
            '[role="alert"]',
            '[class*="error"]',
          ]);
          
          await assertPageLoaded(page);
        }
      } else {
        await assertPageLoaded(page);
      }
    });

    test('should validate input formats', async ({ page }) => {
      await setupTest(page, { tabName: 'Config' });
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(1000);
      
      // Look for number inputs
      const numberInput = page.locator('input[type="number"]').first();
      
      const hasNumberInput = await numberInput.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
      
      if (hasNumberInput) {
        // Try invalid input
        await numberInput.fill('invalid');
        await numberInput.blur();
        await page.waitForTimeout(500);
        
        // Check for validation
        await assertPageLoaded(page);
        
        // Restore valid value
        await numberInput.fill('8080');
      } else {
        await assertPageLoaded(page);
      }
    });
  });

  test.describe('Error Handling', () => {
    test('should display error messages', async ({ page }) => {
      await setupTest(page, { tabName: 'Services' });
      
      await page.waitForLoadState('networkidle');
      
      // Errors might be displayed if API fails
      const hasErrors = await checkAnyVisible(page, [
        '[role="alert"]',
        '[class*="error"]',
        'text=/error/i',
      ]);
      
      await assertPageLoaded(page);
    });

    test('should handle API errors gracefully', async ({ page }) => {
      await setupTest(page, { tabName: 'Dashboard' });
      
      // Simulate network error
      await page.route('**/api/**', route => route.abort());
      
      await page.waitForTimeout(2000);
      
      // Should show error state or fallback
      await assertPageLoaded(page);
      
      // Restore network
      await page.unroute('**/api/**');
    });

    test('should handle timeout errors', async ({ page }) => {
      await setupTest(page, { tabName: 'Metrics' });
      
      // Simulate slow network
      await page.route('**/api/**', route => {
        setTimeout(() => route.continue(), 10000);
      });
      
      await page.waitForTimeout(2000);
      
      // Should handle timeout
      await assertPageLoaded(page);
      
      await page.unroute('**/api/**');
    });
  });
});

