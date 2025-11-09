/**
 * Type definitions for ForgeConnect SDK
 */

/**
 * Configuration options for ForgeConnect
 */
export interface ForgeConnectConfig {
    /**
     * MockForge server URL (auto-discovered if not provided)
     */
    serverUrl?: string;

    /**
     * Auto-discovery ports to try (default: [3000, 3001, 8080, 9080])
     */
    discoveryPorts?: number[];

    /**
     * Mock creation behavior mode
     */
    mockMode?: 'auto' | 'prompt' | 'hybrid';

    /**
     * Auto-create mocks for these HTTP status codes (default: [404, 500, 502, 503, 504])
     */
    autoMockStatusCodes?: number[];

    /**
     * Auto-create mocks for network errors (default: true)
     */
    autoMockNetworkErrors?: boolean;

    /**
     * Enable request/response logging (default: true)
     */
    enableLogging?: boolean;

    /**
     * Callback when a mock is created
     */
    onMockCreated?: (mock: MockConfig) => void;

    /**
     * Callback when connection status changes
     */
    onConnectionChange?: (connected: boolean, url?: string) => void;

    /**
     * Callback when a mock is updated (for live reload)
     */
    onMockUpdated?: (mock: MockConfig) => void;

    /**
     * Callback when a mock is deleted (for live reload)
     */
    onMockDeleted?: (mockId: string) => void;

    /**
     * Custom prompt function for mock creation (used in 'prompt' or 'hybrid' mode)
     */
    promptMockCreation?: (request: CapturedRequest) => Promise<boolean>;

    /**
     * Enable Service Worker for comprehensive request capture (default: true if supported)
     */
    enableServiceWorker?: boolean;

    /**
     * Enable WebSocket for real-time updates (default: false)
     */
    enableWebSocket?: boolean;

    /**
     * Workspace ID for environment management (optional)
     */
    workspaceId?: string;

    /**
     * OAuth passthrough configuration
     */
    oauthPassthrough?: {
        /**
         * URLs or patterns to bypass mocking (e.g., OAuth endpoints)
         */
        bypassUrls?: string[];

        /**
         * Regex patterns for URLs to bypass
         */
        bypassPatterns?: RegExp[];

        /**
         * Function to determine if a request should bypass mocking
         */
        shouldBypass?: (url: string, method: string) => boolean;

        /**
         * Token storage key (default: 'forgeconnect_oauth_token')
         */
        tokenStorageKey?: string;

        /**
         * Auto-inject tokens into requests
         */
        autoInjectTokens?: boolean;

        /**
         * Token injection header name (default: 'Authorization')
         */
        tokenHeaderName?: string;
    };
}

/**
 * Mock configuration matching MockForge API structure
 */
export interface MockConfig {
    /**
     * Unique identifier for the mock
     */
    id?: string;

    /**
     * Human-readable name for the mock
     */
    name: string;

    /**
     * HTTP method (GET, POST, etc.)
     */
    method: string;

    /**
     * API path pattern to match
     */
    path: string;

    /**
     * Response configuration
     */
    response: MockResponse;

    /**
     * Whether this mock is currently enabled
     */
    enabled?: boolean;

    /**
     * Optional latency to inject in milliseconds
     */
    latency_ms?: number;

    /**
     * Optional HTTP status code override
     */
    status_code?: number;
}

/**
 * Mock response configuration
 */
export interface MockResponse {
    /**
     * Response body as JSON
     */
    body: any;

    /**
     * Optional custom response headers
     */
    headers?: Record<string, string>;
}

/**
 * Captured request information
 */
export interface CapturedRequest {
    /**
     * HTTP method
     */
    method: string;

    /**
     * Request URL
     */
    url: string;

    /**
     * Request path (without query string)
     */
    path: string;

    /**
     * Query parameters
     */
    queryParams?: Record<string, string>;

    /**
     * Request headers
     */
    headers?: Record<string, string>;

    /**
     * Request body
     */
    body?: any;

    /**
     * Response status code (if available)
     */
    statusCode?: number;

    /**
     * Response body (if available)
     */
    responseBody?: any;

    /**
     * Response headers (if available)
     */
    responseHeaders?: Record<string, string>;

    /**
     * Error information (if request failed)
     */
    error?: {
        type: 'network' | 'timeout' | 'cors' | 'http';
        message: string;
    };

    /**
     * Timestamp of the request
     */
    timestamp: number;
}

/**
 * Connection status
 */
export interface ConnectionStatus {
    /**
     * Whether connected to MockForge
     */
    connected: boolean;

    /**
     * MockForge server URL
     */
    url?: string;

    /**
     * Last error message (if disconnected)
     */
    error?: string;

    /**
     * Last successful connection time
     */
    lastConnected?: number;
}

/**
 * Environment configuration
 */
export interface Environment {
    /**
     * Unique identifier for the environment
     */
    id: string;

    /**
     * Human-readable name
     */
    name: string;

    /**
     * Optional description
     */
    description?: string;

    /**
     * Whether this is the global environment
     */
    is_global?: boolean;

    /**
     * Whether this environment is currently active
     */
    active?: boolean;

    /**
     * Display order
     */
    order?: number;

    /**
     * Number of variables in this environment
     */
    variable_count?: number;
}

/**
 * Environment variable
 */
export interface EnvironmentVariable {
    /**
     * Variable name
     */
    key: string;

    /**
     * Variable value
     */
    value: string;
}
