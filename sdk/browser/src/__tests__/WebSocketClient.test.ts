import { describe, it, beforeEach, afterEach, expect } from 'vitest';
/**
 * Unit tests for WebSocketClient management-token subprotocols
 */

import { WebSocketClient, managementSubprotocols } from '../core/WebSocketClient';

describe('WebSocketClient management token', () => {
    const OriginalWebSocket = (global as any).WebSocket;
    let constructed: Array<{ url: string; protocols?: string | string[] }>;

    beforeEach(() => {
        constructed = [];
        class FakeWebSocket {
            static OPEN = 1;
            readyState = 0;
            onopen: (() => void) | null = null;
            onmessage: unknown = null;
            onerror: unknown = null;
            onclose: unknown = null;
            constructor(url: string, protocols?: string | string[]) {
                constructed.push({ url, protocols });
                setTimeout(() => this.onopen && this.onopen(), 0);
            }
            close() {}
            send() {}
        }
        (global as any).WebSocket = FakeWebSocket;
    });

    afterEach(() => {
        (global as any).WebSocket = OriginalWebSocket;
    });

    it('builds the subprotocol pair', () => {
        expect(managementSubprotocols('abc')).toEqual([
            'mockforge-management',
            'mockforge-token.abc',
        ]);
    });

    it('passes the token as subprotocols when set', async () => {
        const client = new WebSocketClient('http://localhost:3000', 't'.repeat(40));
        await client.connect();
        expect(constructed[0].url).toBe('ws://localhost:3000/__mockforge/ws');
        expect(constructed[0].protocols).toEqual([
            'mockforge-management',
            `mockforge-token.${'t'.repeat(40)}`,
        ]);
    });

    it('opens without subprotocols when no token is set', async () => {
        const client = new WebSocketClient('http://localhost:3000');
        await client.connect();
        expect(constructed[0].protocols).toBeUndefined();
    });
});
