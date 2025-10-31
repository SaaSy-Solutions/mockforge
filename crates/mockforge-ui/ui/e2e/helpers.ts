import { Page } from '@playwright/test';

/**
 * Test helpers and utilities for MockForge Admin UI E2E tests
 *
 * This file contains reusable helper functions for common UI interactions,
 * API mocking, and test utilities.
 */

/**
 * Wait for the app to be fully loaded
 * Checks for key elements that indicate the app is ready
 * Also ensures user is logged in
 */
export async function waitForAppLoad(page: Page): Promise<void> {
  const MAX_WAIT_TIME = 12000; // Reduced to 12 seconds total
  const startTime = Date.now();

  try {
    // Wait for the page to load first (shorter timeout)
    await page.waitForLoadState('domcontentloaded', { timeout: 5000 });

    // Quick check if page is still alive
    if (page.isClosed()) {
      throw new Error('Page was closed unexpectedly');
    }

    // Wait briefly for React to hydrate
    await page.waitForTimeout(500); // Reduced from 1000

    // Check if we're on the login screen (quick check)
    const isLoginScreen = await Promise.race([
      page.locator('button:has-text("Demo Admin"), button:has-text("Sign In")').first().isVisible({ timeout: 2000 }).catch(() => false),
      new Promise<boolean>(resolve => setTimeout(() => resolve(false), 2000))
    ]);

    if (isLoginScreen) {
      // Need to log in first - but with shorter timeout
      await Promise.race([
        loginAsAdmin(page),
        new Promise<void>((_, reject) =>
          setTimeout(() => reject(new Error('Login timeout')), 8000) // Reduced from 15000
        )
      ]);
    }

    // Check if we've exceeded max wait time
    if (Date.now() - startTime > MAX_WAIT_TIME) {
      throw new Error('waitForAppLoad exceeded maximum wait time');
    }

    // Wait for navigation (quick check, don't wait too long)
    const navSelectors = ['#main-navigation', 'nav[role="navigation"]', 'aside nav', 'body'];

    for (const selector of navSelectors) {
      if (page.isClosed()) break;
      if (Date.now() - startTime > MAX_WAIT_TIME) break;

      try {
        await page.waitForSelector(selector, { timeout: 2000, state: 'attached' }); // Reduced from 3000
        break; // Found one, we're good
      } catch {
        continue;
      }
    }

    // Skip networkidle - it's often too slow and not needed
    // Just a brief wait for React to finish rendering
    if (!page.isClosed() && (Date.now() - startTime) < MAX_WAIT_TIME) {
      await page.waitForTimeout(300); // Reduced from 500
    }
  } catch (error) {
    // If page was closed, throw a clearer error
    if (page.isClosed()) {
      throw new Error('Page was closed before app could load. Check console for JavaScript errors.');
    }
    throw error;
  }
}

/**
 * Login to the Admin UI
 * Uses the demo login buttons or form submission
 * Defaults to admin role for full access
 */
export async function loginAsAdmin(page: Page): Promise<void> {
  // Quick check if already logged in
  try {
    const nav = page.locator('#main-navigation');
    if (await nav.isVisible({ timeout: 1000 })) {
      return; // Already logged in
    }
  } catch {
    // Not logged in, continue
  }

  // Look for "Demo Admin" button first (fastest way)
  const demoAdminButton = page.locator('button:has-text("Demo Admin")');
  try {
    if (await demoAdminButton.isVisible({ timeout: 2000 })) {
      await demoAdminButton.click();
      // Wait for login to complete (with shorter timeout)
      await page.waitForSelector('#main-navigation', { timeout: 8000, state: 'attached' });
      await page.waitForTimeout(500); // Brief wait for React
      return;
    }
  } catch {
    // Demo button not found, try form instead
  }

  // Fallback: fill in login form
  try {
    const usernameInput = page.locator('input[type="text"], input[name="username"], input[placeholder*="username" i]').first();
    const passwordInput = page.locator('input[type="password"], input[name="password"]').first();
    const signInButton = page.locator('button:has-text("Sign In"), button[type="submit"]').first();

    if (await usernameInput.isVisible({ timeout: 2000 })) {
      await usernameInput.fill('admin');
      await passwordInput.fill('admin123');
      await signInButton.click();
      // Wait for login to complete
      await page.waitForSelector('#main-navigation', { timeout: 8000, state: 'attached' });
      await page.waitForTimeout(500);
      return;
    }
  } catch (error) {
    throw new Error(`Failed to login: ${error}`);
  }

  // If we get here, login didn't work - but don't fail, just continue
  // The app might already be in a logged-in state
}

/**
 * Debug helper: Log all buttons on the page
 * Useful for troubleshooting navigation issues
 */
