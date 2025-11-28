/**
 * Type definitions for MockForge SDK
 */

export interface MockServerConfig {
  /** Port to listen on (default: random available port) */
  port?: number;
  /** Host to bind to (default: 127.0.0.1) */
  host?: string;
  /** Path to MockForge configuration file */
  configFile?: string;
  /** Path to OpenAPI specification */
  openApiSpec?: string;
}

export interface ResponseStub {
  /** HTTP method (GET, POST, PUT, DELETE, etc.) */
  method: string;
  /** Path pattern (supports {{path_params}}) */
  path: string;
  /** HTTP status code (default: 200) */
  status?: number;
  /** Response headers */
  headers?: Record<string, string>;
  /** Response body */
  body: any;
  /** Latency in milliseconds */
  latencyMs?: number;
}

export interface StubOptions {
  /** HTTP status code (default: 200) */
  status?: number;
  /** Response headers */
  headers?: Record<string, string>;
  /** Latency in milliseconds */
  latencyMs?: number;
}

/**
 * Pattern for matching requests during verification
 */
export interface VerificationRequest {
  /** HTTP method to match (e.g., 'GET', 'POST'). Case-insensitive. If undefined, matches any method. */
  method?: string;
  /** URL path to match. Supports exact match, wildcards (*, **), and regex. If undefined, matches any path. */
  path?: string;
  /** Query parameters to match (all must be present and match). If empty, query parameters are not checked. */
  queryParams?: Record<string, string>;
  /** Headers to match (all must be present and match). Case-insensitive header names. If empty, headers are not checked. */
  headers?: Record<string, string>;
  /** Request body pattern to match. Supports exact match or regex. If undefined, body is not checked. */
  bodyPattern?: string;
}

/**
 * Count assertion for verification
 */
export type VerificationCount =
  | { type: 'exactly'; value: number }
  | { type: 'at_least'; value: number }
  | { type: 'at_most'; value: number }
  | { type: 'never' }
  | { type: 'at_least_once' };

/**
 * Result of a verification operation
 */
export interface VerificationResult {
  /** Whether the verification passed */
  matched: boolean;
  /** Actual count of matching requests */
  count: number;
  /** Expected count assertion */
  expected: VerificationCount;
  /** All matching request log entries (for inspection) */
  matches: any[];
  /** Error message if verification failed */
  errorMessage?: string;
}
