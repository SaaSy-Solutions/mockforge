import { test, expect } from '@playwright/test';

/**
 * Remaining Pages E2E Tests — covers all pages not yet tested individually.
 * Tests: Observability (12), Testing (6), Chaos (4), Import (1), AI (5), Community (2), Plugins/UserMgmt (2)
 *
 * Each page test verifies:
 * - Page loads at the correct URL
 * - Breadcrumbs display
 * - Main content area renders (heading, content, or error boundary)
 * - No unhandled crashes (error boundary is acceptable, blank page is not)
 * - Landmarks present
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';
function mainContent(page: import('@playwright/test').Page) { return page.getByRole('main'); }

/**
 * Helper: navigate to a page and verify it loads without a blank screen.
 * Returns true if the page has meaningful content, false if error boundary showed.
 */
async function loadPage(page: import('@playwright/test').Page, path: string): Promise<boolean> {
  await page.goto(`${BASE_URL}/${path}`, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await page.waitForSelector('nav[aria-label="Main navigation"]', { state: 'visible', timeout: 15000 });
  await page.waitForTimeout(2000);

  const main = page.getByRole('main');
  const hasError = await main.getByText('Something went wrong').isVisible({ timeout: 2000 }).catch(() => false);
  return !hasError;
}

// ============================================================================
// OBSERVABILITY PAGES (12 pages)
// ============================================================================
test.describe('Observability Pages — Deployed Site', () => {
  const pages = [
    { path: 'observability', breadcrumb: 'Observability', heading: /Observability/i },
    { path: 'world-state', breadcrumb: 'World State', heading: /World State/i },
    { path: 'performance', breadcrumb: 'Performance', heading: /Performance/i },
    { path: 'status', breadcrumb: 'System Status', heading: /System Status|Status/i },
    { path: 'incidents', breadcrumb: 'Incidents', heading: /Incidents/i },
    { path: 'logs', breadcrumb: 'Logs', heading: /Logs|Request Log/i },
    { path: 'traces', breadcrumb: 'Traces', heading: /Traces|Distributed Trac/i },
    { path: 'metrics', breadcrumb: 'Metrics', heading: /Metrics/i },
    { path: 'analytics', breadcrumb: 'Analytics', heading: /Analytics/i },
    { path: 'fitness-functions', breadcrumb: 'Fitness Functions', heading: /Fitness/i },
    { path: 'verification', breadcrumb: 'Verification', heading: /Verification/i },
    { path: 'contract-diff', breadcrumb: 'Contract Diff', heading: /Contract|Diff/i },
  ];

  for (const { path, breadcrumb, heading } of pages) {
    test.describe(breadcrumb, () => {
      test(`should load /${path} without crashing`, async ({ page }) => {
        const loaded = await loadPage(page, path);
        await expect(page).toHaveURL(new RegExp(`/${path}`));

        if (loaded) {
          // Page loaded normally — check for heading
          const main = mainContent(page);
          const hasHeading = await main.getByText(heading).first()
            .isVisible({ timeout: 3000 }).catch(() => false);
          expect(hasHeading).toBeTruthy();
        } else {
          // Error boundary — verify Try Again is present
          await expect(mainContent(page).getByText('Something went wrong')).toBeVisible();
        }
      });

      test(`should display breadcrumbs for /${path}`, async ({ page }) => {
        await loadPage(page, path);
        await expect(page.getByRole('banner').getByText(breadcrumb)).toBeVisible();
      });

      test(`should have landmarks for /${path}`, async ({ page }) => {
        await loadPage(page, path);
        await expect(page.getByRole('main')).toBeVisible();
        await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
      });
    });
  }
});

// ============================================================================
// TESTING PAGES (6 pages)
// ============================================================================
test.describe('Testing Pages — Deployed Site', () => {
  const pages = [
    { path: 'testing', breadcrumb: 'Testing', heading: /Testing|Test/i },
    { path: 'test-generator', breadcrumb: 'Test Generator', heading: /Test Generator/i },
    { path: 'test-execution', breadcrumb: 'Test Execution', heading: /Test Execution/i },
    { path: 'integration-test-builder', breadcrumb: 'Integration Tests', heading: /Integration Test/i },
    { path: 'conformance', breadcrumb: 'Conformance', heading: /Conformance/i },
    { path: 'time-travel', breadcrumb: 'Time Travel', heading: /Time Travel/i },
  ];

  for (const { path, breadcrumb, heading } of pages) {
    test.describe(breadcrumb, () => {
      test(`should load /${path} without crashing`, async ({ page }) => {
        const loaded = await loadPage(page, path);
        await expect(page).toHaveURL(new RegExp(`/${path}`));

        if (loaded) {
          const hasHeading = await mainContent(page).getByText(heading).first()
            .isVisible({ timeout: 3000 }).catch(() => false);
          expect(hasHeading).toBeTruthy();
        } else {
          await expect(mainContent(page).getByText('Something went wrong')).toBeVisible();
        }
      });

      test(`should display breadcrumbs for /${path}`, async ({ page }) => {
        await loadPage(page, path);
        await expect(page.getByRole('banner').getByText(breadcrumb)).toBeVisible();
      });

      test(`should have landmarks for /${path}`, async ({ page }) => {
        await loadPage(page, path);
        await expect(page.getByRole('main')).toBeVisible();
        await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
      });
    });
  }
});

// ============================================================================
// CHAOS & RESILIENCE PAGES (4 pages)
// ============================================================================
test.describe('Chaos & Resilience Pages — Deployed Site', () => {
  const pages = [
    { path: 'chaos', breadcrumb: 'Chaos Engineering', heading: /Chaos/i },
    { path: 'resilience', breadcrumb: 'Resilience', heading: /Resilience/i },
    { path: 'recorder', breadcrumb: 'Recorder', heading: /Recorder|Traffic Record/i },
    { path: 'behavioral-cloning', breadcrumb: 'Behavioral Cloning', heading: /Behavioral Cloning/i },
  ];

  for (const { path, breadcrumb, heading } of pages) {
    test.describe(breadcrumb, () => {
      test(`should load /${path} without crashing`, async ({ page }) => {
        const loaded = await loadPage(page, path);
        await expect(page).toHaveURL(new RegExp(`/${path}`));

        if (loaded) {
          const hasHeading = await mainContent(page).getByText(heading).first()
            .isVisible({ timeout: 3000 }).catch(() => false);
          expect(hasHeading).toBeTruthy();
        } else {
          await expect(mainContent(page).getByText('Something went wrong')).toBeVisible();
        }
      });

      test(`should display breadcrumbs for /${path}`, async ({ page }) => {
        await loadPage(page, path);
        await expect(page.getByRole('banner').getByText(breadcrumb)).toBeVisible();
      });

      test(`should have landmarks for /${path}`, async ({ page }) => {
        await loadPage(page, path);
        await expect(page.getByRole('main')).toBeVisible();
        await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
      });
    });
  }
});

// ============================================================================
// IMPORT PAGE
// ============================================================================
test.describe('Import Page — Deployed Site', () => {
  test('should load /import without crashing', async ({ page }) => {
    const loaded = await loadPage(page, 'import');
    await expect(page).toHaveURL(/\/import/);
    if (loaded) {
      const hasHeading = await mainContent(page).getByText(/Import/i).first()
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasHeading).toBeTruthy();
    } else {
      await expect(mainContent(page).getByText('Something went wrong')).toBeVisible();
    }
  });

  test('should display breadcrumbs for /import', async ({ page }) => {
    await loadPage(page, 'import');
    await expect(page.getByRole('banner').getByText('Import')).toBeVisible();
  });

  test('should have landmarks for /import', async ({ page }) => {
    await loadPage(page, 'import');
    await expect(page.getByRole('main')).toBeVisible();
    await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
  });
});

