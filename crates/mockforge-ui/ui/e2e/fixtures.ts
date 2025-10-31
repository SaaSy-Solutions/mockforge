/**
 * Playwright Custom Fixtures
 * 
 * Custom fixtures that extend Playwright's test functionality,
 * including automatic coverage collection.
 */

import { test as base } from '@playwright/test';
import { collectCoverage, saveCoverage } from './coverage-helpers';

type CoverageFixtures = {
  collectCoverage: boolean;
};

export const test = base.extend<CoverageFixtures>({
  collectCoverage: [process.env.COLLECT_COVERAGE === 'true', { option: true }],

  // Auto-collect coverage after each test
  page: async ({ page, collectCoverage }, use, testInfo) => {
    await use(page);
    
    // Collect coverage if enabled
    if (collectCoverage) {
      try {
        const coverage = await collectCoverage(page);
        if (coverage) {
          const testName = testInfo.titlePath.join(' > ');
          await saveCoverage(page, testName);
        }
      } catch (error) {
        // Coverage collection failed, but don't fail the test
        console.warn('Coverage collection failed:', error);
      }
    }
  },
});

export { expect } from '@playwright/test';

