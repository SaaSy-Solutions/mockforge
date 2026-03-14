// Full sidebar sweep — verifies every page loads without crash
// Usage: paste into browser_run_code Playwright MCP tool
module.exports = async (page) => {
  await page.goto('https://app.mockforge.dev');
  await page.waitForTimeout(3000);

  const sidebar = page.locator('nav[aria-label="Main navigation"]');
  const buttons = sidebar.locator('button');
  const count = await buttons.count();

  const pageNames = [];
  for (let i = 0; i < count; i++) {
    const text = await buttons.nth(i).textContent();
    if (text && text.trim()) pageNames.push(text.trim());
  }

  const results = [];

  for (const pageName of pageNames) {
    await page.goto('https://app.mockforge.dev');
    await page.waitForTimeout(1500);

    try {
      const btn = page.getByRole('button', { name: pageName, exact: true });
      if ((await btn.count()) === 0) {
        results.push({ page: pageName, status: 'SKIP' });
        continue;
      }
      await btn.click();
      await page.waitForTimeout(2500);

      const main = page.locator('main');
      const text = await main.textContent();
      const crashed = text.includes('Something went wrong');

      results.push({ page: pageName, status: crashed ? 'CRASH' : 'PASS' });
    } catch (err) {
      results.push({ page: pageName, status: 'ERROR: ' + err.message.slice(0, 60) });
    }
  }

  const passed = results.filter((r) => r.status === 'PASS').length;
  const crashed = results.filter((r) => r.status === 'CRASH').length;
  const errors = results.filter((r) => r.status.startsWith('ERROR')).length;
  const skipped = results.filter((r) => r.status === 'SKIP').length;

  return {
    summary: `${passed} PASS, ${crashed} CRASH, ${errors} ERROR, ${skipped} SKIP out of ${results.length}`,
    failures: results.filter((r) => r.status !== 'PASS' && r.status !== 'SKIP'),
  };
};
