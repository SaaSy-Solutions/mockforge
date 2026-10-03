import { beforeEach, describe, expect, it, vi } from 'vitest';
import { authenticatedFetch } from '../../../utils/apiClient';
import { cloudFlowsApi, type FlowKind } from '../cloudFlows';

vi.mock('../../../utils/apiClient', () => ({ authenticatedFetch: vi.fn() }));
vi.mock('../../../utils/cloudMode', () => ({ isCloudMode: () => true }));

describe('cloud flow creation API contract', () => {
  beforeEach(() => vi.clearAllMocks());


  it('loads versions from the dedicated registry endpoint', async () => {
    vi.mocked(authenticatedFetch).mockResolvedValue(new Response(JSON.stringify({ id: 'version-id', config: { states: ['idle'] } })));
    const version = await cloudFlowsApi.getVersion('version-id');
    expect(authenticatedFetch).toHaveBeenCalledWith('/api/v1/flow-versions/version-id', undefined);
    expect(version.config).toEqual({ states: ['idle'] });
  });

  it.each<FlowKind>(['scenario', 'orchestration', 'state_machine', 'chain'])(
    'sends the registry-required config for %s, preserving the editor definition',
    async (kind) => {
      const config = { nodes: [{ id: 'start', name: 'Start' }], metadata: { revision: 1 } };
      vi.mocked(authenticatedFetch).mockResolvedValue(new Response(JSON.stringify({ id: 'created-flow', kind }), {
        status: 201, headers: { 'Content-Type': 'application/json' },
      }));
      const result = await cloudFlowsApi.create('selected-workspace', { kind, name: 'QA flow', config });
      const [url, options] = vi.mocked(authenticatedFetch).mock.calls[0];
      expect(url).toBe('/api/v1/workspaces/selected-workspace/flows');
      expect(options?.method).toBe('POST');
      expect(JSON.parse(String(options?.body))).toEqual({ kind, name: 'QA flow', config });
      expect(result.id).toBe('created-flow');
    },
  );
});
