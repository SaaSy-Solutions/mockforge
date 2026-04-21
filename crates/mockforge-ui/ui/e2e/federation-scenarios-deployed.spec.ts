import { test, expect } from '@playwright/test';

/**
 * Federation Scenario Activation E2E — Deployed Site
 *
 * Exercises the ActiveScenarioPanel added to the federation detail view:
 *   - Panel renders on the detail page
 *   - Empty state copy is present when no scenario is active
 *   - Activate form exposes the scenario picker dropdown
 *   - Activate form exposes per-service override inputs
 *
 * Intentionally conservative: does NOT create/activate real scenarios — that
 * would leave test data in the prod registry. Read-only UI checks only. A
 * deeper integration test lives in
 * `crates/mockforge-registry-server/tests/federation_scenarios_e2e.rs` and
 * runs against a disposable local registry.
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts \
 *     federation-scenarios-deployed
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

test.describe('Federation scenario activation — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/federation`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });
    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });
    await page.waitForTimeout(3000);
  });

  test('detail view surfaces an Active Scenario panel', async ({ page }) => {
    const main = page.getByRole('main');

    // Click into any existing federation's detail view. If the account has
    // no federations, the test skips rather than mutating prod state.
    const viewButtons = main.getByRole('button', { name: /View|Details|Open/i });
    const count = await viewButtons.count();
    if (count === 0) {
      test.skip(true, 'No federations available on this account to open details for');
      return;
    }

    await viewButtons.first().click();
    await page.waitForTimeout(1500);

    // The ActiveScenarioPanel always renders its heading, regardless of
    // whether a scenario is active.
    await expect(main.getByRole('heading', { name: /Active Scenario/i })).toBeVisible({
      timeout: 10000,
    });
  });

  test('activate form exposes scenario picker and override fields', async ({ page }) => {
    const main = page.getByRole('main');

    const viewButtons = main.getByRole('button', { name: /View|Details|Open/i });
    if ((await viewButtons.count()) === 0) {
      test.skip(true, 'No federations on this account');
      return;
    }
    await viewButtons.first().click();
    await page.waitForTimeout(1500);

    // Only the empty-state panel has an "Activate Scenario" button; if a
    // scenario is already active on this federation the test skips.
    const activateBtn = main.getByRole('button', { name: /Activate Scenario/i });
    if ((await activateBtn.count()) === 0) {
      test.skip(true, 'Federation already has an active scenario');
      return;
    }

    await activateBtn.click();
    await page.waitForTimeout(500);

    // Picker dropdown is present.
    await expect(main.getByText(/Pick a saved scenario/i)).toBeVisible({ timeout: 5000 });

    // Manifest textarea is present.
    await expect(main.getByText(/Manifest JSON/i)).toBeVisible();

    // Per-service override heading renders when the federation has services.
    const overridesHeading = main.getByText(/Per-service overrides/i);
    await expect(overridesHeading).toBeVisible();

    // Cancel instead of activating — we don't want to mutate prod state.
    const cancelBtn = main.getByRole('button', { name: /Cancel/i });
    await cancelBtn.click();
  });
});
