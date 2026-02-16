/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, waitFor, act } from '@testing-library/react';
import { useWebSocket } from '../useWebSocket';

// Mock WebSocket
class MockWebSocket {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;

  static instances: MockWebSocket[] = [];
  readyState = MockWebSocket.CONNECTING;
  url: string;
  onopen: ((event: Event) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
    // Simulate connection after a short delay
    Promise.resolve().then(() => {
      this.readyState = MockWebSocket.OPEN;
      if (this.onopen) {
        this.onopen(new Event('open'));
      }
    });
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
  beforeEach(() => {
    MockWebSocket.instances = [];
    global.WebSocket = MockWebSocket as any;
    window.WebSocket = MockWebSocket as any;
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('should create WebSocket connection', () => {
    const options = { autoConnect: true };
    const { result } = renderHook(() =>
      useWebSocket('/test', options)
    );

    // Initially not connected (connection happens asynchronously)
    expect(result.current.connected).toBe(false);
  });

  it('should connect when autoConnect is true', async () => {
    const options = { autoConnect: true };
    const { result } = renderHook(() =>
      useWebSocket('/test', options)
    );

    await waitFor(() => {
      expect(result.current.connected).toBe(true);
    });
  });

  it('should not connect when autoConnect is false', () => {
    const options = { autoConnect: false };
    const { result } = renderHook(() =>
      useWebSocket('/test', options)
    );

    expect(result.current.connected).toBe(false);
  });

  it('should send message when connected', async () => {
    const options = { autoConnect: true };
    const { result } = renderHook(() =>
      useWebSocket('/test', options)
    );

    await waitFor(() => {
      expect(result.current.connected).toBe(true);
    });

    const sendSpy = vi.spyOn(MockWebSocket.prototype, 'send');
    result.current.sendMessage('test message');

    expect(sendSpy).toHaveBeenCalled();
  });

  it('should register incoming message handler', async () => {
    const options = { autoConnect: true };
    const { result } = renderHook(() =>
      useWebSocket('/test', options)
    );

    await waitFor(() => {
      expect(result.current.connected).toBe(true);
    }, { timeout: 2000 });

    const ws = MockWebSocket.instances.at(-1);
    expect(typeof ws?.onmessage).toBe('function');
  });

  it('should attempt reconnection on close', async () => {
    const options = {
      autoConnect: true,
      reconnect: {
        enabled: true,
        maxAttempts: 3,
        delay: 100,
      },
    };
    const { result } = renderHook(() =>
      useWebSocket('/test', options)
    );

    await waitFor(() => {
      expect(result.current.connected).toBe(true);
    });

    const initialInstances = MockWebSocket.instances.length;
    const ws = MockWebSocket.instances.at(-1);
    act(() => {
      ws?.close();
    });

    await waitFor(() => {
      expect(MockWebSocket.instances.length).toBeGreaterThan(initialInstances);
    });
  });

  it('should disconnect when disconnect is called', async () => {
    const options = {
      autoConnect: true,
      reconnect: { enabled: false },
    };
    const { result } = renderHook(() =>
      useWebSocket('/test', options)
    );

    await waitFor(() => {
      expect(result.current.connected).toBe(true);
    });

    act(() => {
      result.current.disconnect();
    });

    await waitFor(() => {
      expect(result.current.connected).toBe(false);
    });
  });
});
