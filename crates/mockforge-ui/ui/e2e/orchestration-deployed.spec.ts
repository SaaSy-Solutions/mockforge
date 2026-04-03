import { test, expect } from '@playwright/test';

/**
 * Orchestration Pages E2E Tests (chains, graph, state-machine-editor,
 * scenario-studio, orchestration-builder, orchestration-execution)
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';
function mainContent(page: import('@playwright/test').Page) { return page.getByRole('main'); }

// ---------------------------------------------------------------------------
// Request Chains
// ---------------------------------------------------------------------------
test.describe('Request Chains — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/chains`, { waitUntil: 'domcontentloaded', timeout: 30000 });
    await page.waitForSelector('nav[aria-label="Main navigation"]', { state: 'visible', timeout: 15000 });
    await page.waitForTimeout(2000);
  });

  test('should load the chains page', async ({ page }) => {
    await expect(page).toHaveURL(/\/chains/);
    await expect(page).toHaveTitle(/MockForge/);
  });

  test('should display breadcrumbs', async ({ page }) => {
    await expect(page.getByRole('banner').getByText('Chains')).toBeVisible();
  });

  test('should display Request Chains heading', async ({ page }) => {
    await expect(mainContent(page).getByText('Request Chains')).toBeVisible({ timeout: 5000 });
  });

  test('should display subtitle', async ({ page }) => {
    await expect(mainContent(page).getByText(/Manage and execute request chains/)).toBeVisible();
  });

  test('should display Create Chain button', async ({ page }) => {
    await expect(mainContent(page).getByRole('button', { name: /Create Chain/i }).first()).toBeVisible();
  });

  test('should show chains or empty state', async ({ page }) => {
    const main = mainContent(page);
    const hasEmpty = await main.getByText('No Chains Found').isVisible({ timeout: 3000 }).catch(() => false);
    const hasChains = await main.getByText('Available Chains').isVisible({ timeout: 3000 }).catch(() => false);
    expect(hasEmpty || hasChains).toBeTruthy();
  });

  test('should display empty state CTA button', async ({ page }) => {
    const main = mainContent(page);
    const hasEmpty = await main.getByText('No Chains Found').isVisible({ timeout: 3000 }).catch(() => false);
    if (hasEmpty) {
      await expect(main.getByRole('button', { name: /Create First Chain/i })).toBeVisible();
    }
  });
});

// ---------------------------------------------------------------------------
// Graph
// ---------------------------------------------------------------------------
test.describe('Graph — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/graph`, { waitUntil: 'domcontentloaded', timeout: 30000 });
    await page.waitForSelector('nav[aria-label="Main navigation"]', { state: 'visible', timeout: 15000 });
    await page.waitForTimeout(2000);
  });

  test('should load the graph page', async ({ page }) => {
    await expect(page).toHaveURL(/\/graph/);
    await expect(page).toHaveTitle(/MockForge/);
  });

  test('should display breadcrumbs', async ({ page }) => {
    await expect(page.getByRole('banner').getByText('Graph')).toBeVisible();
  });

  test('should render page content without crashing', async ({ page }) => {
    const main = mainContent(page);
    const text = await main.textContent();
    expect(text!.length).toBeGreaterThan(0);
  });

  test('should not show error UI', async ({ page }) => {
    expect(await page.getByText(/Something went wrong|Application error/i).first().isVisible({ timeout: 2000 }).catch(() => false)).toBeFalsy();
  });
});

// ---------------------------------------------------------------------------
// State Machine Editor
// ---------------------------------------------------------------------------
test.describe('State Machine Editor — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/state-machine-editor`, { waitUntil: 'domcontentloaded', timeout: 30000 });
    await page.waitForSelector('nav[aria-label="Main navigation"]', { state: 'visible', timeout: 15000 });
    await page.waitForTimeout(2000);
  });

  test('should load the state machine editor page', async ({ page }) => {
    await expect(page).toHaveURL(/\/state-machine-editor/);
    await expect(page).toHaveTitle(/MockForge/);
  });

  test('should display breadcrumbs', async ({ page }) => {
    await expect(page.getByRole('banner').getByText('State Machines')).toBeVisible();
  });

  test('should display toolbar buttons', async ({ page }) => {
    const main = mainContent(page);
    // Look for common toolbar buttons
    const hasSave = await main.getByRole('button', { name: /Save/i }).isVisible({ timeout: 3000 }).catch(() => false);
    const hasExport = await main.getByRole('button', { name: /Export/i }).isVisible({ timeout: 3000 }).catch(() => false);
    const hasAddState = await main.getByRole('button', { name: /Add State/i }).isVisible({ timeout: 3000 }).catch(() => false);
    expect(hasSave || hasExport || hasAddState).toBeTruthy();
  });

  test('should not show error UI', async ({ page }) => {
    expect(await page.getByText(/Something went wrong|Application error/i).first().isVisible({ timeout: 2000 }).catch(() => false)).toBeFalsy();
  });
});

// ---------------------------------------------------------------------------
// Scenario Studio
// ---------------------------------------------------------------------------
test.describe('Scenario Studio — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/scenario-studio`, { waitUntil: 'domcontentloaded', timeout: 30000 });
    await page.waitForSelector('nav[aria-label="Main navigation"]', { state: 'visible', timeout: 15000 });
    await page.waitForTimeout(2000);
  });

  test('should load the scenario studio page', async ({ page }) => {
    await expect(page).toHaveURL(/\/scenario-studio/);
    await expect(page).toHaveTitle(/MockForge/);
  });

  test('should display Scenario Studio title', async ({ page }) => {
    await expect(mainContent(page).getByText('Scenario Studio')).toBeVisible({ timeout: 5000 });
  });

  test('should display "New Flow" button', async ({ page }) => {
    await expect(
      mainContent(page).getByRole('button', { name: /New Flow/i })
    ).toBeVisible({ timeout: 5000 });
  });

  test('should display empty state when no flow selected', async ({ page }) => {
    const main = mainContent(page);
    const hasEmpty = await main.getByText(/No flow selected|Select a flow/i)
      .first().isVisible({ timeout: 3000 }).catch(() => false);
    expect(hasEmpty).toBeTruthy();
  });

  test('should display breadcrumbs', async ({ page }) => {
    await expect(page.getByRole('banner').getByText('Scenario Studio')).toBeVisible();
  });

  test('should not show error UI', async ({ page }) => {
    expect(await page.getByText(/Something went wrong|Application error/i).first().isVisible({ timeout: 2000 }).catch(() => false)).toBeFalsy();
  });
});

// ---------------------------------------------------------------------------
// Orchestration Builder
// ---------------------------------------------------------------------------
test.describe('Orchestration Builder — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/orchestration-builder`, { waitUntil: 'domcontentloaded', timeout: 30000 });
    await page.waitForSelector('nav[aria-label="Main navigation"]', { state: 'visible', timeout: 15000 });
    await page.waitForTimeout(2000);
  });

  test('should load the orchestration builder page', async ({ page }) => {
    await expect(page).toHaveURL(/\/orchestration-builder/);
    await expect(page).toHaveTitle(/MockForge/);
  });

  test('should display toolbar with action buttons', async ({ page }) => {
    const main = mainContent(page);
    // Orchestration builder has Execute, Save, Export, Import buttons
    const hasExecute = await main.getByRole('button', { name: /Execute/i }).isVisible({ timeout: 3000 }).catch(() => false);
    const hasSave = await main.getByRole('button', { name: /Save/i }).isVisible({ timeout: 3000 }).catch(() => false);
    expect(hasExecute || hasSave).toBeTruthy();
  });

  test('should display Add Step button or empty state', async ({ page }) => {
    const main = mainContent(page);
    const hasAddStep = await main.getByRole('button', { name: /Add Step/i }).isVisible({ timeout: 3000 }).catch(() => false);
    const hasEmpty = await main.getByText(/No steps added yet/i).isVisible({ timeout: 3000 }).catch(() => false);
    expect(hasAddStep || hasEmpty).toBeTruthy();
  });

  test('should display breadcrumbs', async ({ page }) => {
    await expect(page.getByRole('banner').getByText('Orchestration Builder')).toBeVisible();
  });

  test('should not show error UI', async ({ page }) => {
    expect(await page.getByText(/Something went wrong|Application error/i).first().isVisible({ timeout: 2000 }).catch(() => false)).toBeFalsy();
  });
});

// ---------------------------------------------------------------------------
// Orchestration Execution
// ---------------------------------------------------------------------------
test.describe('Orchestration Execution — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/orchestration-execution`, { waitUntil: 'domcontentloaded', timeout: 30000 });
    await page.waitForSelector('nav[aria-label="Main navigation"]', { state: 'visible', timeout: 15000 });
    await page.waitForTimeout(2000);
  });

  test('should load the orchestration execution page', async ({ page }) => {
    await expect(page).toHaveURL(/\/orchestration-execution/);
    await expect(page).toHaveTitle(/MockForge/);
  });

  test('should display execution controls', async ({ page }) => {
    const main = mainContent(page);
    // Should show Start button (idle state) or status indicators
    const hasStart = await main.getByRole('button', { name: /Start/i }).isVisible({ timeout: 3000 }).catch(() => false);
    const hasStatus = await main.getByText(/Idle|Running|Completed|Failed/i).first().isVisible({ timeout: 3000 }).catch(() => false);
    expect(hasStart || hasStatus).toBeTruthy();
  });

  test('should display breadcrumbs', async ({ page }) => {
    await expect(page.getByRole('banner').getByText('Orchestration Execution')).toBeVisible();
  });

  test('should not show error UI', async ({ page }) => {
    expect(await page.getByText(/Something went wrong|Application error/i).first().isVisible({ timeout: 2000 }).catch(() => false)).toBeFalsy();
  });
});
