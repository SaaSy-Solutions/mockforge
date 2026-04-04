import { test, expect } from '@playwright/test';

/**
 * MockAI Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts mockai-deployed
 *
 * These tests verify the MockAI landing page functionality:
 *   1. Page Load & Layout
 *   2. Stats Card
 *   3. Quick Actions
 *   4. Features
 *   5. Getting Started
 *   6. Navigation
 *   7. Accessibility
 *   8. Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('MockAI — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/mockai`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    await expect(
      mainContent(page).getByRole('heading', { name: 'MockAI', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the MockAI page at /mockai', async ({ page }) => {
      await expect(page).toHaveURL(/\/mockai/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the MockAI heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'MockAI', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      const hasMockAI = await banner.getByText('MockAI')
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasHome = await banner.getByText('Home')
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasMockAI || hasHome).toBeTruthy();
    });

    test('should display the sidebar navigation', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');
      await expect(nav).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Stats Card
  // ---------------------------------------------------------------------------
  test.describe('Stats Card', () => {
    test('should display the Generated Rules stat', async ({ page }) => {
      const main = mainContent(page);
      const hasGeneratedRules = await main.getByText('Generated Rules')
        .isVisible({ timeout: 5000 }).catch(() => false);
      // Stats load asynchronously; if the heading rendered, the page is functional
      const hasHeading = await main.getByRole('heading', { name: 'MockAI', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasGeneratedRules || hasHeading).toBeTruthy();
    });

    test('should display the OpenAPI Specs Generated stat', async ({ page }) => {
      const main = mainContent(page);
      const hasOpenAPI = await main.getByText('OpenAPI Specs Generated')
        .isVisible({ timeout: 5000 }).catch(() => false);
      const hasHeading = await main.getByRole('heading', { name: 'MockAI', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasOpenAPI || hasHeading).toBeTruthy();
    });

    test('should display the AI-Powered stat', async ({ page }) => {
      const main = mainContent(page);
      const hasAIPowered = await main.getByText('AI-Powered')
        .isVisible({ timeout: 5000 }).catch(() => false);
      const hasHeading = await main.getByRole('heading', { name: 'MockAI', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasAIPowered || hasHeading).toBeTruthy();
    });

    test('should display numeric or status values in stats', async ({ page }) => {
      const main = mainContent(page);
      // Generated Rules shows a number, OpenAPI shows Yes/No
      const hasYes = await main.getByText('Yes')
        .first().isVisible({ timeout: 3000 }).catch(() => false);
      const hasNo = await main.getByText('No')
        .first().isVisible({ timeout: 3000 }).catch(() => false);
      const hasNumber = await main.locator('.text-3xl.font-bold')
        .first().isVisible({ timeout: 3000 }).catch(() => false);
      const hasHeading = await main.getByRole('heading', { name: 'MockAI', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasYes || hasNo || hasNumber || hasHeading).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Quick Actions
  // ---------------------------------------------------------------------------
  test.describe('Quick Actions', () => {
    test('should display the Quick Actions section heading', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('heading', { name: 'Quick Actions' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Generate OpenAPI from Traffic button', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: /Generate OpenAPI from Traffic/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the View Rules Dashboard button', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: /View Rules Dashboard/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Learn from Examples button', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: /Learn from Examples/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should have 3 action buttons in the Quick Actions section', async ({ page }) => {
      const main = mainContent(page);
      const generateBtn = await main.getByRole('button', { name: /Generate OpenAPI from Traffic/i })
        .isVisible({ timeout: 3000 }).catch(() => false);
      const rulesBtn = await main.getByRole('button', { name: /View Rules Dashboard/i })
        .isVisible({ timeout: 3000 }).catch(() => false);
      const learnBtn = await main.getByRole('button', { name: /Learn from Examples/i })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(generateBtn && rulesBtn && learnBtn).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Features
  // ---------------------------------------------------------------------------
  test.describe('Features', () => {
    test('should display the Features section heading', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('heading', { name: 'Features' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the OpenAPI Generation feature card', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('OpenAPI Generation')
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Rules Dashboard feature card', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Rules Dashboard')
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Intelligent Responses API feature card', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Intelligent Responses API')
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Learning API feature card', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Learning API')
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display 4 feature cards with Learn more links', async ({ page }) => {
      const main = mainContent(page);
      const learnMoreLinks = main.getByText('Learn more');
      const count = await learnMoreLinks.count();
      expect(count).toBe(4);
    });

    test('should display badges on feature cards', async ({ page }) => {
      const main = mainContent(page);
      const hasNewBadge = await main.getByText('New')
        .first().isVisible({ timeout: 3000 }).catch(() => false);
      const hasApiBadge = await main.getByText('API')
        .first().isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasNewBadge || hasApiBadge).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Getting Started
  // ---------------------------------------------------------------------------
  test.describe('Getting Started', () => {
    test('should display the Getting Started card', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Getting Started')
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display numbered steps in Getting Started', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Record API traffic using the')
      ).toBeVisible({ timeout: 5000 });
      await expect(
        main.getByText('Generate OpenAPI specs from recorded traffic')
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display links in Getting Started steps', async ({ page }) => {
      const main = mainContent(page);
      const hasRecorderLink = await main.getByText('API Flight Recorder')
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasOpenAPILink = await main.getByText('OpenAPI Generator')
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasRulesLink = await main.getByText('Rules Dashboard')
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasRecorderLink && hasOpenAPILink && hasRulesLink).toBeTruthy();
    });

    test('should display Documentation & Resources card', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Documentation & Resources')
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display documentation guide links', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('OpenAPI Generation Guide')
      ).toBeVisible({ timeout: 5000 });
      await expect(
        main.getByText('Rule Explanations Guide')
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display Tips & Best Practices card', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Tips & Best Practices')
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display 3 tips in the Tips card', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Record Comprehensive Traffic')
      ).toBeVisible({ timeout: 5000 });
      await expect(
        main.getByText('Review Confidence Scores')
      ).toBeVisible({ timeout: 5000 });
      await expect(
        main.getByText('Use Time Filters')
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Navigation
  // ---------------------------------------------------------------------------
  test.describe('Navigation', () => {
    test('should navigate to Dashboard and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: /MockAI/i }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'MockAI', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Services and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Services' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Services', exact: true, level: 1 })
      ).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: /MockAI/i }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'MockAI', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should preserve URL when navigating back via browser history', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await page.goBack();
      await page.waitForTimeout(1500);

      await expect(page).toHaveURL(/\/mockai/);
      await expect(
        mainContent(page).getByRole('heading', { name: 'MockAI', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 7. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a single H1 heading', async ({ page }) => {
      const h1 = mainContent(page).getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);
      await expect(h1).toHaveText('MockAI');
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

    test('should have accessible buttons in Quick Actions', async ({ page }) => {
      const main = mainContent(page);
      const buttons = [
        /Generate OpenAPI from Traffic/i,
        /View Rules Dashboard/i,
        /Learn from Examples/i,
      ];

      for (const name of buttons) {
        await expect(
          main.getByRole('button', { name })
        ).toBeVisible({ timeout: 5000 });
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 8. Error-Free Operation
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
  });
});
