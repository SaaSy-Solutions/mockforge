import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright configuration for MockForge integration
 */
export default defineConfig({
  testDir: './tests',

  // Run tests in files in parallel
  fullyParallel: true,

  // Fail the build on CI if you accidentally left test.only in the source code
  forbidOnly: !!process.env.CI,

  // Retry on CI only
  retries: process.env.CI ? 2 : 0,

  // Opt out of parallel tests on CI
  workers: process.env.CI ? 1 : undefined,

  // Reporter to use
  reporter: 'html',

  use: {
    // Base URL to use in actions like `await page.goto('/')`
    baseURL: process.env.MOCKFORGE_URL || 'http://localhost:3000',

    // Collect trace when retrying the failed test
    trace: 'on-first-retry',
  },

  // Configure projects for major browsers
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],

  // Run MockForge server before starting the tests
  webServer: {
    command: 'cargo run --manifest-path ../../../Cargo.toml --bin mockforge-test-server',
    url: 'http://localhost:3000/health',
    reuseExistingServer: !process.env.CI,
    timeout: 30 * 1000,
  },
});
