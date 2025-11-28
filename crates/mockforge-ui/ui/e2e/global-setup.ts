/**
 * Global Setup for Playwright E2E Tests
 * 
 * This file runs once before all tests to ensure coverage collection is ready.
 */

import { chromium, FullConfig } from '@playwright/test';

async function globalSetup(config: FullConfig) {
  // Check if coverage is enabled
  const coverageEnabled = process.env.COLLECT_COVERAGE === 'true';
  
  if (coverageEnabled) {
    console.log('📊 Coverage collection enabled');
  }
}

export default globalSetup;

