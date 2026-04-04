import { test, expect } from '@playwright/test';

/**
 * MockAI OpenAPI Generator Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts mockai-openapi-deployed
 *
 * These tests verify the MockAI OpenAPI Generator page functionality:
 *   1. Page Load & Layout
 *   2. Generation Filters Form
 *   3. Generate Button
 *   4. Results / Empty State
 *   5. Download Buttons
 *   6. Preview Section
 *   7. Navigation
 *   8. Accessibility
 *   9. Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('MockAI OpenAPI Generator — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/mockai-openapi-generator`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    await expect(
      mainContent(page).getByRole('heading', { name: 'Generate OpenAPI from Traffic', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the OpenAPI Generator page at /mockai-openapi-generator', async ({ page }) => {
      await expect(page).toHaveURL(/\/mockai-openapi-generator/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Generate OpenAPI from Traffic', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText(
          'Analyze recorded HTTP traffic and generate OpenAPI 3.0 specifications using AI-powered pattern detection'
        ).first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      const hasBreadcrumb = await banner.getByText(/OpenAPI|Generate/i)
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasHome = await banner.getByText('Home')
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasBreadcrumb || hasHome).toBeTruthy();
    });

    test('should display the Generation Filters card', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Generation Filters').first()
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Generation Filters Form
  // ---------------------------------------------------------------------------
  test.describe('Generation Filters Form', () => {
    test('should display the Database Path input', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Database Path').first()
      ).toBeVisible({ timeout: 5000 });

      const input = main.locator('input[placeholder="./recordings.db"]');
      await expect(input).toBeVisible({ timeout: 5000 });
    });

    test('should display the Start Time input', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Start Time').first()
      ).toBeVisible({ timeout: 5000 });

      const input = main.locator('input[type="datetime-local"]').first();
      await expect(input).toBeVisible({ timeout: 5000 });
    });

    test('should display the End Time input', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('End Time').first()
      ).toBeVisible({ timeout: 5000 });

      const input = main.locator('input[type="datetime-local"]').nth(1);
      await expect(input).toBeVisible({ timeout: 5000 });
    });

    test('should display the Path Pattern input', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Path Pattern').first()
      ).toBeVisible({ timeout: 5000 });

      const input = main.locator('input[placeholder="/api/*"]');
      await expect(input).toBeVisible({ timeout: 5000 });
    });

    test('should display the Minimum Confidence slider', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText(/Minimum Confidence/)
      ).toBeVisible({ timeout: 5000 });

      const slider = main.locator('input[type="range"]');
      await expect(slider).toBeVisible({ timeout: 5000 });
    });

    test('should display the default confidence value as 70%', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('70%').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should allow typing in the Database Path input', async ({ page }) => {
      const main = mainContent(page);
      const input = main.locator('input[placeholder="./recordings.db"]');
      await input.fill('/tmp/test-recordings.db');
      await page.waitForTimeout(300);

      await expect(input).toHaveValue('/tmp/test-recordings.db');

      // Clean up
      await input.clear();
    });

    test('should allow typing in the Path Pattern input', async ({ page }) => {
      const main = mainContent(page);
      const input = main.locator('input[placeholder="/api/*"]');
      await input.fill('/api/v2/*');
      await page.waitForTimeout(300);

      await expect(input).toHaveValue('/api/v2/*');

      // Clean up
      await input.clear();
    });

    test('should allow adjusting the confidence slider', async ({ page }) => {
      const main = mainContent(page);
      const slider = main.locator('input[type="range"]');
      await slider.fill('0.5');
      await page.waitForTimeout(300);

      await expect(
        main.getByText('50%').first()
      ).toBeVisible({ timeout: 3000 });
    });

    test('should display confidence scale labels (0%, 50%, 100%)', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('0%').first()).toBeVisible({ timeout: 3000 });
      await expect(main.getByText('50%').first()).toBeVisible({ timeout: 3000 });
      await expect(main.getByText('100%').first()).toBeVisible({ timeout: 3000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Generate Button
  // ---------------------------------------------------------------------------
  test.describe('Generate Button', () => {
    test('should display the Generate OpenAPI Spec button', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: /Generate OpenAPI Spec/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should have the Generate button enabled by default', async ({ page }) => {
      const main = mainContent(page);
      const button = main.getByRole('button', { name: /Generate OpenAPI Spec/i });
      await expect(button).toBeEnabled();
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Results / Empty State
  // ---------------------------------------------------------------------------
  test.describe('Results / Empty State', () => {
    test('should display the empty state when no spec has been generated', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('No OpenAPI Specification Generated').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the empty state description', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText(/Configure filters and click/)
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Record Traffic First checklist item', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Record Traffic First').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Configure Filters checklist item', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Configure Filters').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Review & Download checklist item', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Review & Download').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display 3 checklist items in the empty state', async ({ page }) => {
      const main = mainContent(page);
      const recordTraffic = await main.getByText('Record Traffic First')
        .isVisible({ timeout: 3000 }).catch(() => false);
      const configFilters = await main.getByText('Configure Filters')
        .isVisible({ timeout: 3000 }).catch(() => false);
      const reviewDownload = await main.getByText('Review & Download')
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(recordTraffic && configFilters && reviewDownload).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Download Buttons
  // ---------------------------------------------------------------------------
  test.describe('Download Buttons', () => {
    test('should not display download buttons when no spec is generated', async ({ page }) => {
      const main = mainContent(page);
      const hasJsonDownload = await main.getByRole('button', { name: 'JSON' })
        .isVisible({ timeout: 2000 }).catch(() => false);
      const hasYamlDownload = await main.getByRole('button', { name: 'YAML' })
        .isVisible({ timeout: 2000 }).catch(() => false);
      expect(hasJsonDownload).toBeFalsy();
      expect(hasYamlDownload).toBeFalsy();
    });

    test('should not display Generation Statistics when no spec is generated', async ({ page }) => {
      const main = mainContent(page);
      const hasStats = await main.getByText('Generation Statistics')
        .isVisible({ timeout: 2000 }).catch(() => false);
      expect(hasStats).toBeFalsy();
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Preview Section
  // ---------------------------------------------------------------------------
  test.describe('Preview Section', () => {
    test('should not display preview when no spec is generated', async ({ page }) => {
      const main = mainContent(page);
      const hasPreview = await main.getByText('OpenAPI Specification Preview')
        .isVisible({ timeout: 2000 }).catch(() => false);
      expect(hasPreview).toBeFalsy();
    });

    test('should not display Path Confidence Scores when no spec is generated', async ({ page }) => {
      const main = mainContent(page);
      const hasConfidence = await main.getByText('Path Confidence Scores')
        .isVisible({ timeout: 2000 }).catch(() => false);
      expect(hasConfidence).toBeFalsy();
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

      await page.goto(`${BASE_URL}/mockai-openapi-generator`, {
        waitUntil: 'domcontentloaded',
        timeout: 30000,
      });
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Generate OpenAPI from Traffic', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Services and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Services' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Services', exact: true, level: 1 })
      ).toBeVisible({ timeout: 5000 });

      await page.goto(`${BASE_URL}/mockai-openapi-generator`, {
        waitUntil: 'domcontentloaded',
        timeout: 30000,
      });
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Generate OpenAPI from Traffic', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should preserve URL when navigating back via browser history', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await page.goBack();
      await page.waitForTimeout(1500);

      await expect(page).toHaveURL(/\/mockai-openapi-generator/);
      await expect(
        mainContent(page).getByRole('heading', { name: 'Generate OpenAPI from Traffic', level: 1 })
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
      await expect(h1).toHaveText('Generate OpenAPI from Traffic');
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

    test('should have labeled form inputs', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Database Path').first()).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('Start Time').first()).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('End Time').first()).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('Path Pattern').first()).toBeVisible({ timeout: 5000 });
      await expect(main.getByText(/Minimum Confidence/)).toBeVisible({ timeout: 5000 });
    });

    test('should have accessible Generate button', async ({ page }) => {
      const main = mainContent(page);
      const button = main.getByRole('button', { name: /Generate OpenAPI Spec/i });
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
  });
});
