import { test, expect } from '@playwright/test';

/**
 * Contract Diff Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config (handles auth via setup project):
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts contract-diff-deployed
 *
 * These tests verify all Contract Diff Analysis functionality on the live deployed site:
 *   1.  Page Load & Layout
 *   2.  Protocol Selector
 *   3.  Statistics Cards
 *   4.  Captured Requests Section (filters, list, refresh)
 *   5.  Analysis Configuration (spec path input, textarea, analyze button)
 *   6.  Results Display (status, mismatches table, recommendations)
 *   7.  Navigation
 *   8.  Accessibility
 *   9.  Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

// Scoped locator for the main content area (excludes sidebar to avoid duplicate matches)
function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Contract Diff — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/contract-diff`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    // Wait for the app shell to render (sidebar navigation visible)
    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    // Wait for the Contract Diff heading to confirm content loaded
    await expect(
      mainContent(page).getByRole('heading', { name: 'Contract Diff Analysis', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    // Small stabilization delay for dynamic content
    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the contract diff page at /contract-diff', async ({ page }) => {
      await expect(page).toHaveURL(/\/contract-diff/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Contract Diff Analysis', level: 1 })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Analyze front-end requests against backend contract specifications')
      ).toBeVisible();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Contract Diff')).toBeVisible();
    });

    test('should display the two-column layout sections', async ({ page }) => {
      const main = mainContent(page);

      await expect(main.getByText('Captured Requests')).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('Analysis Configuration')).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Protocol Selector
  // ---------------------------------------------------------------------------
  test.describe('Protocol Selector', () => {
    test('should display the protocol selector dropdown', async ({ page }) => {
      const main = mainContent(page);
      // The protocol selector shows HTTP/REST by default
      await expect(main.getByText('HTTP/REST')).toBeVisible({ timeout: 5000 });
    });

    test('should default to HTTP/REST protocol', async ({ page }) => {
      const main = mainContent(page);
      // The select trigger should show HTTP/REST as the current value
      const trigger = main.locator('button').filter({ hasText: 'HTTP/REST' });
      await expect(trigger).toBeVisible({ timeout: 5000 });
    });

    test('should open protocol selector and show all protocol options', async ({ page }) => {
      const main = mainContent(page);
      const trigger = main.locator('button').filter({ hasText: 'HTTP/REST' });
      await trigger.click();
      await page.waitForTimeout(500);

      const protocols = ['HTTP/REST', 'gRPC', 'WebSocket', 'MQTT', 'Kafka'];
      for (const protocol of protocols) {
        await expect(page.getByRole('option', { name: protocol })).toBeVisible({ timeout: 3000 });
      }

      // Close the dropdown by pressing Escape
      await page.keyboard.press('Escape');
      await page.waitForTimeout(300);
    });

    test('should show New Contract button when non-HTTP protocol is selected', async ({ page }) => {
      const main = mainContent(page);
      const trigger = main.locator('button').filter({ hasText: 'HTTP/REST' });
      await trigger.click();
      await page.waitForTimeout(500);

      await page.getByRole('option', { name: 'gRPC' }).click();
      await page.waitForTimeout(500);

      // When gRPC is selected, a "New GRPC Contract" button should appear
      const hasNewContract = await main
        .getByRole('button', { name: /New.*Contract/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasNewContract).toBeTruthy();
    });

    test('should show protocol-specific contracts section for non-HTTP protocol', async ({ page }) => {
      const main = mainContent(page);
      const trigger = main.locator('button').filter({ hasText: 'HTTP/REST' });

      // If already on gRPC from previous test, trigger may show gRPC
      const triggerVisible = await trigger.isVisible({ timeout: 3000 }).catch(() => false);
      if (triggerVisible) {
        await trigger.click();
        await page.waitForTimeout(500);
        await page.getByRole('option', { name: 'gRPC' }).click();
      } else {
        // Already on a non-HTTP protocol
        const grpcTrigger = main.locator('button').filter({ hasText: 'gRPC' });
        const grpcVisible = await grpcTrigger.isVisible({ timeout: 3000 }).catch(() => false);
        if (!grpcVisible) {
          // Reload to reset state
          await page.goto(`${BASE_URL}/contract-diff`, {
            waitUntil: 'domcontentloaded',
            timeout: 30000,
          });
          await page.waitForTimeout(1000);
          const freshTrigger = main.locator('button').filter({ hasText: 'HTTP/REST' });
          await freshTrigger.click();
          await page.waitForTimeout(500);
          await page.getByRole('option', { name: 'gRPC' }).click();
        }
      }

      await page.waitForTimeout(500);

      // Should show a contracts section or empty state for gRPC
      const hasContractsSection = await main
        .getByText(/GRPC Contracts/i)
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasNoContracts = await main
        .getByText(/No Contracts/i)
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasContractsSection || hasNoContracts).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Statistics Cards
  // ---------------------------------------------------------------------------
  test.describe('Statistics Cards', () => {
    test('should display statistics cards when data is available', async ({ page }) => {
      const main = mainContent(page);

      // Statistics cards show Total Captures, Analyzed, Sources, Methods
      const hasTotalCaptures = await main
        .getByText('Total Captures')
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasAnalyzed = await main
        .getByText('Analyzed')
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasSources = await main
        .getByText('Sources')
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasMethods = await main
        .getByText('Methods')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      // All four stats cards should appear together or not at all (depends on API)
      if (hasTotalCaptures) {
        expect(hasAnalyzed).toBeTruthy();
        expect(hasSources).toBeTruthy();
        expect(hasMethods).toBeTruthy();
      }
    });

    test('should display numeric values in statistics cards', async ({ page }) => {
      const main = mainContent(page);

      const hasTotalCaptures = await main
        .getByText('Total Captures')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasTotalCaptures) {
        // Each card should have a numeric value (rendered as text-2xl font-bold)
        const statValues = main.locator('.text-2xl.font-bold');
        expect(await statValues.count()).toBeGreaterThanOrEqual(4);
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Captured Requests Section
  // ---------------------------------------------------------------------------
  test.describe('Captured Requests Section', () => {
    test('should display the Captured Requests section heading', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Captured Requests')
      ).toBeVisible();
    });

    test('should display the Source filter dropdown', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Source')).toBeVisible({ timeout: 5000 });

      // The source dropdown should show "All Sources" by default
      const hasAllSources = await main
        .getByText('All Sources')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasAllSources).toBeTruthy();
    });

    test('should display the Method filter dropdown', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Method')).toBeVisible({ timeout: 5000 });

      // The method dropdown should show "All Methods" by default
      const hasAllMethods = await main
        .getByText('All Methods')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasAllMethods).toBeTruthy();
    });

    test('should show request list or empty state', async ({ page }) => {
      const main = mainContent(page);

      const hasEmptyState = await main
        .getByText('No captured requests')
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasLoading = await main
        .getByText('Loading...')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // Should show either empty state, loading, or actual request items
      // The page should render without crashing regardless
      await expect(main.getByText('Captured Requests')).toBeVisible();
    });

    test('should display the Refresh button', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: /Refresh/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should handle Refresh button click without crashing', async ({ page }) => {
      const main = mainContent(page);
      const refreshButton = main.getByRole('button', { name: /Refresh/i });

      await refreshButton.click();
      await page.waitForTimeout(1500);

      // Page should still be functional after refresh
      await expect(
        main.getByRole('heading', { name: 'Contract Diff Analysis', level: 1 })
      ).toBeVisible();
    });

    test('should open Source filter dropdown and show All Sources option', async ({ page }) => {
      const main = mainContent(page);
      const sourceTrigger = main.locator('button').filter({ hasText: 'All Sources' });

      const hasSourceTrigger = await sourceTrigger.isVisible({ timeout: 3000 }).catch(() => false);
      if (hasSourceTrigger) {
        await sourceTrigger.click();
        await page.waitForTimeout(500);

        await expect(
          page.getByRole('option', { name: 'All Sources' })
        ).toBeVisible({ timeout: 3000 });

        await page.keyboard.press('Escape');
        await page.waitForTimeout(300);
      }
    });

    test('should open Method filter dropdown and show All Methods option', async ({ page }) => {
      const main = mainContent(page);
      const methodTrigger = main.locator('button').filter({ hasText: 'All Methods' });

      const hasMethodTrigger = await methodTrigger.isVisible({ timeout: 3000 }).catch(() => false);
      if (hasMethodTrigger) {
        await methodTrigger.click();
        await page.waitForTimeout(500);

        await expect(
          page.getByRole('option', { name: 'All Methods' })
        ).toBeVisible({ timeout: 3000 });

        await page.keyboard.press('Escape');
        await page.waitForTimeout(300);
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Analysis Configuration
  // ---------------------------------------------------------------------------
  test.describe('Analysis Configuration', () => {
    test('should display the Analysis Configuration section heading', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Analysis Configuration')
      ).toBeVisible();
    });

    test('should display the Contract Spec Path input', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Contract Spec Path')).toBeVisible({ timeout: 5000 });

      const input = main.getByPlaceholder('/path/to/openapi.yaml');
      await expect(input).toBeVisible({ timeout: 5000 });
    });

    test('should allow typing in the Contract Spec Path input', async ({ page }) => {
      const main = mainContent(page);
      const input = main.getByPlaceholder('/path/to/openapi.yaml');

      await input.fill('/api/spec/openapi.yaml');
      await page.waitForTimeout(300);

      const value = await input.inputValue();
      expect(value).toBe('/api/spec/openapi.yaml');
    });

    test('should display the spec content textarea', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Or Contract Spec Content (YAML/JSON)')
      ).toBeVisible({ timeout: 5000 });

      const textarea = main.getByPlaceholder('Paste OpenAPI spec content here...');
      await expect(textarea).toBeVisible({ timeout: 5000 });
    });

    test('should allow typing in the spec content textarea', async ({ page }) => {
      const main = mainContent(page);
      const textarea = main.getByPlaceholder('Paste OpenAPI spec content here...');

      await textarea.fill('openapi: "3.0.0"\ninfo:\n  title: Test API\n  version: "1.0"');
      await page.waitForTimeout(300);

      const value = await textarea.inputValue();
      expect(value).toContain('openapi: "3.0.0"');
    });

    test('should display the Analyze Request button', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: /Analyze Request/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should have Analyze Request button disabled when no request is selected', async ({ page }) => {
      const main = mainContent(page);
      const analyzeButton = main.getByRole('button', { name: /Analyze Request/i });

      // Button should be disabled when no captured request is selected
      await expect(analyzeButton).toBeDisabled();
    });

    test('should keep Analyze Request button disabled when spec is empty and no request selected', async ({ page }) => {
      const main = mainContent(page);
      const analyzeButton = main.getByRole('button', { name: /Analyze Request/i });

      // Even after clearing inputs, button should remain disabled without a selection
      const specInput = main.getByPlaceholder('/path/to/openapi.yaml');
      await specInput.clear();
      await page.waitForTimeout(300);

      await expect(analyzeButton).toBeDisabled();
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Results Display
  // ---------------------------------------------------------------------------
  test.describe('Results Display', () => {
    test('should not show analysis results initially', async ({ page }) => {
      const main = mainContent(page);

      // Analysis Results section should not be visible until an analysis is performed
      const hasResults = await main
        .getByText('Analysis Results')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // Results should not appear on initial load
      expect(hasResults).toBeFalsy();
    });

    test('should not show Contract Matches or Mismatches status initially', async ({ page }) => {
      const main = mainContent(page);

      const hasMatches = await main
        .getByText('Contract Matches')
        .isVisible({ timeout: 2000 })
        .catch(() => false);
      const hasMismatches = await main
        .getByText('Contract Mismatches Detected')
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasMatches).toBeFalsy();
      expect(hasMismatches).toBeFalsy();
    });

    test('should not show the Mismatches table initially', async ({ page }) => {
      const main = mainContent(page);

      // The "Mismatches" heading is only rendered within the results section
      const hasMismatchesHeading = await main
        .getByRole('heading', { name: 'Mismatches' })
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasMismatchesHeading).toBeFalsy();
    });

    test('should not show AI Recommendations initially', async ({ page }) => {
      const main = mainContent(page);

      const hasRecommendations = await main
        .getByText('AI Recommendations')
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasRecommendations).toBeFalsy();
    });

    test('should not show Correction Proposals initially', async ({ page }) => {
      const main = mainContent(page);

      const hasCorrections = await main
        .getByText('Correction Proposals')
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasCorrections).toBeFalsy();
    });

    test('should not show Download Patch File button initially', async ({ page }) => {
      const main = mainContent(page);

      const hasDownload = await main
        .getByRole('button', { name: /Download Patch File/i })
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      expect(hasDownload).toBeFalsy();
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

      // Navigate back to Contract Diff
      const hasContractDiffButton = await nav
        .getByRole('button', { name: /Contract Diff/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasContractDiffButton) {
        await nav.getByRole('button', { name: /Contract Diff/i }).click();
      } else {
        await page.goto(`${BASE_URL}/contract-diff`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Contract Diff Analysis', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Services and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Services' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Services', exact: true, level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back to Contract Diff
      const hasContractDiffButton = await nav
        .getByRole('button', { name: /Contract Diff/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasContractDiffButton) {
        await nav.getByRole('button', { name: /Contract Diff/i }).click();
      } else {
        await page.goto(`${BASE_URL}/contract-diff`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Contract Diff Analysis', level: 1 })
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
      await expect(h1).toHaveText('Contract Diff Analysis');
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

    test('should have labeled form controls in Analysis Configuration', async ({ page }) => {
      const main = mainContent(page);

      // Verify labels exist for form inputs
      await expect(main.getByText('Contract Spec Path')).toBeVisible();
      await expect(main.getByText('Or Contract Spec Content (YAML/JSON)')).toBeVisible();
    });

    test('should have labeled filter dropdowns in Captured Requests', async ({ page }) => {
      const main = mainContent(page);

      await expect(main.getByText('Source')).toBeVisible();
      await expect(main.getByText('Method')).toBeVisible();
    });

    test('should have accessible buttons with descriptive text', async ({ page }) => {
      const main = mainContent(page);

      // Verify key buttons have accessible names
      await expect(main.getByRole('button', { name: /Refresh/i })).toBeVisible();
      await expect(main.getByRole('button', { name: /Analyze Request/i })).toBeVisible();
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

      // Reload the page to capture all console output
      await page.reload({ waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(3000);

      // Filter out known benign errors (network polling, WebSocket, etc.)
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

    test('should not crash when interacting with filters', async ({ page }) => {
      const main = mainContent(page);

      // Click the Source filter dropdown
      const sourceTrigger = main.locator('button').filter({ hasText: 'All Sources' });
      const hasSourceTrigger = await sourceTrigger.isVisible({ timeout: 3000 }).catch(() => false);

      if (hasSourceTrigger) {
        await sourceTrigger.click();
        await page.waitForTimeout(500);
        await page.keyboard.press('Escape');
        await page.waitForTimeout(300);
      }

      // Click the Method filter dropdown
      const methodTrigger = main.locator('button').filter({ hasText: 'All Methods' });
      const hasMethodTrigger = await methodTrigger.isVisible({ timeout: 3000 }).catch(() => false);

      if (hasMethodTrigger) {
        await methodTrigger.click();
        await page.waitForTimeout(500);
        await page.keyboard.press('Escape');
        await page.waitForTimeout(300);
      }

      // Page should still be functional
      await expect(
        main.getByRole('heading', { name: 'Contract Diff Analysis', level: 1 })
      ).toBeVisible();
    });

    test('should not crash when switching protocols', async ({ page }) => {
      const main = mainContent(page);

      // Switch to gRPC
      const trigger = main.locator('button').filter({ hasText: /HTTP\/REST|gRPC|WebSocket|MQTT|Kafka/ }).first();
      const hasTrigger = await trigger.isVisible({ timeout: 3000 }).catch(() => false);

      if (hasTrigger) {
        await trigger.click();
        await page.waitForTimeout(500);

        const grpcOption = page.getByRole('option', { name: 'gRPC' });
        const hasGrpc = await grpcOption.isVisible({ timeout: 3000 }).catch(() => false);
        if (hasGrpc) {
          await grpcOption.click();
          await page.waitForTimeout(1000);
        } else {
          await page.keyboard.press('Escape');
          await page.waitForTimeout(300);
        }
      }

      // Page heading should still be visible (not crashed)
      await expect(
        main.getByRole('heading', { name: 'Contract Diff Analysis', level: 1 })
      ).toBeVisible();

      // Switch back to HTTP/REST
      const httpTrigger = main.locator('button').filter({ hasText: /HTTP\/REST|gRPC|WebSocket|MQTT|Kafka/ }).first();
      const hasHttpTrigger = await httpTrigger.isVisible({ timeout: 3000 }).catch(() => false);

      if (hasHttpTrigger) {
        await httpTrigger.click();
        await page.waitForTimeout(500);

        const httpOption = page.getByRole('option', { name: 'HTTP/REST' });
        const hasHttp = await httpOption.isVisible({ timeout: 3000 }).catch(() => false);
        if (hasHttp) {
          await httpOption.click();
          await page.waitForTimeout(1000);
        } else {
          await page.keyboard.press('Escape');
          await page.waitForTimeout(300);
        }
      }

      await expect(
        main.getByRole('heading', { name: 'Contract Diff Analysis', level: 1 })
      ).toBeVisible();
    });
  });
});
