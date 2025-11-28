/**
 * Playwright Test Setup
 * 
 * This file is automatically imported by Playwright before all tests.
 * It sets up automatic coverage collection if enabled.
 */

import { test as baseTest } from '@playwright/test';
import { collectCoverage, saveCoverage } from './coverage-helpers';

const collectCoverageEnabled = process.env.COLLECT_COVERAGE === 'true';

// Extend the base test with coverage collection
export const test = baseTest.extend({
  // Override page fixture to collect coverage after each test
  page: async ({ page }, use, testInfo) => {
    await use(page);
    
    // Collect coverage if enabled
    if (collectCoverageEnabled) {
      try {
        // Wait a bit for coverage data to be ready
        await page.waitForTimeout(100);
        
        const coverage = await collectCoverage(page);
        if (coverage && Object.keys(coverage).length > 0) {
          const testName = testInfo.titlePath
            .join(' > ')
            .replace(/[^a-zA-Z0-9]/g, '-')
            .toLowerCase();
          
          await saveCoverage(page, testName, 'coverage/e2e');
        }
      } catch (error) {
        // Coverage collection failed, but don't fail the test
        // This is expected if coverage instrumentation isn't loaded
        if (collectCoverageEnabled) {
          console.warn(`Coverage collection failed for ${testInfo.title}:`, error);
        }
      }
    }
  },
});

// Re-export expect
export { expect } from '@playwright/test';