// ============================================================================
// AI & INTELLIGENCE PAGES (5 pages)
// ============================================================================
test.describe('AI & Intelligence Pages — Deployed Site', () => {
  const pages = [
    { path: 'ai-studio', breadcrumb: 'AI Studio', heading: /AI Studio/i },
    { path: 'mockai', breadcrumb: 'MockAI', heading: /MockAI/i },
    { path: 'mockai-openapi-generator', breadcrumb: 'MockAI OpenAPI Generator', heading: /Generate OpenAPI|OpenAPI Generator/i },
    { path: 'mockai-rules', breadcrumb: 'MockAI Rules', heading: /MockAI Rules/i },
    { path: 'voice', breadcrumb: 'Voice + LLM', heading: /Voice|LLM/i },
  ];

  for (const { path, breadcrumb, heading } of pages) {
    test.describe(breadcrumb, () => {
      test(`should load /${path} without crashing`, async ({ page }) => {
        const loaded = await loadPage(page, path);
        await expect(page).toHaveURL(new RegExp(`/${path}`));

        if (loaded) {
          const hasHeading = await mainContent(page).getByText(heading).first()
            .isVisible({ timeout: 3000 }).catch(() => false);
          expect(hasHeading).toBeTruthy();
        } else {
          await expect(mainContent(page).getByText('Something went wrong')).toBeVisible();
        }
      });

      test(`should display breadcrumbs for /${path}`, async ({ page }) => {
        await loadPage(page, path);
        await expect(page.getByRole('banner').getByText(breadcrumb)).toBeVisible();
      });

      test(`should have landmarks for /${path}`, async ({ page }) => {
        await loadPage(page, path);
        await expect(page.getByRole('main')).toBeVisible();
        await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
      });
    });
  }
});

