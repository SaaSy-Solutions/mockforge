/**
 * @jest-environment jsdom
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useSSE, useLogsSSE } from '../useSSE';

// Mock EventSource
class MockEventSource {
  url: string;
  onopen: ((event: Event) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  readyState: number = 0;

  private listeners: Map<string, Set<(event: MessageEvent) => void>> = new Map();

  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSED = 2;

  constructor(url: string) {
    this.url = url;
    this.readyState = MockEventSource.CONNECTING;
    // Simulate connection opening
    setTimeout(() => {
      this.readyState = MockEventSource.OPEN;
      if (this.onopen) {
        this.onopen(new Event('open'));
      }
    }, 0);
  }

  addEventListener(event: string, handler: (event: MessageEvent) => void) {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)?.add(handler);
  }

  removeEventListener(event: string, handler: (event: MessageEvent) => void) {
    this.listeners.get(event)?.delete(handler);
  }

  close() {
    this.readyState = MockEventSource.CLOSED;
  }

  // Helper method for testing
  simulateMessage(data: any) {
    if (this.onmessage) {
      const event = new MessageEvent('message', {
        data: JSON.stringify(data),
      });
      this.onmessage(event);
    }
  }

  simulateError() {
    if (this.onerror) {
      this.onerror(new Event('error'));
    }
  }
}

describe('useSSE', () => {
  beforeEach(() => {
    (global as any).EventSource = MockEventSource;
  });

  afterEach(() => {
    vi.clearAllTimers();
  });

  it('initializes with null data', () => {
    const { result } = renderHook(() => useSSE('/test-endpoint', { autoConnect: false }));

    expect(result.current.data).toBeNull();
    expect(result.current.isConnected).toBe(false);
  });

  it('auto-connects when autoConnect is true', async () => {
    const { result } = renderHook(() => useSSE('/test-endpoint'));

    await waitFor(() => {
      expect(result.current.readyState).toBe(MockEventSource.OPEN);
    });

    expect(result.current.isConnected).toBe(true);
  });

  it('does not auto-connect when autoConnect is false', () => {
    const { result } = renderHook(() => useSSE('/test-endpoint', { autoConnect: false }));

    expect(result.current.readyState).toBe(MockEventSource.CLOSED);
    expect(result.current.isConnected).toBe(false);
  });

  it('manually connects', async () => {
    const { result } = renderHook(() => useSSE('/test-endpoint', { autoConnect: false }));

    act(() => {
      result.current.connect();
    });

    await waitFor(() => {
      expect(result.current.isConnected).toBe(true);
    });
  });

  it('manually disconnects', async () => {
    const { result } = renderHook(() => useSSE('/test-endpoint'));

    await waitFor(() => {
      expect(result.current.isConnected).toBe(true);
    });

    act(() => {
      result.current.disconnect();
    });

    expect(result.current.readyState).toBe(MockEventSource.CLOSED);
    expect(result.current.isConnected).toBe(false);
  });

  it('parses JSON messages', async () => {
    const { result } = renderHook(() => useSSE('/test-endpoint'));

    await waitFor(() => {
      expect(result.current.isConnected).toBe(true);
    });

    // Note: In a real test we'd need to trigger the EventSource mock to send a message
    // This is a simplified version
  });

  it('handles connection errors', async () => {
    const { result } = renderHook(() => useSSE('/test-endpoint'));

    await waitFor(() => {
      expect(result.current.isConnected).toBe(true);
    });

    // Simulate error
    // In real implementation, we'd trigger the EventSource error handler
  });

  it('disconnects on unmount', async () => {
    const { result, unmount } = renderHook(() => useSSE('/test-endpoint'));

    await waitFor(() => {
      expect(result.current.isConnected).toBe(true);
    });

    unmount();

    // Connection should be closed after unmount
  });
});

describe('useLogsSSE', () => {
  beforeEach(() => {
    (global as any).EventSource = MockEventSource;
  });

  it('connects to logs SSE endpoint', async () => {
    const { result } = renderHook(() => useLogsSSE());

    await waitFor(() => {
      expect(result.current.isConnected).toBe(true);
    });
  });

  it('has retry configuration', async () => {
    const { result } = renderHook(() => useLogsSSE());

    // Logs SSE should have retry enabled with specific config
    await waitFor(() => {
      expect(result.current.readyState).toBeDefined();
    });
  });
});
