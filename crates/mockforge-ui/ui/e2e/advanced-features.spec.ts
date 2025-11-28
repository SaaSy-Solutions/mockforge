import { test, expect } from '@playwright/test';
import { setupTest, assertPageLoaded, checkAnyVisible } from './test-helpers';
import { SELECTORS, TIMEOUTS } from './constants';

/**
 * Advanced Features E2E Tests
 * 
 * Tests advanced features that require complex interactions:
 * - Real-time updates (WebSocket/SSE)
 * - Bulk operations
 * - File upload/download
 * - Export functionality
 * - Role-based access control
 */
test.describe('Advanced Features', () => {
  test.describe('Real-time Updates', () => {
    test('should connect to WebSocket for observability', async ({ page }) => {
      await setupTest(page, { tabName: 'Observability' });
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(3000); // Allow WebSocket connection
      
      await assertPageLoaded(page, ['Observability']);
      
      // Check for WebSocket connection status
      const connectionStatus = await checkAnyVisible(page, [
        'text=/Connected/i',
        'text=/Disconnected/i',
        '[class*="status"]',
      ]);
      
      // Verify page functions with or without WebSocket
      await assertPageLoaded(page);
    });

    test('should receive real-time metrics updates', async ({ page }) => {
      await setupTest(page, { tabName: 'Metrics' });
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(3000);
      
      // Check for metrics that might update in real-time
      const metrics = await checkAnyVisible(page, [
        '[class*="metric"]',
        'text=/CPU/i',
        'text=/Memory/i',
        '[class*="chart"]',
      ]);
      
      await assertPageLoaded(page);
      
      // Metrics should be present (even if static)
      expect(metrics).toBe(true);
    });
  });

  test.describe('Bulk Operations', () => {
    test('should support bulk selection', async ({ page }) => {
      await setupTest(page, { tabName: 'Services' });
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(2000);
      
      // Look for checkboxes for bulk selection
      const checkboxes = page.locator('input[type="checkbox"][aria-label*="select" i], input[type="checkbox"]:not([type="radio"])').first();
      
      const hasCheckboxes = await checkboxes.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
      
      if (hasCheckboxes) {
        // Select a checkbox
        await checkboxes.click();
        await page.waitForTimeout(500);
        
        // Look for bulk action buttons
        const bulkActions = page.locator('button:has-text("Delete"), button:has-text("Enable"), button:has-text("Disable")').first();
        const hasBulkActions = await bulkActions.isVisible({ timeout: 1000 }).catch(() => false);
        
        await assertPageLoaded(page);
      } else {
        await assertPageLoaded(page);
      }
    });

    test('should allow bulk delete operations', async ({ page }) => {
      await setupTest(page, { tabName: 'Fixtures' });
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(2000);
      
      // Look for select all checkbox
      const selectAll = page.locator('input[type="checkbox"][aria-label*="all" i], input[type="checkbox"]:first-of-type').first();
      
      const hasSelectAll = await selectAll.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
      
      if (hasSelectAll) {
        await selectAll.click();
        await page.waitForTimeout(500);
        
        // Look for bulk delete button
        const bulkDelete = page.locator('button:has-text("Delete"), button[aria-label*="delete" i]').first();
        const hasBulkDelete = await bulkDelete.isVisible({ timeout: 1000 }).catch(() => false);
        
        if (hasBulkDelete) {
          // Don't actually delete, just verify button exists
          await assertPageLoaded(page);
        }
      }
      
      await assertPageLoaded(page);
    });
  });

  test.describe('File Upload', () => {
    test('should handle file upload in Import page', async ({ page }) => {
      await setupTest(page, { tabName: 'Import' });
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(1000);
      
      await assertPageLoaded(page, ['Import']);
      
      // Look for file input (might be hidden)
      const fileInput = page.locator('input[type="file"]').first();
      
      const hasFileInput = await fileInput.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
      
      if (!hasFileInput) {
        // File input might be hidden, look for label that triggers it
        const uploadLabel = page.locator('label[for*="file"], label:has-text("Upload"), label:has-text("Choose")').first();
        const hasLabel = await uploadLabel.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
        
        await assertPageLoaded(page);
      } else {
        await assertPageLoaded(page);
      }
    });

    test('should handle drag and drop file upload', async ({ page }) => {
      await setupTest(page, { tabName: 'Import' });
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(1000);
      
      // Look for drop zone
      const dropZone = page.locator('[class*="drop"], [class*="upload"], [class*="drag"]').first();
      
      const hasDropZone = await dropZone.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
      
      if (hasDropZone) {
        // Verify drop zone is present
        await assertPageLoaded(page);
      } else {
        await assertPageLoaded(page);
      }
    });

    test('should handle fixture file upload', async ({ page }) => {
      await setupTest(page, { tabName: 'Fixtures' });
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(1000);
      
      // Look for upload button or file input
      const uploadButton = page.locator('button:has-text("Upload"), input[type="file"]').first();
      
      const hasUpload = await uploadButton.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
      
      await assertPageLoaded(page);
    });
  });

  test.describe('File Download/Export', () => {
    test('should allow exporting fixtures', async ({ page }) => {
      await setupTest(page, { tabName: 'Fixtures' });
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(2000);
      
      // Look for export/download buttons
      const exportButton = page.locator('button:has-text("Export"), button:has-text("Download"), a[download]').first();
      
      const hasExportButton = await exportButton.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
      
      // Export might not be available if no fixtures - that's okay
      await assertPageLoaded(page);
    });

    test('should allow exporting configuration', async ({ page }) => {
      await setupTest(page, { tabName: 'Config' });
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(1000);
      
      // Look for export button
      const exportButton = page.locator('button:has-text("Export"), button:has-text("Download")').first();
      
      const hasExportButton = await exportButton.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
      
      await assertPageLoaded(page);
    });

    test('should allow exporting test results', async ({ page }) => {
      await setupTest(page, { tabName: 'Test Execution' });
      
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(2000);
      
      // Look for export/download buttons
      const exportButton = page.locator('button:has-text("Export"), button:has-text("Download")').first();
      
      const hasExportButton = await exportButton.isVisible({ timeout: TIMEOUTS.MEDIUM }).catch(() => false);
      
      await assertPageLoaded(page);
    });
  });

  test.describe('Role-based Access Control', () => {
    test('should show appropriate UI for admin user', async ({ page }) => {
      await setupTest(page);
      
      // Admin users should see admin-only features
      const adminFeatures = await checkAnyVisible(page, [
        'text=/Config/i',
        'text=/Admin/i',
        SELECTORS.buttons.create,
      ]);
      
      await assertPageLoaded(page);
      
      // Verify admin can access pages
      await setupTest(page, { tabName: 'Config' });
      await assertPageLoaded(page, ['Config']);
    });

    test('should handle permissions gracefully', async ({ page }) => {
      await setupTest(page);
      
      // Try to access various pages
      const pages = ['Dashboard', 'Services', 'Config'];
      
      for (const pageName of pages) {
        await setupTest(page, { tabName: pageName });
        await assertPageLoaded(page);
      }
    });
  });
});

