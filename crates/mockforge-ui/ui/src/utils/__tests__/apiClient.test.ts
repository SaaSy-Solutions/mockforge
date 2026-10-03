import { beforeEach, describe, expect, it, vi } from 'vitest';
import { authenticatedFetch } from '../apiClient';

const { state } = vi.hoisted(() => ({ state: {
  token: 'access' as string | null, isAuthenticated: true,
  refreshTokenAction: vi.fn(), logout: vi.fn(),
} }));
vi.mock('../../stores/useAuthStore', () => ({ useAuthStore: { getState: () => state } }));
vi.mock('../cloudMode', () => ({ isCloudMode: () => true }));

// apiClient keeps a reference to the original fetch before installing its wrapper.
const networkFetch = vi.hoisted(() => vi.fn());
vi.hoisted(() => { globalThis.fetch = networkFetch; });

describe('authenticated fetch retries', () => {
  beforeEach(() => {
    networkFetch.mockReset();
    state.token = 'access';
    state.isAuthenticated = true;
    state.refreshTokenAction.mockReset();
    state.logout.mockReset();
  });

  it('does not recursively refresh an authentication request', async () => {
    networkFetch.mockResolvedValueOnce(new Response('{}', { status: 401 }));
    await authenticatedFetch('/api/v1/auth/token/refresh', { method: 'POST' });
    expect(state.refreshTokenAction).not.toHaveBeenCalled();
  });

  it('reuses a token already rotated while a request was in flight', async () => {
    networkFetch.mockImplementationOnce(async () => {
      state.token = 'already-rotated';
      return new Response('{}', { status: 401 });
    }).mockResolvedValueOnce(new Response('{}'));
    const response = await authenticatedFetch('/api/v1/workspaces');
    expect(response.status).toBe(200);
    expect(state.refreshTokenAction).not.toHaveBeenCalled();
    expect(networkFetch.mock.calls[1][1].headers.get('Authorization')).toBe('Bearer already-rotated');
  });

  it('refreshes an expired cookie session without an in-memory token', async () => {
    state.token = null;
    networkFetch.mockResolvedValueOnce(new Response('{}', { status: 401 }))
      .mockResolvedValueOnce(new Response('{}'));
    state.refreshTokenAction.mockImplementationOnce(async () => { state.token = 'new-access'; });
    expect((await authenticatedFetch('/api/v1/workspaces')).status).toBe(200);
    expect(state.refreshTokenAction).toHaveBeenCalledTimes(1);
  });

  it('does not send another logout when a refresh is rejected', async () => {
    networkFetch.mockResolvedValueOnce(new Response('{}', { status: 401 }));
    state.refreshTokenAction.mockRejectedValueOnce(new Error('revoked'));
    expect((await authenticatedFetch('/api/v1/workspaces')).status).toBe(401);
    expect(state.logout).not.toHaveBeenCalled();
  });
});

describe('cloud snapshot requests', () => {
  it('sends snapshot diff and delete to the registry instead of returning a local stub', async () => {
    networkFetch.mockReset();
    networkFetch.mockResolvedValue(new Response(JSON.stringify({ snapshot_id: 'snapshot' })));
    await authenticatedFetch('/api/v1/snapshots/snapshot/diff?against=current');
    await authenticatedFetch('/api/v1/snapshots/snapshot', { method: 'DELETE' });
    expect(networkFetch).toHaveBeenCalledTimes(2);
    expect(networkFetch.mock.calls[1][1].method).toBe('DELETE');
  });
});
