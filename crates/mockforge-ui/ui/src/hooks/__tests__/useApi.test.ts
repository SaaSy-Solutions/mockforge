/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';
import {
  useDashboard,
  useHealth,
  useServerInfo,
  useLogs,
  useRestartServers,
  queryKeys,
} from '../useApi';

// Mock the API services
vi.mock('../../services/api', () => ({
  apiService: {},
  dashboardApi: {
    getDashboard: vi.fn(),
    getHealth: vi.fn(),
  },
  serverApi: {
    getServerInfo: vi.fn(),
    getRestartStatus: vi.fn(),
    restartServer: vi.fn(),
  },
  logsApi: {
    getLogs: vi.fn(),
    clearLogs: vi.fn(),
  },
  routesApi: {},
  metricsApi: {},
  configApi: {},
  validationApi: {},
  envApi: {},
  filesApi: {},
  fixturesApi: {},
  smokeTestsApi: {},
  importApi: {},
}));

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });

  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
};

describe('useApi hooks', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('useDashboard', () => {
    it('fetches dashboard data', async () => {
      const { dashboardApi } = await import('../../services/api');
      const mockData = {
        servers: [],
        metrics: { total_requests: 100 },
      };
      (dashboardApi.getDashboard as any).mockResolvedValue(mockData);

      const { result } = renderHook(() => useDashboard(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      expect(result.current.data).toEqual(mockData);
      expect(dashboardApi.getDashboard).toHaveBeenCalled();
    });

    it('handles dashboard fetch errors', async () => {
      const { dashboardApi } = await import('../../services/api');
      (dashboardApi.getDashboard as any).mockRejectedValue(new Error('Failed'));

      const { result } = renderHook(() => useDashboard(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(result.current.isError).toBe(true);
      });

      expect(result.current.error).toBeTruthy();
    });

    it('uses correct query key', () => {
      expect(queryKeys.dashboard).toEqual(['dashboard']);
    });
  });

  describe('useHealth', () => {
    it('fetches health status', async () => {
      const { dashboardApi } = await import('../../services/api');
      const mockHealth = { status: 'healthy', uptime: 3600 };
      (dashboardApi.getHealth as any).mockResolvedValue(mockHealth);

      const { result } = renderHook(() => useHealth(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      expect(result.current.data).toEqual(mockHealth);
    });
  });

  describe('useServerInfo', () => {
    it('fetches server information', async () => {
      const { serverApi } = await import('../../services/api');
      const mockInfo = { version: '1.0.0', environment: 'development' };
      (serverApi.getServerInfo as any).mockResolvedValue(mockInfo);

      const { result } = renderHook(() => useServerInfo(), {
        wrapper: createWrapper(),
      });

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      expect(result.current.data).toEqual(mockInfo);
    });
  });

  describe('useRestartServers', () => {
    it('restarts servers with reason', async () => {
      const { serverApi } = await import('../../services/api');
      (serverApi.restartServer as any).mockResolvedValue({ success: true });

      const { result } = renderHook(() => useRestartServers(), {
        wrapper: createWrapper(),
      });

      result.current.mutate('Configuration change');

      await waitFor(() => {
        expect(result.current.isSuccess).toBe(true);
      });

      expect(serverApi.restartServer).toHaveBeenCalledWith('Configuration change');
    });
  });
});
