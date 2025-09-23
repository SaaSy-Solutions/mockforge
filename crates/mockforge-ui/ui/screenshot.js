import { chromium } from '@playwright/test';

async function takeScreenshots() {
  console.log('🚀 Starting comprehensive MockForge UI screenshot capture...');

  const browser = await chromium.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox']
  });
  const page = await browser.newPage();

  try {
    console.log('📍 Connecting to MockForge UI on port 9080...');

    await page.goto('http://localhost:9080', {
      waitUntil: 'networkidle',
      timeout: 30000
    });

    const title = await page.title();
    console.log(`✅ Connected to MockForge UI: ${title}`);

    // Wait for initial load
    await page.waitForTimeout(3000);

    // Enhanced login handling
    await handleLogin(page);

    // Capture initial dashboard
    await capturePageScreenshot(page, 'dashboard', 'Main Dashboard');

    // Comprehensive navigation discovery and screenshot capture
    await captureAllPages(page);

    console.log('✅ All screenshots captured successfully!');

  } catch (error) {
    console.error('❌ Error taking screenshots:', error.message);

    // Enhanced error handling with more context
    try {
      await page.screenshot({
        path: 'mockforge-error-state.png',
        fullPage: true
      });
      console.log('📸 Error state screenshot saved as mockforge-error-state.png');

      // Also capture current URL and page title for debugging
      const url = page.url();
      const title = await page.title();
      console.log(`🔍 Error occurred at: ${url} (Title: ${title})`);

    } catch (screenshotError) {
      console.error('❌ Failed to take error screenshot:', screenshotError.message);
    }
  } finally {
    await browser.close();
    console.log('🔒 Browser closed. Screenshots saved in project root.');
  }
}

async function handleLogin(page) {
  console.log('🔐 Checking for authentication...');

  // Multiple ways to detect login page
  const loginIndicators = [
    'text=Sign In',
    'text=Sign in',
    'text=Login',
    'text=Log in',
    'input[placeholder*="username" i]',
    'input[placeholder*="password" i]',
    '.login-form',
    '.auth-form'
  ];

  let needsLogin = false;
  for (const indicator of loginIndicators) {
    const element = await page.$(indicator);
    if (element) {
      needsLogin = true;
      console.log(`🔍 Found login indicator: ${indicator}`);
      break;
    }
  }

  if (needsLogin) {
    console.log('🔑 Login required, attempting to sign in...');

    // Try multiple credential combinations
    const credentials = [
      { username: 'admin', password: 'admin123' },
      { username: 'demo', password: 'demo123' },
      { username: 'viewer', password: 'viewer123' }
    ];

    for (const cred of credentials) {
      try {
        console.log(`🔑 Trying credentials: ${cred.username}`);

        // Find and fill username field
        const usernameSelectors = [
          'input[placeholder*="username" i]',
          'input[placeholder*="user" i]',
          'input[name*="username" i]',
          'input[name*="user" i]',
          'input[type="email"]',
          'input[type="text"]'
        ];

        for (const selector of usernameSelectors) {
          const usernameField = await page.$(selector);
          if (usernameField) {
            await usernameField.fill(cred.username);
            break;
          }
        }

        // Find and fill password field
        const passwordSelectors = [
          'input[placeholder*="password" i]',
          'input[name*="password" i]',
          'input[type="password"]'
        ];

        for (const selector of passwordSelectors) {
          const passwordField = await page.$(selector);
          if (passwordField) {
            await passwordField.fill(cred.password);
            break;
          }
        }

        // Find and click sign in button
        const signInSelectors = [
          'text=Sign In',
          'text=Sign in',
          'text=Login',
          'text=Log in',
          'button[type="submit"]',
          'input[type="submit"]'
        ];

        for (const selector of signInSelectors) {
          const signInButton = await page.$(selector);
          if (signInButton) {
            await signInButton.click();
            await page.waitForTimeout(3000);
            break;
          }
        }

        // Check if login was successful by looking for dashboard elements
        await page.waitForTimeout(2000); // Wait for potential redirect

        const currentUrl = page.url();
        const title = await page.title();

        // Check for dashboard indicators
        const dashboardIndicators = [
          'text=Dashboard',
          'text=Services',
          'text=Logs',
          'text=Metrics',
          '.sidebar',
          '.navigation',
          'nav',
          '[data-page]'
        ];

        let foundDashboardElement = false;
        for (const indicator of dashboardIndicators) {
          try {
            const element = await page.$(indicator);
            if (element) {
              foundDashboardElement = true;
              console.log(`✅ Found dashboard element: ${indicator}`);
              break;
            }
          } catch (error) {
            // Continue checking other indicators
          }
        }

        if (foundDashboardElement || (!currentUrl.includes('login') && !currentUrl.includes('signin') && title !== 'MockForge Admin')) {
          console.log(`✅ Successfully logged in as ${cred.username}`);
          return;
        } else {
          console.log(`⚠️ Login may not have completed successfully. Current title: ${title}, URL: ${currentUrl}`);
        }

      } catch (error) {
        console.log(`⚠️ Failed to login with ${cred.username}:`, error.message);
      }
    }

    console.log('⚠️ Could not log in with any credentials, proceeding anyway...');
  } else {
    console.log('✅ No login required or already logged in');

    // Debug: Check what elements are on the current page
    const pageTitle = await page.title();
    const pageUrl = page.url();
    console.log(`📍 Current page: ${pageTitle} (${pageUrl})`);

    // Take a debug screenshot to see what we're working with
    await page.screenshot({
      path: 'mockforge-debug-current-page.png',
      fullPage: true
    });
    console.log('📸 Debug screenshot saved: mockforge-debug-current-page.png');
  }
}

