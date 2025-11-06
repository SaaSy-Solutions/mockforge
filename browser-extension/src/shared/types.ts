/**
 * Shared types for browser extension
 */

export interface ExtensionMessage {
    type: 'REQUEST_CAPTURED' | 'MOCK_CREATED' | 'CONNECTION_CHANGE' | 'GET_MOCKS' | 'CREATE_MOCK' | 'DELETE_MOCK';
    payload?: any;
}

export interface CapturedRequest {
    method: string;
    url: string;
    path: string;
    queryParams?: Record<string, string>;
    headers?: Record<string, string>;
    body?: any;
    statusCode?: number;
    responseBody?: any;
    responseHeaders?: Record<string, string>;
    error?: {
        type: 'network' | 'timeout' | 'cors' | 'http';
        message: string;
    };
    timestamp: number;
}

export interface MockConfig {
    id?: string;
    name: string;
    method: string;
    path: string;
    response: {
        body: any;
        headers?: Record<string, string>;
    };
    enabled?: boolean;
    latency_ms?: number;
    status_code?: number;
}

export interface ConnectionStatus {
    connected: boolean;
    url?: string;
    error?: string;
    lastConnected?: number;
}

