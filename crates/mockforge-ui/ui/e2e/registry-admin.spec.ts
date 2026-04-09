import { test, expect } from '@playwright/test';

/**
 * Registry admin smoke tests.
 *
 * These exercise the /registry-login and /registry-admin pages wired up
 * in Phase 6 (task #17). The backend at /api/admin/registry/* is
 * typically NOT mounted during local dev (it requires
 * MOCKFORGE_REGISTRY_DB_URL to be set), so these tests primarily verify
 * the UI renders, routes resolve, and the "backend unavailable" fallback
 * works when the store hasn't been bootstrapped.
 *
 * A full happy-path e2e (login -> dashboard -> create org -> invite) is
 * covered by the Rust-side integration tests in
 * crates/mockforge-ui/src/registry_admin.rs — those drive the router
 * end-to-end via tower::ServiceExt::oneshot without needing a live
 * browser, which is both faster and more hermetic.
 */
test.describe('Registry admin', () => {
  test('login page renders', async ({ page }) => {
    await page.goto('/registry-login');
    await expect(page.getByText('Registry admin sign in')).toBeVisible();
    await expect(page.getByLabel('Username or email')).toBeVisible();
    await expect(page.getByLabel('Password')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Sign in' })).toBeVisible();
  });

  test('login page shows backend-unavailable banner when /health 404s', async ({ page }) => {
    // Force the health check to fail so the "backend not enabled" banner
    // renders. This simulates running `mockforge serve --admin` WITHOUT
    // MOCKFORGE_REGISTRY_DB_URL set.
    await page.route('**/api/admin/registry/health', (route) =>
      route.fulfill({ status: 404, body: '' }),
    );
    await page.goto('/registry-login');
    await expect(
      page.getByText('The registry admin backend is not enabled on this server.'),
    ).toBeVisible();
    // Sign-in button should be disabled when the backend is unavailable.
    await expect(page.getByRole('button', { name: 'Sign in' })).toBeDisabled();
  });

  test('login form surfaces 401 errors from the backend', async ({ page }) => {
    await page.route('**/api/admin/registry/health', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ status: 'ok' }),
      }),
    );
    await page.route('**/api/admin/registry/auth/login', (route) =>
      route.fulfill({
        status: 401,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'invalid credentials', status: 401 }),
      }),
    );
    await page.goto('/registry-login');
    await page.getByLabel('Username or email').fill('wrong');
    await page.getByLabel('Password').fill('wrong-password');
    await page.getByRole('button', { name: 'Sign in' }).click();
    await expect(page.getByText('invalid credentials')).toBeVisible();
  });

  test('registry-admin redirects to login when no token is stored', async ({ page }) => {
    // Ensure localStorage is clean.
    await page.goto('/');
    await page.evaluate(() => localStorage.removeItem('mockforge_registry_admin_token'));
    await page.goto('/registry-admin');
    // The page sees no token and navigates back to /registry-login.
    await expect(page).toHaveURL(/\/registry-login$/);
  });
});
