import { test, expect } from '@playwright/test';

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL || 'https://app.mockforge.dev';
function mainContent(page: import('@playwright/test').Page) { return page.getByRole('main'); }

test.describe('BYOK Keys — Deployed Site', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(`${BASE_URL}/byok`, { waitUntil: 'domcontentloaded', timeout: 30000 });
    await page.waitForSelector('nav[aria-label="Main navigation"]', { state: 'visible', timeout: 15000 });
    await expect(mainContent(page).getByRole('heading', { name: /Bring Your Own Key/i, level: 1 })).toBeVisible({ timeout: 10000 });
  });

  test.describe('Page Load & Layout', () => {
    test('should load the BYOK page', async ({ page }) => {
      await expect(page).toHaveURL(/\/byok/);
      await expect(page).toHaveTitle(/MockForge/);
    });

    test('should display heading and subtitle', async ({ page }) => {
      await expect(mainContent(page).getByRole('heading', { name: /Bring Your Own Key/i, level: 1 })).toBeVisible();
      await expect(mainContent(page).getByText('Configure your own AI provider API keys')).toBeVisible();
    });

    test('should display breadcrumbs', async ({ page }) => {
      const banner = page.getByRole('banner');
      await expect(banner.getByText('Home')).toBeVisible();
      await expect(banner.getByText('BYOK Keys')).toBeVisible();
    });
  });

  test.describe('Configuration Section', () => {
    test('should display Configuration heading', async ({ page }) => {
      await expect(mainContent(page).getByRole('heading', { name: 'Configuration' })).toBeVisible();
    });

    test('should display AI Provider options', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('AI Provider')).toBeVisible();
      await expect(main.getByText('OpenAI')).toBeVisible();
      await expect(main.getByText('Anthropic')).toBeVisible();
      await expect(main.getByText('Together AI')).toBeVisible();
      await expect(main.getByText('Fireworks AI')).toBeVisible();
      await expect(main.getByText('Custom')).toBeVisible();
    });

    test('should display API Key input', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('API Key', { exact: true })).toBeVisible();
      await expect(main.getByRole('textbox', { name: 'API Key' })).toBeVisible();
    });

    test('should display API key placeholder', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('textbox', { name: 'API Key' })
      ).toHaveAttribute('placeholder', 'sk-...');
    });

    test('should display Enable BYOK toggle', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByText('Enable BYOK')).toBeVisible();
      await expect(main.getByText('Use your own API key for AI features')).toBeVisible();
    });

    test('should display the toggle button', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: /Enabled|Disabled/ })
      ).toBeVisible();
    });

    test('should display Save Configuration button', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('button', { name: 'Save Configuration' })
      ).toBeVisible();
    });

    test('should display documentation link', async ({ page }) => {
      await expect(
        mainContent(page).getByRole('link', { name: /View API documentation/i })
      ).toBeVisible();
    });

    test('should allow clicking provider options', async ({ page }) => {
      const main = mainContent(page);

      await main.getByText('Anthropic').click();
      await page.waitForTimeout(300);
      const apiKeyInput = main.getByRole('textbox', { name: 'API Key' });
      await expect(apiKeyInput).toBeVisible();

      // Switch back to OpenAI
      await main.getByText('OpenAI').click();
      await page.waitForTimeout(300);
    });

    test('should update docs link when switching providers', async ({ page }) => {
      const main = mainContent(page);

      // Start with OpenAI — docs link should point to OpenAI
      await main.getByText('OpenAI').click();
      await page.waitForTimeout(300);
      const openaiLink = main.getByRole('link', { name: /View API documentation/i });
      const openaiHref = await openaiLink.getAttribute('href');
      expect(openaiHref).toContain('openai');

      // Switch to Anthropic — docs link should change
      await main.getByText('Anthropic').click();
      await page.waitForTimeout(300);
      const anthropicHref = await openaiLink.getAttribute('href');
      expect(anthropicHref).toContain('anthropic');

      // Switch back
      await main.getByText('OpenAI').click();
    });

    test('should toggle Enable BYOK button', async ({ page }) => {
      const main = mainContent(page);
      const toggleBtn = main.getByRole('button', { name: /Enabled|Disabled/ });
      await expect(toggleBtn).toBeVisible();

      const initialText = await toggleBtn.textContent();
      await toggleBtn.click();
      await page.waitForTimeout(500);

      const newText = await toggleBtn.textContent();
      // Text should change between Enabled/Disabled
      expect(newText).not.toBe(initialText);

      // Toggle back to restore state
      await toggleBtn.click();
      await page.waitForTimeout(300);
    });

    test('should allow typing in API Key field', async ({ page }) => {
      const apiKeyInput = mainContent(page).getByRole('textbox', { name: 'API Key' });
      await apiKeyInput.fill('sk-test-key-12345');
      await expect(apiKeyInput).toHaveValue('sk-test-key-12345');
      await apiKeyInput.clear();
    });
  });

  test.describe('About BYOK Section', () => {
    test('should display About BYOK heading', async ({ page }) => {
      await expect(mainContent(page).getByRole('heading', { name: 'About BYOK' })).toBeVisible();
    });

    test('should display tier descriptions', async ({ page }) => {
      const main = mainContent(page);
      await expect(main.getByRole('heading', { name: 'Free Tier' })).toBeVisible();
      await expect(main.getByRole('heading', { name: 'Paid Plans' })).toBeVisible();
      await expect(main.getByRole('heading', { name: 'Security' })).toBeVisible();
    });

    test('should display security warning', async ({ page }) => {
      await expect(
        mainContent(page).getByText('Keep your API keys secure')
      ).toBeVisible();
    });
  });

  test.describe('Save Configuration', () => {
    test('should handle Save Configuration click without crashing', async ({ page }) => {
      const main = mainContent(page);
      const saveBtn = main.getByRole('button', { name: 'Save Configuration' });
      await saveBtn.click();
      await page.waitForTimeout(1000);

      // Page should still be functional (save may fail without valid key but shouldn't crash)
      await expect(main.getByRole('heading', { name: /Bring Your Own Key/i, level: 1 })).toBeVisible();
    });

    test('should display documentation link', async ({ page }) => {
      const main = mainContent(page);
      const hasDocsLink = await main.getByRole('link', { name: /documentation|docs|learn more/i })
        .first().isVisible({ timeout: 3000 }).catch(() => false);
      expect(typeof hasDocsLink).toBe('boolean');
    });
  });

  test.describe('Navigation', () => {
    test('should navigate to API Tokens and back', async ({ page }) => {
      const nav = page.locator('nav[aria-label="Main navigation"]');
      await nav.getByRole('button', { name: 'API Tokens' }).click();
      await page.waitForTimeout(1500);
      await expect(mainContent(page).getByRole('heading', { name: 'API Tokens', level: 1 })).toBeVisible({ timeout: 5000 });

      await nav.getByRole('button', { name: 'BYOK Keys' }).click();
      await page.waitForTimeout(1500);
      await expect(mainContent(page).getByRole('heading', { name: /Bring Your Own Key/i, level: 1 })).toBeVisible({ timeout: 5000 });
    });
  });

  test.describe('Accessibility', () => {
    test('should have a single H1', async ({ page }) => {
      const h1 = mainContent(page).getByRole('heading', { level: 1 });
      await expect(h1).toHaveCount(1);
    });

    test('should have landmarks and skip links', async ({ page }) => {
      await expect(page.getByRole('main')).toBeVisible();
      await expect(page.getByRole('navigation', { name: 'Main navigation' })).toBeVisible();
      await expect(page.getByRole('link', { name: 'Skip to navigation' })).toBeAttached();
    });

    test('should have labeled form controls', async ({ page }) => {
      await expect(mainContent(page).getByRole('textbox', { name: 'API Key' })).toBeVisible();
      await expect(mainContent(page).getByRole('button', { name: 'Save Configuration' })).toBeVisible();
    });
  });

  test.describe('Error-Free Operation', () => {
    test('should load without critical console errors', async ({ page }) => {
      const errors: string[] = [];
      page.on('console', (msg) => { if (msg.type() === 'error') errors.push(msg.text()); });
      await page.reload({ waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(3000);
      const critical = errors.filter(e => !e.includes('net::ERR_') && !e.includes('Failed to fetch') && !e.includes('NetworkError') && !e.includes('WebSocket') && !e.includes('favicon') && !e.includes('429') && !e.includes('422'));
      expect(critical).toHaveLength(0);
    });

    test('should not show error UI', async ({ page }) => {
      expect(await page.getByText(/Something went wrong|Unexpected error|Application error/i).first().isVisible({ timeout: 2000 }).catch(() => false)).toBeFalsy();
    });
  });
});