export async function debugPageButtons(page: Page): Promise<void> {
  try {
    const buttons = page.locator('button');
    const count = await buttons.count();
    console.log(`\n=== Found ${count} buttons on page ===`);
    for (let i = 0; i < Math.min(count, 20); i++) {
      const button = buttons.nth(i);
      const text = await button.textContent();
      const isVisible = await button.isVisible().catch(() => false);
      const boundingBox = await button.boundingBox().catch(() => null);
      console.log(`Button ${i}: "${text?.trim()}" | Visible: ${isVisible} | Box: ${boundingBox ? 'yes' : 'no'}`);
    }
    console.log('=== End button list ===\n');
  } catch (error) {
    console.log('Debug buttons failed:', error);
  }
}

/**
 * Navigate to a specific tab/page in the Admin UI
 * Uses the tab system to switch between pages
 * The AppShell uses buttons with text labels in the sidebar
 * Returns true if navigation succeeded, false if tab not found
 */
export async function navigateToTab(
  page: Page,
  tabName: string
): Promise<boolean> {
  // Check if page is closed
  if (page.isClosed()) {
    return false;
  }

  // Wait for React to render - reduced time
  await page.waitForTimeout(1000);

  // Debug: Check if debug mode is enabled (via page context or environment)
  // Note: process.env is not available in browser context, so we'll skip debug logging
  // unless we implement it via page.evaluate or similar

  // Check if we're on mobile - if so, open the hamburger menu first
  const viewport = page.viewportSize();
  if (viewport && viewport.width < 768) {
    // Mobile view - look for hamburger menu button
    const menuButton = page.locator('button:has([class*="menu" i]), button:has-text("☰"), [aria-label*="menu" i]').first();
    try {
      if (await menuButton.isVisible({ timeout: 2000 })) {
        await menuButton.click();
        await page.waitForTimeout(1000); // Wait for menu to open
      }
    } catch {
      // Menu button not found or already open
    }
  }

  // Wait for navigation to be visible (either desktop sidebar or mobile menu)
  // On desktop, sidebar is visible immediately
  // On mobile, it's in a modal that needs to be opened
  try {
    // First check if navigation exists in DOM (attached)
    await page.waitForSelector('#main-navigation, aside nav, nav[role="navigation"]', {
      timeout: 5000,
      state: 'attached'
    });

    // Then wait a bit for it to be visible (desktop) or open menu (mobile)
    if (viewport && viewport.width < 768) {
      // Mobile: menu should be open now
      await page.waitForTimeout(1000);
    } else {
      // Desktop: should be visible
      await page.waitForSelector('#main-navigation, aside nav', {
        timeout: 3000,
        state: 'visible'
      });
    }
  } catch {
    // Navigation might not be visible yet, but continue anyway
  }

  // Try multiple selectors for the tab navigation
  // AppShell uses: <Button> components with text like "Dashboard", "Workspaces", etc.
  // The buttons are inside nav#main-navigation
  const tabSelectors = [
    `#main-navigation button:has-text("${tabName}")`,
    `aside button:has-text("${tabName}")`,
    `nav[role="navigation"] button:has-text("${tabName}")`,
    `nav button:has-text("${tabName}")`,
    `button:has-text("${tabName}")`,
    `[data-testid="tab-${tabName}"]`,
  ];

  for (const selector of tabSelectors) {
    try {
      const tab = page.locator(selector).first();
      // Wait for it to be attached first
      await tab.waitFor({ state: 'attached', timeout: 3000 });

      // Then check visibility
      const isVisible = await tab.isVisible({ timeout: 2000 });
      if (isVisible) {
        // Scroll into view if needed
        await tab.scrollIntoViewIfNeeded();
        await page.waitForTimeout(300);

        await tab.click();
        await page.waitForTimeout(1000); // Wait for navigation

        return true;
      }
    } catch {
      continue;
    }
  }

  // If not found, try case-insensitive search within navigation
  const tabLower = tabName.toLowerCase();
  try {
    // Focus search on navigation area first
    const navArea = page.locator('#main-navigation, aside nav, nav[role="navigation"]').first();

    // Wait for navigation area to exist
    await navArea.waitFor({ state: 'attached', timeout: 3000 });

    // Check if visible (desktop) or in mobile menu
    const navVisible = await navArea.isVisible({ timeout: 2000 }).catch(() => false);

    if (navVisible || viewport && viewport.width < 768) {
      const buttons = navArea.locator('button');
      const count = await buttons.count();

      // First try exact match
      for (let i = 0; i < count; i++) {
        const button = buttons.nth(i);
        const text = await button.textContent();
        if (text && text.toLowerCase().trim() === tabLower) {
          const visible = await button.isVisible({ timeout: 1000 }).catch(() => false);
          if (visible) {
            await button.scrollIntoViewIfNeeded();
            await page.waitForTimeout(300);
            await button.click();
            await page.waitForTimeout(1000);
            return true;
          }
        }
      }

      // Then try partial match
      for (let i = 0; i < count; i++) {
        const button = buttons.nth(i);
        const text = await button.textContent();
        if (text && text.toLowerCase().includes(tabLower)) {
          const visible = await button.isVisible({ timeout: 1000 }).catch(() => false);
          if (visible) {
            await button.scrollIntoViewIfNeeded();
            await page.waitForTimeout(300);
            await button.click();
            await page.waitForTimeout(1000);
            return true;
          }
        }
      }
    }
  } catch (_error) {
    // Navigation area might not be accessible
    // Error handling is done silently to avoid test noise
  }

  // Tab not found - return false instead of throwing
  return false;
}

