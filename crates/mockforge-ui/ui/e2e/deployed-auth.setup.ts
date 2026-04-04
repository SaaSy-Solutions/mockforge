import { test as setup, expect } from '@playwright/test';
import { fileURLToPath } from 'url';
import path from 'path';
import fs from 'fs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';
const E2E_EMAIL = process.env.E2E_EMAIL;
const E2E_PASSWORD = process.env.E2E_PASSWORD;

export const STORAGE_STATE_PATH = path.join(__dirname, '../.auth/deployed-user.json');

// Max age for saved auth state (24 hours)
const MAX_AUTH_AGE_MS = 24 * 60 * 60 * 1000;

/**
 * Authenticate once and save browser storage state for all deployed-site tests.
 *
 * If a recent auth state file exists (< 24h old), reuse it without credentials.
 *
 * Run with:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts
 */
setup('authenticate to MockForge Cloud', async ({ page }) => {
  // Reuse saved auth state if it's recent enough
  if (fs.existsSync(STORAGE_STATE_PATH)) {
    const stat = fs.statSync(STORAGE_STATE_PATH);
    const age = Date.now() - stat.mtimeMs;
    if (age < MAX_AUTH_AGE_MS) {
      // Validate the saved state still works
      await page.context().addCookies([]);
      const saved = JSON.parse(fs.readFileSync(STORAGE_STATE_PATH, 'utf-8'));
      if (saved.origins?.length > 0 || saved.cookies?.length > 0) {
        return; // Auth state is fresh — skip login
      }
    }
  }

  if (!E2E_EMAIL || !E2E_PASSWORD) {
    throw new Error(
      'E2E_EMAIL and E2E_PASSWORD environment variables are required.\n' +
      'Usage: E2E_EMAIL=you@example.com E2E_PASSWORD=secret npx playwright test --config=playwright-deployed.config.ts'
    );
  }

  await page.goto(`${BASE_URL}/login`, {
    waitUntil: 'domcontentloaded',
    timeout: 30000,
  });

  // Fill in login form
  await page.getByPlaceholder('Enter your email').fill(E2E_EMAIL);
  await page.getByPlaceholder('Enter your password').fill(E2E_PASSWORD);
  await page.getByRole('button', { name: 'Sign In' }).click();

  // Wait for successful login — sidebar nav appears
  await expect(
    page.locator('nav[aria-label="Main navigation"]')
  ).toBeVisible({ timeout: 20000 });

  // Save signed-in state for reuse across all test files
  await page.context().storageState({ path: STORAGE_STATE_PATH });
});
