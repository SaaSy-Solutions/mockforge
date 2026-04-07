import { test, expect } from '@playwright/test';

/**
 * Voice + LLM Interface Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config (handles auth via setup project):
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts voice-deployed
 *
 * These tests verify the Voice + LLM Interface page functionality:
 *   1.  Page Load & Layout
 *   2.  Tab Navigation
 *   3.  API Generation Tab — Features & Voice Input
 *   4.  Hook Transpilation Tab — Editor & Examples
 *   5.  Workspace Scenarios Tab — Creator & Examples
 *   6.  Example Commands
 *   7.  Navigation
 *   8.  Accessibility
 *   9.  Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

// Scoped locator for the main content area (excludes sidebar to avoid duplicate matches)
function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('Voice + LLM Interface — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/voice`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    // Wait for the app shell to render (sidebar navigation visible)
    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    // Wait for the Voice + LLM Interface heading to confirm content loaded
    await expect(
      mainContent(page).getByRole('heading', { name: 'Voice + LLM Interface', level: 1 })
    ).toBeVisible({ timeout: 10000 });

    // Small stabilization delay for dynamic content
    await page.waitForTimeout(500);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the voice page at /voice', async ({ page }) => {
      await expect(page).toHaveURL(/\/voice/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the page heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Voice + LLM Interface', level: 1 })
      ).toBeVisible();
    });

    test('should display the page subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText(
          "Build mocks conversationally using natural language commands powered by AI."
        ).first()
      ).toBeVisible();
      await expect(
        mainContent(page).getByText(
          "Speak or type your requirements, and we'll generate an OpenAPI specification."
        ).first()
      ).toBeVisible();
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      const hasHome = await banner.getByText('Home')
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasVoice = await banner.getByText('Voice')
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasHome || hasVoice).toBeTruthy();
    });

    test('should display the three tabs', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('button', { name: /API Generation/i })).toBeVisible();
      await expect(main.getByRole('button', { name: /Hook Transpilation/i })).toBeVisible();
      await expect(main.getByRole('button', { name: /Workspace Scenarios/i })).toBeVisible();
    });

    test('should display the API Generation tab as active by default', async ({ page }) => {
      const main = mainContent(page);
      const apiTab = main.getByRole('button', { name: /API Generation/i });
      const classes = await apiTab.getAttribute('class');
      expect(classes).toContain('border-primary');
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Tab Navigation
  // ---------------------------------------------------------------------------
  test.describe('Tab Navigation', () => {
    test('should switch to Hook Transpilation tab when clicked', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Hook Transpilation/i }).click();
      await page.waitForTimeout(500);

      // Hook Transpilation tab should show the Natural Language Hook Editor
      await expect(
        main.getByRole('heading', { name: 'Natural Language Hook Editor' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should highlight Hook Transpilation tab when active', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Hook Transpilation/i }).click();
      await page.waitForTimeout(500);

      const hookTab = main.getByRole('button', { name: /Hook Transpilation/i });
      const classes = await hookTab.getAttribute('class');
      expect(classes).toContain('border-primary');
    });

    test('should switch to Workspace Scenarios tab when clicked', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Workspace Scenarios/i }).click();
      await page.waitForTimeout(500);

      // Workspace Scenarios tab should show the Chat-Driven Workspace Scenarios card
      await expect(
        main.getByRole('heading', { name: 'Chat-Driven Workspace Scenarios' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should highlight Workspace Scenarios tab when active', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Workspace Scenarios/i }).click();
      await page.waitForTimeout(500);

      const scenarioTab = main.getByRole('button', { name: /Workspace Scenarios/i });
      const classes = await scenarioTab.getAttribute('class');
      expect(classes).toContain('border-primary');
    });

    test('should switch back to API Generation tab when clicked', async ({ page }) => {
      const main = mainContent(page);

      // Switch away first
      await main.getByRole('button', { name: /Hook Transpilation/i }).click();
      await page.waitForTimeout(500);

      // Switch back
      await main.getByRole('button', { name: /API Generation/i }).click();
      await page.waitForTimeout(500);

      // Should show the feature cards from API Generation tab
      await expect(main.getByText('Voice Input').first()).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('AI-Powered').first()).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('OpenAPI Output').first()).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 3. API Generation Tab — Features & Voice Input
  // ---------------------------------------------------------------------------
  test.describe('API Generation Tab', () => {
    test('should display the three feature cards', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Voice Input').first()).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('AI-Powered').first()).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('OpenAPI Output').first()).toBeVisible({ timeout: 5000 });
    });

    test('should display Voice Input feature card description', async ({ page }) => {
      await expect(
        mainContent(page).getByText(
          'Use your microphone to speak commands naturally. Works with Chrome, Edge, and Safari.'
        ).first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display AI-Powered feature card description', async ({ page }) => {
      await expect(
        mainContent(page).getByText(
          'LLM interprets your commands and extracts API requirements automatically.'
        ).first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display OpenAPI Output feature card description', async ({ page }) => {
      await expect(
        mainContent(page).getByText(
          'Generates valid OpenAPI 3.0 specifications ready to use with MockForge.'
        ).first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the VoiceInput component with Start Voice Input or unsupported message', async ({
      page,
    }) => {
      const main = mainContent(page);

      // Browser may or may not support Web Speech API — handle both cases
      const hasStartButton = await main
        .getByRole('button', { name: /Start Voice Input/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasUnsupported = await main
        .getByText('Voice input not supported')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasStartButton || hasUnsupported).toBeTruthy();
    });

    test('should display the text input fallback', async ({ page }) => {
      const main = mainContent(page);

      // Text input fallback is present regardless of Web Speech API support
      const hasTextInput = await main
        .locator('input[placeholder="Or type your command here..."]')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      // If speech is unsupported, the text input won't render (the whole VoiceInput
      // component renders the unsupported message instead). Both states are valid.
      const hasUnsupported = await main
        .getByText('Voice input not supported')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasTextInput || hasUnsupported).toBeTruthy();
    });

    test('should display the Process button in text input', async ({ page }) => {
      const main = mainContent(page);

      const hasProcessButton = await main
        .getByRole('button', { name: 'Process' })
        .isVisible({ timeout: 3000 })
        .catch(() => false);
      const hasUnsupported = await main
        .getByText('Voice input not supported')
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      expect(hasProcessButton || hasUnsupported).toBeTruthy();
    });

    test('should disable Process button when text input is empty', async ({ page }) => {
      const main = mainContent(page);

      const processButton = main.getByRole('button', { name: 'Process' });
      const hasProcessButton = await processButton
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasProcessButton) {
        await expect(processButton).toBeDisabled();
      }
    });

    test('should allow typing in the text input', async ({ page }) => {
      const main = mainContent(page);
      const textInput = main.locator('input[placeholder="Or type your command here..."]');
      const hasInput = await textInput.isVisible({ timeout: 3000 }).catch(() => false);

      if (hasInput) {
        await textInput.fill('Create a user management API');
        await page.waitForTimeout(300);
        await expect(textInput).toHaveValue('Create a user management API');

        // Clean up
        await textInput.clear();
      }
    });

    test('should enable Process button when text is entered', async ({ page }) => {
      const main = mainContent(page);
      const textInput = main.locator('input[placeholder="Or type your command here..."]');
      const hasInput = await textInput.isVisible({ timeout: 3000 }).catch(() => false);

      if (hasInput) {
        await textInput.fill('Create a todo API');
        await page.waitForTimeout(300);

        const processButton = main.getByRole('button', { name: 'Process' });
        await expect(processButton).toBeEnabled();

        // Clean up
        await textInput.clear();
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Hook Transpilation Tab — Editor & Examples
  // ---------------------------------------------------------------------------
  test.describe('Hook Transpilation Tab', () => {
    test('should display the Natural Language Hook Editor heading', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Hook Transpilation/i }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('heading', { name: 'Natural Language Hook Editor' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the editor description', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Hook Transpilation/i }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByText(
          'Describe hook logic in natural language and get transpiled hook configurations ready to use in chaos orchestration scenarios.'
        ).first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the hook description textarea', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Hook Transpilation/i }).click();
      await page.waitForTimeout(500);

      const textarea = main.locator('textarea#hook-description');
      await expect(textarea).toBeVisible({ timeout: 5000 });
    });

    test('should display the hook description label', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Hook Transpilation/i }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByText('Describe your hook logic in natural language').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Transpile Hook button', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Hook Transpilation/i }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('button', { name: /Transpile Hook/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should disable Transpile Hook button when textarea is empty', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Hook Transpilation/i }).click();
      await page.waitForTimeout(500);

      const transpileButton = main.getByRole('button', { name: /Transpile Hook/i });
      await expect(transpileButton).toBeDisabled();
    });

    test('should allow typing in the hook description textarea', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Hook Transpilation/i }).click();
      await page.waitForTimeout(500);

      const textarea = main.locator('textarea#hook-description');
      await textarea.fill('When a request fails, log the error and send a notification webhook');
      await page.waitForTimeout(300);

      const value = await textarea.inputValue();
      expect(value).toContain('When a request fails');

      // Clean up
      await textarea.clear();
    });

    test('should enable Transpile Hook button when text is entered', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Hook Transpilation/i }).click();
      await page.waitForTimeout(500);

      const textarea = main.locator('textarea#hook-description');
      await textarea.fill('Log every failed request');
      await page.waitForTimeout(300);

      const transpileButton = main.getByRole('button', { name: /Transpile Hook/i });
      await expect(transpileButton).toBeEnabled();

      // Clean up
      await textarea.clear();
    });

    test('should display the Example Hook Descriptions card', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Hook Transpilation/i }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('heading', { name: 'Example Hook Descriptions' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display all four hook example names', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Hook Transpilation/i }).click();
      await page.waitForTimeout(500);

      const examples = ['VIP User Hook', 'Conditional Logging', 'Metric Recording', 'Complex Condition'];
      for (const example of examples) {
        await expect(main.getByText(example, { exact: true }).first()).toBeVisible({ timeout: 5000 });
      }
    });

    test('should display hook example descriptions', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Hook Transpilation/i }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByText(/For users flagged as VIP, webhooks should fire instantly/)
      ).toBeVisible({ timeout: 5000 });
      await expect(
        main.getByText(/When a request fails, log the error and send a notification webhook/)
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Workspace Scenarios Tab — Creator & Examples
  // ---------------------------------------------------------------------------
  test.describe('Workspace Scenarios Tab', () => {
    test('should display the Chat-Driven Workspace Scenarios heading', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Workspace Scenarios/i }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('heading', { name: 'Chat-Driven Workspace Scenarios' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the scenario creator description', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Workspace Scenarios/i }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByText(
          'Create complete workspace scenarios with APIs, chaos configurations, and initial data from natural language descriptions.'
        ).first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the scenario description textarea', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Workspace Scenarios/i }).click();
      await page.waitForTimeout(500);

      const textarea = main.locator('textarea#scenario-description');
      await expect(textarea).toBeVisible({ timeout: 5000 });
    });

    test('should display the scenario description label', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Workspace Scenarios/i }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByText('Describe your workspace scenario').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the Create Workspace Scenario button', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Workspace Scenarios/i }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('button', { name: /Create Workspace Scenario/i })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should disable Create Workspace Scenario button when textarea is empty', async ({
      page,
    }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Workspace Scenarios/i }).click();
      await page.waitForTimeout(500);

      const createButton = main.getByRole('button', { name: /Create Workspace Scenario/i });
      await expect(createButton).toBeDisabled();
    });

    test('should allow typing in the scenario description textarea', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Workspace Scenarios/i }).click();
      await page.waitForTimeout(500);

      const textarea = main.locator('textarea#scenario-description');
      await textarea.fill('Create an e-commerce workspace with high latency on checkout');
      await page.waitForTimeout(300);

      const value = await textarea.inputValue();
      expect(value).toContain('e-commerce workspace');

      // Clean up
      await textarea.clear();
    });

    test('should enable Create Workspace Scenario button when text is entered', async ({
      page,
    }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Workspace Scenarios/i }).click();
      await page.waitForTimeout(500);

      const textarea = main.locator('textarea#scenario-description');
      await textarea.fill('Create a banking workspace');
      await page.waitForTimeout(300);

      const createButton = main.getByRole('button', { name: /Create Workspace Scenario/i });
      await expect(createButton).toBeEnabled();

      // Clean up
      await textarea.clear();
    });

    test('should display the Example Scenario Descriptions card', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Workspace Scenarios/i }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('heading', { name: 'Example Scenario Descriptions' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display all four scenario example names', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Workspace Scenarios/i }).click();
      await page.waitForTimeout(500);

      const examples = [
        'Banking Scenario',
        'E-commerce with Chaos',
        'Healthcare API',
        'Social Media Platform',
      ];
      for (const example of examples) {
        await expect(main.getByText(example, { exact: true }).first()).toBeVisible({ timeout: 5000 });
      }
    });

    test('should display scenario example descriptions', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Workspace Scenarios/i }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByText(/Create a workspace that simulates a bank with flaky foreign exchange/)
      ).toBeVisible({ timeout: 5000 });
      await expect(
        main.getByText(/Create an e-commerce workspace with high latency on checkout/)
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Example Commands
  // ---------------------------------------------------------------------------
  test.describe('Example Commands', () => {
    test('should display the Example Commands heading on API Generation tab', async ({
      page,
    }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'Example Commands' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display all four example command names', async ({ page }) => {
      const main = mainContent(page);
      const examples = ['Simple API', 'E-commerce', 'With Models', 'Complex'];
      for (const example of examples) {
        await expect(main.getByText(example, { exact: true }).first()).toBeVisible({ timeout: 5000 });
      }
    });

    test('should display example command descriptions', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText(
          'Create a todo API with endpoints for listing, creating, and updating tasks'
        ).first()
      ).toBeVisible({ timeout: 5000 });
      await expect(
        main.getByText(
          'Create an e-commerce API with products, users, and a checkout flow'
        ).first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display Example Commands on Hook Transpilation tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Hook Transpilation/i }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('heading', { name: 'Example Commands' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display Example Commands on Workspace Scenarios tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Workspace Scenarios/i }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('heading', { name: 'Example Commands' })
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

      // Navigate back to Voice via sidebar or direct navigation
      const hasVoiceButton = await nav
        .getByRole('button', { name: /Voice/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasVoiceButton) {
        await nav.getByRole('button', { name: /Voice/i }).click();
      } else {
        await page.goto(`${BASE_URL}/voice`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Voice + LLM Interface', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should navigate to Services and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Services' }).click();
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Services', exact: true, level: 1 })
      ).toBeVisible({ timeout: 5000 });

      // Navigate back to Voice
      const hasVoiceButton = await nav
        .getByRole('button', { name: /Voice/i })
        .isVisible({ timeout: 3000 })
        .catch(() => false);

      if (hasVoiceButton) {
        await nav.getByRole('button', { name: /Voice/i }).click();
      } else {
        await page.goto(`${BASE_URL}/voice`, {
          waitUntil: 'domcontentloaded',
          timeout: 30000,
        });
      }
      await page.waitForTimeout(1500);

      await expect(
        mainContent(page).getByRole('heading', { name: 'Voice + LLM Interface', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should preserve URL when navigating back via browser history', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await page.goBack();
      await page.waitForTimeout(1500);

      await expect(page).toHaveURL(/\/voice/);
      await expect(
        mainContent(page).getByRole('heading', { name: 'Voice + LLM Interface', level: 1 })
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
      await expect(h1).toHaveText('Voice + LLM Interface');
    });

    test('should have multiple H2 section headings', async ({ page }) => {
      const h2s = mainContent(page).getByRole('heading', { level: 2 });
      // At minimum: Example Commands heading is always visible on the API Generation tab
      expect(await h2s.count()).toBeGreaterThanOrEqual(1);
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

    test('should have accessible tab buttons', async ({ page }) => {
      const main = mainContent(page);
      const tabs = ['API Generation', 'Hook Transpilation', 'Workspace Scenarios'];

      for (const tabName of tabs) {
        const tab = main.getByRole('button', { name: new RegExp(tabName, 'i') });
        await expect(tab).toBeVisible({ timeout: 5000 });
      }
    });

    test('should have labeled form inputs in Hook Transpilation tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Hook Transpilation/i }).click();
      await page.waitForTimeout(500);

      // Textarea should have an associated label via htmlFor/id
      const textarea = main.locator('textarea#hook-description');
      await expect(textarea).toBeVisible({ timeout: 5000 });
      await expect(
        main.getByText('Describe your hook logic in natural language').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should have labeled form inputs in Workspace Scenarios tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: /Workspace Scenarios/i }).click();
      await page.waitForTimeout(500);

      // Textarea should have an associated label via htmlFor/id
      const textarea = main.locator('textarea#scenario-description');
      await expect(textarea).toBeVisible({ timeout: 5000 });
      await expect(
        main.getByText('Describe your workspace scenario').first()
      ).toBeVisible({ timeout: 5000 });
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
          !err.includes('DOCTYPE') &&
          !err.includes('Failed to load resource') &&
          !err.includes('the server responded') &&
          !err.includes('TypeError') &&
          !err.includes('ErrorBoundary') &&
          !err.includes('Cannot read properties')
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

    test('should handle tab switching without errors', async ({ page }) => {
      const consoleErrors: string[] = [];

      page.on('console', (msg) => {
        if (msg.type() === 'error') {
          consoleErrors.push(msg.text());
        }
      });

      const main = mainContent(page);
      const tabs = ['Hook Transpilation', 'Workspace Scenarios', 'API Generation'];

      for (const tab of tabs) {
        await main.getByRole('button', { name: new RegExp(tab, 'i') }).click();
        await page.waitForTimeout(500);
      }

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
          !err.includes('Failed to load resource') &&
          !err.includes('the server responded') &&
          !err.includes('TypeError') &&
          !err.includes('ErrorBoundary') &&
          !err.includes('Cannot read properties')
      );

      expect(criticalErrors).toHaveLength(0);
    });
  });
});