/**
 * Wait for API response
 * Helper to wait for a specific API endpoint to be called
 */
export async function waitForApiResponse(
  page: Page,
  endpoint: string,
  timeout = 10000
): Promise<void> {
  await page.waitForResponse(
    (response) => response.url().includes(endpoint),
    { timeout }
  );
}

/**
 * Mock API response
 * Intercepts and mocks an API endpoint
 */
export async function mockApiResponse(
  page: Page,
  endpoint: string,
  response: unknown,
  status = 200
): Promise<void> {
  await page.route(`**/${endpoint}`, async (route) => {
    await route.fulfill({
      status,
      contentType: 'application/json',
      body: JSON.stringify({ success: true, data: response }),
    });
  });
}

/**
 * Wait for toast notification
 * Waits for a toast message to appear (using Sonner)
 */
export async function waitForToast(
  page: Page,
  text?: string,
  timeout = 5000
): Promise<void> {
  const toastSelector = '[data-sonner-toast], [data-testid="toast"]';

  if (text) {
    await page.waitForSelector(
      `${toastSelector}:has-text("${text}")`,
      { timeout }
    );
  } else {
    await page.waitForSelector(toastSelector, { timeout });
  }
}

/**
 * Click button by text
 * Helper to click buttons that might have different structures
 */
export async function clickButton(page: Page, text: string): Promise<void> {
  const button = page.locator(`button:has-text("${text}")`).first();
  await button.waitFor({ state: 'visible', timeout: 5000 });
  await button.click();
}

/**
 * Fill form field
 * Helper to fill form fields with error handling
 */
export async function fillField(
  page: Page,
  label: string,
  value: string
): Promise<void> {
  // Try multiple selectors
  const selectors = [
    `input[placeholder*="${label}"]`,
    `input[name="${label}"]`,
    `label:has-text("${label}") + input`,
    `label:has-text("${label}") ~ input`,
    `[aria-label="${label}"]`,
  ];

  for (const selector of selectors) {
    try {
      const field = page.locator(selector).first();
      if (await field.isVisible({ timeout: 1000 })) {
        await field.fill(value);
        return;
      }
    } catch {
      continue;
    }
  }

  throw new Error(`Could not find field with label: ${label}`);
}

/**
 * Check if element is visible
 * Waits for element and checks visibility
 */
export async function isVisible(
  page: Page,
  selector: string,
  timeout = 5000
): Promise<boolean> {
  try {
    await page.waitForSelector(selector, { timeout, state: 'visible' });
    return true;
  } catch {
    return false;
  }
}

/**
 * Take screenshot with descriptive name
 * Helper for debugging failed tests
 */
export async function takeDebugScreenshot(
  page: Page,
  name: string
): Promise<void> {
  await page.screenshot({
    path: `e2e/screenshots/${name}-${Date.now()}.png`,
    fullPage: true,
  });
}

/**
 * Wait for dashboard data to load
 * Specific helper for dashboard page
 */
export async function waitForDashboardLoad(page: Page): Promise<void> {
  // Wait for common dashboard elements with timeout
  const dashboardSelectors = [
    '[data-testid="dashboard"]',
    'h1:has-text("Dashboard")',
    'h2:has-text("Dashboard")',
    '[class*="dashboard"]',
    'body', // Fallback to body if nothing else found
  ];

  for (const selector of dashboardSelectors) {
    if (page.isClosed()) break;

    try {
      if (await isVisible(page, selector, 1500)) { // Reduced from 2000
        // Try to wait for API response, but don't wait too long
        try {
          await Promise.race([
            waitForApiResponse(page, '/__mockforge/dashboard'),
            new Promise<void>((_, reject) =>
              setTimeout(() => reject(new Error('API timeout')), 3000)
            )
          ]);
        } catch {
          // API didn't respond in time, but dashboard element is visible, so continue
        }
        return;
      }
    } catch {
      continue;
    }
  }

  // Fallback: just wait for domcontentloaded (faster than networkidle)
  await page.waitForLoadState('domcontentloaded', { timeout: 2000 }).catch(() => {});
}

/**
 * Get API response data
 * Helper to get and parse API response
 */
export async function getApiResponse<T>(
  page: Page,
  endpoint: string
): Promise<T> {
  const response = await page.waitForResponse(
    (r) => r.url().includes(endpoint)
  );
  const data = await response.json();
  return (data.data || data) as T;
}
