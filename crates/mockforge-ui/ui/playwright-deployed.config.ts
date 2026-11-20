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

  // Specs below assert pages that exist only in the local MockForge server
  // UI, so they must not run against the cloud app (issue #1051). In cloud
  // mode the sidebar hides these routes because cloud siblings supersede
  // them — see cloudHiddenNavItemIds in src/components/layout/navigation.ts
  // — and their data comes from the local admin API (/api/chaos/*,
  // /api/recorder/*, /api/users, ...), which the registry does not serve.
  // Most local variants of these pages stay covered by the plain *.spec.ts
  // files under the local project (playwright.config.ts), which ignores
  // *-deployed.spec.ts. behavioral-cloning, incidents and contract-diff have
  // no local spec yet, so no project runs them for now (user-management's
  // page was retired). These files are kept in-tree on purpose, not deleted.
  testIgnore: [
    'e2e/chaos-deployed.spec.ts', // /chaos → cloud-chaos (CloudChaosPage)
    'e2e/recorder-deployed.spec.ts', // /recorder → cloud-recorder
    'e2e/behavioral-cloning-deployed.spec.ts', // /behavioral-cloning → cloud-behavioral-cloning
    'e2e/incidents-deployed.spec.ts', // /incidents → cloud-incidents
    'e2e/traces-deployed.spec.ts', // /traces → cloud-traces
    'e2e/contract-diff-deployed.spec.ts', // /contract-diff → cloud-contract
    'e2e/plugins-deployed.spec.ts', // /plugins → plugin-registry
    'e2e/test-execution-deployed.spec.ts', // mock-data-only dashboard → cloud-test-runs
    'e2e/metrics-deployed.spec.ts', // /metrics → pillar-analytics
    'e2e/performance-deployed.spec.ts', // /performance → cloud-test-runs
    'e2e/playground-deployed.spec.ts', // /playground is absent from cloudNavItemIds
    'e2e/user-management-deployed.spec.ts', // retired (#15) → Organization page tabs
  ],

  timeout: 60_000,
  expect: { timeout: 10_000 },

  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  // Deployed registry rate-limits at ~100 req/min/user, so cap parallelism
  // aggressively and allow multiple retries when a 429 burst slips through.
  retries: 3,
  workers: 2,

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
