/**
 * Smoke Tests API service.
 */
import type { SmokeTestResult, SmokeTestContext } from '../../types';
import { fetchJson } from './client';

class SmokeTestsApiService {
  async getSmokeTests(): Promise<SmokeTestResult[]> {
    return fetchJson('/__mockforge/smoke') as Promise<SmokeTestResult[]>;
  }

  async runSmokeTests(): Promise<SmokeTestContext> {
    return fetchJson('/__mockforge/smoke/run', {
      method: 'GET',
    }) as Promise<SmokeTestContext>;
  }
}

export { SmokeTestsApiService };
