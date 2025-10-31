/**
 * Global Teardown for Playwright E2E Tests
 * 
 * This file runs once after all tests to merge and generate coverage reports.
 */

import { FullConfig } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

async function globalTeardown(config: FullConfig) {
  const coverageEnabled = process.env.COLLECT_COVERAGE === 'true';
  
  if (!coverageEnabled) {
    return;
  }

  console.log('📊 Merging coverage data...');
  
  // Collect all coverage JSON files
  const coverageDir = path.join(process.cwd(), 'coverage', 'e2e');
  if (!fs.existsSync(coverageDir)) {
    console.warn('⚠️  Coverage directory not found');
    return;
  }

  const coverageFiles = fs.readdirSync(coverageDir)
    .filter(file => file.endsWith('.json'))
    .map(file => path.join(coverageDir, file));

  if (coverageFiles.length === 0) {
    console.warn('⚠️  No coverage files found');
    return;
  }

  // Merge coverage data
  const mergedCoverage: Record<string, any> = {};
  
  for (const file of coverageFiles) {
    try {
      const data = JSON.parse(fs.readFileSync(file, 'utf-8'));
      Object.assign(mergedCoverage, data);
    } catch (error) {
      console.warn(`⚠️  Failed to read coverage file: ${file}`, error);
    }
  }

  // Save merged coverage
  const mergedFile = path.join(coverageDir, 'merged-coverage.json');
  fs.writeFileSync(mergedFile, JSON.stringify(mergedCoverage, null, 2));
  
  console.log(`✅ Merged ${coverageFiles.length} coverage files`);
  console.log(`📁 Merged coverage saved to: ${mergedFile}`);
}

export default globalTeardown;

