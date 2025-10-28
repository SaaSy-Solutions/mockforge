/**
 * WebSocket hook for real-time analytics updates
 */

import { useEffect, useRef, useState } from 'react';

export interface MetricsUpdate {
  timestamp: number;
  total_requests: number;
  total_errors: number;
  error_rate: number;
  avg_latency_ms: number;
  p95_latency_ms: number;
  p99_latency_ms: number;
  active_connections: number;
  requests_per_second: number;
}

export interface StreamConfig {
  interval_seconds?: number;
  duration_seconds?: number;
  protocol?: string;
  endpoint?: string;
  workspace_id?: string;
}

export interface UseAnalyticsStreamOptions {
  enabled?: boolean;
  config?: StreamConfig;
  onMessage?: (update: MetricsUpdate) => void;
  onError?: (error: Event) => void;
  onConnect?: () => void;
  onDisconnect?: () => void;
}

export function useAnalyticsStream(options: UseAnalyticsStreamOptions = {}) {
  const {
    enabled = true,
    config,
    onMessage,
    onError,
    onConnect,
    onDisconnect,
  } = options;

  const [isConnected, setIsConnected] = useState(false);
  const [lastUpdate, setLastUpdate] = useState<MetricsUpdate | null>(null);
  const [error, setError] = useState<string | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const reconnectAttemptsRef = useRef(0);

  const MAX_RECONNECT_ATTEMPTS = 5;
  const RECONNECT_DELAY = 3000; // 3 seconds

  const connect = () => {
    if (!enabled || wsRef.current?.readyState === WebSocket.OPEN) {
      return;
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    const url = `${protocol}//${host}/api/v2/analytics/stream`;

    const ws = new WebSocket(url);

    ws.onopen = () => {
      console.log('Analytics stream connected');
      setIsConnected(true);
      setError(null);
      reconnectAttemptsRef.current = 0;

      // Send configuration if provided
      if (config) {
        ws.send(JSON.stringify(config));
      }

      onConnect?.();
    };

    ws.onmessage = (event) => {
      try {
        const update: MetricsUpdate = JSON.parse(event.data);
        setLastUpdate(update);
        onMessage?.(update);
      } catch (err) {
        console.error('Failed to parse metrics update:', err);
        setError('Failed to parse metrics update');
      }
    };

    ws.onerror = (event) => {
      console.error('WebSocket error:', event);
      setError('WebSocket connection error');
      onError?.(event);
    };

    ws.onclose = () => {
      console.log('Analytics stream disconnected');
      setIsConnected(false);
      onDisconnect?.();

      // Attempt reconnection if enabled
      if (enabled && reconnectAttemptsRef.current < MAX_RECONNECT_ATTEMPTS) {
        reconnectAttemptsRef.current++;
        console.log(
          `Reconnecting... (attempt ${reconnectAttemptsRef.current}/${MAX_RECONNECT_ATTEMPTS})`
        );

        reconnectTimeoutRef.current = setTimeout(() => {
          connect();
        }, RECONNECT_DELAY);
      } else if (reconnectAttemptsRef.current >= MAX_RECONNECT_ATTEMPTS) {
        setError('Max reconnection attempts reached');
      }
    };

    wsRef.current = ws;
  };

  const disconnect = () => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    setIsConnected(false);
  };

  const updateConfig = (newConfig: StreamConfig) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(newConfig));
    }
  };

  useEffect(() => {
    if (enabled) {
      connect();
    }

    return () => {
      disconnect();
    };
  }, [enabled]);

  // Update config when it changes
  useEffect(() => {
    if (config && isConnected) {
      updateConfig(config);
    }
  }, [config, isConnected]);

  return {
    isConnected,
    lastUpdate,
    error,
    updateConfig,
    reconnect: connect,
    disconnect,
  };
}
