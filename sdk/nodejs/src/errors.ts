/**
 * Standardized error types for MockForge SDK
 */

/**
 * Error codes for MockServer operations
 */
export enum MockServerErrorCode {
  CLI_NOT_FOUND = 'CLI_NOT_FOUND',
  SERVER_START_FAILED = 'SERVER_START_FAILED',
  PORT_DETECTION_FAILED = 'PORT_DETECTION_FAILED',
  ADMIN_API_ERROR = 'ADMIN_API_ERROR',
  HEALTH_CHECK_TIMEOUT = 'HEALTH_CHECK_TIMEOUT',
  INVALID_CONFIG = 'INVALID_CONFIG',
  STUB_NOT_FOUND = 'STUB_NOT_FOUND',
  NETWORK_ERROR = 'NETWORK_ERROR',
  UNKNOWN_ERROR = 'UNKNOWN_ERROR',
}

/**
 * Standardized error class for MockServer operations
 */
export class MockServerError extends Error {
  public readonly code: MockServerErrorCode;
  public readonly cause?: Error;
  public readonly details?: Record<string, any>;

  constructor(
    code: MockServerErrorCode,
    message: string,
    cause?: Error,
    details?: Record<string, any>
  ) {
    super(message);
    this.name = 'MockServerError';
    this.code = code;
    this.cause = cause;
    this.details = details;

    // Maintains proper stack trace for where our error was thrown (only available on V8)
    if (typeof Error.captureStackTrace === 'function') {
      Error.captureStackTrace(this, MockServerError);
    }
  }

  /**
   * Create an error for CLI not found
   */
  static cliNotFound(cause?: Error): MockServerError {
    return new MockServerError(
      MockServerErrorCode.CLI_NOT_FOUND,
      'MockForge CLI not found. Install with: cargo install mockforge-cli',
      cause,
      { hint: 'Ensure mockforge is in your PATH' }
    );
  }

  /**
   * Create an error for server start failure
   */
  static serverStartFailed(message: string, cause?: Error): MockServerError {
    return new MockServerError(
      MockServerErrorCode.SERVER_START_FAILED,
      `Failed to start MockForge server: ${message}`,
      cause
    );
  }

  /**
   * Create an error for port detection failure
   */
  static portDetectionFailed(cause?: Error): MockServerError {
    return new MockServerError(
      MockServerErrorCode.PORT_DETECTION_FAILED,
      'Failed to detect server port from MockForge output. The server may have failed to start.',
      cause,
      { hint: 'Check that mockforge CLI is installed and the server started successfully' }
    );
  }

  /**
   * Create an error for Admin API operations
   */
  static adminApiError(operation: string, message: string, cause?: Error): MockServerError {
    return new MockServerError(
      MockServerErrorCode.ADMIN_API_ERROR,
      `Admin API ${operation} failed: ${message}`,
      cause,
      { operation }
    );
  }

  /**
   * Create an error for health check timeout
   */
  static healthCheckTimeout(timeout: number, port: number): MockServerError {
    return new MockServerError(
      MockServerErrorCode.HEALTH_CHECK_TIMEOUT,
      `Health check timed out after ${timeout}ms. Could not connect to http://127.0.0.1:${port}/health`,
      undefined,
      { timeout, port, hint: 'Check that the server started successfully' }
    );
  }

  /**
   * Create an error for invalid configuration
   */
  static invalidConfig(message: string, details?: Record<string, any>): MockServerError {
    return new MockServerError(
      MockServerErrorCode.INVALID_CONFIG,
      `Invalid configuration: ${message}`,
      undefined,
      details
    );
  }

  /**
   * Create an error for stub not found
   */
  static stubNotFound(method: string, path: string): MockServerError {
    return new MockServerError(
      MockServerErrorCode.STUB_NOT_FOUND,
      `Stub not found: ${method} ${path}`,
      undefined,
      { method, path }
    );
  }

  /**
   * Create an error for network operations
   */
  static networkError(message: string, cause?: Error): MockServerError {
    return new MockServerError(
      MockServerErrorCode.NETWORK_ERROR,
      `Network error: ${message}`,
      cause
    );
  }
}
