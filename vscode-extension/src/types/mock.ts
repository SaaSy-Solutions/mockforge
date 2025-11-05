/**
 * Type definitions for MockForge mocks and server configuration
 */

/**
 * Mock response configuration
 */
export interface MockResponse {
    /** Response body as JSON value */
    body: unknown;
    /** Optional custom response headers */
    headers?: Record<string, string>;
}

/**
 * Mock configuration matching the server's MockConfig struct
 */
export interface MockConfig {
    /** Unique identifier for the mock */
    id: string;
    /** Human-readable name for the mock */
    name: string;
    /** HTTP method (GET, POST, etc.) */
    method: string;
    /** API path pattern to match */
    path: string;
    /** Response configuration */
    response: MockResponse;
    /** Whether this mock is currently enabled */
    enabled: boolean;
    /** Optional latency to inject in milliseconds */
    latency_ms?: number;
    /** Optional HTTP status code override */
    status_code?: number;
}

/**
 * Server statistics
 */
export interface ServerStats {
    /** Server uptime in seconds */
    uptime_seconds: number;
    /** Total number of requests processed */
    total_requests: number;
    /** Number of active mock configurations */
    active_mocks: number;
    /** Number of currently enabled mocks */
    enabled_mocks: number;
    /** Number of registered API routes */
    registered_routes: number;
}

/**
 * Server configuration info
 */
export interface ServerConfig {
    /** MockForge version string */
    version: string;
    /** Server port number */
    port: number;
    /** Whether an OpenAPI spec is loaded */
    has_openapi_spec: boolean;
    /** Optional path to the OpenAPI spec file */
    spec_path?: string;
}

/**
 * Connection state for WebSocket
 */
export type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'reconnecting';

