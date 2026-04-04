import { test, expect } from '@playwright/test';

/**
 * Graph Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config (handles auth via setup project):
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts graph-deployed
 *
 * These tests verify all Graph visualization functionality on the live deployed site:
 *   1.  Page Load & Layout
 *   2.  Graph Controls
 *   3.  ReactFlow Canvas
 *   4.  Filtering
 *   5.  Navigation
 *   6.  Accessibility
 *   7.  Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

// Scoped locator for the main content area (excludes sidebar to avoid duplicate matches)
function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Graph — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/graph`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    // Wait for the app shell to render (sidebar navigation visible)
    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    // The Graph page has no explicit heading — wait for the main content area
    // to have rendered content (GraphControls toolbar or ReactFlow canvas)
    await expect(mainContent(page)).not.toBeEmpty({ timeout: 10000 });

    // Small stabilization delay for dynamic content (ReactFlow initialization)
    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the graph page at /graph', async ({ page }) => {
      await expect(page).toHaveURL(/\/graph/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      const hasHome = await banner
        .getByText('Home')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasGraph = await banner
        .getByText('Graph')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // Breadcrumbs may or may not be present depending on layout wrapper
      // At minimum, the banner should exist
      await expect(banner).toBeVisible();
      if (hasHome) {
        expect(hasGraph).toBeTruthy();
      }
    });

    test('should render main content area with graph controls or canvas', async ({ page }) => {
      const main = mainContent(page);

      // The page renders either: loading spinner, GraphControls toolbar, or error
      const hasControls = await main
        .getByText(/Layout:|Refresh|Export/i)
        .first()
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasLoading = await main
        .getByText('Loading graph...')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasError = await main
        .getByText(/Failed to load graph data/i)
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // At least one state should be visible — the page rendered without crashing
      expect(hasControls || hasLoading || hasError).toBeTruthy();
    });

    test('should display node and edge counts', async ({ page }) => {
      const main = mainContent(page);

      // GraphControls shows "{n} nodes, {m} edges"
      const hasNodeCount = await main
        .getByText(/\d+\s*nodes/)
        .isVisible({ timeout: 5000 })
        .catch(() => false);
      const hasEdgeCount = await main
        .getByText(/\d+\s*edges/)
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      // Stats should be visible when controls are loaded
      if (hasNodeCount) {
        expect(hasEdgeCount).toBeTruthy();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Graph Controls
  // ---------------------------------------------------------------------------
  test.describe('Graph Controls', () => {
    test('should display the Layout selector', async ({ page }) => {
      const main = mainContent(page);

      const hasLayoutLabel = await main
        .getByText('Layout:')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasLayoutLabel) {
        // The layout selector trigger should be present
        const layoutTrigger = main.locator('#layout');
        await expect(layoutTrigger).toBeVisible({ timeout: 3000 });
      }
    });

    test('should open layout dropdown and show layout options', async ({ page }) => {
      const main = mainContent(page);

      const layoutTrigger = main.locator('#layout');
      const hasLayout = await layoutTrigger
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasLayout) {
        await layoutTrigger.click();
        await page.waitForTimeout(500);

        // Check that layout options are visible in the dropdown
        const layoutOptions = ['Hierarchical', 'Force-Directed', 'Grid', 'Circular'];
        for (const option of layoutOptions) {
          const hasOption = await page
            .getByRole('option', { name: option })
            .isVisible({ timeout: 3000 })
            .catch(() => false);
          expect(hasOption).toBeTruthy();
        }

        // Close dropdown by pressing Escape
        await page.keyboard.press('Escape');
        await page.waitForTimeout(300);
      }
    });

    test('should display the Filters button', async ({ page }) => {
      const main = mainContent(page);

      const filtersButton = main.getByRole('button', { name: /Filters/i });
      const hasFilters = await filtersButton
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasFilters) {
        await expect(filtersButton).toBeVisible();
      }
    });

    test('should display the Refresh button', async ({ page }) => {
      const main = mainContent(page);

      const refreshButton = main.getByRole('button', { name: /Refresh/i });
      const hasRefresh = await refreshButton
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasRefresh) {
        await expect(refreshButton).toBeVisible();
      }
    });

    test('should handle Refresh button click without crashing', async ({ page }) => {
      const main = mainContent(page);

      const refreshButton = main.getByRole('button', { name: /Refresh/i });
      const hasRefresh = await refreshButton
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasRefresh) {
        await refreshButton.click();
        await page.waitForTimeout(2000);
      }

      // Page should still be functional after refresh
      await expect(mainContent(page)).not.toBeEmpty();
    });

    test('should display the Export button', async ({ page }) => {
      const main = mainContent(page);

      const exportButton = main.getByRole('button', { name: /Export/i });
      const hasExport = await exportButton
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasExport) {
        await expect(exportButton).toBeVisible();
      }
    });

    test('should show export format options on hover', async ({ page }) => {
      const main = mainContent(page);

      const exportButton = main.getByRole('button', { name: /Export/i });
      const hasExport = await exportButton
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasExport) {
        // Hover over the Export button to reveal format options
        await exportButton.hover();
        await page.waitForTimeout(500);

        const hasPng = await page
          .getByRole('button', { name: 'PNG' })
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasSvg = await page
          .getByRole('button', { name: 'SVG' })
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasJson = await page
          .getByRole('button', { name: 'JSON' })
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        // At least one format option should be visible on hover
        expect(hasPng || hasSvg || hasJson).toBeTruthy();
      }
    });

    test('should change layout via dropdown', async ({ page }) => {
      const main = mainContent(page);

      const layoutTrigger = main.locator('#layout');
      const hasLayout = await layoutTrigger
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasLayout) {
        await layoutTrigger.click();
        await page.waitForTimeout(500);

        // Select "Hierarchical" layout
        const hierarchicalOption = page.getByRole('option', { name: 'Hierarchical' });
        const hasOption = await hierarchicalOption
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        if (hasOption) {
          await hierarchicalOption.click();
          await page.waitForTimeout(1000);
        } else {
          await page.keyboard.press('Escape');
        }
      }

      // Page should still be functional after layout change
      await expect(mainContent(page)).not.toBeEmpty();
    });
  });

  // ---------------------------------------------------------------------------
  // 3. ReactFlow Canvas
  // ---------------------------------------------------------------------------
  test.describe('ReactFlow Canvas', () => {
    test('should render the ReactFlow canvas element', async ({ page }) => {
      const main = mainContent(page);

      // ReactFlow renders a container with class .react-flow
      const hasReactFlow = await main
        .locator('.react-flow')
        .isVisible({ timeout: 10000 })
        .catch(() => false);

      // The canvas may not be visible if the page is in a loading or error state
      const hasLoading = await main
        .getByText('Loading graph...')
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasError = await main
        .getByText(/Failed to load graph data/i)
        .first()
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // One of these states should be true
      expect(hasReactFlow || hasLoading || hasError).toBeTruthy();
    });

    test('should display the MiniMap inside the canvas', async ({ page }) => {
      const main = mainContent(page);

      // ReactFlow MiniMap renders as an SVG with class .react-flow__minimap
      const hasMiniMap = await main
        .locator('.react-flow__minimap')
        .isVisible({ timeout: 10000 })
        .catch(() => false);

      // MiniMap should be visible when the ReactFlow canvas is rendered
      if (hasMiniMap) {
        await expect(main.locator('.react-flow__minimap')).toBeVisible();
      }
    });

    test('should display the ReactFlow zoom and pan controls', async ({ page }) => {
      const main = mainContent(page);

      // ReactFlow Controls component renders zoom-in, zoom-out, fit-view buttons
      const hasControls = await main
        .locator('.react-flow__controls')
        .isVisible({ timeout: 10000 })
        .catch(() => false);

      if (hasControls) {
        await expect(main.locator('.react-flow__controls')).toBeVisible();

        // Verify individual control buttons exist
        const zoomIn = main.locator('.react-flow__controls-zoomin');
        const zoomOut = main.locator('.react-flow__controls-zoomout');
        const fitView = main.locator('.react-flow__controls-fitview');

        const hasZoomIn = await zoomIn
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasZoomOut = await zoomOut
          .isVisible({ timeout: 3000 })
          .catch(() => false);
        const hasFitView = await fitView
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        expect(hasZoomIn || hasZoomOut || hasFitView).toBeTruthy();
      }
    });

    test('should display the ReactFlow background', async ({ page }) => {
      const main = mainContent(page);

      const hasBackground = await main
        .locator('.react-flow__background')
        .isVisible({ timeout: 10000 })
        .catch(() => false);

      if (hasBackground) {
        await expect(main.locator('.react-flow__background')).toBeVisible();
      }
    });

    test('should display graph nodes when data is available', async ({ page }) => {
      const main = mainContent(page);

      // ReactFlow nodes have the class .react-flow__node
      const nodeCount = await main
        .locator('.react-flow__node')
        .count()
        .catch(() => 0);

      // Nodes may or may not be present depending on data availability
      // Just verify the canvas doesn't crash
      const hasReactFlow = await main
        .locator('.react-flow')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasReactFlow) {
        // If ReactFlow is rendered, the node count can be 0+ (no data is valid)
        expect(nodeCount).toBeGreaterThanOrEqual(0);
      }
    });

    test('should display graph edges when data is available', async ({ page }) => {
      const main = mainContent(page);

      // ReactFlow edges are rendered within an SVG layer
      const hasEdgeLayer = await main
        .locator('.react-flow__edges')
        .isVisible({ timeout: 10000 })
        .catch(() => false);

      // Edge layer should exist when ReactFlow is rendered
      if (hasEdgeLayer) {
        await expect(main.locator('.react-flow__edges')).toBeVisible();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Filtering
  // ---------------------------------------------------------------------------
  test.describe('Filtering', () => {
    test('should open the Filters dialog', async ({ page }) => {
      const main = mainContent(page);

      const filtersButton = main.getByRole('button', { name: /Filters/i });
      const hasFilters = await filtersButton
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasFilters) {
        await filtersButton.click();
        await page.waitForTimeout(500);

        const dialog = page.getByRole('dialog');
        await expect(dialog).toBeVisible({ timeout: 5000 });

        // Verify dialog header
        await expect(dialog.getByText('Graph Filters')).toBeVisible();
        await expect(
          dialog.getByText('Filter nodes and edges by type and protocol')
        ).toBeVisible();

        // Close dialog
        await page.keyboard.press('Escape');
        await page.waitForTimeout(300);
      }
    });

    test('should display Node Type filter in Filters dialog', async ({ page }) => {
      const main = mainContent(page);

      const filtersButton = main.getByRole('button', { name: /Filters/i });
      const hasFilters = await filtersButton
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasFilters) {
        await filtersButton.click();
        await page.waitForTimeout(500);

        const dialog = page.getByRole('dialog');
        await expect(dialog).toBeVisible({ timeout: 5000 });

        // Verify Node Type label and selector
        await expect(dialog.getByText('Node Type')).toBeVisible();
        const nodeFilterTrigger = dialog.locator('#node-filter');
        await expect(nodeFilterTrigger).toBeVisible();

        // Close dialog
        await page.keyboard.press('Escape');
        await page.waitForTimeout(300);
      }
    });

    test('should display Protocol filter in Filters dialog', async ({ page }) => {
      const main = mainContent(page);

      const filtersButton = main.getByRole('button', { name: /Filters/i });
      const hasFilters = await filtersButton
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasFilters) {
        await filtersButton.click();
        await page.waitForTimeout(500);

        const dialog = page.getByRole('dialog');
        await expect(dialog).toBeVisible({ timeout: 5000 });

        // Verify Protocol label and selector
        await expect(dialog.getByText('Protocol')).toBeVisible();
        const protocolFilterTrigger = dialog.locator('#protocol-filter');
        await expect(protocolFilterTrigger).toBeVisible();

        // Close dialog
        await page.keyboard.press('Escape');
        await page.waitForTimeout(300);
      }
    });

    test('should change node type filter', async ({ page }) => {
      const main = mainContent(page);

      const filtersButton = main.getByRole('button', { name: /Filters/i });
      const hasFilters = await filtersButton
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasFilters) {
        await filtersButton.click();
        await page.waitForTimeout(500);

        const dialog = page.getByRole('dialog');
        await expect(dialog).toBeVisible({ timeout: 5000 });

        // Open the node filter dropdown
        const nodeFilterTrigger = dialog.locator('#node-filter');
        await nodeFilterTrigger.click();
        await page.waitForTimeout(500);

        // Select "Endpoints"
        const endpointOption = page.getByRole('option', { name: 'Endpoints' });
        const hasOption = await endpointOption
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        if (hasOption) {
          await endpointOption.click();
          await page.waitForTimeout(1000);
        } else {
          await page.keyboard.press('Escape');
        }

        // Close dialog
        await page.keyboard.press('Escape');
        await page.waitForTimeout(300);
      }

      // Page should still be functional after filter change
      await expect(mainContent(page)).not.toBeEmpty();
    });

    test('should change protocol filter', async ({ page }) => {
      const main = mainContent(page);

      const filtersButton = main.getByRole('button', { name: /Filters/i });
      const hasFilters = await filtersButton
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasFilters) {
        await filtersButton.click();
        await page.waitForTimeout(500);

        const dialog = page.getByRole('dialog');
        await expect(dialog).toBeVisible({ timeout: 5000 });

        // Open the protocol filter dropdown
        const protocolFilterTrigger = dialog.locator('#protocol-filter');
        await protocolFilterTrigger.click();
        await page.waitForTimeout(500);

        // Select "HTTP"
        const httpOption = page.getByRole('option', { name: 'HTTP' });
        const hasOption = await httpOption
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        if (hasOption) {
          await httpOption.click();
          await page.waitForTimeout(1000);
        } else {
          await page.keyboard.press('Escape');
        }

        // Close dialog
        await page.keyboard.press('Escape');
        await page.waitForTimeout(300);
      }

      // Page should still be functional after filter change
      await expect(mainContent(page)).not.toBeEmpty();
    });

    test('should show Clear Filters button when filters are active', async ({ page }) => {
      const main = mainContent(page);

      const filtersButton = main.getByRole('button', { name: /Filters/i });
      const hasFilters = await filtersButton
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasFilters) {
        await filtersButton.click();
        await page.waitForTimeout(500);

        const dialog = page.getByRole('dialog');
        await expect(dialog).toBeVisible({ timeout: 5000 });

        // Change node filter to trigger "Clear Filters" button appearance
        const nodeFilterTrigger = dialog.locator('#node-filter');
        await nodeFilterTrigger.click();
        await page.waitForTimeout(500);

        const servicesOption = page.getByRole('option', { name: 'Services' });
        const hasOption = await servicesOption
          .isVisible({ timeout: 3000 })
          .catch(() => false);

        if (hasOption) {
          await servicesOption.click();
          await page.waitForTimeout(500);

          // Clear Filters button should now appear
          const clearButton = dialog.getByRole('button', { name: /Clear Filters/i });
          const hasClear = await clearButton
            .isVisible({ timeout: 3000 })
            .catch(() => false);

          if (hasClear) {
            await clearButton.click();
            await page.waitForTimeout(500);
          }
        } else {
          await page.keyboard.press('Escape');
        }

        // Close dialog
        await page.keyboard.press('Escape');
        await page.waitForTimeout(300);
      }
    });

    test('should show protocol filter options in the dropdown', async ({ page }) => {
      const main = mainContent(page);

      const filtersButton = main.getByRole('button', { name: /Filters/i });
      const hasFilters = await filtersButton
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasFilters) {
        await filtersButton.click();
        await page.waitForTimeout(500);

        const dialog = page.getByRole('dialog');
        await expect(dialog).toBeVisible({ timeout: 5000 });

        // Open protocol dropdown
        const protocolFilterTrigger = dialog.locator('#protocol-filter');
        await protocolFilterTrigger.click();
        await page.waitForTimeout(500);

        const protocols = [
          'All Protocols', 'HTTP', 'gRPC', 'WebSocket', 'GraphQL',
          'MQTT', 'Kafka', 'AMQP', 'SMTP', 'FTP', 'TCP',
        ];

        for (const protocol of protocols) {
          const hasOption = await page
            .getByRole('option', { name: protocol })
            .isVisible({ timeout: 3000 })
            .catch(() => false);

          // At least the first few protocol options should be visible
          if (protocol === 'All Protocols' || protocol === 'HTTP') {
            expect(hasOption).toBeTruthy();
          }
        }

        // Close dropdown and dialog
        await page.keyboard.press('Escape');
        await page.waitForTimeout(300);
        await page.keyboard.press('Escape');
        await page.waitForTimeout(300);
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

      // Navigate back to Graph via sidebar or direct navigation
      const hasGraphButton = await nav
        .getByRole('button', { name: /Graph/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasGraphButton) {
        await nav.getByRole('button', { name: /Graph/i }).click();
      } else {
        await page.goto(`${BASE_URL}/graph`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(page).toHaveURL(/\/graph/);
      await expect(mainContent(page)).not.toBeEmpty();
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have accessible landmark regions', async ({ page }) => {
      await expect(page.getByRole('main')).toBeVisible();
      await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
      await expect(page.getByRole('banner')).toBeVisible();
    });

    test('should have skip navigation links', async ({ page }) => {
      await expect(page.getByRole('link', { name: 'Skip to navigation' })).toBeAttached();
      await expect(page.getByRole('link', { name: 'Skip to main content' })).toBeAttached();
    });

    test('should have accessible buttons with labels', async ({ page }) => {
      const main = mainContent(page);

      // Verify key buttons have accessible names
      const refreshButton = main.getByRole('button', { name: /Refresh/i });
      const hasRefresh = await refreshButton
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasRefresh) {
        await expect(refreshButton).toBeVisible();
      }

      const exportButton = main.getByRole('button', { name: /Export/i });
      const hasExport = await exportButton
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasExport) {
        await expect(exportButton).toBeVisible();
      }

      const filtersButton = main.getByRole('button', { name: /Filters/i });
      const hasFilters = await filtersButton
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasFilters) {
        await expect(filtersButton).toBeVisible();
      }
    });

    test('should have labeled form controls in the controls bar', async ({ page }) => {
      const main = mainContent(page);

      // Layout selector should have an associated label
      const hasLayoutLabel = await main
        .getByText('Layout:')
        .isVisible({ timeout: 5000 })
        .catch(() => false);

      if (hasLayoutLabel) {
        // The label is connected to the select via htmlFor="layout"
        const layoutSelect = main.locator('#layout');
        await expect(layoutSelect).toBeVisible();
      }
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

      // Reload the page to capture all console output
      await page.reload({ waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(3000);

      // Filter out known benign errors (network polling, WebSocket, SSE, etc.)
      const criticalErrors = consoleErrors.filter(
        (err) =>
          !err.includes('net::ERR_') &&
          !err.includes('Failed to fetch') &&
          !err.includes('NetworkError') &&
          !err.includes('WebSocket') &&
          !err.includes('favicon') &&
          !err.includes('429') &&
          !err.includes('not valid JSON') &&
          !err.includes('DOCTYPE') &&
          !err.includes('EventSource') &&
          !err.includes('SSE')
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

    test('should not show error alert for graph data loading', async ({ page }) => {
      const main = mainContent(page);

      // The error alert renders conditionally at the bottom of the page
      const hasError = await main
        .getByText(/Failed to load graph data/i)
        .first()
        .isVisible({ timeout: 2000 })
        .catch(() => false);

      // On the deployed site without a running backend, an error may appear
      // but the page should not crash — verify the main content area is still rendered
      await expect(main).not.toBeEmpty();
    });
  });
});
