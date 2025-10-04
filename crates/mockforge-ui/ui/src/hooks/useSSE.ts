/**
 * React hook for Server-Sent Events (SSE) connections
 */

import { useEffect, useRef, useState, useCallback } from 'react';

export interface SSEHookOptions {
  /**
   * Whether to automatically connect when the hook is mounted
   */
  autoConnect?: boolean;
  
  /**
   * Retry configuration
   */
  retry?: {
    enabled?: boolean;
    maxAttempts?: number;
    delay?: number;
  };
}

export interface SSEHookReturn<T = unknown> {
  /**
   * Latest data received from SSE
   */
  data: T | null;
  
  /**
   * Connection state
   */
  readyState: number;
  
  /**
   * Error if any occurred
   */
  error: Event | null;
  
  /**
   * Manually connect to SSE
   */
  connect: () => void;
  
  /**
   * Manually disconnect from SSE
   */
  disconnect: () => void;
  
  /**
   * Whether the connection is open
   */
  isConnected: boolean;
}

/**
 * Hook for Server-Sent Events
 */
export function useSSE<T = unknown>(
  url: string,
  options: SSEHookOptions = {}
): SSEHookReturn<T> {
  const {
    autoConnect = true,
    retry = { enabled: true, maxAttempts: 3, delay: 1000 }
  } = options;
  
  const [data, setData] = useState<T | null>(null);
  const [readyState, setReadyState] = useState<number>(EventSource.CLOSED);
  const [error, setError] = useState<Event | null>(null);
  const eventSourceRef = useRef<EventSource | null>(null);
  const retryAttemptsRef = useRef(0);
  const retryTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  
  const disconnect = useCallback(() => {
    if (eventSourceRef.current) {
      if (import.meta.env.DEV) {
        console.log('SSE: Disconnecting from', url);
      }
      eventSourceRef.current.close();
      eventSourceRef.current = null;
      setReadyState(EventSource.CLOSED);
    }

    if (retryTimeoutRef.current) {
      clearTimeout(retryTimeoutRef.current);
      retryTimeoutRef.current = null;
    }
  }, [url]);

  const connect = useCallback(() => {
    // Prevent multiple connections
    if (eventSourceRef.current && eventSourceRef.current.readyState !== EventSource.CLOSED) {
      if (import.meta.env.DEV) {
        console.log('SSE: Already connected to', url);
      }
      return;
    }

    // Clean up existing connection
    disconnect();

    try {
      if (import.meta.env.DEV) {
        console.log('SSE: Connecting to', url);
      }
      const eventSource = new EventSource(url);
      eventSourceRef.current = eventSource;

      eventSource.onopen = () => {
        if (import.meta.env.DEV) {
          console.log('SSE: Connection opened to', url);
        }
        setReadyState(EventSource.OPEN);
        setError(null);
        retryAttemptsRef.current = 0;
      };

      eventSource.onmessage = (event) => {
        try {
          const parsedData = JSON.parse(event.data);
          setData(parsedData);
        } catch (e) {
          if (import.meta.env.DEV) {
            console.warn('SSE: Failed to parse message data:', e);
          }
          // If parsing fails, use raw data
          setData(event.data as T);
        }
      };

      eventSource.onerror = (event) => {
        if (import.meta.env.DEV) {
          console.error('SSE: Connection error', event, 'ReadyState:', eventSource.readyState);
        }
        setError(event);
        setReadyState(eventSource.readyState);

        // Handle retry logic
        if (retry.enabled && retryAttemptsRef.current < (retry.maxAttempts || 3)) {
          retryAttemptsRef.current += 1;
          if (import.meta.env.DEV) {
            console.log('SSE: Retrying connection, attempt', retryAttemptsRef.current);
          }

          retryTimeoutRef.current = setTimeout(() => {
            connect();
          }, retry.delay || 1000);
        }
      };

      // Handle custom events (like 'new_logs')
      eventSource.addEventListener('new_logs', (event: MessageEvent) => {
        if (import.meta.env.DEV) {
          console.log('SSE: Received new_logs event');
        }
        try {
          const parsedData = JSON.parse(event.data);
          setData(parsedData);
        } catch (e) {
          if (import.meta.env.DEV) {
            console.error('SSE: Failed to parse logs data:', e);
          }
          setData(event.data as T);
        }
      });

      // Handle keep-alive events
      eventSource.addEventListener('keep_alive', () => {
        if (import.meta.env.DEV) {
          console.log('SSE: Received keep_alive event');
        }
        // Just acknowledge the keep-alive, don't update data
      });

      // Handle test events
      eventSource.addEventListener('test', (event) => {
        if (import.meta.env.DEV) {
          console.log('SSE: Received test event:', event.data);
        }
      });

    } catch (e) {
      setError(e as Event);
      setReadyState(EventSource.CLOSED);
    }
  }, [url, disconnect, retry.enabled, retry.maxAttempts, retry.delay]);
  
  // Auto-connect on mount
  useEffect(() => {
    if (autoConnect) {
      connect();
    }

    return () => {
      disconnect();
    };
  }, [autoConnect, url, connect, disconnect]);

  // Update connection when URL changes - removed to prevent reconnection loops
  // The connection is already handled by the mount effect above
  
  const isConnected = readyState === EventSource.OPEN;
  
  return {
    data,
    readyState,
    error,
    connect,
    disconnect,
    isConnected,
  };
}

/**
 * Hook specifically for real-time logs via SSE
 */
export function useLogsSSE() {
  return useSSE<unknown[]>('/__mockforge/logs/sse', {
    autoConnect: true,
    retry: {
      enabled: true,
      maxAttempts: 5,
      delay: 2000,
    },
  });
}