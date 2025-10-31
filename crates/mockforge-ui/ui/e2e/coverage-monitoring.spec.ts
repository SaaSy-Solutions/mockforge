import { test, expect } from '@playwright/test';
import { setupTest, assertPageLoaded } from './test-helpers';
import { collectCoverage } from './coverage-helpers';

/**
 * Coverage Monitoring E2E Tests
 * 
 * Tests to ensure code coverage collection is working:
 * - Verify coverage instrumentation
 * - Track coverage metrics
 * - Validate coverage thresholds
 */
test.describe('Coverage Monitoring', () => {
  test('should collect coverage data when enabled', async ({ page }) => {
    const collectCoverageEnabled = process.env.COLLECT_COVERAGE === 'true';
    
    if (!collectCoverageEnabled) {
      test.skip();
    }
    
    await setupTest(page, { tabName: 'Dashboard' });
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    
    // Check if coverage is available
    const coverage = await collectCoverage(page);
    
    if (coverage) {
      const fileCount = Object.keys(coverage).length;
      
      // Should have coverage data
      expect(fileCount).toBeGreaterThan(0);
      
      // Log coverage summary
      console.log(`Coverage collected: ${fileCount} files`);
    } else {
      console.warn('Coverage not available - make sure dev server is running with vite.config.coverage.ts');
    }
    
    await assertPageLoaded(page);
  });

  test('should visit all pages for comprehensive coverage', async ({ page }) => {
    const collectCoverageEnabled = process.env.COLLECT_COVERAGE === 'true';
    
    if (!collectCoverageEnabled) {
      test.skip();
    }
    
    const pages = [
      'Dashboard',
      'Services',
      'Chains',
      'Logs',
      'Metrics',
      'Analytics',
      'Fixtures',
      'Import',
      'Workspaces',
      'Testing',
      'Plugins',
      'Config',
      'Observability',
      'Traces',
      'Test Generator',
      'Test Execution',
      'Integration Tests',
      'Chaos Engineering',
      'Resilience',
      'Recorder',
      'Orchestration Builder',
      'Orchestration Execution',
      'Template Marketplace',
      'Plugin Registry',
    ];
    
    for (const pageName of pages) {
      await setupTest(page, { tabName: pageName });
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(1000);
      
      await assertPageLoaded(page);
    }
    
    // Collect final coverage
    const coverage = await collectCoverage(page);
    
    if (coverage) {
      const fileCount = Object.keys(coverage).length;
      console.log(`Coverage after visiting all pages: ${fileCount} files`);
      
      // Should have good coverage after visiting all pages
      expect(fileCount).toBeGreaterThan(10);
    }
  });

  test('should track coverage trends', async ({ page }) => {
    const collectCoverageEnabled = process.env.COLLECT_COVERAGE === 'true';
    
    if (!collectCoverageEnabled) {
      test.skip();
    }
    
    // Visit key pages
    const keyPages = ['Dashboard', 'Services', 'Chains', 'Logs'];
    
    let totalStatements = 0;
    let totalBranches = 0;
    let totalFunctions = 0;
    let totalLines = 0;
    
    for (const pageName of keyPages) {
      await setupTest(page, { tabName: pageName });
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(1000);
      
      const coverage = await collectCoverage(page);
      
      if (coverage) {
        // Calculate coverage metrics
        Object.values(coverage).forEach((file: any) => {
          if (file.s) totalStatements += Object.keys(file.s).length;
          if (file.b) totalBranches += Object.keys(file.b).length;
          if (file.f) totalFunctions += Object.keys(file.f).length;
          if (file.statementMap) totalLines += Object.keys(file.statementMap).length;
        });
      }
    }
    
    console.log(`Coverage metrics:
      Statements: ${totalStatements}
      Branches: ${totalBranches}
      Functions: ${totalFunctions}
      Lines: ${totalLines}`);
    
    await assertPageLoaded(page);
  });

  test('should validate coverage thresholds', async ({ page }) => {
    const collectCoverageEnabled = process.env.COLLECT_COVERAGE === 'true';
    
    if (!collectCoverageEnabled) {
      test.skip();
    }
    
    // Visit all pages
    await setupTest(page, { tabName: 'Dashboard' });
    
    const pages = [
      'Services', 'Chains', 'Logs', 'Metrics', 'Fixtures',
      'Workspaces', 'Testing', 'Plugins', 'Config',
    ];
    
    for (const pageName of pages) {
      await setupTest(page, { tabName: pageName });
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(500);
    }
    
    // Collect coverage
    const coverage = await collectCoverage(page);
    
    if (coverage) {
      const fileCount = Object.keys(coverage).length;
      
      // Minimum threshold: at least 20 files should be covered
      expect(fileCount).toBeGreaterThanOrEqual(20);
      
      console.log(`Coverage threshold check: ${fileCount} files covered`);
    }
  });
});

