/**
 * WebSocket Client for Real-Time Updates
 * 
 * Provides WebSocket connection to MockForge for real-time
 * mock updates, connection status, and live request monitoring
 */

import { MockConfig, ConnectionStatus } from '../types';

/**
 * WebSocket event types
 */
export type WebSocketEventType =
    | 'mock_created'
    | 'mock_updated'
    | 'mock_deleted'
    | 'request_captured'
    | 'connection_status'
    | 'stats_updated'
    | 'error';

/**
 * WebSocket event
 */
export interface WebSocketEvent {
    type: WebSocketEventType;
    payload: any;
    timestamp: number;
}

/**
 * WebSocket client for MockForge real-time updates
 */
export class WebSocketClient {
    private ws: WebSocket | null = null;
    private baseUrl: string;
    private reconnectAttempts: number = 0;
    private maxReconnectAttempts: number = 5;
    private reconnectDelay: number = 1000; // Start with 1 second
    private reconnectTimer: NodeJS.Timeout | null = null;
    private eventListeners: Map<WebSocketEventType, Set<(event: WebSocketEvent) => void>> = new Map();
    private connected: boolean = false;

    constructor(baseUrl: string) {
        // Convert HTTP URL to WebSocket URL
        this.baseUrl = baseUrl.replace(/\/$/, '').replace(/^http/, 'ws');
    }

    /**
     * Connect to WebSocket server
     */
    async connect(): Promise<boolean> {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            return true;
        }

        return new Promise((resolve) => {
            try {
                // MockForge WebSocket endpoint
                const wsUrl = `${this.baseUrl}/__mockforge/ws`;
                this.ws = new WebSocket(wsUrl);

                this.ws.onopen = () => {
                    this.connected = true;
                    this.reconnectAttempts = 0;
                    this.reconnectDelay = 1000;
                    console.log('[ForgeConnect] WebSocket connected');
                    this.emit('connection_status', { connected: true });
                    resolve(true);
                };

                this.ws.onmessage = (event) => {
                    try {
                        const data = JSON.parse(event.data);
                        this.handleMessage(data);
                    } catch (error) {
                        console.warn('[ForgeConnect] Failed to parse WebSocket message:', error);
                    }
                };

                this.ws.onerror = (error) => {
                    console.error('[ForgeConnect] WebSocket error:', error);
                    this.emit('error', { error: 'WebSocket connection error' });
                    resolve(false);
                };

                this.ws.onclose = () => {
                    this.connected = false;
                    this.emit('connection_status', { connected: false });
                    console.log('[ForgeConnect] WebSocket disconnected');
                    this.attemptReconnect();
                    resolve(false);
                };
            } catch (error) {
                console.error('[ForgeConnect] Failed to create WebSocket:', error);
                resolve(false);
            }
        });
    }

    /**
     * Disconnect from WebSocket server
     */
    disconnect(): void {
        if (this.reconnectTimer) {
            clearTimeout(this.reconnectTimer);
            this.reconnectTimer = null;
        }

        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }

        this.connected = false;
    }

    /**
     * Subscribe to WebSocket events
     */
    on(eventType: WebSocketEventType, callback: (event: WebSocketEvent) => void): void {
        if (!this.eventListeners.has(eventType)) {
            this.eventListeners.set(eventType, new Set());
        }
        this.eventListeners.get(eventType)!.add(callback);
    }

    /**
     * Unsubscribe from WebSocket events
     */
    off(eventType: WebSocketEventType, callback: (event: WebSocketEvent) => void): void {
        const listeners = this.eventListeners.get(eventType);
        if (listeners) {
            listeners.delete(callback);
        }
    }

    /**
     * Send message to WebSocket server
     */
    send(type: string, payload: any): void {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify({ type, payload }));
        } else {
            console.warn('[ForgeConnect] WebSocket not connected, cannot send message');
        }
    }

    /**
     * Check if WebSocket is connected
     */
    isConnected(): boolean {
        return this.connected && this.ws !== null && this.ws.readyState === WebSocket.OPEN;
    }

    /**
     * Get connection status
     */
    getConnectionStatus(): ConnectionStatus {
        return {
            connected: this.isConnected(),
            url: this.baseUrl.replace(/^ws/, 'http'),
        };
    }

    /**
     * Handle incoming WebSocket message
     */
    private handleMessage(data: any): void {
        // MockForge sends events in format: { type: "mock_created", mock: {...}, timestamp: "..." }
        const eventType = this.mapMockForgeEventType(data.type);
        const payload = data;

        this.emit(eventType, payload);
    }

    /**
     * Map MockForge event types to our event types
     */
    private mapMockForgeEventType(mockForgeType: string): WebSocketEventType {
        // MockForge uses snake_case event types
        switch (mockForgeType) {
            case 'mock_created':
                return 'mock_created';
            case 'mock_updated':
                return 'mock_updated';
            case 'mock_deleted':
                return 'mock_deleted';
            case 'stats_updated':
                return 'stats_updated';
            case 'connected':
                return 'connection_status';
            default:
                // Try to map unknown types
                if (mockForgeType.includes('created') || mockForgeType.includes('create')) {
                    return 'mock_created';
                }
                if (mockForgeType.includes('updated') || mockForgeType.includes('update')) {
                    return 'mock_updated';
                }
                if (mockForgeType.includes('deleted') || mockForgeType.includes('delete')) {
                    return 'mock_deleted';
                }
                return 'error';
        }
    }

    /**
     * Emit event to listeners
     */
    private emit(eventType: WebSocketEventType, payload: any): void {
        const listeners = this.eventListeners.get(eventType);
        if (listeners) {
            const event: WebSocketEvent = {
                type: eventType,
                payload,
                timestamp: Date.now(),
            };
            listeners.forEach((callback) => {
                try {
                    callback(event);
                } catch (error) {
                    console.error('[ForgeConnect] Error in WebSocket event listener:', error);
                }
            });
        }
    }

    /**
     * Attempt to reconnect with exponential backoff
     */
    private attemptReconnect(): void {
        if (this.reconnectAttempts >= this.maxReconnectAttempts) {
            console.warn('[ForgeConnect] Max reconnection attempts reached');
            return;
        }

        this.reconnectAttempts++;
        this.reconnectDelay = Math.min(this.reconnectDelay * 2, 30000); // Max 30 seconds

        console.log(
            `[ForgeConnect] Attempting to reconnect (${this.reconnectAttempts}/${this.maxReconnectAttempts}) in ${this.reconnectDelay}ms`
        );

        this.reconnectTimer = setTimeout(() => {
            this.connect().catch(() => {
                // Reconnection will be attempted again
            });
        }, this.reconnectDelay);
    }
}

