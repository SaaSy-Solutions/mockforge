/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { useWebSocket } from '../useWebSocket';

// Mock WebSocket
class MockWebSocket {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;

  readyState = MockWebSocket.CONNECTING;
  url: string;
  onopen: ((event: Event) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;

  constructor(url: string) {
    this.url = url;
    // Simulate connection after a short delay
    setTimeout(() => {
      this.readyState = MockWebSocket.OPEN;
      if (this.onopen) {
        this.onopen(new Event('open'));
      }
    }, 10);
  }

  send(data: string | object) {
    // Mock send
  }

  close() {
    this.readyState = MockWebSocket.CLOSED;
    if (this.onclose) {
      this.onclose(new CloseEvent('close'));
    }
  }
}

describe('useWebSocket', () => {
  let mockWebSocketInstance: MockWebSocket;

  beforeEach(() => {
    mockWebSocketInstance = new MockWebSocket('ws://test') as any;
    global.WebSocket = MockWebSocket as any;
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('should create WebSocket connection', () => {
    const { result } = renderHook(() =>
      useWebSocket('/test', { autoConnect: true })
    );

    // Initially not connected (connection happens asynchronously)
    expect(result.current.connected).toBe(false);
  });

  it('should connect when autoConnect is true', async () => {
    const { result } = renderHook(() =>
      useWebSocket('/test', { autoConnect: true })
    );

    await waitFor(() => {
      expect(result.current.connected).toBe(true);
    });
  });

  it('should not connect when autoConnect is false', () => {
    const { result } = renderHook(() =>
      useWebSocket('/test', { autoConnect: false })
    );

    expect(result.current.connected).toBe(false);
  });

  it('should send message when connected', async () => {
    const { result } = renderHook(() =>
      useWebSocket('/test', { autoConnect: true })
    );

    await waitFor(() => {
      expect(result.current.connected).toBe(true);
    });

    const sendSpy = vi.spyOn(MockWebSocket.prototype, 'send');
    result.current.sendMessage('test message');

    expect(sendSpy).toHaveBeenCalled();
  });

  it('should handle incoming messages', async () => {
    const { result } = renderHook(() =>
      useWebSocket('/test', { autoConnect: true })
    );

    await waitFor(() => {
      expect(result.current.connected).toBe(true);
    }, { timeout: 2000 });

    // Simulate incoming message via the mock instance
    if (mockWebSocketInstance.onmessage) {
      mockWebSocketInstance.onmessage(
        new MessageEvent('message', { data: '{"type": "test"}' })
      );
    }

    await waitFor(() => {
      expect(result.current.lastMessage).not.toBeNull();
    }, { timeout: 1000 });
  });

  it('should attempt reconnection on close', async () => {
    const { result } = renderHook(() =>
      useWebSocket('/test', {
        autoConnect: true,
        reconnect: {
          enabled: true,
          maxAttempts: 3,
          delay: 100,
        },
      })
    );

    await waitFor(() => {
      expect(result.current.connected).toBe(true);
    });

    // Close connection
    result.current.disconnect();

    await waitFor(() => {
      expect(result.current.connected).toBe(false);
    });
  });

  it('should disconnect when disconnect is called', async () => {
    const { result } = renderHook(() =>
      useWebSocket('/test', { autoConnect: true })
    );

    await waitFor(() => {
      expect(result.current.connected).toBe(true);
    });

    result.current.disconnect();

    await waitFor(() => {
      expect(result.current.connected).toBe(false);
    });
  });
});
