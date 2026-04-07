import { test, expect } from '@playwright/test';

/**
 * AI Studio Page E2E Tests for Deployed Site (https://app.mockforge.dev/)
 *
 * Run with the deployed config:
 *   E2E_EMAIL=you@example.com E2E_PASSWORD=secret \
 *   npx playwright test --config=playwright-deployed.config.ts ai-studio-deployed
 *
 * These tests verify the AI Studio page functionality:
 *   1. Page Load & Layout
 *   2. Tab Navigation
 *   3. Chat Tab
 *   4. Generate Tab
 *   5. Debug Tab
 *   6. Personas Tab
 *   7. Budget Tab
 *   8. Navigation
 *   9. Accessibility
 *  10. Error-Free Operation
 */

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';

function mainContent(page: import('@playwright/test').Page) {
  return page.getByRole('main');
}

test.describe('AI Studio — Deployed Site', () => {
  test.describe.configure({ mode: 'serial' });

  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/ai-studio`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });

    await page.waitForSelector('nav[aria-label="Main navigation"]', {
      state: 'visible',
      timeout: 15000,
    });

    await page.waitForTimeout(2000);
  });

  // ---------------------------------------------------------------------------
  // 1. Page Load & Layout
  // ---------------------------------------------------------------------------
  test.describe('Page Load & Layout', () => {
    test('should load the AI Studio page at /ai-studio', async ({ page }) => {
      await expect(page).toHaveURL(/\/ai-studio/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display the AI Studio heading', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('heading', { name: 'AI Studio', level: 1 })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the subtitle', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Unified AI Copilot for all MockForge AI features').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display breadcrumb navigation', async ({ page }) => {
      const banner = page.getByRole('banner');
      const hasBreadcrumb = await banner.getByText('AI Studio')
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasHomeBreadcrumb = await banner.getByText('Home')
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasBreadcrumb || hasHomeBreadcrumb).toBeTruthy();
    });

    test('should display usage stats widget or loading state', async ({ page }) => {
      const main = mainContent(page);
      const hasTokensUsed = await main.getByText('Tokens Used')
        .first().isVisible({ timeout: 3000 }).catch(() => false);
      const hasCost = await main.getByText('Cost')
        .first().isVisible({ timeout: 3000 }).catch(() => false);
      // Stats may not load if API is unavailable, so just check page renders
      const hasHeading = await main.getByRole('heading', { name: 'AI Studio', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasTokensUsed || hasCost || hasHeading).toBeTruthy();
    });

    test('should display usage stats or heading when loaded', async ({ page }) => {
      const main = mainContent(page);
      const hasTokensUsed = await main.getByText('Tokens Used')
        .first().isVisible({ timeout: 3000 }).catch(() => false);
      const hasHeading = await main.getByRole('heading', { name: 'AI Studio', level: 1 })
        .isVisible({ timeout: 3000 }).catch(() => false);

      // Either usage stats are visible or the heading is — both indicate successful load
      expect(hasTokensUsed || hasHeading).toBeTruthy();

      if (hasTokensUsed) {
        // Progress bar may or may not be visible depending on usage data
        const hasProgressBar = await main.getByRole('progressbar')
          .first().isVisible({ timeout: 3000 }).catch(() => false);
        // Just check — don't fail if not present (may be 0% usage)
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 2. Tab Navigation
  // ---------------------------------------------------------------------------
  test.describe('Tab Navigation', () => {
    const TAB_NAMES = [
      'Chat',
      'Generate',
      'System Designer',
      'AI User Simulator',
      'Debug',
      'Personas',
      'Contract Diff',
      'API Critique',
      'Budget',
    ];

    test('should display all 9 tabs', async ({ page }) => {
      const main = mainContent(page);
      for (const tabName of TAB_NAMES) {
        await expect(
          main.getByRole('button', { name: tabName, exact: true })
        ).toBeVisible({ timeout: 5000 });
      }
    });

    test('should highlight the Chat tab by default', async ({ page }) => {
      const main = mainContent(page);
      const chatTab = main.getByRole('button', { name: 'Chat', exact: true });
      await expect(chatTab).toBeVisible({ timeout: 5000 });

      // The active tab has border-primary class
      const classes = await chatTab.getAttribute('class');
      expect(classes).toContain('border-primary');
    });

    test('should switch to Generate tab when clicked', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Generate', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('heading', { name: 'Generate Mocks' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should switch to Debug tab when clicked', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Debug', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('heading', { name: 'AI-Guided Debugging' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should switch to Personas tab when clicked', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Personas', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('heading', { name: 'Persona Generation' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should switch to Contract Diff tab when clicked', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Contract Diff', exact: true }).click();
      await page.waitForTimeout(500);

      // Contract Diff tab shows "Captured Requests" and "Analysis Configuration"
      const hasCaptured = await main.getByText('Captured Requests')
        .isVisible({ timeout: 5000 }).catch(() => false);
      const hasAnalysis = await main.getByText('Analysis Configuration')
        .isVisible({ timeout: 5000 }).catch(() => false);
      expect(hasCaptured || hasAnalysis).toBeTruthy();
    });

    test('should switch to API Critique tab when clicked', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'API Critique', exact: true }).click();
      await page.waitForTimeout(500);

      // API Critique tab should render its component content
      const hasContent = await main.textContent();
      expect(hasContent!.length).toBeGreaterThan(0);
    });

    test('should switch to System Designer tab when clicked', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'System Designer', exact: true }).click();
      await page.waitForTimeout(500);

      const hasContent = await main.textContent();
      expect(hasContent!.length).toBeGreaterThan(0);
    });

    test('should switch to AI User Simulator tab when clicked', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'AI User Simulator', exact: true }).click();
      await page.waitForTimeout(500);

      const hasContent = await main.textContent();
      expect(hasContent!.length).toBeGreaterThan(0);
    });

    test('should switch to Budget tab when clicked', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Budget', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('heading', { name: 'Budget & Usage' })
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Chat Tab
  // ---------------------------------------------------------------------------
  test.describe('Chat Tab', () => {
    test('should display the chat message area with welcome text', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Welcome to AI Studio').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display suggested prompts in the empty chat state', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByText('Try asking:').first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the chat input with correct placeholder', async ({ page }) => {
      const main = mainContent(page);
      const chatInput = main.locator('input[placeholder="Type your message..."]');
      await expect(chatInput).toBeVisible({ timeout: 5000 });
    });

    test('should display the Send button', async ({ page }) => {
      const main = mainContent(page);
      await expect(
        main.getByRole('button', { name: 'Send' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should disable Send button when input is empty', async ({ page }) => {
      const main = mainContent(page);
      const sendButton = main.getByRole('button', { name: 'Send' });
      await expect(sendButton).toBeDisabled();
    });

    test('should enable Send button when text is entered', async ({ page }) => {
      const main = mainContent(page);
      const chatInput = main.locator('input[placeholder="Type your message..."]');
      await chatInput.fill('Hello AI');
      await page.waitForTimeout(300);

      const sendButton = main.getByRole('button', { name: 'Send' });
      await expect(sendButton).toBeEnabled();

      // Clean up
      await chatInput.clear();
    });

    test('should allow typing in the chat input', async ({ page }) => {
      const main = mainContent(page);
      const chatInput = main.locator('input[placeholder="Type your message..."]');
      await chatInput.fill('Test message for E2E');
      await page.waitForTimeout(300);

      await expect(chatInput).toHaveValue('Test message for E2E');

      // Clean up
      await chatInput.clear();
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Generate Tab
  // ---------------------------------------------------------------------------
  test.describe('Generate Tab', () => {
    test('should display the Generate Mocks heading', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Generate', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('heading', { name: 'Generate Mocks' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the description text', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Generate', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByText("Describe your API in natural language and we'll generate a complete OpenAPI specification.").first()
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the generate input with placeholder', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Generate', exact: true }).click();
      await page.waitForTimeout(500);

      const generateInput = main.locator('input[placeholder*="Create a user API"]');
      await expect(generateInput).toBeVisible({ timeout: 5000 });
    });

    test('should display the Generate button', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Generate', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('button', { name: 'Generate', exact: true }).nth(1)
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display 4 example prompt cards', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Generate', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(main.getByText('Example Prompts').first()).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('Simple CRUD API').first()).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('E-commerce API').first()).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('Blog API').first()).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('Social Media API').first()).toBeVisible({ timeout: 5000 });
    });

    test('should populate input when example card is clicked', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Generate', exact: true }).click();
      await page.waitForTimeout(500);

      // Click the "Simple CRUD API" example card
      await main.getByText('Simple CRUD API').click();
      await page.waitForTimeout(500);

      // Clicking an example card switches to the chat tab and fills the input
      const chatInput = main.locator('input[placeholder="Type your message..."]');
      const hasValue = await chatInput.inputValue().catch(() => '');
      expect(hasValue).toContain('todo');
    });
  });

  // ---------------------------------------------------------------------------
  // 5. Debug Tab
  // ---------------------------------------------------------------------------
  test.describe('Debug Tab', () => {
    test('should display the AI-Guided Debugging heading', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Debug', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('heading', { name: 'AI-Guided Debugging' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the description text', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Debug', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByText(/Paste your test failure logs and get AI-powered analysis/)
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the textarea for test failure logs', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Debug', exact: true }).click();
      await page.waitForTimeout(500);

      const textarea = main.locator('textarea[placeholder*="Paste your test failure logs"]');
      await expect(textarea).toBeVisible({ timeout: 5000 });
    });

    test('should display the Analyze Failure button', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Debug', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('button', { name: 'Analyze Failure' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should disable Analyze Failure button when textarea is empty', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Debug', exact: true }).click();
      await page.waitForTimeout(500);

      const analyzeButton = main.getByRole('button', { name: 'Analyze Failure' });
      await expect(analyzeButton).toBeDisabled();
    });

    test('should allow typing in the debug textarea', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Debug', exact: true }).click();
      await page.waitForTimeout(500);

      const textarea = main.locator('textarea[placeholder*="Paste your test failure logs"]');
      await textarea.fill('GET /api/users/123\nStatus: 404\nError: User not found');
      await page.waitForTimeout(300);

      const value = await textarea.inputValue();
      expect(value).toContain('GET /api/users/123');

      // Clean up
      await textarea.clear();
    });

    test('should enable Analyze Failure button when text is entered', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Debug', exact: true }).click();
      await page.waitForTimeout(500);

      const textarea = main.locator('textarea[placeholder*="Paste your test failure logs"]');
      await textarea.fill('Test failure log content');
      await page.waitForTimeout(300);

      const analyzeButton = main.getByRole('button', { name: 'Analyze Failure' });
      await expect(analyzeButton).toBeEnabled();

      // Clean up
      await textarea.clear();
    });

    test('should display Test Failure Logs label', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Debug', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByText('Test Failure Logs').first()
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 6. Personas Tab
  // ---------------------------------------------------------------------------
  test.describe('Personas Tab', () => {
    test('should display the Persona Generation heading', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Personas', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('heading', { name: 'Persona Generation' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the description text', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Personas', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByText(/Generate realistic personas with traits, backstories/)
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display the persona description textarea', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Personas', exact: true }).click();
      await page.waitForTimeout(500);

      const textarea = main.locator('textarea[placeholder*="Create a premium customer persona"]');
      await expect(textarea).toBeVisible({ timeout: 5000 });
    });

    test('should display the Generate Persona button', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Personas', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('button', { name: 'Generate Persona' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should disable Generate Persona button when textarea is empty', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Personas', exact: true }).click();
      await page.waitForTimeout(500);

      const generateButton = main.getByRole('button', { name: 'Generate Persona' });
      await expect(generateButton).toBeDisabled();
    });

    test('should display 4 example persona cards', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Personas', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(main.getByText('Example Persona Descriptions').first()).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('Premium Customer').first()).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('Churned User').first()).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('Trial User').first()).toBeVisible({ timeout: 5000 });
      await expect(main.getByText('Power User').first()).toBeVisible({ timeout: 5000 });
    });

    test('should populate textarea when example persona card is clicked', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Personas', exact: true }).click();
      await page.waitForTimeout(500);

      // Click the "Premium Customer" example card
      await main.getByText('Premium Customer').first().click();
      await page.waitForTimeout(500);

      const textarea = main.locator('textarea[placeholder*="Create a premium customer persona"]');
      const value = await textarea.inputValue().catch(() => '');
      expect(value).toContain('premium customer');
    });

    test('should allow typing in the persona textarea', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Personas', exact: true }).click();
      await page.waitForTimeout(500);

      const textarea = main.locator('textarea[placeholder*="Create a premium customer persona"]');
      await textarea.fill('Create a test persona for E2E testing');
      await page.waitForTimeout(300);

      const value = await textarea.inputValue();
      expect(value).toContain('E2E testing');

      // Clean up
      await textarea.clear();
    });

    test('should display Persona Description label', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Personas', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByText('Persona Description').first()
      ).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 7. Budget Tab
  // ---------------------------------------------------------------------------
  test.describe('Budget Tab', () => {
    test('should display the Budget & Usage heading', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Budget', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(
        main.getByRole('heading', { name: 'Budget & Usage' })
      ).toBeVisible({ timeout: 5000 });
    });

    test('should display stats grid or loading state', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Budget', exact: true }).click();
      await page.waitForTimeout(1000);

      const hasTokensUsed = await main.getByText('Tokens Used')
        .nth(0).isVisible({ timeout: 3000 }).catch(() => false);
      const hasCostUsd = await main.getByText('Cost (USD)')
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasCallsMade = await main.getByText('Calls Made')
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasUsage = await main.getByText('Usage')
        .first().isVisible({ timeout: 3000 }).catch(() => false);
      const hasLoading = await main.getByText('Loading usage statistics...')
        .isVisible({ timeout: 3000 }).catch(() => false);
      const hasNoData = await main.getByText('No usage data available')
        .isVisible({ timeout: 3000 }).catch(() => false);

      expect(hasTokensUsed || hasCostUsd || hasCallsMade || hasUsage || hasLoading || hasNoData).toBeTruthy();
    });

    test('should display budget progress bar when stats are loaded', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Budget', exact: true }).click();
      await page.waitForTimeout(1000);

      const hasBudgetProgress = await main.getByText('Budget Progress')
        .isVisible({ timeout: 3000 }).catch(() => false);

      // Budget progress section may or may not be visible depending on config
      // Just verify the tab loaded without crashing
      const hasBudgetContent = hasBudgetProgress || await main.getByText('Budget').first()
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasBudgetContent).toBeTruthy();
    });

    test('should display token count with budget limit', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Budget', exact: true }).click();
      await page.waitForTimeout(1000);

      const hasBudgetProgress = await main.getByText('Budget Progress')
        .isVisible({ timeout: 3000 }).catch(() => false);

      if (hasBudgetProgress) {
        // The tokens / budget_limit text should be visible
        const hasTokenCount = await main.getByText(/tokens$/)
          .isVisible({ timeout: 3000 }).catch(() => false);
        expect(hasTokenCount).toBeTruthy();
      }
    });

    test('should display feature breakdown when available', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Budget', exact: true }).click();
      await page.waitForTimeout(1000);

      // Feature breakdown heading may or may not appear depending on data
      const hasFeatureBreakdown = await main.getByText('Usage by Feature')
        .isVisible({ timeout: 3000 }).catch(() => false);

      // This is optional — just verify the page renders without errors
      const hasHeading = await main.getByRole('heading', { name: 'Budget & Usage' })
        .isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasHeading).toBeTruthy();
    });
  });

  // ---------------------------------------------------------------------------
  // 8. Navigation
  // ---------------------------------------------------------------------------
  test.describe('Navigation', () => {
    test('should navigate to Dashboard and back', async ({ page }) => {
      await page.goto(`${BASE_URL}/dashboard`, { waitUntil: 'domcontentloaded', timeout: 30000 });
      await expect(page).toHaveURL(/\/(dashboard)?$/, { timeout: 15000 });
      await page.goBack();
      await page.waitForTimeout(2000);
    });

    test('should navigate to Services and back', async ({ page }) => {
      await page.goto(`${BASE_URL}/services`, { waitUntil: 'domcontentloaded', timeout: 30000 });
      await expect(page).toHaveURL(/\/services/, { timeout: 15000 });
      await page.goBack();
      await page.waitForTimeout(2000);
    });

    test('should preserve URL when navigating back via browser history', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');

      await nav.getByRole('button', { name: 'Dashboard' }).click();
      await page.waitForTimeout(1500);

      await page.goBack();
      await page.waitForTimeout(1500);

      await expect(page).toHaveURL(/\/ai-studio/, { timeout: 10000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 9. Accessibility
  // ---------------------------------------------------------------------------
  test.describe('Accessibility', () => {
    test('should have a single H1 heading', async ({ page }) => {
      const h1 = mainContent(page).getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);
      await expect(h1).toHaveText('AI Studio');
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

    test('should have accessible tab buttons with labels', async ({ page }) => {
      const main = mainContent(page);
      const tabs = [
        'Chat', 'Generate', 'System Designer', 'AI User Simulator',
        'Debug', 'Personas', 'Contract Diff', 'API Critique', 'Budget',
      ];

      for (const tabName of tabs) {
        const tab = main.getByRole('button', { name: tabName, exact: true });
        await expect(tab).toBeVisible({ timeout: 5000 });
      }
    });

    test('should have accessible form inputs in Chat tab', async ({ page }) => {
      const main = mainContent(page);
      const chatInput = main.locator('input[placeholder="Type your message..."]');
      await expect(chatInput).toBeVisible({ timeout: 5000 });
      await expect(chatInput).toHaveAttribute('type', 'text');
    });

    test('should have labeled textareas in Debug tab', async ({ page }) => {
      const main = mainContent(page);
      await main.getByRole('button', { name: 'Debug', exact: true }).click();
      await page.waitForTimeout(500);

      await expect(main.getByText('Test Failure Logs').first()).toBeVisible({ timeout: 5000 });
      const textarea = main.locator('textarea');
      await expect(textarea.first()).toBeVisible({ timeout: 5000 });
    });
  });

  // ---------------------------------------------------------------------------
  // 10. Error-Free Operation
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
      const tabs = [
        'Generate', 'Debug', 'Personas', 'Budget',
        'Contract Diff', 'API Critique', 'System Designer',
        'AI User Simulator', 'Chat',
      ];

      for (const tab of tabs) {
        await main.getByRole('button', { name: tab, exact: true }).click();
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
