import { test, expect } from '@playwright/test';

/**
 * Regression tests for the UI auth-race + PWA meta-tag fix (PR #121).
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts \
 *     --grep "Auth race + PWA meta"
 *
 * Covered:
 *   1. /api/v1/workspaces must NOT be requested before the user is
 *      authenticated. The old bug: loadWorkspaces() fired on mount in
 *      parallel with AuthGuard.checkAuth() and sent a stale/revoked
 *      token, producing a 401.
 *   2. The app must not trigger the Chrome deprecation warning for
 *      <meta name="apple-mobile-web-app-capable"> — we now pair it
 *      with <meta name="mobile-web-app-capable">.
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';
const WORKSPACES_PATH = /\/api\/v1\/workspaces(?:\?|$)/;
const DEPRECATION_PATTERN = /apple-mobile-web-app-capable.*deprecated/i;

test.describe('Auth race + PWA meta — Deployed Site', () => {
  test('does not request /api/v1/workspaces before authentication', async ({ browser }) => {
    // Fresh context with no saved auth — simulates a brand-new tab with
    // no persisted JWT in localStorage.
    const context = await browser.newContext({ storageState: undefined });
    const page = await context.newPage();

    const workspacesRequests: string[] = [];
    page.on('request', (req) => {
      if (WORKSPACES_PATH.test(req.url())) {
        workspacesRequests.push(`${req.method()} ${req.url()}`);
      }
    });

    try {
      await page.goto(`${BASE_URL}/`, {
        waitUntil: 'domcontentloaded',
        timeout: 30_000,
      });

      // AuthGuard renders the login form when unauthenticated. Waiting
      // on it ensures the initial render + its effects have run, which
      // is when the old loadWorkspaces() race would have fired.
      await expect(
        page.getByRole('button', { name: 'Sign In' })
      ).toBeVisible({ timeout: 20_000 });

      // Give any lingering effects a chance to fire a stray request.
      await page.waitForTimeout(1_500);

      expect(
        workspacesRequests,
        `Unauthenticated app load must not hit /api/v1/workspaces. Got: ${JSON.stringify(workspacesRequests)}`
      ).toHaveLength(0);
    } finally {
      await context.close();
    }
  });

  test('console has no apple-mobile-web-app-capable deprecation warning', async ({ page }) => {
    const deprecationMessages: string[] = [];

    page.on('console', (msg) => {
      const text = msg.text();
      if (DEPRECATION_PATTERN.test(text)) {
        deprecationMessages.push(`[${msg.type()}] ${text}`);
      }
    });
    page.on('pageerror', (err) => {
      if (DEPRECATION_PATTERN.test(err.message)) {
        deprecationMessages.push(`[pageerror] ${err.message}`);
      }
    });

    await page.goto(`${BASE_URL}/dashboard`, {
      waitUntil: 'domcontentloaded',
      timeout: 30_000,
    });

    // Give the browser time to emit the deprecation warning on parse.
    await page.waitForTimeout(2_000);

    expect(
      deprecationMessages,
      `Page must not trigger the apple-mobile-web-app-capable deprecation warning. Got: ${JSON.stringify(deprecationMessages)}`
    ).toHaveLength(0);
  });

  test('pairs mobile-web-app-capable with apple-mobile-web-app-capable', async ({ page }) => {
    await page.goto(`${BASE_URL}/`, {
      waitUntil: 'domcontentloaded',
      timeout: 30_000,
    });

    // Both meta tags must be present — browsers warn when only the
    // Apple-prefixed one is set.
    const mobile = page.locator('meta[name="mobile-web-app-capable"]');
    const apple = page.locator('meta[name="apple-mobile-web-app-capable"]');

    await expect(mobile).toHaveAttribute('content', 'yes');
    await expect(apple).toHaveAttribute('content', 'yes');
  });
});
