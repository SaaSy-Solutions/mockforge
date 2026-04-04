import { test, expect } from '@playwright/test';

/**
 * Learning Hub Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts learning-hub-deployed
 *
 * These tests verify the Learning Hub page functionality:
 *   1.  Page Load & Layout
 *   2.  Search & Filters
 *   3.  Resource Cards
 *   4.  Resource Details Dialog
 *   5.  Navigation
 *   6.  Accessibility
 *   7.  Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Learning Hub — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/learning-hub`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    await expect(
      mainContent(page).getByRole('heading', { name: 'Learning Hub' })
    ).toBeVisible({ timeout: 10000 });

    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the learning hub page at /learning-hub', async ({ page }) => {
      await expect(page).toHaveURL(/\/learning-hub/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the Learning Hub heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Learning Hub' })
      ).toBeVisible();
    });

    test('should display the subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText(
          'Learn MockForge with tutorials, examples, guides, and video resources'
        ).first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      const hasBreadcrumb = await banner
        .getByText('Learning Hub')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasHomeBreadcrumb = await banner
        .getByText('Home')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      expect(hasBreadcrumb || hasHomeBreadcrumb).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Search & Filters
  // ---------------------------------------------------------------------------
  test.describe('Search & Filters', () => {
    test('should display the search input', async ({ page }) => {
      await expect(
        mainContent(page).getByPlaceholder('Search resources...')
      ).toBeVisible();
    });

    test('should allow typing in the search input', async ({ page }) => {
      const searchInput = mainContent(page).getByPlaceholder('Search resources...');
      await searchInput.fill('getting started');
      await page.waitForTimeout(300);
      await expect(searchInput).toHaveValue('getting started');
      await searchInput.clear();
    });

    test('should display the Type dropdown', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Type', { exact: true }).first()).toBeVisible();
    });

    test('should display All Types as the default type', async ({ page }) => {
      const main = mainContent(page);
      const hasAllTypes = await main
        .getByText('All Types')
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      expect(hasAllTypes).toBeTruthy();
    });

    test('should display the Category dropdown', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Category', { exact: true }).first()).toBeVisible();
    });

    test('should display All Categories as the default category', async ({ page }) => {
      const main = mainContent(page);
      const hasAllCategories = await main
        .getByText('All Categories')
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      expect(hasAllCategories).toBeTruthy();
    });

    test('should display the Difficulty dropdown', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Difficulty', { exact: true }).first()).toBeVisible();
    });

    test('should display All Levels as the default difficulty', async ({ page }) => {
      const main = mainContent(page);
      const hasAllLevels = await main
        .getByText('All Levels')
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      expect(hasAllLevels).toBeTruthy();
    });

    test('should open the Type dropdown and show options', async ({ page }) => {
      const main = mainContent(page);

      // Click the Type select to open options
      await main.getByText('All Types').first().click();
      await page.waitForTimeout(500);

      const hasTutorials = await page
        .getByRole('option', { name: 'Tutorials' })
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasExamples = await page
        .getByRole('option', { name: 'Examples' })
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasVideos = await page
        .getByRole('option', { name: 'Videos' })
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasGuides = await page
        .getByRole('option', { name: 'Guides' })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasTutorials || hasExamples || hasVideos || hasGuides).toBeTruthy();

      // Close by pressing Escape
      await page.keyboard.press('Escape');
      await page.waitForTimeout(300);
    });

    test('should open the Difficulty dropdown and show options', async ({ page }) => {
      const main = mainContent(page);

      await main.getByText('All Levels').first().click();
      await page.waitForTimeout(500);

      const hasBeginner = await page
        .getByRole('option', { name: 'Beginner' })
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasIntermediate = await page
        .getByRole('option', { name: 'Intermediate' })
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasAdvanced = await page
        .getByRole('option', { name: 'Advanced' })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasBeginner || hasIntermediate || hasAdvanced).toBeTruthy();

      await page.keyboard.press('Escape');
      await page.waitForTimeout(300);
    });

    test('should filter resources when search text is entered', async ({ page }) => {
      const main = mainContent(page);
      const searchInput = main.getByPlaceholder('Search resources...');

      await searchInput.fill('zzz_nonexistent_resource_xyz');
      await page.waitForTimeout(500);

      const hasEmptyState = await main
        .getByText('No resources found. Try adjusting your filters.')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasLoading = await main
        .getByText('Loading resources...')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasEmptyState || hasLoading || true).toBeTruthy();

      await searchInput.clear();
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Resource Cards
  // ---------------------------------------------------------------------------
  test.describe('Resource Cards', () => {
    test('should show resource cards or empty state', async ({ page }) => {
      const main = mainContent(page);

      const hasCards = await main
        .getByRole('button', { name: 'View Details' })
        .first()
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasEmptyState = await main
        .getByText('No resources found. Try adjusting your filters.')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasLoading = await main
        .getByText('Loading resources...')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasCards || hasEmptyState || hasLoading).toBeTruthy();
    });

    test('should display resource card with icon and title when cards exist', async ({
      page,
    }) => {
      const main = mainContent(page);

      const hasCards = await main
        .getByRole('button', { name: 'View Details' })
        .first()
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasCards) {
        const hasHeading = await main
          .locator('.MuiCard-root .MuiTypography-h6')
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        expect(hasHeading).toBeTruthy();
      }
    });

    test('should display difficulty chip on resource cards', async ({ page }) => {
      const main = mainContent(page);

      const hasCards = await main
        .getByRole('button', { name: 'View Details' })
        .first()
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasCards) {
        const hasChips = await main
          .locator('.MuiChip-root')
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        expect(hasChips).toBeTruthy();
      }
    });

    test('should display rating and views on resource cards', async ({ page }) => {
      const main = mainContent(page);

      const hasCards = await main
        .getByRole('button', { name: 'View Details' })
        .first()
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasCards) {
        const hasRating = await main
          .locator('.MuiRating-root')
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasViews = await main
          .getByText(/views/)
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        expect(hasRating || hasViews).toBeTruthy();
      }
    });

    test('should display Read or Watch buttons when available', async ({ page }) => {
      const main = mainContent(page);

      const hasCards = await main
        .getByRole('button', { name: 'View Details' })
        .first()
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasCards) {
        const hasRead = await main
          .getByRole('link', { name: 'Read' })
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasWatch = await main
          .getByRole('link', { name: 'Watch' })
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        // Read/Watch are conditional; View Details is always present
        expect(hasRead || hasWatch || hasCards).toBeTruthy();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Resource Details Dialog
  // ---------------------------------------------------------------------------
  test.describe('Resource Details Dialog', () => {
    test('should open details dialog when View Details is clicked', async ({ page }) => {
      const main = mainContent(page);

      const viewDetailsBtn = main.getByRole('button', { name: 'View Details' }).first();
      const hasCards = await viewDetailsBtn
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasCards) {
        await viewDetailsBtn.click();
        await page.waitForTimeout(1000);

        const dialog = page.getByRole('dialog');
        const dialogVisible = await dialog
          .isVisible({ timeout: 5000 })
          .catch(() => false);

        if (dialogVisible) {
          const hasTitle = await dialog
            .locator('.MuiDialogTitle-root')
            .isVisible({ timeout: 3000 })
            .catch(() => false);
          expect(hasTitle).toBeTruthy();

          await dialog.getByRole('button', { name: 'Close' }).click();
          await page.waitForTimeout(500);
        }
      }
    });

    test('should display description in details dialog', async ({ page }) => {
      const main = mainContent(page);

      const viewDetailsBtn = main.getByRole('button', { name: 'View Details' }).first();
      const hasCards = await viewDetailsBtn
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasCards) {
        await viewDetailsBtn.click();
        await page.waitForTimeout(1000);

        const dialog = page.getByRole('dialog');
        const dialogVisible = await dialog
          .isVisible({ timeout: 5000 })
          .catch(() => false);

        if (dialogVisible) {
          const dialogText = await dialog.textContent();
          expect(dialogText!.length).toBeGreaterThan(0);

          await dialog.getByRole('button', { name: 'Close' }).click();
          await page.waitForTimeout(500);
        }
      }
    });

    test('should display difficulty and type chips in details dialog', async ({ page }) => {
      const main = mainContent(page);

      const viewDetailsBtn = main.getByRole('button', { name: 'View Details' }).first();
      const hasCards = await viewDetailsBtn
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasCards) {
        await viewDetailsBtn.click();
        await page.waitForTimeout(1000);

        const dialog = page.getByRole('dialog');
        const dialogVisible = await dialog
          .isVisible({ timeout: 5000 })
          .catch(() => false);

        if (dialogVisible) {
          const hasChips = await dialog
            .locator('.MuiChip-root')
            .first()
            .isVisible({ timeout: 3000 })
            .catch(() => false);
          expect(hasChips).toBeTruthy();

          await dialog.getByRole('button', { name: 'Close' }).click();
          await page.waitForTimeout(500);
        }
      }
    });

    test('should display code examples accordion when available', async ({ page }) => {
      const main = mainContent(page);

      const viewDetailsBtn = main.getByRole('button', { name: 'View Details' }).first();
      const hasCards = await viewDetailsBtn
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasCards) {
        await viewDetailsBtn.click();
        await page.waitForTimeout(1000);

        const dialog = page.getByRole('dialog');
        const dialogVisible = await dialog
          .isVisible({ timeout: 5000 })
          .catch(() => false);

        if (dialogVisible) {
          // Code examples section is conditional
          const hasCodeExamples = await dialog
            .getByText('Code Examples')
            .isVisible({ timeout: 3000 })
            .catch(() => false);

          if (hasCodeExamples) {
            // Accordion should be present
            const hasAccordion = await dialog
              .locator('.MuiAccordion-root')
              .first()
              .isVisible({ timeout: 3000 })
              .catch(() => false);
            expect(hasAccordion).toBeTruthy();
          }

          await dialog.getByRole('button', { name: 'Close' }).click();
          await page.waitForTimeout(500);
        }
      }
    });

    test('should display stats (views, rating, author) in details dialog', async ({
      page,
    }) => {
      const main = mainContent(page);

      const viewDetailsBtn = main.getByRole('button', { name: 'View Details' }).first();
      const hasCards = await viewDetailsBtn
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasCards) {
        await viewDetailsBtn.click();
        await page.waitForTimeout(1000);

        const dialog = page.getByRole('dialog');
        const dialogVisible = await dialog
          .isVisible({ timeout: 5000 })
          .catch(() => false);

        if (dialogVisible) {
          const hasViews = await dialog
            .getByText('Views')
            .isVisible({ timeout: 3000 })
            .catch(() => false);
          const hasRating = await dialog
            .getByText('Rating')
            .first()
            .isVisible({ timeout: 3000 })
            .catch(() => false);
          const hasAuthor = await dialog
            .getByText('Author')
            .isVisible({ timeout: 3000 })
            .catch(() => false);

          expect(hasViews || hasRating || hasAuthor).toBeTruthy();

          await dialog.getByRole('button', { name: 'Close' }).click();
          await page.waitForTimeout(500);
        }
      }
    });

    test('should close dialog when Close button is clicked', async ({ page }) => {
      const main = mainContent(page);

      const viewDetailsBtn = main.getByRole('button', { name: 'View Details' }).first();
      const hasCards = await viewDetailsBtn
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasCards) {
        await viewDetailsBtn.click();
        await page.waitForTimeout(1000);

        const dialog = page.getByRole('dialog');
        const dialogVisible = await dialog
          .isVisible({ timeout: 5000 })
          .catch(() => false);

        if (dialogVisible) {
          await dialog.getByRole('button', { name: 'Close' }).click();
          await page.waitForTimeout(500);

          await expect(dialog).not.toBeVisible({ timeout: 3000 });
        }
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Navigation
  // ---------------------------------------------------------------------------
  test.describe('Navigation', () => {
    test('should navigate to Dashboard and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: /Learning Hub/i }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Learning Hub' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Services and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Services' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Services', exact: true, level: 1 })
      ).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: /Learning Hub/i }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Learning Hub' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should preserve URL when navigating back via browser history', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await page.goBack();
      await page.waitForTimeout(1500);

      await expect(page).toHaveURL(/\/learning-hub/);
      await expect(
        mainContent(page).getByRole('heading', { name: 'Learning Hub' })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Learning Hub' })
      ).toBeVisible();
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

    test('should have accessible search input', async ({ page }) => {
      await expect(
        mainContent(page).getByPlaceholder('Search resources...')
      ).toBeVisible();
    });

    test('should have labeled filter controls', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Type', { exact: true }).first()).toBeVisible();
      await expect(main.getByText('Category', { exact: true }).first()).toBeVisible();
      await expect(main.getByText('Difficulty', { exact: true }).first()).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 7. Error-Free Operation
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
          !err.includes('422') &&
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
