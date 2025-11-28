/**
 * Playwright Coverage Configuration
 * 
 * This configuration enables code coverage collection during Playwright E2E tests.
 * It uses vite-plugin-istanbul to instrument the code and collects coverage data
 * during test execution.
 */

import { defineConfig } from '@playwright/test';
import baseConfig from './playwright.config';

export default defineConfig({
  ...baseConfig,
  
  // Enable coverage collection
  use: {
    ...baseConfig.use,
    // Collect coverage trace
    trace: 'on',
  },
  
  // Coverage-specific reporter
  reporter: [
    ['html', { outputFolder: 'playwright-report' }],
    ['json', { outputFile: 'coverage/playwright-coverage.json' }],
    ['list'],
  ],
});