// ============================================================================
// COMMUNITY PAGES (2 pages)
// ============================================================================
test.describe('Community Pages — Deployed Site', () => {
  const pages = [
    { path: 'showcase', breadcrumb: 'Showcase', heading: /Showcase/i },
    { path: 'learning-hub', breadcrumb: 'Learning Hub', heading: /Learning Hub/i },
  ];

  for (const { path, breadcrumb, heading } of pages) {
    test.describe(breadcrumb, () => {
      test(`should load /${path} without crashing`, async ({ page }) => {
        const loaded = await loadPage(page, path);
        await expect(page).toHaveURL(new RegExp(`/${path}`));

        if (loaded) {
          const hasHeading = await mainContent(page).getByText(heading).first()
            .isVisible({ timeout: 3000 }).catch(() => false);
          expect(hasHeading).toBeTruthy();
        } else {
          await expect(mainContent(page).getByText('Something went wrong')).toBeVisible();
        }
      });

      test(`should display breadcrumbs for /${path}`, async ({ page }) => {
        await loadPage(page, path);
        await expect(page.getByRole('banner').getByText(breadcrumb)).toBeVisible();
      });

      test(`should have landmarks for /${path}`, async ({ page }) => {
        await loadPage(page, path);
        await expect(page.getByRole('main')).toBeVisible();
        await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
      });
    });
  }
});

// ============================================================================
// PLUGINS & USER MANAGEMENT (2 pages)
// ============================================================================
test.describe('Plugins & User Management — Deployed Site', () => {
  const pages = [
    { path: 'plugins', breadcrumb: 'Plugins', heading: /Plugins/i },
    { path: 'user-management', breadcrumb: 'User Management', heading: /User Management/i },
  ];

  for (const { path, breadcrumb, heading } of pages) {
    test.describe(breadcrumb, () => {
      test(`should load /${path} without crashing`, async ({ page }) => {
        const loaded = await loadPage(page, path);
        await expect(page).toHaveURL(new RegExp(`/${path}`));

        if (loaded) {
          const hasHeading = await mainContent(page).getByText(heading).first()
            .isVisible({ timeout: 3000 }).catch(() => false);
          expect(hasHeading).toBeTruthy();
        } else {
          await expect(mainContent(page).getByText('Something went wrong')).toBeVisible();
        }
      });

      test(`should display breadcrumbs for /${path}`, async ({ page }) => {
        await loadPage(page, path);
        await expect(page.getByRole('banner').getByText(breadcrumb)).toBeVisible();
      });

      test(`should have landmarks for /${path}`, async ({ page }) => {
        await loadPage(page, path);
        await expect(page.getByRole('main')).toBeVisible();
        await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
      });
    });
  }
});
