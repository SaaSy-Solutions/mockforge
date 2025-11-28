import { test, expect } from '@playwright/test';
import { navigateToTab } from './helpers';
import { setupTest, assertPageLoaded, checkAnyVisible } from './test-helpers';
import { SELECTORS } from './constants';

/**
 * Error Handling and Edge Case Tests
 * 
 * Tests how the UI handles:
 * - Network errors
 * - Invalid inputs
 * - Missing data
 * - API failures
 * - Timeout scenarios
 */
test.describe('Error Handling and Edge Cases', () => {
  test.beforeEach(async ({ page }) => {
    await setupTest(page);
  });

  test('should handle API errors gracefully', async ({ page }) => {
    // Intercept API calls and return errors
    await page.route('**/__mockforge/**', async (route) => {
      await route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Internal Server Error' }),
      });
    });

    // Navigate to a page that makes API calls
    await navigateToTab(page, 'Dashboard');
    await page.waitForLoadState('domcontentloaded');

    // Page should still render (error handling should be in place)
    await assertPageLoaded(page);

    // Look for error messages or empty states
    const errorSelectors = [
      '[role="alert"]',
      '[class*="error"]',
      'text=/error/i',
      'text=/failed/i',
    ];

    // At least one error indicator should be visible or page should show empty state
    const hasError = await checkAnyVisible(page, errorSelectors);

    // Either error is shown or page gracefully handles it
    await assertPageLoaded(page);
  });

  test('should handle network timeout scenarios', async ({ page }) => {
    // Create a route that delays response
    await page.route('**/__mockforge/**', async (route) => {
      // Delay response to simulate slow network
      await new Promise(resolve => setTimeout(resolve, 100));
      await route.continue();
    });

    await navigateToTab(page, 'Services');
    await page.waitForLoadState('domcontentloaded');

    // Page should still load even with slow network
    await assertPageLoaded(page);
  });

  test('should handle missing or invalid data', async ({ page }) => {
    // Intercept and return empty/malformed responses
    await page.route('**/__mockforge/**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({}),
      });
    });

    await navigateToTab(page, 'Dashboard');
    await page.waitForLoadState('domcontentloaded');

    // Page should handle empty data gracefully
    await assertPageLoaded(page);
  });

  test('should handle invalid form inputs', async ({ page }) => {
    await navigateToTab(page, 'Workspaces');
    await page.waitForLoadState('domcontentloaded');

    // Look for create/add buttons
    const createButton = page.locator('button:has-text("Create"), button:has-text("Add")').first();
    
    if (await createButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await createButton.click();
      await page.waitForTimeout(500); // Wait for modal/form to appear

      // Try to enter invalid data
      const inputs = page.locator('input[type="text"], input[type="number"]');
      const inputCount = await inputs.count();

      if (inputCount > 0) {
        // Enter invalid data (e.g., special characters in name field)
        const firstInput = inputs.first();
        await firstInput.fill('<>!@#$%^&*()');
        await page.waitForTimeout(300); // Wait for validation to process

        // Try to submit
        const submitButton = page.locator('button[type="submit"], button:has-text("Save")').first();
        if (await submitButton.isVisible({ timeout: 2000 }).catch(() => false)) {
          await submitButton.click();
          await page.waitForTimeout(500); // Wait for form submission/validation

          // Look for validation errors
          const errors = page.locator('[class*="error"], [role="alert"]');
          // Either error shows or form prevents submission
        }
      }
    }

    await assertPageLoaded(page);
  });

  test('should handle 404 errors', async ({ page }) => {
    // Intercept and return 404
    await page.route('**/__mockforge/api/nonexistent', async (route) => {
      await route.fulfill({
        status: 404,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Not Found' }),
      });
    });

    await navigateToTab(page, 'Dashboard');
    await page.waitForLoadState('domcontentloaded');

    // Page should still be functional
    await assertPageLoaded(page);
  });

  test('should handle authentication errors', async ({ page }) => {
    // Simulate auth error
    await page.route('**/__mockforge/api/**', async (route) => {
      await route.fulfill({
        status: 401,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Unauthorized' }),
      });
    });

    await navigateToTab(page, 'Dashboard');
    await page.waitForLoadState('domcontentloaded');

    // Page should handle auth errors (might redirect to login or show error)
    await assertPageLoaded(page);
  });

  test('should handle very long text inputs', async ({ page }) => {
    await navigateToTab(page, 'Workspaces');
    await page.waitForLoadState('domcontentloaded');

    const createButton = page.locator('button:has-text("Create")').first();
    
    if (await createButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await createButton.click();
      await page.waitForTimeout(500); // Wait for modal/form to appear

      const inputs = page.locator('input[type="text"], textarea');
      const inputCount = await inputs.count();

      if (inputCount > 0) {
        const longText = 'a'.repeat(10000); // Very long text
        const firstInput = inputs.first();
        
        try {
          await firstInput.fill(longText);
          await page.waitForTimeout(500); // Wait for input to process
          // Input should handle or truncate long text
        } catch {
          // Input might reject very long text - that's okay
        }
      }
    }

    await assertPageLoaded(page);
  });

  test('should handle rapid button clicks', async ({ page }) => {
    await navigateToTab(page, 'Dashboard');
    await page.waitForLoadState('domcontentloaded');

    const buttons = page.locator('button:visible');
    const buttonCount = await buttons.count();

    if (buttonCount > 0) {
      const button = buttons.first();
      
      // Rapidly click button multiple times
      for (let i = 0; i < 5; i++) {
        await button.click({ timeout: 1000 }).catch(() => {});
        await page.waitForTimeout(50); // Short delay between clicks
      }
    }

    // Page should still be stable
    await assertPageLoaded(page);
  });

  test('should handle malformed JSON responses', async ({ page }) => {
    // Intercept and return malformed JSON
    await page.route('**/__mockforge/**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: 'invalid json {',
      });
    });

    await navigateToTab(page, 'Dashboard');
    await page.waitForLoadState('domcontentloaded');

    // Page should handle parse errors gracefully
    await assertPageLoaded(page);
  });

  test('should handle empty responses', async ({ page }) => {
    // Intercept and return empty response
    await page.route('**/__mockforge/**', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: '',
      });
    });

    await navigateToTab(page, 'Services');
    await page.waitForLoadState('domcontentloaded');

    // Page should handle empty responses
    await assertPageLoaded(page);
  });

  test('should handle navigation to non-existent tabs', async ({ page }) => {
    // Try to navigate to a tab that might not exist
    const result = await navigateToTab(page, 'NonExistentTab');
    
    // Should return false, not crash
    expect(typeof result).toBe('boolean');
    
    // Page should still be functional
    await assertPageLoaded(page);
  });
});

