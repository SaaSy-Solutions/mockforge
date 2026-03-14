// Phase 1: Authentication & Account Pages deep testing
// Tests: Registration form, Organization, Billing, Hosted Mocks, API Tokens
// Requires: logged-in session at app.mockforge.dev
module.exports = async (page) => {
  const results = {};

  // Helper: navigate to page via sidebar
  async function goToPage(name) {
    await page.goto('https://app.mockforge.dev');
    await page.waitForTimeout(2000);
    const btn = page.getByRole('button', { name, exact: true });
    await btn.click();
    await page.waitForTimeout(3000);
  }

  // ===== 1.2 Registration Page (in separate context, no auth) =====
  {
    const browser = page.context().browser();
    const ctx = await browser.newContext();
    const p = await ctx.newPage();
    await p.goto('https://app.mockforge.dev/login');
    await p.waitForTimeout(2000);
    await p.locator('a:has-text("Sign up"), button:has-text("Sign up")').click();
    await p.waitForTimeout(2000);

    const body = await p.locator('body').textContent();
    results['1.2_form_renders'] = body.includes('Create') && body.includes('Username');
    results['1.2_has_username'] = (await p.locator('input[placeholder*="username" i]').count()) > 0;
    results['1.2_has_email'] = (await p.locator('input[type="email"]').count()) > 0;
    results['1.2_has_password'] = (await p.locator('input[type="password"]').count()) > 0;
    const submitBtn = p.locator('button:has-text("Create Account")');
    results['1.2_submit_disabled_when_empty'] = await submitBtn.isDisabled();
    results['1.2_has_signin_link'] = body.includes('Sign in');
    await ctx.close();
  }

  // ===== 1.5 Organization Page =====
  await goToPage('Organization');
  let main = await page.locator('main').textContent();
  results['1.5_renders'] = !main.includes('Something went wrong');
  results['1.5_has_personal_org'] = main.includes('testuser2-personal') || main.includes('personal');
  // Try clicking org to see details
  const orgItem = page.locator('main').getByText('personal').first();
  if (await orgItem.count() > 0) {
    await orgItem.click();
    await page.waitForTimeout(1500);
    main = await page.locator('main').textContent();
    results['1.5_details_open'] = main.includes('Members') || main.includes('Settings');
    results['1.5_has_members_tab'] = main.includes('Members');
    results['1.5_has_settings_tab'] = main.includes('Settings');
  }

  // ===== 1.7 Billing Page =====
  await goToPage('Billing');
  main = await page.locator('main').textContent();
  results['1.7_renders'] = !main.includes('Something went wrong');
  results['1.7_has_free_plan'] = main.toLowerCase().includes('free');
  results['1.7_has_pro'] = main.includes('Pro');
  results['1.7_has_team'] = main.includes('Team');
  results['1.7_has_prices'] = main.includes('$0') || main.includes('$19') || main.includes('$79');

  // Check tabs
  const overviewTab = page.locator('main button:has-text("Overview"), main [role="tab"]:has-text("Overview")');
  const usageTab = page.locator('main button:has-text("Usage"), main [role="tab"]:has-text("Usage")');
  const plansTab = page.locator('main button:has-text("Plans"), main [role="tab"]:has-text("Plans")');
  results['1.7_has_overview_tab'] = (await overviewTab.count()) > 0;
  results['1.7_has_usage_tab'] = (await usageTab.count()) > 0;
  results['1.7_has_plans_tab'] = (await plansTab.count()) > 0;

  // Click Usage tab
  if (results['1.7_has_usage_tab']) {
    await usageTab.first().click();
    await page.waitForTimeout(1000);
    main = await page.locator('main').textContent();
    results['1.7_usage_renders'] = !main.includes('Something went wrong');
  }

  // Click Plans tab
  if (results['1.7_has_plans_tab']) {
    await plansTab.first().click();
    await page.waitForTimeout(1000);
    main = await page.locator('main').textContent();
    results['1.7_plans_renders'] = !main.includes('Something went wrong');
    results['1.7_upgrade_btn'] = main.includes('Upgrade');
  }

  // ===== 1.8 Hosted Mocks =====
  await goToPage('Hosted Mocks');
  main = await page.locator('main').textContent();
  results['1.8_renders'] = !main.includes('Something went wrong');
  const deployBtn = page.locator(
    'main button:has-text("Deploy"), main button:has-text("Create"), main button:has-text("New Mock")'
  );
  results['1.8_has_deploy_btn'] = (await deployBtn.count()) > 0;
  if (results['1.8_has_deploy_btn']) {
    await deployBtn.first().click();
    await page.waitForTimeout(1500);
    const nameInput = page.locator('input[placeholder*="name" i], input[name="name"]');
    results['1.8_modal_has_name'] = (await nameInput.count()) > 0;
    // Close
    const cancelBtn = page.locator('button:has-text("Cancel"), button:has-text("Close")');
    if ((await cancelBtn.count()) > 0) await cancelBtn.first().click();
    await page.waitForTimeout(500);
  }

  // ===== 1.6 API Tokens (verify page renders and create button exists) =====
  await goToPage('API Tokens');
  main = await page.locator('main').textContent();
  results['1.6_renders'] = !main.includes('Something went wrong');
  const createTokenBtn = page.locator('main button:has-text("Create"), main button:has-text("New Token")');
  results['1.6_has_create_btn'] = (await createTokenBtn.count()) > 0;

  return results;
};
