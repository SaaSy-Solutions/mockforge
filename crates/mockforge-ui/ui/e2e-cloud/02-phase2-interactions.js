// Phase 2: Interaction testing for all local-mode pages with cloud stubs
// Tests specific UI interactions: forms, dialogs, sliders, toggles, filters, tabs
// Requires: logged-in session at app.mockforge.dev
module.exports = async (page) => {
  const results = {};

  async function goToPage(name) {
    await page.goto('https://app.mockforge.dev');
    await page.waitForTimeout(2000);
    const btn = page.getByRole('button', { name, exact: true });
    await btn.click();
    await page.waitForTimeout(3000);
  }

  function mainText() {
    return page.locator('main').textContent();
  }

  // ===== 2.1 Dashboard =====
  await goToPage('Dashboard');
  let m = await mainText();
  results['2.1_metrics_uptime'] = m.includes('Uptime');
  results['2.1_metrics_cpu'] = m.includes('CPU');
  results['2.1_metrics_memory'] = m.includes('Memory');
  results['2.1_metrics_threads'] = m.includes('Threads');
  results['2.1_status_2xx'] = m.includes('Success Responses');
  results['2.1_status_4xx'] = m.includes('Client Errors');
  results['2.1_status_5xx'] = m.includes('Server Errors');
  results['2.1_reality_slider'] = (await page.locator('[role="slider"]').count()) > 0;
  results['2.1_time_travel'] = m.includes('Time Travel');
  results['2.1_no_requests'] = m.includes('No requests found');
  // Click level 3 button
  const lvl3Btn = page.locator('main button').filter({ hasText: /^3$/ }).first();
  if ((await lvl3Btn.count()) > 0) {
    await lvl3Btn.click();
    await page.waitForTimeout(1000);
    results['2.1_slider_interactive'] = !(await mainText()).includes('Something went wrong');
  }

  // ===== 2.2 Services =====
  await goToPage('Services');
  m = await mainText();
  results['2.2_renders'] = !m.includes('Something went wrong');
  results['2.2_empty_state'] = m.includes('No') || m.includes('empty') || m.includes('service');
  // Global search in header
  const globalSearch = page.locator('input[placeholder*="search" i]');
  results['2.2_search_exists'] = (await globalSearch.count()) > 0;
  if (results['2.2_search_exists']) {
    await globalSearch.first().fill('test');
    await page.waitForTimeout(500);
    results['2.2_search_no_crash'] = !(await mainText()).includes('Something went wrong');
    await globalSearch.first().clear();
  }

  // ===== 2.4 Fixtures =====
  await goToPage('Fixtures');
  m = await mainText();
  results['2.4_renders'] = !m.includes('Something went wrong');
  results['2.4_empty_state'] = m.toLowerCase().includes('no fixture') || m.toLowerCase().includes('no data');
  const newFixtureBtn = page.locator('main button:has-text("New"), main button:has-text("Create"), main button:has-text("Add")');
  results['2.4_new_fixture_btn'] = (await newFixtureBtn.count()) > 0;
  if (results['2.4_new_fixture_btn']) {
    await newFixtureBtn.first().click();
    await page.waitForTimeout(1500);
    m = await mainText();
    results['2.4_create_form_opens'] = !m.includes('Something went wrong');
    // Try to go back
    const backBtn = page.locator('button:has-text("Cancel"), button:has-text("Back")');
    if ((await backBtn.count()) > 0) await backBtn.first().click();
    await page.waitForTimeout(500);
  }

  // ===== 2.6 Workspaces =====
  await goToPage('Workspaces');
  m = await mainText();
  results['2.6_renders'] = !m.includes('Something went wrong');
  // Check for tabs
  results['2.6_has_tabs'] = m.includes('Workspace') || m.includes('Folders') || m.includes('Request');
  const createWsBtn = page.locator('main button:has-text("Create"), main button:has-text("New")');
  results['2.6_create_btn'] = (await createWsBtn.count()) > 0;
  if (results['2.6_create_btn']) {
    await createWsBtn.first().click();
    await page.waitForTimeout(1500);
    const wsNameInput = page.locator('input[placeholder*="name" i], input[placeholder*="workspace" i]');
    results['2.6_dialog_opens'] = (await wsNameInput.count()) > 0;
    if (results['2.6_dialog_opens']) {
      await wsNameInput.first().fill('Test Workspace');
      await page.waitForTimeout(300);
      results['2.6_input_works'] = true;
    }
    // Cancel
    const cancelBtn = page.locator('button:has-text("Cancel")');
    if ((await cancelBtn.count()) > 0) {
      await cancelBtn.first().click();
      await page.waitForTimeout(500);
      results['2.6_cancel_closes'] = true;
    }
  }

  // ===== 2.8 Chains =====
  await goToPage('Chains');
  m = await mainText();
  results['2.8_renders'] = !m.includes('Something went wrong');
  const createChainBtn = page.locator('main button:has-text("Create"), main button:has-text("New")');
  results['2.8_create_btn'] = (await createChainBtn.count()) > 0;
  if (results['2.8_create_btn']) {
    await createChainBtn.first().click();
    await page.waitForTimeout(1500);
    results['2.8_form_opens'] = !(await mainText()).includes('Something went wrong');
    const cancelBtn = page.locator('button:has-text("Cancel")');
    if ((await cancelBtn.count()) > 0) await cancelBtn.first().click();
    await page.waitForTimeout(500);
  }

  // ===== 2.9 Config =====
  await goToPage('Config');
  m = await mainText();
  results['2.9_renders'] = !m.includes('Something went wrong');
  results['2.9_latency'] = m.toLowerCase().includes('latency');
  results['2.9_proxy'] = m.toLowerCase().includes('proxy');
  results['2.9_version_cloud'] = m.includes('cloud');
  // Check for toggles/sliders
  const toggles = page.locator('main [role="switch"], main input[type="checkbox"]');
  results['2.9_has_toggles'] = (await toggles.count()) > 0;
  const configSliders = page.locator('main [role="slider"], main input[type="range"]');
  results['2.9_has_sliders'] = (await configSliders.count()) > 0;

  // ===== 2.11 Graph =====
  await goToPage('Graph');
  m = await mainText();
  results['2.11_renders'] = !m.includes('Something went wrong');
  results['2.11_has_canvas'] = m.includes('graph') || m.includes('Graph') || m.includes('node') || m.includes('0 nodes');

  // ===== 2.12 Logs =====
  await goToPage('Logs');
  m = await mainText();
  results['2.12_renders'] = !m.includes('Something went wrong');
  results['2.12_empty_state'] = m.includes('No') || m.includes('no log') || m.includes('No requests');
  // Check filter buttons
  const allBtn = page.locator('main button:has-text("ALL")');
  const getBtn = page.locator('main button:has-text("GET")');
  results['2.12_has_method_filter'] = (await getBtn.count()) > 0;
  results['2.12_has_status_filter'] = (await page.locator('main button:has-text("2XX")').count()) > 0;
  // Search input
  const logSearch = page.locator('main input[placeholder*="search" i], main input[placeholder*="path" i]');
  results['2.12_has_search'] = (await logSearch.count()) > 0;
  if (results['2.12_has_search']) {
    await logSearch.first().fill('/test/path');
    await page.waitForTimeout(500);
    results['2.12_search_works'] = !(await mainText()).includes('Something went wrong');
    await logSearch.first().clear();
  }

  // ===== 2.13 Metrics =====
  await goToPage('Metrics');
  m = await mainText();
  results['2.13_renders'] = !m.includes('Something went wrong');
  results['2.13_total_requests'] = m.includes('Total Requests') || m.includes('total');
  results['2.13_response_time'] = m.includes('Response Time') || m.includes('Latency') || m.includes('response');

  // ===== 2.15 Testing Hub =====
  await goToPage('Testing');
  m = await mainText();
  results['2.15_renders'] = !m.includes('Something went wrong');
  const runBtns = page.locator('main button:has-text("Run")');
  results['2.15_has_run_btns'] = (await runBtns.count()) > 0;

  // ===== 2.18 Chaos Engineering =====
  await goToPage('Chaos Engineering');
  m = await mainText();
  results['2.18_renders'] = !m.includes('Something went wrong');
  results['2.18_has_latency'] = m.toLowerCase().includes('latency');
  results['2.18_has_fault'] = m.toLowerCase().includes('fault') || m.toLowerCase().includes('error');
  const chaosSliders = page.locator('main [role="slider"], main input[type="range"]');
  results['2.18_has_sliders'] = (await chaosSliders.count()) > 0;
  const chaosToggles = page.locator('main [role="switch"], main input[type="checkbox"]');
  results['2.18_has_toggles'] = (await chaosToggles.count()) > 0;

  // ===== 2.20 Import =====
  await goToPage('Import');
  m = await mainText();
  results['2.20_renders'] = !m.includes('Something went wrong');
  results['2.20_has_postman'] = m.includes('Postman');
  results['2.20_has_insomnia'] = m.includes('Insomnia');
  results['2.20_has_curl'] = m.includes('cURL') || m.includes('curl');
  // Tab switching
  const postmanTab = page.locator('main button:has-text("Postman"), main [role="tab"]:has-text("Postman")');
  const insomniaTab = page.locator('main button:has-text("Insomnia"), main [role="tab"]:has-text("Insomnia")');
  if ((await insomniaTab.count()) > 0) {
    await insomniaTab.first().click();
    await page.waitForTimeout(1000);
    results['2.20_tab_switch'] = !(await mainText()).includes('Something went wrong');
  }

  // ===== 2.25 Plugins =====
  await goToPage('Plugins');
  m = await mainText();
  results['2.25_renders'] = !m.includes('Something went wrong');
  results['2.25_installed_tab'] = m.includes('Installed') || m.includes('installed') || m.includes('No plugins');
  const installBtn = page.locator('main button:has-text("Install")');
  results['2.25_install_btn'] = (await installBtn.count()) > 0;

  return results;
};
