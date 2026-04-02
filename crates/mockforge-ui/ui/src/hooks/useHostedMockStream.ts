/**
 * Real-time streaming for hosted mock deployments.
 *
 * Connects a WebSocket to the hosted mock's management endpoint
 * (e.g. wss://<deployment>.fly.dev/__mockforge/ws) so that Pro/Team
 * users get live request logs and metrics updates in the cloud UI.
 *
 * The hook also maintains the global connection store so the header
 * indicator shows "Connected" when at least one stream is active.
 */

import { useEffect, useRef, useCallback, useState } from 'react';
import { useConnectionStore } from '@/components/layout/ConnectionStatus';
import { useWebSocket } from './useWebSocket';
import { logger } from '@/utils/logger';

export interface HostedMockLogEntry {
  timestamp: string;
  method: string;
  path: string;
  status: number;
  latency_ms: number;
  request_id?: string;
}

export interface HostedMockMetricsSnapshot {
  total_requests: number;
  total_errors: number;
  error_rate: number;
  avg_latency_ms: number;
  p95_latency_ms: number;
  active_connections: number;
  requests_per_second: number;
}

interface HostedMockEvent {
  type: string;
  timestamp?: string;
  log?: HostedMockLogEntry;
  stats?: HostedMockMetricsSnapshot;
  message?: string;
}

export interface UseHostedMockStreamOptions {
  /** Whether the stream should be active */
  enabled?: boolean;
  /** Max number of log entries to keep in memory */
  maxLogs?: number;
}

export interface UseHostedMockStreamReturn {
  /** Whether the WebSocket is currently connected */
  connected: boolean;
  /** Rolling window of recent log entries */
  logs: HostedMockLogEntry[];
  /** Latest metrics snapshot */
  metrics: HostedMockMetricsSnapshot | null;
  /** Manually reconnect */
  reconnect: () => void;
  /** Manually disconnect */
  disconnect: () => void;
}

/**
 * Derive a WebSocket URL from a hosted mock's HTTP deployment URL.
 * e.g. "https://my-mock.fly.dev" → "wss://my-mock.fly.dev/__mockforge/ws"
 */
function toWsUrl(deploymentUrl: string): string {
  const url = new URL(deploymentUrl);
  const protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${protocol}//${url.host}/__mockforge/ws`;
}

export function useHostedMockStream(
  deploymentUrl: string | undefined,
  options: UseHostedMockStreamOptions = {},
): UseHostedMockStreamReturn {
  const { enabled = true, maxLogs = 200 } = options;

  const [logs, setLogs] = useState<HostedMockLogEntry[]>([]);
  const [metrics, setMetrics] = useState<HostedMockMetricsSnapshot | null>(null);

  const incrementStreams = useConnectionStore((s) => s.incrementHostedMockStreams);
  const decrementStreams = useConnectionStore((s) => s.decrementHostedMockStreams);
  const trackedRef = useRef(false);

  // Build the absolute WS URL — useWebSocket allows absolute URLs even in cloud mode
  const wsUrl = deploymentUrl ? toWsUrl(deploymentUrl) : '';

  const { lastMessage, connected, connect, disconnect } = useWebSocket(wsUrl, {
    autoConnect: enabled && !!deploymentUrl,
    reconnect: {
      enabled: true,
      maxAttempts: 10,
      delay: 3000,
    },
  });

  // Track connected streams in global store for the header indicator
  useEffect(() => {
    if (connected && !trackedRef.current) {
      trackedRef.current = true;
      incrementStreams();
    } else if (!connected && trackedRef.current) {
      trackedRef.current = false;
      decrementStreams();
    }
  }, [connected, incrementStreams, decrementStreams]);

  // Clean up on unmount
  useEffect(() => {
    return () => {
      if (trackedRef.current) {
        decrementStreams();
        trackedRef.current = false;
      }
    };
  }, [decrementStreams]);

  // Process incoming messages
  const handleMessage = useCallback(
    (msg: MessageEvent) => {
      try {
        const event: HostedMockEvent = JSON.parse(msg.data as string);

        switch (event.type) {
          case 'request_logged':
          case 'log':
            if (event.log) {
              setLogs((prev) => {
                const next = [event.log!, ...prev];
                return next.length > maxLogs ? next.slice(0, maxLogs) : next;
              });
            }
            break;

          case 'stats_updated':
          case 'metrics':
            if (event.stats) {
              setMetrics(event.stats);
            }
            break;

          case 'connected':
            logger.info('Hosted mock stream connected:', event.message);
            break;

          default:
            break;
        }
      } catch (err) {
        logger.warn('Failed to parse hosted mock stream message', err);
      }
    },
    [maxLogs],
  );

  useEffect(() => {
    if (lastMessage) {
      handleMessage(lastMessage);
    }
  }, [lastMessage, handleMessage]);

  return {
    connected,
    logs,
    metrics,
    reconnect: connect,
    disconnect,
  };
}
