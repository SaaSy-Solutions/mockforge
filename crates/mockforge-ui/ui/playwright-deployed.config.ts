import { defineConfig, devices } from '@playwright/test';
import { fileURLToPath } from 'url';
import path from 'path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const STORAGE_STATE_PATH = path.join(__dirname, '.auth/deployed-user.json');

/**
 * Playwright configuration for testing the deployed MockForge Cloud site.
 *
 * Usage:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts
 */
export default defineConfig({
  testDir: './e2e',
  testMatch: '*-deployed.spec.ts',

  timeout: 60_000,
  expect: { timeout: 10_000 },

  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  // Deployed registry rate-limits at ~100 req/min/user, so cap parallelism
  // and retry on the rare 429 cascade.
  retries: 2,
  workers: 3,

  reporter: process.env.CI ? [['html'], ['github']] : [['html'], ['list']],

  use: {
    baseURL: process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    viewport: { width: 1280, height: 720 },
  },

  projects: [
    // Auth setup — runs once before all tests
    {
      name: 'deployed-auth-setup',
      testMatch: /deployed-auth\.setup\.ts/,
    },
    // Dashboard tests — use saved auth state
    {
      name: 'deployed-chromium',
      use: {
        ...devices['Desktop Chrome'],
        storageState: STORAGE_STATE_PATH,
      },
      dependencies: ['deployed-auth-setup'],
    },
  ],
});
