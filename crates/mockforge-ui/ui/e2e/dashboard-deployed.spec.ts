import { test, expect, type Page, type Locator } from '@playwright/test';
import type { DashboardData } from '../src/types';

// Deployed dashboard is a cloud workspace view. Local listener metrics,
// request logs and simulation controls are covered by the local project.
const main = (page: Page) => page.getByRole('main');
const heading = (page: Page) => main(page).getByRole('heading', { name: 'Dashboard', level: 1, exact: true });
const panel = (page: Page, name: string) => main(page).locator('section').filter({
  has: page.getByRole('heading', { name, level: 2, exact: true }),
});
const metric = (page: Page, name: string) => main(page).getByText(name, { exact: true }).locator('..');

async function loadDashboard(page: Page): Promise<DashboardData> {
  const response = page.waitForResponse(r => new URL(r.url()).pathname === '/api/v1/dashboard'
    && r.request().method() === 'GET');
  await page.goto('/dashboard', { waitUntil: 'domcontentloaded' });
  const received = await response;
  expect(received.status(), 'cloud dashboard API must succeed').toBe(200);
  const data: DashboardData = await received.json();
  await expect(heading(page)).toBeVisible();
  await expect(main(page).getByText('Active deployments', { exact: true })).toBeVisible();
  return data;
}

async function expectActivityResult(activity: Locator) {
  // Organization activity can legitimately be empty; neither branch accepts
  // loading placeholders or unrelated page text as successful coverage.
  const table = activity.getByRole('table').filter({ has: activity.getByRole('columnheader', { name: 'Event', exact: true }) });
  await expect(async () => {
    if (await table.count()) {
      await expect(table).toBeVisible();
      for (const name of ['Event', 'Description', 'Time']) {
        await expect(table.getByRole('columnheader', { name, exact: true })).toBeVisible();
      }
    } else {
      await expect(activity.getByText('No recent activity', { exact: true })).toBeVisible();
    }
  }).toPass({ timeout: 10_000 });
}

