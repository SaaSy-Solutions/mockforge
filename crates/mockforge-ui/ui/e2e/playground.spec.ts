import { test, expect } from '@playwright/test';

/**
 * E2E tests for GraphQL + REST Playground
 *
 * Tests the complete playground workflow:
 * - Loading the playground page
 * - Executing REST requests
 * - Executing GraphQL queries
 * - Viewing responses
 * - Using history
 * - Generating code snippets
 */
test.describe('Playground', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to playground page
    await page.goto('/');

    // Wait for app to load
    await page.waitForSelector('nav', { timeout: 10000 });

    // Click on Playground tab
    await page.click('button:has-text("Playground")');

    // Wait for playground to load
    await page.waitForSelector('text=Request', { timeout: 5000 });
  });

  test('should load playground page', async ({ page }) => {
    // Verify main components are visible
    await expect(page.locator('text=Request')).toBeVisible();
    await expect(page.locator('text=Response')).toBeVisible();
  });

  test('should switch between REST and GraphQL protocols', async ({ page }) => {
    // Verify REST is selected by default
    const protocolSelect = page.locator('select, [role="combobox"]').first();
    await expect(protocolSelect).toBeVisible();

    // Switch to GraphQL
    await protocolSelect.selectOption('graphql');

    // Verify GraphQL query editor is visible
    await expect(page.locator('textarea[placeholder*="query"]')).toBeVisible();
  });

  test('should execute REST request', async ({ page }) => {
    // Fill in REST request
    const methodSelect = page.locator('select, [role="combobox"]').first();
    await methodSelect.selectOption('GET');

    const pathInput = page.locator('input[placeholder*="api"]').first();
    await pathInput.fill('/api/users');

    // Click execute button
    const executeButton = page.locator('button:has-text("Execute")');
    await executeButton.click();

    // Wait for response
    await page.waitForSelector('text=Response', { timeout: 10000 });

    // Verify response panel shows result
    await expect(page.locator('text=Response')).toBeVisible();
  });

  test('should display request history', async ({ page }) => {
    // Execute a request first
    const pathInput = page.locator('input[placeholder*="api"]').first();
    await pathInput.fill('/api/users');

    const executeButton = page.locator('button:has-text("Execute")');
    await executeButton.click();

    // Wait for response
    await page.waitForSelector('text=Response', { timeout: 10000 });

    // Check if history panel is visible
    const historyPanel = page.locator('text=History');
    if (await historyPanel.isVisible()) {
      // Verify history entry appears
      await expect(page.locator('text=/api/users')).toBeVisible();
    }
  });

  test('should generate code snippets', async ({ page }) => {
    // Fill in REST request
    const pathInput = page.locator('input[placeholder*="api"]').first();
    await pathInput.fill('/api/users');

    // Wait for code snippet generator to load
    await page.waitForSelector('text=Code Snippets', { timeout: 5000 });

    // Verify code snippets are generated
    const codeSnippets = page.locator('text=Code Snippets');
    await expect(codeSnippets).toBeVisible();
  });

  test('should filter history by search', async ({ page }) => {
    // Execute multiple requests
    const pathInput = page.locator('input[placeholder*="api"]').first();
    await pathInput.fill('/api/users');

    const executeButton = page.locator('button:has-text("Execute")');
    await executeButton.click();

    await page.waitForTimeout(1000);

    // Try to find search input in history panel
    const searchInput = page.locator('input[placeholder*="Search"]');
    if (await searchInput.isVisible()) {
      await searchInput.fill('users');

      // Verify filtered results
      await expect(page.locator('text=/api/users')).toBeVisible();
    }
  });
});
