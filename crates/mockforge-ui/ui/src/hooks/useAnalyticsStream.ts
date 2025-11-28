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
  /**
   * Initial delay for reconnection in milliseconds (default: 1000)
   * Used as base for exponential backoff calculation
   */
  reconnectInitialDelay?: number;
  /**
   * Maximum delay between reconnection attempts in milliseconds (default: 30000)
   * Exponential backoff will cap at this value
   */
  reconnectMaxDelay?: number;
  /**
   * Maximum number of reconnection attempts (default: 10)
   * Set to 0 for infinite retries
   */
  reconnectMaxRetries?: number;
}

export function useAnalyticsStream(options: UseAnalyticsStreamOptions = {}) {
  const {
    enabled = true,
    config,
    onMessage,
    onError,
    onConnect,
    onDisconnect,
    reconnectInitialDelay = 1000,
    reconnectMaxDelay = 30000,
    reconnectMaxRetries = 10,
  } = options;

  const [isConnected, setIsConnected] = useState(false);
  const [lastUpdate, setLastUpdate] = useState<MetricsUpdate | null>(null);
  const [error, setError] = useState<string | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const reconnectAttemptsRef = useRef(0);
  const manualDisconnectRef = useRef(false);

  /**
   * Attempt to reconnect with exponential backoff
   * Matches the pattern used in VS Code extension's MockForgeClient
   */
  const attemptReconnect = () => {
    // Check if we've exceeded max retries (0 means infinite)
    if (reconnectMaxRetries > 0 && reconnectAttemptsRef.current >= reconnectMaxRetries) {
      console.warn(
        `Max reconnection attempts (${reconnectMaxRetries}) reached. Stopping reconnection.`
      );
      setError('Max reconnection attempts reached');
      return;
    }

    reconnectAttemptsRef.current++;

    // Calculate delay with exponential backoff
    // Formula: delay = min(initialDelay * 2^(attempt - 1), maxDelay)
    const delay = Math.min(
      reconnectInitialDelay * Math.pow(2, reconnectAttemptsRef.current - 1),
      reconnectMaxDelay
    );

    console.log(
      `Attempting to reconnect analytics stream in ${delay}ms (attempt ${reconnectAttemptsRef.current}/${reconnectMaxRetries || '∞'})`
    );

    reconnectTimeoutRef.current = setTimeout(() => {
      connect();
    }, delay);
  };

  const connect = () => {
    if (!enabled || wsRef.current?.readyState === WebSocket.OPEN) {
      return;
    }

    // Clear any existing reconnection timer
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    // Close existing connection if any
    if (wsRef.current) {
      // Clear event handlers to prevent memory leaks
      wsRef.current.onopen = null;
      wsRef.current.onmessage = null;
      wsRef.current.onerror = null;
      wsRef.current.onclose = null;
      wsRef.current.close();
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    const url = `${protocol}//${host}/api/v2/analytics/stream`;

    const ws = new WebSocket(url);

    ws.onopen = () => {
      console.log('Analytics stream connected');
      setIsConnected(true);
      setError(null);
      reconnectAttemptsRef.current = 0; // Reset on successful connection
      manualDisconnectRef.current = false;

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

      // Attempt reconnection if not manually disconnected and enabled
      if (!manualDisconnectRef.current && enabled) {
        attemptReconnect();
      }
    };

    wsRef.current = ws;
  };

  const disconnect = () => {
    manualDisconnectRef.current = true;

    // Clear reconnection timer
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    // Close WebSocket
    if (wsRef.current) {
      // Clear event handlers to prevent memory leaks
      wsRef.current.onopen = null;
      wsRef.current.onmessage = null;
      wsRef.current.onerror = null;
      wsRef.current.onclose = null;
      wsRef.current.close();
      wsRef.current = null;
    }

    setIsConnected(false);
    reconnectAttemptsRef.current = 0;
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
