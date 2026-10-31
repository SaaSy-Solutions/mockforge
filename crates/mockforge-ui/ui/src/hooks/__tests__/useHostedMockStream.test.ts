import { describe, expect, it, beforeEach, afterEach } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useHostedMockStream } from '../useHostedMockStream';

class RecordingWebSocket {
  static OPEN = 1;
  static instances: RecordingWebSocket[] = [];
  readyState = 0;
  onopen: ((event: Event) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;

  url: string;
  protocols?: string | string[];

  constructor(url: string, protocols?: string | string[]) {
    this.url = url;
    this.protocols = protocols;
    RecordingWebSocket.instances.push(this);
  }

  send(): void {}

  close(): void {
    this.readyState = 3;
  }
}

describe('useHostedMockStream', () => {
  const original = globalThis.WebSocket;

  beforeEach(() => {
    RecordingWebSocket.instances = [];
    globalThis.WebSocket = RecordingWebSocket as unknown as typeof WebSocket;
  });

  afterEach(() => {
    globalThis.WebSocket = original;
  });

  it('authenticates with subprotocols and keeps the token out of the URL', () => {
    renderHook(() =>
      useHostedMockStream('https://my-mock.example.dev', { managementToken: 'mfm_secret' }),
    );
    expect(RecordingWebSocket.instances.length).toBeGreaterThanOrEqual(1);
    const ws = RecordingWebSocket.instances[RecordingWebSocket.instances.length - 1];
    expect(ws.url).toBe('wss://my-mock.example.dev/__mockforge/ws');
    expect(ws.url).not.toContain('mfm_secret');
    expect(ws.protocols).toEqual(['mockforge-management', 'mockforge-token.mfm_secret']);
  });

  it('does not connect without a management token', () => {
    renderHook(() => useHostedMockStream('https://my-mock.example.dev', { managementToken: null }));
    expect(RecordingWebSocket.instances).toHaveLength(0);
  });
});
