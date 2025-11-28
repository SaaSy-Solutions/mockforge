import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright configuration for MockForge Admin UI v2
 * 
 * This configuration enables end-to-end testing of the Admin UI,
 * including all major pages and functionality.
 */
export default defineConfig({
  // Test directory
  testDir: './e2e',

  // Global setup/teardown for coverage collection
  globalSetup: process.env.COLLECT_COVERAGE === 'true' ? './e2e/global-setup.ts' : undefined,
  globalTeardown: process.env.COLLECT_COVERAGE === 'true' ? './e2e/global-teardown.ts' : undefined,

  // Test setup file (auto-imported before all tests)
  // Note: We'll use a different approach - see coverage-helpers.ts for manual collection
  
  // Timeout for each test
  timeout: 60 * 1000, // 60 seconds - increased for login and navigation

  // Expect timeout for assertions
  expect: {
    timeout: 5 * 1000,
  },

  // Run tests in files in parallel
  fullyParallel: true,

  // Fail the build on CI if you accidentally left test.only in the source code
  forbidOnly: !!process.env.CI,

  // Retry on CI only
  retries: process.env.CI ? 2 : 0,

  // Opt out of parallel tests on CI
  workers: process.env.CI ? 1 : undefined,

  // Reporter configuration
  reporter: process.env.CI ? [['html'], ['github']] : [['html'], ['list']],

  // Shared settings for all projects
  use: {
    // Base URL - adjust based on your setup
    // In dev: Vite dev server proxies to backend
    // In production: Admin UI is served directly from backend
    baseURL: process.env.PLAYWRIGHT_BASE_URL || 'http://localhost:5173',

    // Collect trace when retrying the failed test
    trace: 'on-first-retry',

    // Screenshot on failure
    screenshot: 'only-on-failure',

    // Video on failure
    video: 'retain-on-failure',

    // Viewport size
    viewport: { width: 1280, height: 720 },
  },

  // Configure projects for different browsers
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    // Temporarily disable Firefox/WebKit until navigation is fully working
    // {
    //   name: 'firefox',
    //   use: { ...devices['Desktop Firefox'] },
    // },
    // {
    //   name: 'webkit',
    //   use: { ...devices['Desktop Safari'] },
    // },
    // Mobile viewports - also disabled until navigation is fixed
    // {
    //   name: 'Mobile Chrome',
    //   use: { ...devices['Pixel 5'] },
    // },
    // {
    //   name: 'Mobile Safari',
    //   use: { ...devices['iPhone 12'] },
    // },
  ],

  // Optional: Start dev server before running tests
  // Uncomment if you want Playwright to start the dev server automatically
  // webServer: {
  //   command: 'npm run dev',
  //   url: 'http://localhost:5173',
  //   reuseExistingServer: !process.env.CI,
  //   timeout: 120 * 1000,
  //   stdout: 'ignore',
  //   stderr: 'pipe',
  // },
});

