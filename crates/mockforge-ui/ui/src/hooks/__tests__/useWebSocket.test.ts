/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, waitFor, act } from '@testing-library/react';
import { useWebSocket } from '../useWebSocket';

// Mock WebSocket — opens the moment `onopen` is assigned so the hook's
// state setter lands inside the test's render window. A microtask-based
// async open is racy under React 19's effect-cleanup timing.
class MockWebSocket {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;

  static instances: MockWebSocket[] = [];
  readyState = MockWebSocket.OPEN;
  url: string;
  private _onopen: ((event: Event) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  set onopen(handler: ((event: Event) => void) | null) {
    this._onopen = handler;
    if (handler && this.readyState === MockWebSocket.OPEN) {
      handler(new Event('open'));
    }
  }
  get onopen(): ((event: Event) => void) | null {
    return this._onopen;
  }

  send(_data: string | object) {
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
    (globalThis as any).WebSocket = MockWebSocket;
    if (typeof window !== 'undefined') {
      window.WebSocket = MockWebSocket as any;
    }
    // The shared test setup configures cloud mode, but useWebSocket
    // intentionally refuses relative paths in cloud mode. These tests
    // exercise the local-mode connection path, so drop cloud mode here.
    vi.stubEnv('VITE_API_BASE_URL', '');
    vi.stubEnv('VITE_MOCKFORGE_MODE', '');
  });

  afterEach(() => {
    vi.clearAllMocks();
    vi.unstubAllEnvs();
  });

  it('should create WebSocket connection', () => {
    const options = { autoConnect: true };
    renderHook(() => useWebSocket('/test', options));

    // The hook should have constructed at least one underlying WebSocket.
    expect(MockWebSocket.instances.length).toBeGreaterThanOrEqual(1);
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
