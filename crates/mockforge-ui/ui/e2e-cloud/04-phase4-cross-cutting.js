// Phase 4: Cross-cutting verification
// Tests: Sidebar active highlight, responsive layout, sidebar collapse/expand, auth persistence
module.exports = async (page) => {
  const results = {};

  // ===== 4.2 Auth persistence =====
  await page.goto('https://app.mockforge.dev');
  await page.waitForTimeout(3000);
  let m = await page.locator('main').textContent();
  results['4.2_dashboard_loads'] = m.includes('Dashboard') || m.includes('System Metrics');

  // Navigate to 5 different pages, verify no re-login
  for (const pageName of ['Fixtures', 'Logs', 'Chaos Engineering', 'Billing', 'AI Studio']) {
    const btn = page.getByRole('button', { name: pageName, exact: true });
    await btn.click();
    await page.waitForTimeout(2000);
    m = await page.locator('main').textContent();
    const needsLogin = m.includes('Sign in') && !m.includes('Dashboard');
    results[`4.2_auth_${pageName.toLowerCase().replace(/\s+/g, '_')}`] = !needsLogin;
  }

  // ===== 4.3 Sidebar active highlight =====
  await page.goto('https://app.mockforge.dev');
  await page.waitForTimeout(2000);
  const dashBtn = page.getByRole('button', { name: 'Dashboard', exact: true });
  await dashBtn.click();
  await page.waitForTimeout(1500);
  // Check if Dashboard button has active/selected styling
  const dashClasses = await dashBtn.getAttribute('class');
  const dashAriaSelected = await dashBtn.getAttribute('aria-selected');
  const dashAriaCurrentPage = await dashBtn.getAttribute('aria-current');
  results['4.3_dash_has_active_class'] =
    (dashClasses && (dashClasses.includes('active') || dashClasses.includes('selected') || dashClasses.includes('bg-'))) ||
    dashAriaSelected === 'true' ||
    dashAriaCurrentPage === 'page';

  // Click Logs and check
  const logsBtn = page.getByRole('button', { name: 'Logs', exact: true });
  await logsBtn.click();
  await page.waitForTimeout(1500);
  const logsClasses = await logsBtn.getAttribute('class');
  results['4.3_logs_has_active_class'] =
    logsClasses && (logsClasses.includes('active') || logsClasses.includes('selected') || logsClasses.includes('bg-'));

  // ===== 4.3 Sidebar scroll =====
  const sidebar = page.locator('nav[aria-label="Main navigation"]');
  const sidebarParent = sidebar.locator('..');
  const scrollHeight = await sidebarParent.evaluate((el) => el.scrollHeight);
  const clientHeight = await sidebarParent.evaluate((el) => el.clientHeight);
  results['4.3_sidebar_scrollable'] = scrollHeight > clientHeight;
  // Scroll to bottom
  await sidebarParent.evaluate((el) => (el.scrollTop = el.scrollHeight));
  await page.waitForTimeout(500);
  const userMgmtBtn = page.getByRole('button', { name: 'User Management', exact: true });
  results['4.3_bottom_visible_after_scroll'] = await userMgmtBtn.isVisible();

  // ===== 4.4 Responsive layout =====
  // Test at mobile viewport
  const originalSize = page.viewportSize();
  await page.setViewportSize({ width: 375, height: 812 });
  await page.waitForTimeout(1000);
  m = await page.locator('main').textContent();
  results['4.4_mobile_no_crash'] = !m.includes('Something went wrong');
  // Check if sidebar is collapsed/hidden
  const sidebarVisible = await sidebar.isVisible();
  results['4.4_sidebar_hidden_on_mobile'] = !sidebarVisible;

  // Test at tablet viewport
  await page.setViewportSize({ width: 768, height: 1024 });
  await page.waitForTimeout(1000);
  results['4.4_tablet_no_crash'] = !(await page.locator('main').textContent()).includes('Something went wrong');

  // Restore original viewport
  await page.setViewportSize(originalSize || { width: 1280, height: 720 });
  await page.waitForTimeout(1000);

  return results;
};