async function captureAllPages(page) {
  console.log('🔍 Discovering and capturing all pages...');

  // Wait for navigation to be ready
  await page.waitForTimeout(2000);

  // Enhanced navigation discovery
  const navigationTargets = [
    // Main navigation pages
    { name: 'Dashboard', selectors: ['text=Dashboard', 'nav a[href*="dashboard"]', '[data-page="dashboard"]'] },
    { name: 'Services', selectors: ['text=Services', 'nav a[href*="services"]', '[data-page="services"]'] },
    { name: 'Logs', selectors: ['text=Logs', 'nav a[href*="logs"]', '[data-page="logs"]'] },
    { name: 'Metrics', selectors: ['text=Metrics', 'nav a[href*="metrics"]', '[data-page="metrics"]'] },
    { name: 'Fixtures', selectors: ['text=Fixtures', 'nav a[href*="fixtures"]', '[data-page="fixtures"]'] },
    { name: 'Testing', selectors: ['text=Testing', 'nav a[href*="testing"]', '[data-page="testing"]'] },
    { name: 'Config', selectors: ['text=Config', 'nav a[href*="config"]', '[data-page="config"]'] },

    // Additional navigation patterns
    { name: 'Settings', selectors: ['text=Settings', 'nav a[href*="settings"]', '.settings-link'] },
    { name: 'Profile', selectors: ['text=Profile', 'nav a[href*="profile"]', '.profile-link'] },
    { name: 'Help', selectors: ['text=Help', 'nav a[href*="help"]', '.help-link'] },
  ];

  const capturedPages = new Set();

  // Try to find sidebar or main navigation
  const navContainers = [
    'nav',
    '.sidebar',
    '.navigation',
    '.nav-menu',
    '.menu',
    'aside',
    '.drawer'
  ];

  for (const container of navContainers) {
    try {
      const navElement = await page.$(container);
      if (navElement) {
        console.log(`📍 Found navigation container: ${container}`);

        // Get all links within navigation
        const links = await navElement.$$('a');
        console.log(`🔗 Found ${links.length} links in navigation`);

        for (const link of links) {
          try {
            const linkText = await link.textContent();
            const href = await link.getAttribute('href');

            if (linkText && linkText.trim()) {
              const cleanText = linkText.trim();
              console.log(`🔗 Navigation link: "${cleanText}" -> ${href}`);

              // Check if this matches any of our targets
              const matchingTarget = navigationTargets.find(target =>
                target.name.toLowerCase() === cleanText.toLowerCase() ||
                target.selectors.some(selector => selector.includes(cleanText.toLowerCase()))
              );

              if (matchingTarget && !capturedPages.has(matchingTarget.name)) {
                await navigateAndCapture(page, matchingTarget.name, link);
                capturedPages.add(matchingTarget.name);
              }
            }
          } catch (error) {
            console.log(`⚠️ Error processing navigation link:`, error.message);
          }
        }
      }
    } catch (error) {
      console.log(`⚠️ Error with navigation container ${container}:`, error.message);
    }
  }

  // Fallback: Try direct text matching for each target
  for (const target of navigationTargets) {
    if (capturedPages.has(target.name)) continue;

    for (const selector of target.selectors) {
      try {
        const element = await page.$(selector);
        if (element) {
          console.log(`🎯 Found ${target.name} via selector: ${selector}`);
          await navigateAndCapture(page, target.name, element);
          capturedPages.add(target.name);
          break;
        }
      } catch (error) {
        console.log(`⚠️ Error with selector ${selector}:`, error.message);
      }
    }
  }

  // Try to find any clickable navigation elements we might have missed
  try {
    const allButtons = await page.$$('button');
    const allLinks = await page.$$('a');

    console.log(`🔍 Found ${allButtons.length} buttons and ${allLinks.length} links to check`);

    // Debug: List all clickable elements with their text and selectors
    console.log('📋 Listing all clickable elements:');
    const allClickable = [...allButtons, ...allLinks];

    for (let i = 0; i < Math.min(allClickable.length, 30); i++) {
      try {
        const element = allClickable[i];
        const text = await element.textContent();
        const tagName = await element.evaluate(el => el.tagName.toLowerCase());
        const className = await element.getAttribute('class') || '';
        const id = await element.getAttribute('id') || '';
        const href = await element.getAttribute('href') || '';

        if (text && text.trim().length > 0) {
          const cleanText = text.trim();
          console.log(`  ${i + 1}. ${tagName}${id ? '#' + id : ''}${className ? '.' + className.replace(/\s+/g, '.') : ''}: "${cleanText}"${href ? ' -> ' + href : ''}`);

          const matchingTarget = navigationTargets.find(target =>
            target.name.toLowerCase() === cleanText.toLowerCase()
          );

          if (matchingTarget && !capturedPages.has(matchingTarget.name)) {
            console.log(`🎯 Found ${matchingTarget.name} via general search: "${cleanText}"`);
            await navigateAndCapture(page, matchingTarget.name, element);
            capturedPages.add(matchingTarget.name);
          }
        }
      } catch (error) {
        console.log(`⚠️ Error inspecting element ${i + 1}:`, error.message);
      }
    }

    // Also check for any divs or spans that might be clickable navigation
    console.log('🔍 Looking for clickable divs and spans...');
    const clickableDivs = await page.$$('div[role="button"], span[role="button"], div[onclick], span[onclick]');
    console.log(`Found ${clickableDivs.length} clickable divs/spans`);

    for (const element of clickableDivs.slice(0, 10)) {
      try {
        const text = await element.textContent();
        if (text && text.trim().length > 0) {
          const cleanText = text.trim();
          console.log(`  Clickable: "${cleanText}"`);

          const matchingTarget = navigationTargets.find(target =>
            target.name.toLowerCase() === cleanText.toLowerCase()
          );

          if (matchingTarget && !capturedPages.has(matchingTarget.name)) {
            console.log(`🎯 Found ${matchingTarget.name} in clickable element: "${cleanText}"`);
            await navigateAndCapture(page, matchingTarget.name, element);
            capturedPages.add(matchingTarget.name);
          }
        }
      } catch (error) {
        // Skip problematic elements
      }
    }

  } catch (error) {
    console.log(`⚠️ Error during general navigation search:`, error.message);
  }

  // Final attempt: Look for common navigation patterns in the page source
  try {
    console.log('🔍 Inspecting page structure...');
    const pageContent = await page.content();
    const navigationKeywords = ['dashboard', 'services', 'logs', 'metrics', 'fixtures', 'config', 'settings'];

    for (const keyword of navigationKeywords) {
      if (pageContent.toLowerCase().includes(keyword)) {
        console.log(`📝 Page contains reference to: ${keyword}`);
      }
    }

    // Look for navigation-related HTML structures
    const navPatterns = [
      /<nav[^>]*>[\s\S]*?<\/nav>/gi,
      /<aside[^>]*>[\s\S]*?<\/aside>/gi,
      /<div[^>]*class="[^"]*nav[^"]*"[^>]*>[\s\S]*?<\/div>/gi,
      /<ul[^>]*class="[^"]*menu[^"]*"[^>]*>[\s\S]*?<\/ul>/gi
    ];

    for (const pattern of navPatterns) {
      const matches = pageContent.match(pattern);
      if (matches) {
        console.log(`🏗️ Found navigation structure: ${pattern}`);
        console.log(`   ${matches.length} matches`);
      }
    }

  } catch (error) {
    console.log(`⚠️ Error inspecting page structure:`, error.message);
  }

  // Summary
  console.log(`📊 Navigation discovery complete. Captured ${capturedPages.size} pages:`);
  capturedPages.forEach(page => console.log(`  ✅ ${page}`));

  const missedPages = navigationTargets.filter(target => !capturedPages.has(target.name));
  if (missedPages.length > 0) {
    console.log(`⚠️ Could not capture ${missedPages.length} pages:`);
    missedPages.forEach(page => console.log(`  ❌ ${page.name}`));
  }
}

