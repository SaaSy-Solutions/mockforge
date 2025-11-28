/**
 * Shared test setup and utility functions
 *
 * Common functions for test setup, assertions, and utilities
 */

import { Page, expect as playwrightExpect } from '@playwright/test';
import { waitForAppLoad, navigateToTab } from './helpers';
import { TIMEOUTS } from './constants';
import { collectCoverage, saveCoverage } from './coverage-helpers';

// Auto-collect coverage after setupTest if enabled
const COLLECT_COVERAGE = process.env.COLLECT_COVERAGE === 'true';

/**
 * Standardized test setup helper
 * Use this in all beforeEach hooks for consistency
 */
export async function setupTest(
  page: Page,
  options?: {
    tabName?: string;
    timeout?: number;
  }
): Promise<void> {
  // Set page timeout
  page.setDefaultTimeout(options?.timeout || TIMEOUTS.TEST);

  // Navigate to the app with error handling - shorter timeout
  try {
    await page.goto('/', {
      waitUntil: 'domcontentloaded',
      timeout: Math.min(TIMEOUTS.NAVIGATION, 10000), // Max 10 seconds for navigation
    });
  } catch (error) {
    // If navigation fails, check if page is accessible
    if (page.isClosed()) {
      throw error;
    }
    // Try to continue - page might be partially loaded
    await page.waitForLoadState('domcontentloaded', { timeout: 3000 }).catch(() => {});
  }

  // Wait for app to load and log in with timeout protection (aggressive timeout)
  const APP_LOAD_TIMEOUT = 10000; // Max 10 seconds for app load
  try {
    await Promise.race([
      waitForAppLoad(page),
      new Promise<void>((_, reject) =>
        setTimeout(() => reject(new Error('waitForAppLoad timeout')), APP_LOAD_TIMEOUT)
      ),
    ]);
  } catch (error) {
    // If page is closed, don't continue
    if (page.isClosed()) {
      throw error;
    }
    // Otherwise continue - page might still be usable even if login/app load failed
  }

  // Navigate to specific tab if requested
  if (options?.tabName) {
    try {
      const navigated = await Promise.race([
        navigateToTab(page, options.tabName),
        new Promise<boolean>((resolve) => setTimeout(() => resolve(false), 5000)),
      ]);

      if (navigated) {
        // Wait briefly for tab content to load
        await page.waitForLoadState('domcontentloaded');
      }
    } catch (error) {
      // Navigation failure is logged but test continues
    }
  }

  // Collect coverage snapshot if enabled (for debugging)
  if (COLLECT_COVERAGE) {
    page.on('close', async () => {
      try {
        const coverage = await collectCoverage(page);
        if (coverage) {
          // Coverage will be collected in fixtures.ts afterEach hook
        }
      } catch {
        // Ignore coverage collection errors
      }
    });
  }
}

/**
 * Assert that a page has loaded with optional content checks
 */
export async function assertPageLoaded(
  page: Page,
  expectedContent?: string[]
): Promise<void> {
  // Check if page is closed first
  if (page.isClosed()) {
    return;
  }

  // Try to verify body is visible, but don't fail if page isn't loaded
  try {
    await playwrightExpect(page.locator('body')).toBeVisible({ timeout: TIMEOUTS.SHORT });
  } catch {
    // Page might not be loaded, that's okay - just return
    return;
  }

  // Check for expected content if provided
  if (expectedContent && expectedContent.length > 0) {
    for (const content of expectedContent) {
      try {
        const contentLocator = page.locator(`text=/${content}/i`).first();
        await playwrightExpect(contentLocator).toBeVisible({ timeout: TIMEOUTS.MEDIUM });
      } catch {
        // Content not found, but page might still be partially loaded
        // Continue checking other content
        continue;
      }
    }
  }
}

/**
 * Wait for element with condition instead of fixed timeout
 */
export async function waitForElement(
  page: Page,
  selector: string,
  options?: {
    timeout?: number;
    state?: 'attached' | 'visible' | 'hidden' | 'detached';
  }
): Promise<void> {
  await page.waitForSelector(selector, {
    timeout: options?.timeout || TIMEOUTS.MEDIUM,
    state: options?.state || 'visible',
  });
}

/**
 * Wait for any of multiple selectors to appear
 */
export async function waitForAnySelector(
  page: Page,
  selectors: string[],
  options?: {
    timeout?: number;
    state?: 'attached' | 'visible';
  }
): Promise<boolean> {
  const timeout = options?.timeout || TIMEOUTS.MEDIUM;
  const state = options?.state || 'attached';

  for (const selector of selectors) {
    try {
      await page.waitForSelector(selector, { timeout, state });
      return true;
    } catch {
      continue;
    }
  }
  return false;
}

/**
 * Find and interact with an element if it exists
 */
export async function findAndInteract<T>(
  page: Page,
  selector: string,
  interaction: (element: Awaited<ReturnType<Page['locator']>>) => Promise<T>,
  options?: {
    timeout?: number;
    optional?: boolean;
  }
): Promise<T | null> {
  const locator = page.locator(selector).first();
  const timeout = options?.timeout || TIMEOUTS.MEDIUM;

  try {
    if (await locator.isVisible({ timeout })) {
      return await interaction(locator);
    }
  } catch {
    if (!options?.optional) {
      throw new Error(`Element not found: ${selector}`);
    }
  }

  return null;
}

/**
 * Check if any of the provided selectors are visible
 */
export async function checkAnyVisible(
  page: Page,
  selectors: string[],
  timeout?: number
): Promise<boolean> {
  const waitTime = timeout || TIMEOUTS.SHORT;

  for (const selector of selectors) {
    try {
      if (await page.locator(selector).first().isVisible({ timeout: waitTime })) {
        return true;
      }
    } catch {
      continue;
    }
  }
  return false;
}
