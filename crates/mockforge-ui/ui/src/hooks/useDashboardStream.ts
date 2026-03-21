/**
 * Real-time dashboard updates via WebSocket
 *
 * Connects to the MockForge management WebSocket at /__mockforge/ws
 * and pushes StatsUpdated events into React Query's cache, reducing
 * the need for polling while keeping polling as a fallback.
 */

import { useEffect, useRef, useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useWebSocket } from './useWebSocket';
import { useConnectionStore } from '@/components/layout/ConnectionStatus';
import { queryKeys } from './api/queryKeys';
import { logger } from '@/utils/logger';
import type { DashboardData } from '../types';

/**
 * Shape of a MockEvent received over the management WebSocket.
 * The backend tags events with `"type": "event_name"` (snake_case).
 */
interface MockWsEvent {
  type: string;
  timestamp: string;
  /** Present when type === "stats_updated" */
  stats?: {
    uptime_seconds: number;
    total_requests: number;
    active_mocks: number;
    enabled_mocks: number;
    registered_routes: number;
  };
  /** Present when type === "mock_created" or "mock_updated" */
  mock?: {
    id: string;
    name: string;
    method: string;
    path: string;
    enabled: boolean;
  };
  /** Present when type === "mock_deleted" */
  id?: string;
  /** Present when type === "connected" */
  message?: string;
}

interface UseDashboardStreamOptions {
  /** Whether the stream is enabled (default: true) */
  enabled?: boolean;
}

/**
 * Hook that connects the dashboard to the management WebSocket for
 * real-time updates. When connected, it patches React Query cache
 * with incoming data so the UI updates instantly.
 *
 * Polling (via useDashboard/useLogs) continues as a fallback but
 * at a reduced frequency when the WebSocket is active.
 */
export function useDashboardStream(options: UseDashboardStreamOptions = {}) {
  const { enabled = true } = options;
  const queryClient = useQueryClient();
  const setWsState = useConnectionStore((state) => state.setWsState);
  const connectedRef = useRef(false);

  const { lastMessage, connected, connect, disconnect } = useWebSocket(
    '/__mockforge/ws',
    {
      autoConnect: enabled,
      reconnect: {
        enabled: true,
        maxAttempts: 10,
        delay: 3000,
      },
    }
  );

  // Update global connection state
  useEffect(() => {
    if (connected) {
      setWsState('connected');
      connectedRef.current = true;
    } else if (connectedRef.current) {
      // Only set reconnecting if we were previously connected
      setWsState('reconnecting');
    }
  }, [connected, setWsState]);

  // Process incoming WebSocket messages
  useEffect(() => {
    if (!lastMessage) return;

    try {
      const event: MockWsEvent = JSON.parse(lastMessage.data as string);
      handleEvent(event);
    } catch (err) {
      logger.warn('Failed to parse WebSocket message', err);
    }
  }, [lastMessage]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleEvent = useCallback(
    (event: MockWsEvent) => {
      switch (event.type) {
        case 'stats_updated':
          handleStatsUpdated(event);
          break;

        case 'mock_created':
        case 'mock_updated':
        case 'mock_deleted':
          // When mocks change, invalidate dashboard + routes to trigger a refetch
          queryClient.invalidateQueries({ queryKey: queryKeys.dashboard });
          queryClient.invalidateQueries({ queryKey: queryKeys.routes });
          // Also refresh logs since new mock configs may affect routing
          queryClient.invalidateQueries({ queryKey: queryKeys.logs });
          break;

        case 'connected':
          logger.info('Dashboard WebSocket connected:', event.message);
          break;

        default:
          // Other events (state_machine_*, state_transitioned, etc.)
          // can trigger targeted invalidations in the future
          break;
      }
    },
    [queryClient] // eslint-disable-line react-hooks/exhaustive-deps
  );

  const handleStatsUpdated = useCallback(
    (event: MockWsEvent) => {
      if (!event.stats) return;

      const { stats } = event;

      // Patch the dashboard cache directly for instant UI updates
      queryClient.setQueryData<DashboardData>(
        queryKeys.dashboard,
        (oldData) => {
          if (!oldData) return oldData;

          return {
            ...oldData,
            system: {
              ...oldData.system,
              uptime_seconds: stats.uptime_seconds,
              total_routes: stats.registered_routes,
            },
            metrics: {
              ...oldData.metrics,
              total_requests: stats.total_requests,
            },
          };
        }
      );

      // Also invalidate logs on stats updates so request log stays fresh
      queryClient.invalidateQueries({ queryKey: queryKeys.logs });
    },
    [queryClient]
  );

  return {
    /** Whether the WebSocket is currently connected */
    connected,
    /** Manually reconnect */
    reconnect: connect,
    /** Manually disconnect */
    disconnect,
  };
}
