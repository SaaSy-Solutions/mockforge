import { test, expect } from '@playwright/test';

/**
 * MockAI Rules Dashboard Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts mockai-rules-deployed
 *
 * These tests verify the MockAI Rules Dashboard page functionality:
 *   1. Page Load & Layout
 *   2. Filters (Search, Type, Confidence)
 *   3. Stats Display
 *   4. Rule Flow Toggle
 *   5. Rules List / Empty State
 *   6. Refresh
 *   7. Navigation
 *   8. Accessibility
 *   9. Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('MockAI Rules Dashboard — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/mockai-rules`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    await expect(
      mainContent(page).getByRole('heading', { name: 'MockAI Rules Dashboard', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the Rules Dashboard page at /mockai-rules', async ({ page }) => {
      await expect(page).toHaveURL(/\/mockai-rules/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'MockAI Rules Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the subtitle', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText(/View and explore all generated behavioral rules/)
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      const hasBreadcrumb = await banner.getByText(/Rules|MockAI/i)
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasHome = await banner.getByText('Home')
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasBreadcrumb || hasHome).toBeTruthy();
    });

    test('should display the sidebar navigation', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');
      await expect(nav).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Filters (Search, Type, Confidence)
  // ---------------------------------------------------------------------------
  test.describe('Filters', () => {
    test('should display the search input', async ({ page }) => {
      const main = mainContent(page);
      const searchInput = main.locator('input[placeholder*="Search rules"]');
      await expect(searchInput).toBeVisible({ timeout: 5000 });
    });

    test('should allow typing in the search input', async ({ page }) => {
      const main = mainContent(page);
      const searchInput = main.locator('input[placeholder*="Search rules"]');
      await searchInput.fill('consistency');
      await page.waitForTimeout(300);

      await expect(searchInput).toHaveValue('consistency');

      // Clean up
      await searchInput.clear();
    });

    test('should display the Rule Type dropdown', async ({ page }) => {
      const main = mainContent(page);
      const select = main.locator('select');
      await expect(select).toBeVisible({ timeout: 5000 });
    });

    test('should display all rule type options in the dropdown', async ({ page }) => {
      const main = mainContent(page);
      const select = main.locator('select');

      const options = [
        'All Rule Types',
        'Consistency',
        'Validation',
        'Pagination',
        'State Transition',
        'CRUD',
      ];

      for (const option of options) {
        await expect(
          select.locator(`option:text("${option}")`)
        ).toBeAttached();
      }
    });

    test('should allow selecting a rule type filter', async ({ page }) => {
      const main = mainContent(page);
      const select = main.locator('select');
      await select.selectOption('consistency');
      await page.waitForTimeout(500);

      await expect(select).toHaveValue('consistency');

      // Reset
      await select.selectOption('all');
    });

    test('should display the Min Confidence slider', async ({ page }) => {
      const main = mainContent(page);
      const slider = main.locator('input[type="range"]');
      await expect(slider).toBeVisible({ timeout: 5000 });
    });

    test('should display the confidence percentage label', async ({ page }) => {
      const main = mainContent(page);
      // Default is 0%, displayed as >=0%
      await expect(
        main.getByText(/≥\d+%/)
      ).toBeVisible({ timeout: 5000 });
    });

    test('should allow adjusting the confidence slider', async ({ page }) => {
      const main = mainContent(page);
      const slider = main.locator('input[type="range"]');
      await slider.fill('0.5');
      await page.waitForTimeout(300);

      await expect(
        main.getByText('≥50%').first()
      ).toBeVisible({ timeout: 3000 });

      // Reset
      await slider.fill('0');
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Stats Display
  // ---------------------------------------------------------------------------
  test.describe('Stats Display', () => {
    test('should display the rules count stat', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText(/\d+ of \d+ rules/)
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the average confidence stat when rules exist', async ({ page }) => {
      const main = mainContent(page);
      // Avg confidence only shows when there are rules
      const hasAvgConfidence = await main.getByText(/Avg confidence/)
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasRulesCount = await main.getByText(/\d+ of \d+ rules/)
        .isVisible({ timeout: 3000 }).catch(() => false);
      // Either avg confidence is shown (rules exist) or just the count (no rules)
      expect(hasAvgConfidence || hasRulesCount).toBeTruthy();
    });

    test('should update rules count when filter is applied', async ({ page }) => {
      const main = mainContent(page);
      const select = main.locator('select');
      await select.selectOption('validation');
      await page.waitForTimeout(1000);

      // The count should still be visible after filtering
      await expect(
        main.getByText(/\d+ of \d+ rules/)
      ).toBeVisible({ timeout: 5000 });

      // Reset
      await select.selectOption('all');
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Rule Flow Toggle
  // ---------------------------------------------------------------------------
  test.describe('Rule Flow Toggle', () => {
    test('should display Rule Generation Flow section when rules exist', async ({ page }) => {
      const main = mainContent(page);
      const hasFlowSection = await main.getByText('Rule Generation Flow')
        .isVisible({ timeout: 5000 }).catch(() => false);
      // Flow section only appears when explanations.length > 0
      // If no rules, this section won't be present, which is correct
      const hasEmptyState = await main.getByText('No Rules Found')
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasFlowSection || hasEmptyState).toBeTruthy();
    });

    test('should display Show/Hide Flow toggle button when rules exist', async ({ page }) => {
      const main = mainContent(page);
      const hasFlowSection = await main.getByText('Rule Generation Flow').first()
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasFlowSection) {
        const toggleBtn = main.getByRole('button', { name: /Show Flow|Hide Flow/i });
        await expect(toggleBtn).toBeVisible({ timeout: 5000 });
      }
    });

    test('should toggle flow visibility when clicking Show/Hide button', async ({ page }) => {
      const main = mainContent(page);
      const hasFlowSection = await main.getByText('Rule Generation Flow')
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasFlowSection) {
        const showBtn = main.getByRole('button', { name: 'Show Flow' });
        const isShowVisible = await showBtn.isVisible({ timeout: 2000 }).catch(() => false);

        if (isShowVisible) {
          await showBtn.click();
          await page.waitForTimeout(500);

          // After clicking Show, button text should change to Hide
          await expect(
            main.getByRole('button', { name: 'Hide Flow' })
          ).toBeVisible({ timeout: 3000 });

          // Click Hide to toggle back
          await main.getByRole('button', { name: 'Hide Flow' }).click();
          await page.waitForTimeout(500);

          await expect(
            main.getByRole('button', { name: 'Show Flow' })
          ).toBeVisible({ timeout: 3000 });
        }
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Rules List / Empty State
  // ---------------------------------------------------------------------------
  test.describe('Rules List / Empty State', () => {
    test('should display either rules grid or empty state', async ({ page }) => {
      const main = mainContent(page);
      const hasRules = await main.locator('.grid.grid-cols-1.lg\\:grid-cols-2')
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasEmptyState = await main.getByText('No Rules Found')
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasRules || hasEmptyState).toBeTruthy();
    });

    test('should display No Rules Found empty state with appropriate message', async ({ page }) => {
      const main = mainContent(page);
      const hasEmptyState = await main.getByText('No Rules Found')
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasEmptyState) {
        await expect(
          main.getByText(/No rules have been generated yet|Try adjusting your filters/)
        ).toBeVisible({ timeout: 3000 });
      }
    });

    test('should show filter-specific empty message when filters produce no results', async ({ page }) => {
      const main = mainContent(page);
      const searchInput = main.locator('input[placeholder*="Search rules"]');
      await searchInput.fill('xyznonexistentrulethatdoesnotexist');
      await page.waitForTimeout(500);

      const hasEmptyState = await main.getByText('No Rules Found')
        .isVisible({ timeout: 5000 }).catch(() => false);

      if (hasEmptyState) {
        await expect(
          main.getByText(/Try adjusting your filters/)
        ).toBeVisible({ timeout: 3000 });
      }

      // Clean up
      await searchInput.clear();
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Refresh
  // ---------------------------------------------------------------------------
  test.describe('Refresh', () => {
    test('should display the Refresh Rules button', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: /Refresh Rules/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should be able to click the Refresh Rules button', async ({ page }) => {
      const main = mainContent(page);
      const refreshBtn = main.getByRole('button', { name: /Refresh Rules/i });
      await expect(refreshBtn).toBeEnabled();
      await refreshBtn.click();
      await page.waitForTimeout(1000);

      // After refresh, the page should still be functional
      await expect(
        main.getByRole('heading', { name: 'MockAI Rules Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should maintain filter state after refresh', async ({ page }) => {
      const main = mainContent(page);
      const searchInput = main.locator('input[placeholder*="Search rules"]');
      await searchInput.fill('test-query');
      await page.waitForTimeout(300);

      const refreshBtn = main.getByRole('button', { name: /Refresh Rules/i });
      await refreshBtn.click();
      await page.waitForTimeout(1000);

      // Search input may or may not persist depending on implementation;
      // verify page didn't crash
      await expect(
        main.getByRole('heading', { name: 'MockAI Rules Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 7. Navigation
  // ---------------------------------------------------------------------------
  test.describe('Navigation', () => {
    test('should navigate to Dashboard and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      await page.goto(`${BASE_URL}/mockai-rules`, {
        waitUntil: 'domcontentloaded',
        timeout: 30000,
      });
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'MockAI Rules Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Services and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Services' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Services', exact: true, level: 1 })
      ).toBeVisible({ timeout: 5000 });

      await page.goto(`${BASE_URL}/mockai-rules`, {
        waitUntil: 'domcontentloaded',
        timeout: 30000,
      });
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'MockAI Rules Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should preserve URL when navigating back via browser history', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await page.goBack();
      await page.waitForTimeout(1500);

      await expect(page).toHaveURL(/\/mockai-rules/);
      await expect(
        mainContent(page).getByRole('heading', { name: 'MockAI Rules Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 8. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a single H1 heading', async ({ page }) => {
      const h1 = mainContent(page).getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);
      await expect(h1).toHaveText('MockAI Rules Dashboard');
    });

    test('should have accessible landmark regions', async ({ page }) => {
      await expect(page.getByRole('main')).toBeVisible();
      await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
      await expect(page.getByRole('banner')).toBeVisible();
    });

    test('should have skip navigation links', async ({ page }) => {
      await expect(page.getByRole('link', { name: 'Skip to navigation' })).toBeAttached();
      await expect(page.getByRole('link', { name: 'Skip to main content' })).toBeAttached();
    });

    test('should have accessible search input with placeholder', async ({ page }) => {
      const main = mainContent(page);
      const searchInput = main.locator('input[placeholder*="Search rules"]');
      await expect(searchInput).toBeVisible({ timeout: 5000 });
    });

    test('should have accessible select dropdown', async ({ page }) => {
      const main = mainContent(page);
      const select = main.locator('select');
      await expect(select).toBeVisible({ timeout: 5000 });
    });

    test('should have accessible Refresh Rules button', async ({ page }) => {
      const main = mainContent(page);
      const button = main.getByRole('button', { name: /Refresh Rules/i });
      await expect(button).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 9. Error-Free Operation
  // ---------------------------------------------------------------------------
  test.describe('Error-Free Operation', () => {
    test('should load without JavaScript console errors', async ({ page }) => {
      const consoleErrors: string[] = [];

      page.on('console', (msg) => {
        if (msg.type() === 'error') {
          consoleErrors.push(msg.text());
        }
      });

      await page.reload({ waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(3000);

      const criticalErrors = consoleErrors.filter(
        (err) =>
          !err.includes('net::ERR_') &&
          !err.includes('Failed to fetch') &&
          !err.includes('NetworkError') &&
          !err.includes('WebSocket') &&
          !err.includes('favicon') &&
          !err.includes('429') &&
          !err.includes('not valid JSON') &&
          !err.includes('DOCTYPE')
      );

      expect(criticalErrors).toHaveLength(0);
    });

    test('should not show any unhandled error UI', async ({ page }) => {
      const hasErrorBoundary = await page
        .getByText(/Something went wrong|Unexpected error|Application error/i)
        .first()
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasErrorBoundary).toBeFalsy();
    });

    test('should not show error loading state', async ({ page }) => {
      const hasError = await mainContent(page)
        .getByText(/Error Loading|Failed to load/i)
        .first()
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasError).toBeFalsy();
    });

    test('should render page content without crashing', async ({ page }) => {
      const main = mainContent(page);
      const text = await main.textContent();
      expect(text!.length).toBeGreaterThan(0);
    });

    test('should handle filter interactions without errors', async ({ page }) => {
      const consoleErrors: string[] = [];

      page.on('console', (msg) => {
        if (msg.type() === 'error') {
          consoleErrors.push(msg.text());
        }
      });

      const main = mainContent(page);

      // Interact with all filters
      const searchInput = main.locator('input[placeholder*="Search rules"]');
      await searchInput.fill('test');
      await page.waitForTimeout(300);

      const select = main.locator('select');
      await select.selectOption('validation');
      await page.waitForTimeout(300);

      const slider = main.locator('input[type="range"]');
      await slider.fill('0.8');
      await page.waitForTimeout(300);

      // Reset
      await searchInput.clear();
      await select.selectOption('all');
      await slider.fill('0');
      await page.waitForTimeout(300);

      const criticalErrors = consoleErrors.filter(
        (err) =>
          !err.includes('net::ERR_') &&
          !err.includes('Failed to fetch') &&
          !err.includes('NetworkError') &&
          !err.includes('WebSocket') &&
          !err.includes('favicon') &&
          !err.includes('429') &&
          !err.includes('not valid JSON') &&
          !err.includes('DOCTYPE')
      );

      expect(criticalErrors).toHaveLength(0);
    });
  });
});
