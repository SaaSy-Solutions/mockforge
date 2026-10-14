import { test, expect } from '@playwright/test';

// Cloud configuration is workspace-scoped. Local ports, fault injection,
// validation and process restart controls belong to config-local-server.spec.ts.
test.describe('Cloud workspace configuration', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/config', { waitUntil: 'domcontentloaded' });
    await expect(page.getByRole('main').getByRole('heading', {
      name: 'Configuration', level: 1, exact: true,
    })).toBeVisible();
  });

  test('offers the two workspace configuration sections', async ({ page }) => {
    const main = page.getByRole('main');
    await expect(main.getByRole('button', { name: /Reality Slider/ })).toBeVisible();
    await expect(main.getByRole('button', { name: /^Environment/ })).toBeVisible();
    for (const label of ['General', 'Protocols', 'Latency', 'Fault Injection',
      'Traffic Shaping', 'Proxy', 'Validation']) {
      await expect(main.getByRole('button', { name: new RegExp(`^${label}`) })).toHaveCount(0);
    }
  });

  test('does not expose local-server save and reset operations', async ({ page }) => {
    const main = page.getByRole('main');
    await expect(main.getByRole('button', { name: 'Reset All', exact: true })).toHaveCount(0);
    await expect(main.getByRole('button', { name: 'Save All Changes', exact: true })).toHaveCount(0);
  });

  test('switches between workspace sections without changing the route', async ({ page }) => {
    const main = page.getByRole('main');
    await main.getByRole('button', { name: /^Environment/ }).click();
    await expect(page).toHaveURL(/\/config(?:[?#]|$)/);
    await expect(main.getByRole('heading', { name: 'Environments & Variables', level: 2, exact: true })).toBeVisible();
    await main.getByRole('button', { name: /Reality Slider/ }).click();
    await expect(main.locator('[data-section="reality"]')).toBeVisible();
  });

  test('does not query local-server configuration endpoints', async ({ page }) => {
    const requested: string[] = [];
    page.on('request', request => {
      const path = new URL(request.url()).pathname;
      if (/^\/__mockforge\/(config|validation|server-info)(?:\/|$)/.test(path)) requested.push(path);
    });
    // Observe a complete reload; beforeEach requests are deliberately excluded.
    await page.reload({ waitUntil: 'domcontentloaded' });
    await expect(page.getByRole('main').getByRole('heading', {
      name: 'Configuration', level: 1, exact: true,
    })).toBeVisible();
    await expect(page.getByRole('main').getByRole('button', { name: /Reality Slider/ })).toBeVisible();
    expect(requested).toEqual([]);
  });
});
