/**
 * Coverage Collection Test
 * 
 * This test visits all pages to collect comprehensive coverage data.
 * Run this test with COLLECT_COVERAGE=true to generate coverage reports.
 */

import { test, expect } from '@playwright/test';
import { setupTest } from './test-helpers';
import { navigateToTab } from './helpers';
import { collectCoverage, saveCoverage } from './coverage-helpers';

test.describe('Coverage Collection', () => {
  test('collect coverage from all pages', async ({ page }) => {
    // Setup and login
    await setupTest(page);
    
    // List of all pages to visit for coverage (organized by logical groups)
    const pages = [
      // Core
      'Dashboard',
      'Workspaces',
      // Services & Data
      'Services',
      'Fixtures',
      // Orchestration
      'Chains',
      'Orchestration Builder',
      'Orchestration Execution',
      // Observability & Monitoring
      'Observability',
      'Logs',
      'Traces',
      'Metrics',
      'Analytics',
      // Testing
      'Testing',
      'Test Generator',
      'Test Execution',
      'Integration Tests',
      // Chaos & Resilience
      'Chaos Engineering',
      'Resilience',
      'Recorder',
      // Import & Templates
      'Import',
      'Template Marketplace',
      // Plugins
      'Plugins',
      'Plugin Registry',
      // Configuration
      'Config',
    ];
    
    // Visit each page to ensure code is executed
    for (const pageName of pages) {
      if (page.isClosed()) {
        break; // Stop if page closed
      }
      
      try {
        const navigated = await navigateToTab(page, pageName);
        if (navigated && !page.isClosed()) {
          await page.waitForLoadState('networkidle');
          await page.waitForTimeout(1000); // Allow time for code execution
        }
      } catch (error) {
        // Skip navigation errors - continue with other pages
        if (page.isClosed()) {
          break;
        }
      }
    }
    
    // Go back to dashboard for final coverage snapshot
    if (!page.isClosed()) {
      await navigateToTab(page, 'Dashboard');
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(2000);
      
      // Collect coverage data
      const coverage = await collectCoverage(page);
    
      if (coverage) {
        await saveCoverage(page, 'all-pages-coverage', 'coverage/e2e');
        console.log(`✅ Coverage collected: ${Object.keys(coverage).length} files`);
        
        // Verify coverage was collected
        expect(Object.keys(coverage).length).toBeGreaterThan(0);
      } else {
        console.warn('⚠️  No coverage data found. Make sure dev server is running with vite.config.coverage.ts');
      }
    }
  });
});

