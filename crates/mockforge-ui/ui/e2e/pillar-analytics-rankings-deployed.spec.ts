import { test, expect } from '@playwright/test';

/**
 * Pillar Analytics — Rankings Card & Extended Metric Fields
 *
 * Covers the elements added alongside wiring the /api/v2/analytics/pillars/*
 * admin-UI routes: the "Pillar Usage Rankings" card and the surfacing of
 * ai_contract_diffs / cli_commands / collaborative_workspaces inside the
 * per-pillar detail cards.
 *
 * Run against the deployed site with:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts pillar-analytics-rankings
 *
 * Run against a local dev build (after logging in):
 *   PLAYWRIGHT_BASE_URL=http://localhost:49080 \
 *   npx playwright test pillar-analytics-rankings
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Pillar Analytics — Rankings & Extended Fields', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/pillar-analytics`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await expect(
      mainContent(page).getByRole('heading', { name: 'Pillar Usage Analytics', level: 1 })
    ).toBeVisible({ timeout: 15000 });
  });

  test('renders the Pillar Usage Rankings card', async ({ page }) => {
    await expect(
      mainContent(page).getByRole('heading', { name: 'Pillar Usage Rankings', level: 2 })
    ).toBeVisible();
  });

  test('rankings card shows empty state or ranked list', async ({ page }) => {
    const main = mainContent(page);
    const emptyState = main.getByText('No pillar usage data recorded yet.');
    // At least one of the five pillar rows becomes a "#1 / #2 …" heading in the ranked list.
    const rankedRow = main.getByText(/^#\d+$/).first();

    const hasEmpty = await emptyState.isVisible({ timeout: 3000 }).catch(() => false);
    const hasRanked = await rankedRow.isVisible({ timeout: 3000 }).catch(() => false);

    expect(hasEmpty || hasRanked).toBeTruthy();
  });

  test('rankings card renders with total or empty-state sibling when ranked', async ({ page }) => {
    const main = mainContent(page);
    const rankedRow = main.getByText(/^#\d+$/).first();
    const hasRanked = await rankedRow.isVisible({ timeout: 3000 }).catch(() => false);

    if (hasRanked) {
      // When a ranked list is shown, the most-used / least-used pills must appear.
      await expect(main.getByText(/Most used/i).first()).toBeVisible();
      await expect(main.getByText(/Least used/i).first()).toBeVisible();
      // And the total-usage pill next to the heading.
      await expect(main.getByText(/^Total:\s/)).toBeVisible();
    } else {
      // Empty state must show the expected copy (asserted in the prior test) — nothing to add.
      test.skip();
    }
  });

  test.describe('detailed pillar cards surface newly wired fields', () => {
    // These assertions only run when a workspace has been selected and the
    // per-pillar detail cards render. When no workspace is selected the page
    // shows only the overview cards, so we skip rather than fail.
    test('DevX card shows CLI commands', async ({ page }) => {
      const main = mainContent(page);
      const devxSection = main.getByRole('heading', { name: 'DevX Pillar', level: 3 });
      const hasDevx = await devxSection.isVisible({ timeout: 3000 }).catch(() => false);
      if (!hasDevx) {
        test.skip();
      }
      await expect(main.getByText(/CLI commands:/)).toBeVisible();
    });

    test('Cloud card shows Collaborative workspaces', async ({ page }) => {
      const main = mainContent(page);
      const cloudSection = main.getByRole('heading', { name: 'Cloud Pillar', level: 3 });
      const hasCloud = await cloudSection.isVisible({ timeout: 3000 }).catch(() => false);
      if (!hasCloud) {
        test.skip();
      }
      await expect(main.getByText(/Collaborative workspaces:/)).toBeVisible();
    });

    test('AI card shows AI contract diffs', async ({ page }) => {
      const main = mainContent(page);
      const aiSection = main.getByRole('heading', { name: 'AI Pillar', level: 3 });
      const hasAi = await aiSection.isVisible({ timeout: 3000 }).catch(() => false);
      if (!hasAi) {
        test.skip();
      }
      await expect(main.getByText(/AI contract diffs:/)).toBeVisible();
    });
  });
});
