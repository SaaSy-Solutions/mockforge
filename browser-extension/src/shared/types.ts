/**
 * Shared types for browser extension
 */

export interface ExtensionMessage {
    type: 'REQUEST_CAPTURED' | 'MOCK_CREATED' | 'MOCK_UPDATED' | 'MOCK_DELETED' | 'CONNECTION_CHANGE' | 'GET_MOCKS' | 'CREATE_MOCK' | 'DELETE_MOCK' | 'GET_CAPTURED_REQUESTS' | 'GET_ENVIRONMENTS' | 'SET_ACTIVE_ENVIRONMENT' | 'GET_ENVIRONMENT_VARIABLES' | 'GET_PERSONAS' | 'SET_ACTIVE_PERSONA' | 'GET_SCENARIOS' | 'SET_ACTIVE_SCENARIO' | 'GET_WORKSPACE_STATE' | 'COMPARE_SNAPSHOTS';
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
    /**
     * Request timing information (if available)
     */
    timing?: {
        startTime: number;
        endTime?: number;
        duration?: number;
    };
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

export interface Environment {
    id: string;
    name: string;
    description?: string;
    is_global?: boolean;
    active?: boolean;
    order?: number;
    variable_count?: number;
}

export interface Persona {
    id: string;
    name?: string;
    domain?: string;
    traits?: Record<string, string>;
    backstory?: string;
    relationships?: Record<string, string[]>;
    lifecycle?: {
        state: string;
        transitions?: string[];
    };
}

export interface Scenario {
    id: string;
    name: string;
    description?: string;
    version?: string;
    category?: string;
    tags?: string[];
}
