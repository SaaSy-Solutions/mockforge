/**
 * Coverage Collection Helpers for Playwright
 * 
 * These helpers enable collecting code coverage data during Playwright E2E tests.
 * Coverage data is collected from the browser and saved for reporting.
 */

import { Page } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

/**
 * Collect coverage data from the page
 */
export async function collectCoverage(page: Page): Promise<any> {
  // Check if coverage data is available (via vite-plugin-istanbul)
  const coverage = await page.evaluate(() => {
    // @ts-ignore - Coverage data is injected by vite-plugin-istanbul
    if (window.__coverage__) {
      // @ts-ignore
      return window.__coverage__;
    }
    return null;
  });

  return coverage;
}

/**
 * Save coverage data to file
 */
export async function saveCoverage(
  page: Page,
  testName: string,
  outputDir: string = 'coverage'
): Promise<void> {
  const coverage = await collectCoverage(page);

  if (!coverage) {
    console.warn(`No coverage data found for test: ${testName}`);
    return;
  }

  // Ensure output directory exists
  if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
  }

  // Save coverage data
  const filename = `${testName.replace(/\s+/g, '-').toLowerCase()}-coverage.json`;
  const filepath = path.join(outputDir, filename);
  fs.writeFileSync(filepath, JSON.stringify(coverage, null, 2));

  console.log(`Coverage data saved: ${filepath}`);
}

/**
 * Initialize coverage collection in the page
 */
export async function initCoverage(page: Page): Promise<void> {
  // Wait for page to load and coverage to be available
  await page.waitForFunction(() => {
    // @ts-ignore
    return typeof window.__coverage__ !== 'undefined';
  }, { timeout: 5000 }).catch(() => {
    console.warn('Coverage instrumentation not detected. Make sure vite.config.coverage.ts is used.');
  });
}

