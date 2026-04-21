/**
 * @jest-environment jsdom
 *
 * Covers the cloud-mode branch of useServiceStore. The test harness sets
 * `VITE_API_BASE_URL` in test/setup.ts, so `isCloud` is true throughout.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';

vi.mock('../../services/api/cloudServices', () => {
  const list = vi.fn();
  const get = vi.fn();
  const create = vi.fn();
  const update = vi.fn();
  const remove = vi.fn();
  return {
    cloudServicesApi: { list, get, create, update, remove },
    CLOUD_SERVICES_BASE: '/api/v1/services',
  };
});

import { useServiceStore } from '../useServiceStore';
import { cloudServicesApi } from '../../services/api/cloudServices';

const listMock = vi.mocked(cloudServicesApi.list);
const createMock = vi.mocked(cloudServicesApi.create);
const updateMock = vi.mocked(cloudServicesApi.update);
const removeMock = vi.mocked(cloudServicesApi.remove);

const ORG_ID = '00000000-0000-0000-0000-000000000001';
const USER_ID = '00000000-0000-0000-0000-000000000002';

const cloudService = (overrides: Partial<{
  id: string;
  name: string;
  workspace_id: string | null;
  enabled: boolean;
  routes: unknown;
  tags: unknown;
}> = {}) => ({
  id: overrides.id ?? 'svc-1',
  org_id: ORG_ID,
  workspace_id: overrides.workspace_id ?? null,
  name: overrides.name ?? 'User Service',
  description: 'desc',
  base_url: 'https://api.example.com',
  enabled: overrides.enabled ?? true,
  tags: overrides.tags ?? ['api'],
  routes: overrides.routes ?? [
    { method: 'GET', path: '/api/users', enabled: true, tags: [] },
  ],
  created_by: USER_ID,
  created_at: '2026-04-21T00:00:00Z',
  updated_at: '2026-04-21T00:00:00Z',
});

const resetStore = async () => {
  const { result } = renderHook(() => useServiceStore());
  await act(async () => {
    result.current.setServices([]);
    result.current.clearError();
    result.current.clearMutationError();
  });
};

describe('useServiceStore (cloud)', () => {
  beforeEach(async () => {
    listMock.mockReset();
    createMock.mockReset();
    updateMock.mockReset();
    removeMock.mockReset();
    await resetStore();
  });

  it('reports cloud mode in state', () => {
    const { result } = renderHook(() => useServiceStore());
    expect(result.current.isCloud).toBe(true);
  });

  it('fetchServices maps cloud services into UI shape', async () => {
    listMock.mockResolvedValue([
      cloudService({
        routes: [
          { method: 'GET', path: '/api/users', enabled: true, tags: ['users'] },
          { method: 'POST', path: '/api/users', enabled: false, tags: ['users'] },
        ],
      }),
    ]);

    const { result } = renderHook(() => useServiceStore());
    await act(async () => {
      await result.current.fetchServices();
    });

    expect(listMock).toHaveBeenCalledWith(undefined);
    expect(result.current.isLoading).toBe(false);
    expect(result.current.services).toHaveLength(1);
    expect(result.current.services[0].name).toBe('User Service');
    expect(result.current.services[0].routes).toHaveLength(2);
    expect(result.current.filteredRoutes).toHaveLength(2);
  });

  it('fetchServices passes workspaceId when set via setWorkspaceFilter', async () => {
    listMock.mockResolvedValue([]);
    const { result } = renderHook(() => useServiceStore());

    await act(async () => {
      await result.current.setWorkspaceFilter('ws-123');
    });

    expect(result.current.workspaceFilter).toBe('ws-123');
    expect(listMock).toHaveBeenLastCalledWith({ workspaceId: 'ws-123' });
  });

  it('createService appends to state and passes workspace_id through', async () => {
    const created = cloudService({ id: 'svc-new', name: 'Orders', workspace_id: 'ws-1' });
    createMock.mockResolvedValue(created);

    const { result } = renderHook(() => useServiceStore());
    await act(async () => {
      await result.current.createService({
        name: 'Orders',
        description: '',
        base_url: '',
        workspace_id: 'ws-1',
      });
    });

    expect(createMock).toHaveBeenCalledWith(
      expect.objectContaining({ name: 'Orders', workspace_id: 'ws-1' })
    );
    expect(result.current.services.map((s) => s.id)).toEqual(['svc-new']);
    expect(result.current.services[0].workspace_id).toBe('ws-1');
  });

  it('removeService optimistically removes then calls delete', async () => {
    listMock.mockResolvedValue([cloudService({ id: 'svc-a' }), cloudService({ id: 'svc-b' })]);
    removeMock.mockResolvedValue(undefined);
    const { result } = renderHook(() => useServiceStore());
    await act(async () => {
      await result.current.fetchServices();
    });
    expect(result.current.services).toHaveLength(2);

    await act(async () => {
      await result.current.removeService('svc-a');
    });

    expect(removeMock).toHaveBeenCalledWith('svc-a');
    expect(result.current.services.map((s) => s.id)).toEqual(['svc-b']);
  });

  it('removeService rolls back state when delete fails', async () => {
    listMock.mockResolvedValue([cloudService({ id: 'svc-a' })]);
    removeMock.mockRejectedValue(new Error('boom'));
    const { result } = renderHook(() => useServiceStore());
    await act(async () => {
      await result.current.fetchServices();
    });

    await act(async () => {
      await expect(result.current.removeService('svc-a')).rejects.toThrow('boom');
    });

    expect(result.current.services).toHaveLength(1);
    expect(result.current.services[0].id).toBe('svc-a');
    expect(result.current.mutationError).toBe('boom');
  });

  it('toggleRoute persists via PATCH and rolls back on failure', async () => {
    listMock.mockResolvedValue([
      cloudService({
        id: 'svc-a',
        routes: [{ method: 'GET', path: '/api/users', enabled: true, tags: [] }],
      }),
    ]);
    const { result } = renderHook(() => useServiceStore());
    await act(async () => {
      await result.current.fetchServices();
    });

    // Success path first.
    updateMock.mockResolvedValueOnce(
      cloudService({
        id: 'svc-a',
        routes: [{ method: 'GET', path: '/api/users', enabled: false, tags: [] }],
      })
    );
    await act(async () => {
      await result.current.toggleRoute('svc-a', 'GET-/api/users', false);
    });
    expect(updateMock).toHaveBeenCalledWith(
      'svc-a',
      expect.objectContaining({
        routes: expect.arrayContaining([
          expect.objectContaining({ path: '/api/users', enabled: false }),
        ]),
      })
    );
    expect(result.current.services[0].routes[0].enabled).toBe(false);

    // Failure path rolls back.
    updateMock.mockRejectedValueOnce(new Error('network'));
    await act(async () => {
      await result.current.toggleRoute('svc-a', 'GET-/api/users', true);
    });
    await waitFor(() => {
      expect(result.current.mutationError).toBe('network');
    });
    expect(result.current.services[0].routes[0].enabled).toBe(false);
  });

  it('updateServiceDetails forwards explicit null to unassign workspace', async () => {
    listMock.mockResolvedValue([
      cloudService({ id: 'svc-a', workspace_id: 'ws-1' }),
    ]);
    const { result } = renderHook(() => useServiceStore());
    await act(async () => {
      await result.current.fetchServices();
    });

    updateMock.mockResolvedValueOnce(
      cloudService({ id: 'svc-a', workspace_id: null })
    );
    await act(async () => {
      await result.current.updateServiceDetails('svc-a', { workspace_id: null });
    });

    expect(updateMock).toHaveBeenCalledWith(
      'svc-a',
      expect.objectContaining({ workspace_id: null })
    );
    expect(result.current.services[0].workspace_id).toBeNull();
  });

  it('updateServiceDetails omits workspace_id when undefined', async () => {
    listMock.mockResolvedValue([
      cloudService({ id: 'svc-a', workspace_id: 'ws-1' }),
    ]);
    const { result } = renderHook(() => useServiceStore());
    await act(async () => {
      await result.current.fetchServices();
    });

    updateMock.mockResolvedValueOnce(
      cloudService({ id: 'svc-a', workspace_id: 'ws-1', name: 'Renamed' })
    );
    await act(async () => {
      await result.current.updateServiceDetails('svc-a', { name: 'Renamed' });
    });

    const [, patch] = updateMock.mock.calls[0];
    expect(patch).not.toHaveProperty('workspace_id');
  });

  it('updateService optimistically applies then reconciles with server response', async () => {
    listMock.mockResolvedValue([cloudService({ id: 'svc-a', enabled: true })]);
    const { result } = renderHook(() => useServiceStore());
    await act(async () => {
      await result.current.fetchServices();
    });

    updateMock.mockResolvedValueOnce(
      cloudService({ id: 'svc-a', enabled: false })
    );
    await act(async () => {
      await result.current.updateService('svc-a', { enabled: false });
    });

    expect(updateMock).toHaveBeenCalledWith(
      'svc-a',
      expect.objectContaining({ enabled: false })
    );
    expect(result.current.services[0].enabled).toBe(false);
  });
});