test.describe('Cloud dashboard', () => {
  test.beforeEach(async ({ page }) => { await loadDashboard(page); });

  test('renders the cloud page and billing scope', async ({ page }) => {
    await expect(page).toHaveURL(/\/dashboard(?:[?#]|$)/);
    await expect(page).toHaveTitle(/MockForge/);
    await expect(main(page).getByText('Traffic and health across your hosted deployments.', { exact: true })).toBeVisible();
    await expect(panel(page, 'Response status').getByText('This billing period, all deployments', { exact: true })).toBeVisible();
  });

  test('renders every cloud KPI with a value', async ({ page }) => {
    for (const label of ['Active deployments', 'Requests', 'Avg response time', 'Egress']) {
      const card = metric(page, label);
      await expect(card).toBeVisible();
      await expect(card.locator('p.font-mono')).toHaveText(/\d/);
    }
  });

  test('shows deployment and request totals from the cloud API', async ({ page }) => {
    const data = await loadDashboard(page);
    await expect(metric(page, 'Active deployments').getByText((data.cloud_metrics?.active_deployments ?? 0).toLocaleString(), { exact: true })).toBeVisible();
    await expect(metric(page, 'Requests').getByText((data.metrics?.total_requests ?? 0).toLocaleString(), { exact: true })).toBeVisible();
    await expect(metric(page, 'Avg response time').getByText(`${Math.round(data.metrics?.average_response_time ?? 0)} ms`, { exact: true })).toBeVisible();
  });

  test('renders cloud system inventory and version', async ({ page }) => {
    const system = panel(page, 'System');
    for (const label of ['Workspaces', 'Services', 'Fixtures', 'Federations', 'Version']) {
      await expect(system.locator('dt').filter({ hasText: new RegExp(`^${label}$`) })).toBeVisible();
    }
    await expect(system.locator('dd')).toHaveCount(5);
    await expect(system.locator('dd').last()).not.toHaveText('');
  });

  test('does not expose local process or simulation controls', async ({ page }) => {
    for (const title of ['Simulation', 'Reality Slider', 'Time Travel', 'Servers & traffic', 'Response time distribution']) {
      await expect(main(page).getByRole('heading', { name: title, exact: true })).toHaveCount(0);
    }
    await expect(main(page).getByRole('slider')).toHaveCount(0);
    for (const label of ['CPU', 'Memory', 'Uptime', 'Threads']) {
      await expect(panel(page, 'System').locator('dt').filter({ hasText: new RegExp(`^${label}$`) })).toHaveCount(0);
    }
  });

  test('renders status counts or the explicit empty state', async ({ page }) => {
    const data = await loadDashboard(page);
    const counts = data.cloud_metrics;
    const total = (counts?.requests_2xx ?? 0) + (counts?.requests_4xx ?? 0) + (counts?.requests_5xx ?? 0);
    const status = panel(page, 'Response status');
    if (total === 0) {
      await expect(status.getByText('No responses recorded yet.', { exact: true })).toBeVisible();
    } else {
      for (const label of ['2xx Success', '4xx Client errors', '5xx Server errors']) {
        await expect(status.getByText(label, { exact: true })).toBeVisible();
      }
      await expect(status.getByRole('img', { name: /2xx Success.*4xx Client errors.*5xx Server errors/ })).toBeVisible();
    }
  });

  test('renders deployment inventory and recent organization activity', async ({ page }) => {
    const activity = panel(page, 'Deployments & activity');
    await expect(activity.getByRole('heading', { name: 'Server Instances', exact: true })).toBeVisible();
    await expect(activity.getByText('Recent Activity', { exact: true })).toBeVisible();
    await expectActivityResult(activity);
  });

  test('filters activity and restores the list when cleared', async ({ page }) => {
    const activity = panel(page, 'Deployments & activity');
    const search = activity.getByPlaceholder('Search event, description, IP…', { exact: true });
    await search.fill('qa-no-matching-event-9a6f18');
    await expect(activity.getByText('No recent activity', { exact: true })).toBeVisible();
    await search.clear();
    await expectActivityResult(activity);
  });

  test('shows derived health rather than a fixed operational claim', async ({ page }) => {
    const data = await loadDashboard(page);
    const servers = data.servers ?? [];
    const stopped = servers.filter(server => !server.running).length;
    const expected = servers.length > 0 && stopped === servers.length ? 'No servers running'
      : stopped > 0 ? `${stopped} of ${servers.length} servers stopped`
      : (data.metrics?.error_rate ?? 0) >= 5 ? 'Elevated error rate' : 'Healthy';
    await expect(panel(page, 'System').getByText(expected, { exact: true })).toBeVisible();
  });

  test('navigates to services and back without stale content', async ({ page }) => {
    const nav = page.getByRole('navigation', { name: 'Main navigation', exact: true });
    await nav.getByRole('link', { name: 'Services', exact: true }).click();
    await expect(page).toHaveURL(/\/services(?:[?#]|$)/);
    await expect(main(page).getByRole('heading', { name: 'Services', level: 1, exact: true })).toBeVisible();
    await expect(heading(page)).toHaveCount(0);
    await nav.getByRole('link', { name: 'Dashboard', exact: true }).click();
    await expect(heading(page)).toBeVisible();
    await expect(main(page).getByText('Active deployments', { exact: true })).toBeVisible();
  });

  test('has one page heading and the navigation landmarks', async ({ page }) => {
    await expect(main(page).getByRole('heading', { level: 1 })).toHaveCount(1);
    await expect(page.getByRole('navigation', { name: 'Main navigation', exact: true })).toBeVisible();
    await expect(page.getByRole('navigation', { name: 'Breadcrumb', exact: true }).getByText('Dashboard', { exact: true })).toBeVisible();
    await expect(main(page)).toHaveAttribute('id', 'main-content');
  });

  test('refreshes cloud metrics on browser reload', async ({ page }) => {
    const response = page.waitForResponse(r => new URL(r.url()).pathname === '/api/v1/dashboard');
    await page.reload({ waitUntil: 'domcontentloaded' });
    expect((await response).status()).toBe(200);
    await expect(heading(page)).toBeVisible();
  });

  test('opens global search, filters results and closes with Escape', async ({ page }) => {
    await page.getByRole('button', { name: 'Search', exact: true }).click();
    const input = page.getByPlaceholder('Search pages, workspaces and actions…', { exact: true });
    await expect(input).toBeVisible();
    await input.fill('qa-no-matching-page-9a6f18');
    await expect(page.getByText(/No results for/)).toBeVisible();
    await input.press('Escape');
    await expect(input).toHaveCount(0);
    await expect(heading(page)).toBeVisible();
  });

  test('toggles the theme and restores the original preference', async ({ page }) => {
    const originalDark = await page.locator('html').evaluate(element => element.classList.contains('dark'));
    const toggle = page.getByRole('button', { name: /^Switch to (dark|light) mode$/ });
    try {
      await toggle.click();
      await expect.poll(() => page.locator('html').evaluate(element => element.classList.contains('dark'))).toBe(!originalDark);
    } finally {
      const nowDark = await page.locator('html').evaluate(element => element.classList.contains('dark'));
      if (nowDark !== originalDark) await toggle.click();
    }
  });

  test('polls the cloud activity feed', async ({ page }) => {
    const response = await page.waitForResponse(r => new URL(r.url()).pathname === '/api/v1/dashboard/logs', { timeout: 12_000 });
    expect(response.status()).toBe(200);
    await expectActivityResult(panel(page, 'Deployments & activity'));
  });

  test('does not start local dashboard HTTP or WebSocket streams', async ({ page }) => {
    const requests: string[] = [];
    page.on('request', request => {
      const path = new URL(request.url()).pathname;
      if (/^\/__mockforge\/(dashboard|logs|ws)(?:\/|$)/.test(path)) requests.push(path);
    });
    page.on('websocket', socket => {
      const path = new URL(socket.url()).pathname;
      if (path === '/__mockforge/ws') requests.push(path);
    });
    await loadDashboard(page);
    await page.waitForResponse(r => new URL(r.url()).pathname === '/api/v1/dashboard/logs', { timeout: 12_000 });
    expect(requests).toEqual([]);
  });
});

test.describe('Cloud dashboard failure recovery', () => {
  test('shows an API failure and recovers through Retry', async ({ page }) => {
    await page.route('**/api/v1/dashboard', route => route.fulfill({ status: 503, json: { error: 'Synthetic dashboard unavailable' } }));
    await page.goto('/dashboard', { waitUntil: 'domcontentloaded' });
    await expect(main(page).getByRole('heading', { name: 'Failed to load dashboard', exact: true })).toBeVisible({ timeout: 20_000 });
    await page.unroute('**/api/v1/dashboard');
    await main(page).getByRole('button', { name: 'Try Again', exact: true }).click();
    await expect(main(page).getByText('Active deployments', { exact: true })).toBeVisible();
    await expect(main(page).getByRole('heading', { name: 'Failed to load dashboard', exact: true })).toHaveCount(0);
  });
});
