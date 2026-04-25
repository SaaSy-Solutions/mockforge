/**
 * Type definitions for MockForge SDK
 */

export interface MockServerConfig {
  /** HTTP port to listen on. Default: `0` (random available port). */
  port?: number;
  /** Host to bind to. Default: `127.0.0.1`. */
  host?: string;
  /**
   * Admin UI port. Default: `0` (random available port). The admin API is
   * required for dynamic stubbing — `stubResponse`, `clearStubs`, etc. all
   * go through it.
   */
  adminPort?: number;
  /**
   * WebSocket server port. Default: `0` (random). Set to a real port only if
   * you intend to use the WebSocket mock from your tests; otherwise leave as
   * `0` so it doesn't collide with any existing MockForge instance.
   */
  wsPort?: number;
  /**
   * gRPC server port. Default: `0`, which disables the gRPC server entirely
   * (the CLI skips starting gRPC when port == 0 and gRPC is enabled).
   */
  grpcPort?: number;
  /**
   * Prometheus metrics port. Default: `0`. Metrics are only started if the
   * CLI receives `--metrics`, which the SDK does not pass, so this is almost
   * always ignored — but forcing it to `0` keeps things safe if the user's
   * `mockforge.yaml` enables metrics.
   */
  metricsPort?: number;
  /** Path to MockForge configuration file. */
  configFile?: string;
  /**
   * Skip auto-discovery of `mockforge.yaml` / `mockforge.config.{ts,js}` from
   * the current working directory and its ancestors. Default: `true`. Set to
   * `false` to restore legacy behavior where a mockforge config in the host
   * project would be picked up implicitly.
   */
  noConfig?: boolean;
  /** Path to OpenAPI specification. */
  openApiSpec?: string;
  /**
   * Maximum time in milliseconds to wait for the server to start. Default:
   * `12_000`. Useful to tune on slow CI runners.
   */
  startupTimeoutMs?: number;
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
