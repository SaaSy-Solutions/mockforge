import { describe, expect, it, vi } from 'vitest';
import { cloudResilienceApi } from '../api/cloudResilience';
import { authenticatedFetch } from '../../utils/apiClient';
vi.mock('../../utils/apiClient', () => ({ authenticatedFetch: vi.fn() }));
vi.mock('../../utils/cloudMode', () => ({ isCloudMode: () => true }));
describe('cloud resilience response metadata', () => {
  it('preserves empty runtime data and its unreachable state', async () => {
    vi.mocked(authenticatedFetch).mockImplementation(async () => new Response(JSON.stringify({ runtime_state: 'unreachable', data: [] })));
    expect(await cloudResilienceApi.listCircuitBreakers('deployment')).toEqual({ runtime_state: 'unreachable', data: [] });
    expect(await cloudResilienceApi.listBulkheads('deployment')).toEqual({ runtime_state: 'unreachable', data: [] });
  });
});
