/**
 * Module-export contract for services/api/cloudServices.
 *
 * Regression guard for PR #142 / #143: the store imports the singleton
 * `cloudServicesApi` directly from this module. A previous iteration only
 * instantiated it in the `services/api/index.ts` barrel, so the production
 * rollup build bailed on the unresolved import — but every Vitest file used
 * `vi.mock` on this module, which masked the missing real export.
 *
 * This file deliberately does NOT mock the module. If anyone removes or
 * renames the singleton, this test fails loudly and the bug is caught before
 * it reaches production.
 */

import { describe, it, expect } from 'vitest';
import * as cloudServicesModule from '../cloudServices';

describe('services/api/cloudServices module exports', () => {
  it('exports a `cloudServicesApi` singleton with the expected CRUD surface', () => {
    expect(cloudServicesModule).toHaveProperty('cloudServicesApi');
    const api = cloudServicesModule.cloudServicesApi;
    expect(api).toBeDefined();
    for (const method of ['list', 'get', 'create', 'update', 'remove'] as const) {
      expect(typeof api[method]).toBe('function');
    }
  });

  it('exports the `CloudServicesApiService` class', () => {
    expect(cloudServicesModule).toHaveProperty('CloudServicesApiService');
    expect(typeof cloudServicesModule.CloudServicesApiService).toBe('function');
  });

  it('exposes the endpoint base path so callers can build URLs', () => {
    expect(cloudServicesModule.CLOUD_SERVICES_BASE).toBe('/api/v1/services');
  });
});
