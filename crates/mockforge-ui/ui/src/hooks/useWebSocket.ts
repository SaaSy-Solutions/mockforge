//! WebSocket hook for real-time updates
//!
//! Provides a React hook for managing WebSocket connections with automatic
//! reconnection and message handling.

import { useEffect, useRef, useState, useCallback } from 'react';
import { logger } from '@/utils/logger';

interface UseWebSocketOptions {
  autoConnect?: boolean;
  reconnect?: {
    enabled: boolean;
    maxAttempts?: number;
    delay?: number;
  };
}

interface UseWebSocketReturn {
  lastMessage: MessageEvent | null;
  sendMessage: (message: string | object) => void;
  connected: boolean;
  connect: () => void;
  disconnect: () => void;
}

export function useWebSocket(
  url: string,
  options: UseWebSocketOptions = {}
): UseWebSocketReturn {
  const {
    autoConnect = true,
    reconnect = {
      enabled: true,
      maxAttempts: 5,
      delay: 2000,
    },
  } = options;

  const [lastMessage, setLastMessage] = useState<MessageEvent | null>(null);
  const [connected, setConnected] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectAttemptsRef = useRef(0);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const shouldReconnectRef = useRef(true);

  const connect = useCallback(() => {
    shouldReconnectRef.current = true;

    if (wsRef.current?.readyState === WebSocket.OPEN) {
      return; // Already connected
    }

    try {
      // Convert relative URL to WebSocket URL
      const wsUrl = url.startsWith('ws://') || url.startsWith('wss://')
        ? url
        : `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}${url}`;

      const ws = new WebSocket(wsUrl);

      ws.onopen = () => {
        logger.info('WebSocket connected', { url });
        setConnected(true);
        reconnectAttemptsRef.current = 0;
      };

      ws.onmessage = (event) => {
        setLastMessage(event);
      };

      ws.onerror = (error) => {
        logger.error('WebSocket error', error);
      };

      ws.onclose = () => {
        logger.info('WebSocket disconnected', { url });
        setConnected(false);
        setLastMessage(null);

        // Attempt reconnection if enabled
        if (reconnect.enabled && shouldReconnectRef.current) {
          const maxAttempts = reconnect.maxAttempts || 5;
          const delay = reconnect.delay || 2000;

          if (reconnectAttemptsRef.current < maxAttempts) {
            reconnectAttemptsRef.current += 1;
            logger.info(`Reconnecting WebSocket (attempt ${reconnectAttemptsRef.current}/${maxAttempts})...`);

            reconnectTimeoutRef.current = setTimeout(() => {
              connect();
            }, delay);
          } else {
            logger.warn('WebSocket reconnection failed: max attempts reached');
          }
        }
      };

      wsRef.current = ws;
    } catch (error) {
      logger.error('Failed to create WebSocket connection', error);
      setConnected(false);
    }
  }, [url, reconnect]);

  const disconnect = useCallback(() => {
    shouldReconnectRef.current = false;

    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    setConnected(false);
    setLastMessage(null);
  }, []);

  const sendMessage = useCallback((message: string | object) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      const data = typeof message === 'string' ? message : JSON.stringify(message);
      wsRef.current.send(data);
    } else {
      logger.warn('WebSocket is not connected, cannot send message');
    }
  }, []);

  useEffect(() => {
    if (autoConnect) {
      connect();
    }

    return () => {
      disconnect();
    };
  }, [autoConnect, connect, disconnect]);

  return {
    lastMessage,
    sendMessage,
    connected,
    connect,
    disconnect,
  };
}