async function navigateAndCapture(page, pageName, element) {
  try {
    console.log(`📸 Navigating to ${pageName}...`);

    // Click the element
    await element.click();

    // Wait for navigation and page load
    await page.waitForTimeout(4000);

    // Wait for any loading indicators to disappear
    try {
      await page.waitForSelector('.loading, .spinner, [data-loading="true"]', {
        state: 'detached',
        timeout: 5000
      });
    } catch (error) {
      // Loading indicators might not exist, that's fine
    }

    // Additional wait for dynamic content
    await page.waitForTimeout(2000);

    // Capture screenshot
    const filename = `mockforge-${pageName.toLowerCase().replace(/\s+/g, '-')}.png`;
    await page.screenshot({
      path: filename,
      fullPage: true
    });

    console.log(`✅ Screenshot saved: ${filename}`);

    // Try to return to dashboard for next navigation
    await returnToDashboard(page);

  } catch (error) {
    console.log(`❌ Error capturing ${pageName}:`, error.message);
  }
}

async function returnToDashboard(page) {
  try {
    const dashboardSelectors = [
      'text=Dashboard',
      'nav a[href*="dashboard"]',
      '[data-page="dashboard"]',
      'a[href="/"]',
      'a[href="/dashboard"]'
    ];

    for (const selector of dashboardSelectors) {
      try {
        const dashboardLink = await page.$(selector);
        if (dashboardLink) {
          await dashboardLink.click();
          await page.waitForTimeout(2000);
          console.log('🏠 Returned to dashboard');
          return;
        }
      } catch (error) {
        // Continue trying other selectors
      }
    }

    console.log('⚠️ Could not return to dashboard, continuing...');
  } catch (error) {
    console.log('⚠️ Error returning to dashboard:', error.message);
  }
}

async function capturePageScreenshot(page, pageName, description = '') {
  try {
    const filename = `mockforge-${pageName.toLowerCase().replace(/\s+/g, '-')}.png`;
    await page.screenshot({
      path: filename,
      fullPage: true
    });

    console.log(`📸 ${description ? description : pageName} screenshot saved: ${filename}`);
  } catch (error) {
    console.log(`❌ Error capturing ${pageName} screenshot:`, error.message);
  }
}

takeScreenshots().catch(console.error);
