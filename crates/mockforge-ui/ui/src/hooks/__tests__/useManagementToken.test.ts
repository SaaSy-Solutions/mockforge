import { describe, expect, it, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { fetchJsonWithErrorBody } from '@/services/api/client';
import {
  managementDocsUrl,
  managementHeaders,
  managementWsProtocols,
  useManagementToken,
} from '../useManagementToken';

vi.mock('@/services/api/client', () => ({ fetchJsonWithErrorBody: vi.fn() }));

describe('management token helpers', () => {
  it('puts the token in the docs URL fragment, URL-encoded', () => {
    expect(managementDocsUrl('https://m.example.dev/', 'a b&c#d')).toBe(
      'https://m.example.dev/__mockforge/docs#management_token=a%20b%26c%23d',
    );
  });

  it('links to the bare docs page without a token', () => {
    expect(managementDocsUrl('https://m.example.dev', null)).toBe(
      'https://m.example.dev/__mockforge/docs',
    );
  });

  it('builds the header and WebSocket subprotocols', () => {
    expect(managementHeaders('tok')).toEqual({ 'X-MockForge-Management-Token': 'tok' });
    expect(managementWsProtocols('tok')).toEqual([
      'mockforge-management',
      'mockforge-token.tok',
    ]);
  });
});

describe('useManagementToken', () => {
  beforeEach(() => {
    vi.mocked(fetchJsonWithErrorBody).mockReset();
  });

  it('loads the token from the registry', async () => {
    vi.mocked(fetchJsonWithErrorBody).mockResolvedValue({
      token: 'mfm_1',
      header: 'X-MockForge-Management-Token',
    });
    const { result } = renderHook(() => useManagementToken('dep 1'));
    expect(result.current.status).toBe('loading');
    await waitFor(() => expect(result.current.status).toBe('ready'));
    expect(result.current.token).toBe('mfm_1');
    expect(fetchJsonWithErrorBody).toHaveBeenCalledWith(
      '/api/v1/hosted-mocks/dep%201/management-token',
    );
  });

  it('settles on unavailable without retrying when access is denied', async () => {
    vi.mocked(fetchJsonWithErrorBody).mockRejectedValue(new Error('Access denied'));
    const { result, rerender } = renderHook(() => useManagementToken('dep-1'));
    await waitFor(() => expect(result.current.status).toBe('unavailable'));
    rerender();
    rerender();
    expect(result.current.token).toBeNull();
    expect(result.current.error).toBe('Access denied');
    expect(fetchJsonWithErrorBody).toHaveBeenCalledTimes(1);
  });

  it('does not fetch when disabled or without a deployment id', () => {
    renderHook(() => useManagementToken('dep-1', false));
    renderHook(() => useManagementToken(null));
    expect(fetchJsonWithErrorBody).not.toHaveBeenCalled();
  });
});
