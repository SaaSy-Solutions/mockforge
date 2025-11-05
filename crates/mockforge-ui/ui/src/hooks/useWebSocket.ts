import { useEffect, useRef, useState } from 'react';

interface UseWebSocketOptions {
  onOpen?: () => void;
  onClose?: () => void;
  onError?: (error: Event) => void;
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

export function useWebSocket(
  url: string,
  options: UseWebSocketOptions = {}
) {
  const {
    onOpen,
    onClose,
    onError,
    reconnectInitialDelay = 1000,
    reconnectMaxDelay = 30000,
    reconnectMaxRetries = 10,
  } = options;

  const [lastMessage, setLastMessage] = useState<string | null>(null);
  const [readyState, setReadyState] = useState<number>(WebSocket.CONNECTING);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectAttempts = useRef(0);
  const reconnectTimeout = useRef<NodeJS.Timeout | null>(null);
  const manualDisconnect = useRef(false);

  const connect = () => {
    // Clear any existing reconnection timer
    if (reconnectTimeout.current) {
      clearTimeout(reconnectTimeout.current);
      reconnectTimeout.current = null;
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

    try {
      // Convert HTTP URL to WebSocket URL
      const wsUrl = url.startsWith('ws')
        ? url
        : `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}${url}`;

      const ws = new WebSocket(wsUrl);

      ws.onopen = () => {
        console.log('WebSocket connected:', url);
        setReadyState(WebSocket.OPEN);
        reconnectAttempts.current = 0; // Reset on successful connection
        manualDisconnect.current = false;
        onOpen?.();
      };

      ws.onmessage = (event) => {
        setLastMessage(event.data);
      };

      ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        onError?.(error);
      };

      ws.onclose = () => {
        console.log('WebSocket closed:', url);
        setReadyState(WebSocket.CLOSED);
        onClose?.();

        // Attempt reconnection if not manually disconnected and reconnection is enabled
        if (!manualDisconnect.current) {
          attemptReconnect();
        }
      };

      wsRef.current = ws;
    } catch (error) {
      console.error('Failed to create WebSocket:', error);
    }
  };

  /**
   * Attempt to reconnect with exponential backoff
   * Matches the pattern used in VS Code extension's MockForgeClient
   */
  const attemptReconnect = () => {
    // Check if we've exceeded max retries (0 means infinite)
    if (reconnectMaxRetries > 0 && reconnectAttempts.current >= reconnectMaxRetries) {
      console.warn(
        `Max reconnection attempts (${reconnectMaxRetries}) reached. Stopping reconnection.`
      );
      return;
    }

    reconnectAttempts.current++;

    // Calculate delay with exponential backoff
    // Formula: delay = min(initialDelay * 2^(attempt - 1), maxDelay)
    const delay = Math.min(
      reconnectInitialDelay * Math.pow(2, reconnectAttempts.current - 1),
      reconnectMaxDelay
    );

    console.log(
      `Attempting to reconnect in ${delay}ms (attempt ${reconnectAttempts.current}/${reconnectMaxRetries || '∞'})`
    );

    reconnectTimeout.current = setTimeout(() => {
      connect();
    }, delay);
  };

  const disconnect = () => {
    manualDisconnect.current = true;

    // Clear reconnection timer
    if (reconnectTimeout.current) {
      clearTimeout(reconnectTimeout.current);
      reconnectTimeout.current = null;
    }

    // Close WebSocket
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    reconnectAttempts.current = 0;
  };

  const send = (data: string | object) => {
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      const message = typeof data === 'string' ? data : JSON.stringify(data);
      wsRef.current.send(message);
    } else {
      console.warn('WebSocket is not open. Cannot send message.');
    }
  };

  useEffect(() => {
    connect();

    return () => {
      disconnect();
    };
  }, [url]);

  return {
    lastMessage,
    readyState,
    send,
    disconnect,
    reconnect: connect,
  };
}
