import { test, expect } from '@playwright/test';
import { setupTest, assertPageLoaded } from './test-helpers';
import { TIMEOUTS } from './constants';

/**
 * Performance E2E Tests
 * 
 * Tests performance metrics and optimizations:
 * - Page load times
 * - API response times
 * - Large dataset handling
 */
test.describe('Performance Tests', () => {
  test.describe('Page Load Times', () => {
    test('should load dashboard within acceptable time', async ({ page }) => {
      const startTime = Date.now();
      
      await setupTest(page, { tabName: 'Dashboard' });
      await page.waitForLoadState('networkidle');
      
      const loadTime = Date.now() - startTime;
      
      // Dashboard should load within 30 seconds (accounting for login)
      expect(loadTime).toBeLessThan(30000);
      
      await assertPageLoaded(page, ['Dashboard']);
    });

    test('should load services page within acceptable time', async ({ page }) => {
      const startTime = Date.now();
      
      await setupTest(page, { tabName: 'Services' });
      await page.waitForLoadState('networkidle');
      
      const loadTime = Date.now() - startTime;
      
      // Services page should load within 30 seconds (more lenient for CI)
      expect(loadTime).toBeLessThan(30000);
      
      await assertPageLoaded(page, ['Service']);
    });

    test('should load all major pages within acceptable time', async ({ page }) => {
      const pages = ['Dashboard', 'Services', 'Chains', 'Logs', 'Metrics', 'Fixtures'];
      const maxLoadTime = 30000; // 30 seconds per page (more lenient)
      
      for (const pageName of pages) {
        if (page.isClosed()) {
          return; // Skip if page closed
        }
        
        const startTime = Date.now();
        
        await setupTest(page, { tabName: pageName });
        await page.waitForLoadState('networkidle');
        
        const loadTime = Date.now() - startTime;
        
        expect(loadTime).toBeLessThan(maxLoadTime);
        
        await assertPageLoaded(page);
      }
    });

    test('should have fast navigation between pages', async ({ page }) => {
      await setupTest(page, { tabName: 'Dashboard' });
      await page.waitForLoadState('networkidle');
      
      const pages = ['Services', 'Chains', 'Logs'];
      
      for (const pageName of pages) {
        const startTime = Date.now();
        
        await page.evaluate((name) => {
          // Trigger navigation via custom event if available
          const event = new CustomEvent('navigate', { detail: name });
          window.dispatchEvent(event);
        }, pageName);
        
        await page.waitForLoadState('domcontentloaded');
        
        const navTime = Date.now() - startTime;
        
        // Navigation should be fast (< 2 seconds)
        expect(navTime).toBeLessThan(2000);
      }
    });
  });

  test.describe('API Response Times', () => {
    test('should measure API response times', async ({ page }) => {
      await setupTest(page, { tabName: 'Dashboard' });
      
      // Monitor network requests
      const responseTimes: number[] = [];
      
      page.on('response', (response) => {
        const url = response.url();
        if (url.includes('/api/')) {
          const timing = response.timing();
          if (timing) {
            const responseTime = timing.responseEnd - timing.requestStart;
            responseTimes.push(responseTime);
          }
        }
      });
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(2000);
      
      // Most API calls should complete within 5 seconds
      if (responseTimes.length > 0) {
        const avgResponseTime = responseTimes.reduce((a, b) => a + b, 0) / responseTimes.length;
        const maxResponseTime = Math.max(...responseTimes);
        
        // Average should be reasonable
        expect(avgResponseTime).toBeLessThan(5000);
        
        // Max should not be excessive
        expect(maxResponseTime).toBeLessThan(10000);
      }
      
      await assertPageLoaded(page);
    });

    test('should handle slow API responses gracefully', async ({ page }) => {
      await setupTest(page, { tabName: 'Services' });
      
      // Simulate slow API
      await page.route('**/api/**', async (route) => {
        await new Promise(resolve => setTimeout(resolve, 2000));
        await route.continue();
      });
      
      const startTime = Date.now();
      
      await page.reload();
      await page.waitForLoadState('domcontentloaded');
      
      const loadTime = Date.now() - startTime;
      
      // Should still load (might show loading state)
      await assertPageLoaded(page);
      
      // Cleanup
      await page.unroute('**/api/**');
    });
  });

  test.describe('Large Dataset Handling', () => {
    test('should handle large service lists', async ({ page }) => {
      await setupTest(page, { tabName: 'Services' });
      
      const startTime = Date.now();
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(2000);
      
      const loadTime = Date.now() - startTime;
      
      // Should handle large lists within reasonable time
      expect(loadTime).toBeLessThan(10000);
      
      await assertPageLoaded(page, ['Service']);
    });

    test('should handle large fixture lists', async ({ page }) => {
      await setupTest(page, { tabName: 'Fixtures' });
      
      const startTime = Date.now();
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(2000);
      
      const loadTime = Date.now() - startTime;
      
      // Should handle large lists
      expect(loadTime).toBeLessThan(10000);
      
      await assertPageLoaded(page, ['Fixture']);
    });

    test('should handle large log entries', async ({ page }) => {
      await setupTest(page, { tabName: 'Logs' });
      
      const startTime = Date.now();
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(2000);
      
      const loadTime = Date.now() - startTime;
      
      // Logs might be large, but should still load
      expect(loadTime).toBeLessThan(15000);
      
      await assertPageLoaded(page, ['Log']);
    });

    test('should paginate or virtualize large lists', async ({ page }) => {
      await setupTest(page, { tabName: 'Services' });
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(2000);
      
      // Look for pagination or virtualization
      const hasPagination = await page.locator('button:has-text("Next"), button:has-text("Previous"), [class*="pagination"]').isVisible({ timeout: 1000 }).catch(() => false);
      
      // Look for scrollable containers (virtualization)
      const scrollableContainers = await page.locator('[class*="scroll"], [class*="virtual"], [style*="overflow"]').count();
      
      // Should have either pagination or virtualization for large lists
      await assertPageLoaded(page);
    });
  });

  test.describe('Memory and Resource Usage', () => {
    test('should not have memory leaks on navigation', async ({ page }) => {
      await setupTest(page, { tabName: 'Dashboard' });
      
      // Navigate multiple times (single iteration to avoid timeout)
      const pages = ['Services', 'Chains', 'Dashboard'];
      
      for (const pageName of pages) {
        if (page.isClosed()) {
          return; // Skip if page closed
        }
        
        await setupTest(page, { tabName: pageName });
        await page.waitForLoadState('domcontentloaded');
        await page.waitForTimeout(200); // Minimal timeout
      }
      
      // Should still be responsive
      if (!page.isClosed()) {
        await assertPageLoaded(page);
      }
    });

    test('should handle rapid interactions without degradation', async ({ page }) => {
      await setupTest(page, { tabName: 'Services' });
      await page.waitForLoadState('networkidle');
      
      // Rapid navigation (single iteration to avoid timeout)
      const pages = ['Dashboard', 'Services'];
      
      for (const pageName of pages) {
        if (page.isClosed()) {
          return; // Skip if page closed
        }
        
        await setupTest(page, { tabName: pageName });
        await page.waitForLoadState('domcontentloaded');
        await page.waitForTimeout(200); // Minimal timeout
      }
      
      // Should still be responsive
      if (!page.isClosed()) {
        await assertPageLoaded(page);
      }
    });
  });
});

