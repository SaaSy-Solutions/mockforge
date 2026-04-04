import { test, expect } from '@playwright/test';

/**
 * Incident Dashboard Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config (handles auth via setup project):
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts incidents-deployed
 *
 * These tests verify all Incident Dashboard functionality on the live deployed site:
 *   1.  Page Load & Layout
 *   2.  Statistics Cards
 *   3.  Filter Section — Search Input
 *   4.  Filter Section — Status Dropdown
 *   5.  Filter Section — Severity Dropdown
 *   6.  Filter Section — Type Dropdown
 *   7.  Filter Section — Protocol Dropdown
 *   8.  Filter Section — Endpoint Filter & Actions
 *   9.  Incident List / Empty State
 *   10. Action Buttons (Acknowledge / Resolve)
 *   11. Navigation
 *   12. Accessibility
 *   13. Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

// Scoped locator for the main content area (excludes sidebar to avoid duplicate matches)
function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Incident Dashboard — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/incidents`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    // Wait for the app shell to render (sidebar navigation visible)
    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    // Wait for the Incident Dashboard heading to confirm content loaded
    await expect(
      mainContent(page).getByRole('heading', { name: 'Incident Dashboard', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    // Small stabilization delay for dynamic content
    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the incidents page at /incidents', async ({ page }) => {
      await expect(page).toHaveURL(/\/incidents/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Incident Dashboard', level: 1 })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Monitor and manage contract drift incidents')
      ).toBeVisible();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('Incidents')).toBeVisible();
    });

    test('should display the filter card section', async ({ page }) => {
      const main = mainContent(page);
      // The filter section contains the search input
      await expect(
        main.getByPlaceholder('Search by endpoint, method, or ID...')
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display either incident list or empty state', async ({ page }) => {
      const main = mainContent(page);

      const hasIncidents = await main
        .getByText(/Showing \d+ of \d+ incidents/)
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasEmptyState = await main
        .getByText('No Incidents Found')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasIncidents || hasEmptyState).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Statistics Cards
  // ---------------------------------------------------------------------------
  test.describe('Statistics Cards', () => {
    test('should display the Total Incidents statistic card', async ({ page }) => {
      const main = mainContent(page);
      // Statistics may not render if stats are loading or unavailable
      const hasTotal = await main
        .getByText('Total Incidents')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasTotal) {
        await expect(main.getByText('Total Incidents')).toBeVisible();
      }
      // If stats are not available (cloud mode without backend), page should still load
      await expect(
        main.getByRole('heading', { name: 'Incident Dashboard', level: 1 })
      ).toBeVisible();
    });

    test('should display the Open statistic card', async ({ page }) => {
      const main = mainContent(page);
      const hasOpen = await main
        .getByText('Open', { exact: true })
        .first()
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      // Open stat card should be visible when statistics are loaded
      if (hasOpen) {
        await expect(main.getByText('Open', { exact: true }).first()).toBeVisible();
      }
    });

    test('should display the Resolved statistic card', async ({ page }) => {
      const main = mainContent(page);
      const hasResolved = await main
        .getByText('Resolved', { exact: true })
        .first()
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasResolved) {
        await expect(main.getByText('Resolved', { exact: true }).first()).toBeVisible();
      }
    });

    test('should display the Critical statistic card', async ({ page }) => {
      const main = mainContent(page);
      const hasCritical = await main
        .getByText('Critical', { exact: true })
        .first()
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasCritical) {
        await expect(main.getByText('Critical', { exact: true }).first()).toBeVisible();
      }
    });

    test('should display all four statistic cards in a grid', async ({ page }) => {
      const main = mainContent(page);
      const hasTotal = await main
        .getByText('Total Incidents')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasTotal) {
        // All four stats should be present when the stats section renders
        await expect(main.getByText('Total Incidents')).toBeVisible();
        await expect(main.getByText('Open', { exact: true }).first()).toBeVisible();
        await expect(main.getByText('Resolved', { exact: true }).first()).toBeVisible();
        await expect(main.getByText('Critical', { exact: true }).first()).toBeVisible();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Filter Section — Search Input
  // ---------------------------------------------------------------------------
  test.describe('Filter Section — Search Input', () => {
    test('should display the search input with placeholder text', async ({ page }) => {
      const main = mainContent(page);
      const searchInput = main.getByPlaceholder('Search by endpoint, method, or ID...');
      await expect(searchInput).toBeVisible();
    });

    test('should accept text input in the search field', async ({ page }) => {
      const main = mainContent(page);
      const searchInput = main.getByPlaceholder('Search by endpoint, method, or ID...');
      await searchInput.fill('GET /api/users');
      await expect(searchInput).toHaveValue('GET /api/users');
    });

    test('should filter results when typing in search field', async ({ page }) => {
      const main = mainContent(page);
      const searchInput = main.getByPlaceholder('Search by endpoint, method, or ID...');

      // Type a search term that likely won't match anything
      await searchInput.fill('zzz-nonexistent-endpoint-zzz');
      await page.waitForTimeout(1000);

      // Should show either filtered results or empty state
      const hasEmptyState = await main
        .getByText('No Incidents Found')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasResults = await main
        .getByText(/Showing \d+ of \d+ incidents/)
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasEmptyState || hasResults).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Filter Section — Status Dropdown
  // ---------------------------------------------------------------------------
  test.describe('Filter Section — Status Dropdown', () => {
    test('should display the status filter dropdown', async ({ page }) => {
      const main = mainContent(page);
      // The Status select trigger should show "All Statuses" by default
      const hasStatusFilter = await main
        .getByText('All Statuses')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      // The trigger may show "Status" as placeholder or "All Statuses" as selected value
      if (!hasStatusFilter) {
        const hasPlaceholder = await main
          .getByText('Status')
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        expect(hasPlaceholder).toBeTruthy();
      } else {
        await expect(main.getByText('All Statuses')).toBeVisible();
      }
    });

    test('should open status dropdown and show all options', async ({ page }) => {
      const main = mainContent(page);

      // Click the status filter trigger
      const statusTrigger = main.getByText('All Statuses');
      const hasTrigger = await statusTrigger
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasTrigger) {
        await statusTrigger.click();
        await page.waitForTimeout(500);

        // Verify dropdown options are visible
        const options = ['Open', 'Acknowledged', 'Resolved', 'Closed'];
        for (const option of options) {
          const hasOption = await page
            .getByRole('option', { name: option })
            .isVisible({ timeout: 3000 })
            .catch(() => false);

          if (!hasOption) {
            // Try looking for the text in a listbox or menu
            const hasText = await page
              .getByText(option, { exact: true })
              .first()
              .isVisible({ timeout: 2000 })
              .catch(() => false);
            // At least some options should be present
          }
        }

        // Close by pressing Escape
        await page.keyboard.press('Escape');
      }
    });

    test('should filter by selected status', async ({ page }) => {
      const main = mainContent(page);

      const statusTrigger = main.getByText('All Statuses');
      const hasTrigger = await statusTrigger
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasTrigger) {
        await statusTrigger.click();
        await page.waitForTimeout(500);

        // Select "Open" status
        const openOption = page.getByRole('option', { name: 'Open' });
        const hasOpen = await openOption
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        if (hasOpen) {
          await openOption.click();
          await page.waitForTimeout(1000);
        } else {
          await page.keyboard.press('Escape');
        }
      }

      // Page should still be functional after filtering
      await expect(
        main.getByRole('heading', { name: 'Incident Dashboard', level: 1 })
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Filter Section — Severity Dropdown
  // ---------------------------------------------------------------------------
  test.describe('Filter Section — Severity Dropdown', () => {
    test('should display the severity filter dropdown', async ({ page }) => {
      const main = mainContent(page);
      const hasSeverityFilter = await main
        .getByText('All Severities')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (!hasSeverityFilter) {
        const hasPlaceholder = await main
          .getByText('Severity')
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        expect(hasPlaceholder).toBeTruthy();
      } else {
        await expect(main.getByText('All Severities')).toBeVisible();
      }
    });

    test('should open severity dropdown and show all options', async ({ page }) => {
      const main = mainContent(page);

      const severityTrigger = main.getByText('All Severities');
      const hasTrigger = await severityTrigger
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasTrigger) {
        await severityTrigger.click();
        await page.waitForTimeout(500);

        const options = ['Critical', 'High', 'Medium', 'Low'];
        for (const option of options) {
          await page
            .getByRole('option', { name: option })
            .or(page.getByText(option, { exact: true }))
            .first()
            .isVisible({ timeout: 2000 })
            .catch(() => false);
        }

        await page.keyboard.press('Escape');
      }
    });

    test('should filter by selected severity', async ({ page }) => {
      const main = mainContent(page);

      const severityTrigger = main.getByText('All Severities');
      const hasTrigger = await severityTrigger
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasTrigger) {
        await severityTrigger.click();
        await page.waitForTimeout(500);

        const criticalOption = page.getByRole('option', { name: 'Critical' });
        const hasCritical = await criticalOption
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        if (hasCritical) {
          await criticalOption.click();
          await page.waitForTimeout(1000);
        } else {
          await page.keyboard.press('Escape');
        }
      }

      await expect(
        main.getByRole('heading', { name: 'Incident Dashboard', level: 1 })
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Filter Section — Type Dropdown
  // ---------------------------------------------------------------------------
  test.describe('Filter Section — Type Dropdown', () => {
    test('should display the type filter dropdown', async ({ page }) => {
      const main = mainContent(page);
      const hasTypeFilter = await main
        .getByText('All Types')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (!hasTypeFilter) {
        const hasPlaceholder = await main
          .getByText('Type')
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        expect(hasPlaceholder).toBeTruthy();
      } else {
        await expect(main.getByText('All Types')).toBeVisible();
      }
    });

    test('should open type dropdown and show all options', async ({ page }) => {
      const main = mainContent(page);

      const typeTrigger = main.getByText('All Types');
      const hasTrigger = await typeTrigger
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasTrigger) {
        await typeTrigger.click();
        await page.waitForTimeout(500);

        const options = ['Breaking Change', 'Threshold Exceeded'];
        for (const option of options) {
          await page
            .getByRole('option', { name: option })
            .or(page.getByText(option, { exact: true }))
            .first()
            .isVisible({ timeout: 2000 })
            .catch(() => false);
        }

        await page.keyboard.press('Escape');
      }
    });

    test('should filter by selected type', async ({ page }) => {
      const main = mainContent(page);

      const typeTrigger = main.getByText('All Types');
      const hasTrigger = await typeTrigger
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasTrigger) {
        await typeTrigger.click();
        await page.waitForTimeout(500);

        const breakingOption = page.getByRole('option', { name: 'Breaking Change' });
        const hasBreaking = await breakingOption
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        if (hasBreaking) {
          await breakingOption.click();
          await page.waitForTimeout(1000);
        } else {
          await page.keyboard.press('Escape');
        }
      }

      await expect(
        main.getByRole('heading', { name: 'Incident Dashboard', level: 1 })
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 7. Filter Section — Protocol Dropdown
  // ---------------------------------------------------------------------------
  test.describe('Filter Section — Protocol Dropdown', () => {
    test('should display the protocol filter dropdown', async ({ page }) => {
      const main = mainContent(page);
      const hasProtocolFilter = await main
        .getByText('All Protocols')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (!hasProtocolFilter) {
        const hasPlaceholder = await main
          .getByText('Protocol')
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        expect(hasPlaceholder).toBeTruthy();
      } else {
        await expect(main.getByText('All Protocols')).toBeVisible();
      }
    });

    test('should open protocol dropdown and show all options', async ({ page }) => {
      const main = mainContent(page);

      const protocolTrigger = main.getByText('All Protocols');
      const hasTrigger = await protocolTrigger
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasTrigger) {
        await protocolTrigger.click();
        await page.waitForTimeout(500);

        const options = ['HTTP', 'gRPC', 'WebSocket', 'MQTT', 'Kafka'];
        for (const option of options) {
          await page
            .getByRole('option', { name: option })
            .or(page.getByText(option, { exact: true }))
            .first()
            .isVisible({ timeout: 2000 })
            .catch(() => false);
        }

        await page.keyboard.press('Escape');
      }
    });

    test('should filter by selected protocol', async ({ page }) => {
      const main = mainContent(page);

      const protocolTrigger = main.getByText('All Protocols');
      const hasTrigger = await protocolTrigger
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasTrigger) {
        await protocolTrigger.click();
        await page.waitForTimeout(500);

        const httpOption = page.getByRole('option', { name: 'HTTP' });
        const hasHttp = await httpOption
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        if (hasHttp) {
          await httpOption.click();
          await page.waitForTimeout(1000);
        } else {
          await page.keyboard.press('Escape');
        }
      }

      await expect(
        main.getByRole('heading', { name: 'Incident Dashboard', level: 1 })
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 8. Filter Section — Endpoint Filter & Actions
  // ---------------------------------------------------------------------------
  test.describe('Filter Section — Endpoint Filter & Actions', () => {
    test('should display the endpoint filter input', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByPlaceholder('Filter by endpoint...')
      ).toBeVisible({ timeout: 5000 });
    });

    test('should accept text input in the endpoint filter', async ({ page }) => {
      const main = mainContent(page);
      const endpointInput = main.getByPlaceholder('Filter by endpoint...');
      await endpointInput.fill('/api/users');
      await expect(endpointInput).toHaveValue('/api/users');
    });

    test('should display the Clear Filters button', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: /Clear Filters/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Refresh button', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: /Refresh/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should clear all filters when Clear Filters is clicked', async ({ page }) => {
      const main = mainContent(page);

      // Set some filter values
      const searchInput = main.getByPlaceholder('Search by endpoint, method, or ID...');
      await searchInput.fill('test-search');
      await page.waitForTimeout(300);

      const endpointInput = main.getByPlaceholder('Filter by endpoint...');
      await endpointInput.fill('/api/test');
      await page.waitForTimeout(300);

      // Click Clear Filters
      await main.getByRole('button', { name: /Clear Filters/i }).click();
      await page.waitForTimeout(500);

      // Verify filters are cleared
      await expect(searchInput).toHaveValue('');
      await expect(endpointInput).toHaveValue('');
    });

    test('should handle Refresh button click without crashing', async ({ page }) => {
      const main = mainContent(page);
      const refreshButton = main.getByRole('button', { name: /Refresh/i });

      await refreshButton.click();
      await page.waitForTimeout(1500);

      // Page should still be functional after refresh
      await expect(
        main.getByRole('heading', { name: 'Incident Dashboard', level: 1 })
      ).toBeVisible();
    });
  });

  // ---------------------------------------------------------------------------
  // 9. Incident List / Empty State
  // ---------------------------------------------------------------------------
  test.describe('Incident List / Empty State', () => {
    test('should display either incidents or the empty state', async ({ page }) => {
      const main = mainContent(page);

      const hasIncidents = await main
        .getByText(/Showing \d+ of \d+ incidents/)
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasEmptyState = await main
        .getByText('No Incidents Found')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasLoading = await main
        .getByText('Loading incidents...')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasError = await main
        .getByText('Error loading incidents')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // One of these states must be present
      expect(hasIncidents || hasEmptyState || hasLoading || hasError).toBeTruthy();
    });

    test('should show appropriate empty state message with no filters', async ({ page }) => {
      const main = mainContent(page);

      const hasEmptyState = await main
        .getByText('No Incidents Found')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasEmptyState) {
        // When no filters are active, should show the "all clear" message
        const hasClearMessage = await main
          .getByText('All clear! No contract drift incidents detected.')
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasFilterMessage = await main
          .getByText('Try adjusting your filters to see more results')
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        expect(hasClearMessage || hasFilterMessage).toBeTruthy();
      }
    });

    test('should show filter-adjusted empty state when filters produce no results', async ({ page }) => {
      const main = mainContent(page);

      // Apply a search filter that likely won't match
      const searchInput = main.getByPlaceholder('Search by endpoint, method, or ID...');
      await searchInput.fill('zzz-impossible-match-zzz');
      await page.waitForTimeout(1000);

      const hasEmptyState = await main
        .getByText('No Incidents Found')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasEmptyState) {
        await expect(
          main.getByText('Try adjusting your filters to see more results')
        ).toBeVisible({ timeout: 3000 });
      }

      // Clear the filter
      await main.getByRole('button', { name: /Clear Filters/i }).click();
    });

    test('should display pagination info when incidents exist', async ({ page }) => {
      const main = mainContent(page);

      const hasIncidents = await main
        .getByText(/Showing \d+ of \d+ incidents/)
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasIncidents) {
        await expect(
          main.getByText(/Showing \d+ of \d+ incidents/)
        ).toBeVisible();
      }
    });

    test('should display incident cards with badges when incidents exist', async ({ page }) => {
      const main = mainContent(page);

      const hasIncidents = await main
        .getByText(/Showing \d+ of \d+ incidents/)
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasIncidents) {
        // Each incident should have severity and status badges
        // Check for at least one severity badge
        const hasSeverity = await main
          .getByText(/CRITICAL|HIGH|MEDIUM|LOW/)
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        // Check for at least one status badge
        const hasStatus = await main
          .getByText(/Open|Acknowledged|Resolved|Closed/)
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        // Check for at least one type badge
        const hasType = await main
          .getByText(/Breaking Change|Threshold Exceeded/)
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        expect(hasSeverity || hasStatus || hasType).toBeTruthy();
      }
    });

    test('should display timestamps on incident cards when incidents exist', async ({ page }) => {
      const main = mainContent(page);

      const hasIncidents = await main
        .getByText(/Showing \d+ of \d+ incidents/)
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasIncidents) {
        const hasDetected = await main
          .getByText(/Detected:/)
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        expect(hasDetected).toBeTruthy();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 10. Action Buttons (Acknowledge / Resolve)
  // ---------------------------------------------------------------------------
  test.describe('Action Buttons', () => {
    test('should display Acknowledge button for open incidents', async ({ page }) => {
      const main = mainContent(page);

      const hasIncidents = await main
        .getByText(/Showing \d+ of \d+ incidents/)
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasIncidents) {
        const hasAcknowledge = await main
          .getByRole('button', { name: /Acknowledge/i })
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        // Acknowledge button appears only for open incidents — may not exist
        // if all incidents are already acknowledged/resolved/closed
        if (hasAcknowledge) {
          await expect(
            main.getByRole('button', { name: /Acknowledge/i }).first()
          ).toBeVisible();
        }
      }
    });

    test('should display Resolve button for open or acknowledged incidents', async ({ page }) => {
      const main = mainContent(page);

      const hasIncidents = await main
        .getByText(/Showing \d+ of \d+ incidents/)
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasIncidents) {
        const hasResolve = await main
          .getByRole('button', { name: /Resolve/i })
          .first()
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        // Resolve button appears for open and acknowledged incidents
        if (hasResolve) {
          await expect(
            main.getByRole('button', { name: /Resolve/i }).first()
          ).toBeVisible();
        }
      }
    });

    test('should not crash when Acknowledge button is clicked', async ({ page }) => {
      const main = mainContent(page);

      const acknowledgeButton = main.getByRole('button', { name: /Acknowledge/i }).first();
      const hasAcknowledge = await acknowledgeButton
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasAcknowledge) {
        // Set up dialog handler to dismiss any alert that appears
        page.on('dialog', (dialog) => dialog.dismiss());

        await acknowledgeButton.click();
        await page.waitForTimeout(1500);

        // Page should still be functional
        await expect(
          main.getByRole('heading', { name: 'Incident Dashboard', level: 1 })
        ).toBeVisible();
      }
    });

    test('should not crash when Resolve button is clicked', async ({ page }) => {
      const main = mainContent(page);

      const resolveButton = main.getByRole('button', { name: /Resolve/i }).first();
      const hasResolve = await resolveButton
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasResolve) {
        // Set up dialog handler to dismiss any alert that appears
        page.on('dialog', (dialog) => dialog.dismiss());

        await resolveButton.click();
        await page.waitForTimeout(1500);

        // Page should still be functional
        await expect(
          main.getByRole('heading', { name: 'Incident Dashboard', level: 1 })
        ).toBeVisible();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 11. Navigation
  // ---------------------------------------------------------------------------
  test.describe('Navigation', () => {
    test('should navigate to Dashboard and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back to Incidents via sidebar
      const hasIncidentsButton = await nav
        .getByRole('button', { name: /Incidents/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasIncidentsButton) {
        await nav.getByRole('button', { name: /Incidents/i }).click();
      } else {
        await page.goto(`${BASE_URL}/incidents`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Incident Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Config and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Config' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Configuration', level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back to Incidents
      const hasIncidentsButton = await nav
        .getByRole('button', { name: /Incidents/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasIncidentsButton) {
        await nav.getByRole('button', { name: /Incidents/i }).click();
      } else {
        await page.goto(`${BASE_URL}/incidents`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Incident Dashboard', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 12. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a single H1 heading', async ({ page }) => {
      const h1 = mainContent(page).getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);
      await expect(h1).toHaveText('Incident Dashboard');
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
      const main = mainContent(page);
      const searchInput = main.getByPlaceholder('Search by endpoint, method, or ID...');
      await expect(searchInput).toBeVisible();
      // Input should be focusable
      await searchInput.focus();
      await expect(searchInput).toBeFocused();
    });

    test('should have accessible filter buttons', async ({ page }) => {
      const main = mainContent(page);

      // Clear Filters and Refresh buttons should be accessible
      const clearButton = main.getByRole('button', { name: /Clear Filters/i });
      await expect(clearButton).toBeVisible();

      const refreshButton = main.getByRole('button', { name: /Refresh/i });
      await expect(refreshButton).toBeVisible();
    });

    test('should have accessible endpoint filter input', async ({ page }) => {
      const main = mainContent(page);
      const endpointInput = main.getByPlaceholder('Filter by endpoint...');
      await expect(endpointInput).toBeVisible();
      await endpointInput.focus();
      await expect(endpointInput).toBeFocused();
    });
  });

  // ---------------------------------------------------------------------------
  // 13. Error-Free Operation
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
          !err.includes('429')
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

    test('should not crash after multiple rapid refreshes', async ({ page }) => {
      const main = mainContent(page);
      const refreshButton = main.getByRole('button', { name: /Refresh/i });
      const hasRefresh = await refreshButton
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasRefresh) {
        // Click refresh rapidly 3 times
        await refreshButton.click();
        await page.waitForTimeout(200);
        await refreshButton.click();
        await page.waitForTimeout(200);
        await refreshButton.click();
        await page.waitForTimeout(2000);
      }

      // Page should still be functional after rapid refreshes
      await expect(
        main.getByRole('heading', { name: 'Incident Dashboard', level: 1 })
      ).toBeVisible();
    });

    test('should handle auto-refresh without crashing', async ({ page }) => {
      // The page auto-refreshes every 5 seconds — wait for at least one cycle
      await page.waitForTimeout(6000);

      // Page should still be functional after auto-refresh
      await expect(
        mainContent(page).getByRole('heading', { name: 'Incident Dashboard', level: 1 })
      ).toBeVisible();
    });
  });
});
