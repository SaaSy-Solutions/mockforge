package com.mockforge.sdk;

import java.util.HashMap;
import java.util.Map;

/**
 * Standardized exception class for MockServer operations
 */
public class MockServerException extends Exception {
    /**
     * Error codes for MockServer operations
     */
    public enum ErrorCode {
        CLI_NOT_FOUND,
        SERVER_START_FAILED,
        PORT_DETECTION_FAILED,
        ADMIN_API_ERROR,
        HEALTH_CHECK_TIMEOUT,
        INVALID_CONFIG,
        STUB_NOT_FOUND,
        NETWORK_ERROR,
        UNKNOWN_ERROR
    }

    private final ErrorCode code;
    private final Map<String, Object> details;

    public MockServerException(ErrorCode code, String message) {
        super(message);
        this.code = code;
        this.details = new HashMap<>();
    }

    public MockServerException(ErrorCode code, String message, Throwable cause) {
        super(message, cause);
        this.code = code;
        this.details = new HashMap<>();
    }

    public ErrorCode getCode() {
        return code;
    }

    public Map<String, Object> getDetails() {
        return details;
    }

    /**
     * Create an error for CLI not found
     */
    public static MockServerException cliNotFound(Throwable cause) {
        MockServerException e = new MockServerException(
            ErrorCode.CLI_NOT_FOUND,
            "MockForge CLI not found. Install with: cargo install mockforge-cli",
            cause
        );
        e.details.put("hint", "Ensure mockforge is in your PATH");
        return e;
    }

    /**
     * Create an error for server start failure
     */
    public static MockServerException serverStartFailed(String message, Throwable cause) {
        return new MockServerException(
            ErrorCode.SERVER_START_FAILED,
            "Failed to start MockForge server: " + message,
            cause
        );
    }

    /**
     * Create an error for port detection failure
     */
    public static MockServerException portDetectionFailed(Throwable cause) {
        MockServerException e = new MockServerException(
            ErrorCode.PORT_DETECTION_FAILED,
            "Failed to detect server port from MockForge output. The server may have failed to start.",
            cause
        );
        e.details.put("hint", "Check that mockforge CLI is installed and the server started successfully");
        return e;
    }

    /**
     * Create an error for Admin API operations
     */
    public static MockServerException adminApiError(String operation, String message, Throwable cause) {
        MockServerException e = new MockServerException(
            ErrorCode.ADMIN_API_ERROR,
            "Admin API " + operation + " failed: " + message,
            cause
        );
        e.details.put("operation", operation);
        return e;
    }

    /**
     * Create an error for health check timeout
     */
    public static MockServerException healthCheckTimeout(int timeout, int port) {
        MockServerException e = new MockServerException(
            ErrorCode.HEALTH_CHECK_TIMEOUT,
            String.format("Health check timed out after %dms. Could not connect to http://127.0.0.1:%d/health", timeout, port)
        );
        e.details.put("timeout", timeout);
        e.details.put("port", port);
        e.details.put("hint", "Check that the server started successfully");
        return e;
    }

    /**
     * Create an error for invalid configuration
     */
    public static MockServerException invalidConfig(String message) {
        return new MockServerException(
            ErrorCode.INVALID_CONFIG,
            "Invalid configuration: " + message
        );
    }

    /**
     * Create an error for stub not found
     */
    public static MockServerException stubNotFound(String method, String path) {
        MockServerException e = new MockServerException(
            ErrorCode.STUB_NOT_FOUND,
            "Stub not found: " + method + " " + path
        );
        e.details.put("method", method);
        e.details.put("path", path);
        return e;
    }

    /**
     * Create an error for network operations
     */
    public static MockServerException networkError(String message, Throwable cause) {
        return new MockServerException(
            ErrorCode.NETWORK_ERROR,
            "Network error: " + message,
            cause
        );
    }
}
