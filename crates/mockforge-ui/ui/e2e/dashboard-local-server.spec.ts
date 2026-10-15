import { test, expect, type Page } from '@playwright/test';

// Changes instance-wide settings: run against a disposable loopback backend.
// Default mode is sequential within the file, but continues after failures.
test.describe('Local dashboard simulation', () => {
  test.describe.configure({ mode: 'default' });
  let originalLevel: number | undefined;
  let originalTime: { enabled: boolean; current_time?: string; scale_factor: number } | undefined;
  const main = (page: Page) => page.getByRole('main');
  const simulation = (page: Page) => main(page).locator('section').filter({
    has: page.getByRole('heading', { name: 'Simulation', level: 2, exact: true }),
  });
  const clock = async (page: Page) => (await page.request.get('/__mockforge/time-travel/status')).json();
  const reality = async (page: Page) => (await page.request.get('/__mockforge/reality/level')).json();

  test.beforeEach(async ({ page, baseURL }) => {
    originalLevel = undefined;
    originalTime = undefined;
    expect(['localhost', '127.0.0.1', '[::1]']).toContain(new URL(baseURL ?? 'http://localhost:5173').hostname);
    const realityResponse = await page.request.get('/__mockforge/reality/level');
    expect(realityResponse.status(), 'local reality API must be available').toBe(200);
    originalLevel = (await realityResponse.json()).level;
    const temporal = await page.request.get('/__mockforge/time-travel/status');
    expect(temporal.status(), 'local clock API must be available').toBe(200);
    originalTime = await temporal.json();
    expect((await page.request.put('/__mockforge/reality/level', { data: { level: 1 } })).status()).toBe(200);
    expect((await page.request.post('/__mockforge/time-travel/disable')).status()).toBe(200);
    await page.goto('/dashboard', { waitUntil: 'domcontentloaded' });
    await expect(main(page).getByRole('heading', { name: 'Dashboard', level: 1, exact: true })).toBeVisible();
    await expect(simulation(page).getByRole('heading', { name: 'Reality Slider', level: 3, exact: true })).toBeVisible();
    await expect(simulation(page).getByRole('slider')).toHaveValue('1');
  });

  test.afterEach(async ({ page }) => {
    if (originalLevel !== undefined) {
      expect((await page.request.put('/__mockforge/reality/level', { data: { level: originalLevel } })).status()).toBe(200);
    }
    if (originalTime?.enabled) {
      expect((await page.request.post('/__mockforge/time-travel/enable', { data: {
        time: originalTime.current_time, scale: originalTime.scale_factor,
      } })).status()).toBe(200);
    } else if (originalTime) {
      expect((await page.request.post('/__mockforge/time-travel/disable')).status()).toBe(200);
    }
  });

  test('renders local traffic and process inventory', async ({ page }) => {
    await expect(main(page).getByText('Traffic, latency and health for this MockForge instance.', { exact: true })).toBeVisible();
    await expect(main(page).getByRole('heading', { name: 'Servers & traffic', level: 2, exact: true })).toBeVisible();
    for (const label of ['Uptime', 'CPU', 'Memory', 'Threads', 'Version']) {
      await expect(main(page).locator('dt').filter({ hasText: new RegExp(`^${label}$`) })).toBeVisible();
    }
    await expect(main(page).getByText('Last 100 requests', { exact: true })).toBeVisible();
  });

  test('renders both widgets, five levels and the current value', async ({ page }) => {
    await expect(simulation(page).getByRole('heading', { name: 'Time Travel', level: 3, exact: true })).toBeVisible();
    for (let level = 1; level <= 5; level++) {
      await expect(simulation(page).getByRole('button', { name: String(level), exact: true })).toBeVisible();
    }
    await expect(simulation(page).getByText('Realism Level', { exact: true })).toBeVisible();
    await expect(simulation(page).getByText('1 / 5', { exact: true })).toBeVisible();
  });

  test('reveals each realism description on hover', async ({ page }) => {
    const names = ['Static Stubs', 'Light Simulation', 'Moderate Realism', 'High Realism', 'Production Chaos'];
    for (let index = 0; index < names.length; index++) {
      await simulation(page).getByRole('button', { name: String(index + 1), exact: true }).hover();
      await expect(simulation(page).getByText(names[index], { exact: true }).last()).toBeVisible();
    }
  });

  test('persists a quick-set realism level to the backend', async ({ page }) => {
    const response = page.waitForResponse(r => new URL(r.url()).pathname === '/__mockforge/reality/level' && r.request().method() === 'PUT');
    await simulation(page).getByRole('button', { name: '3', exact: true }).click();
    expect((await response).status()).toBe(200);
    await expect(simulation(page).getByRole('slider')).toHaveValue('3');
    await expect(simulation(page).getByText('3 / 5', { exact: true })).toBeVisible();
    expect((await reality(page)).level).toBe(3);
  });

  test('changes the range with the keyboard instead of unsupported fill', async ({ page }) => {
    const slider = simulation(page).getByRole('slider');
    await slider.focus();
    await slider.press('End');
    await expect(slider).toHaveValue('5');
    await expect.poll(async () => (await reality(page)).level).toBe(5);
    await slider.press('Home');
    await expect(slider).toHaveValue('1');
    await expect.poll(async () => (await reality(page)).level).toBe(1);
  });

  test('updates the level description and chaos indicators', async ({ page }) => {
    await simulation(page).getByRole('button', { name: '5', exact: true }).click();
    await expect(simulation(page).getByText('5 / 5', { exact: true })).toBeVisible();
    await expect(simulation(page).getByText('Production Chaos', { exact: true }).first()).toBeVisible();
    for (const label of ['Chaos', 'Latency', 'MockAI']) {
      await expect(simulation(page).getByText(label, { exact: true })).toBeVisible();
    }
  });

  test('shows real time while virtual time is disabled', async ({ page }) => {
    await expect(simulation(page).getByText('Using real time', { exact: true })).toBeVisible();
    await expect(simulation(page).getByText('Real Time', { exact: true })).toBeVisible();
    await expect(simulation(page).getByText(/[A-Z][a-z]{2} \d{1,2}, \d{4}/)).toBeVisible();
    await expect(simulation(page).getByRole('button', { name: 'Enable Time Travel', exact: true })).toBeVisible();
    expect((await clock(page)).enabled).toBe(false);
  });

  test('enables virtual time and returns to real time', async ({ page }) => {
    await simulation(page).getByRole('button', { name: 'Enable Time Travel', exact: true }).click();
    await expect(simulation(page).getByText('Virtual time active', { exact: true })).toBeVisible();
    await expect(simulation(page).getByRole('button', { name: 'Disable', exact: true })).toBeVisible();
    expect((await clock(page)).enabled).toBe(true);
    await simulation(page).getByRole('button', { name: 'Disable', exact: true }).click();
    await expect(simulation(page).getByText('Using real time', { exact: true })).toBeVisible();
    expect((await clock(page)).enabled).toBe(false);
  });

  test('advances virtual time through the quick control', async ({ page }) => {
    await simulation(page).getByRole('button', { name: 'Enable Time Travel', exact: true }).click();
    await expect(simulation(page).getByRole('button', { name: '+1h', exact: true })).toBeVisible();
    const before = await clock(page);
    await simulation(page).getByRole('button', { name: '+1h', exact: true }).click();
    await expect.poll(async () => Date.parse((await clock(page)).current_time) - Date.parse(before.current_time)).toBeGreaterThan(3_550_000);
    expect(Date.parse((await clock(page)).current_time) - Date.parse(before.current_time)).toBeLessThan(3_650_000);
  });
});
