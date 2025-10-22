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
