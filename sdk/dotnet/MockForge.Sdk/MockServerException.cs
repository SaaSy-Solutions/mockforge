namespace MockForge.Sdk;

using System.Collections.Generic;

/// <summary>
/// Standardized exception class for MockServer operations
/// </summary>
public class MockServerException : Exception
{
    /// <summary>
    /// Error codes for MockServer operations
    /// </summary>
    public enum ErrorCode
    {
        CliNotFound,
        ServerStartFailed,
        PortDetectionFailed,
        AdminApiError,
        HealthCheckTimeout,
        InvalidConfig,
        StubNotFound,
        NetworkError,
        UnknownError
    }

    /// <summary>
    /// Error code for this exception
    /// </summary>
    public ErrorCode Code { get; }

    /// <summary>
    /// Additional error details
    /// </summary>
    public Dictionary<string, object> Details { get; }

    public MockServerException(ErrorCode code, string message) : base(message)
    {
        Code = code;
        Details = new Dictionary<string, object>();
    }

    public MockServerException(ErrorCode code, string message, Exception innerException)
        : base(message, innerException)
    {
        Code = code;
        Details = new Dictionary<string, object>();
    }

    /// <summary>
    /// Create an error for CLI not found
    /// </summary>
    public static MockServerException CliNotFound(Exception? cause = null)
    {
        var ex = new MockServerException(
            ErrorCode.CliNotFound,
            "MockForge CLI not found. Install with: cargo install mockforge-cli",
            cause
        );
        ex.Details["hint"] = "Ensure mockforge is in your PATH";
        return ex;
    }

    /// <summary>
    /// Create an error for server start failure
    /// </summary>
    public static MockServerException ServerStartFailed(string message, Exception? cause = null)
    {
        return new MockServerException(
            ErrorCode.ServerStartFailed,
            $"Failed to start MockForge server: {message}",
            cause
        );
    }

    /// <summary>
    /// Create an error for port detection failure
    /// </summary>
    public static MockServerException PortDetectionFailed(Exception? cause = null)
    {
        var ex = new MockServerException(
            ErrorCode.PortDetectionFailed,
            "Failed to detect server port from MockForge output. The server may have failed to start.",
            cause
        );
        ex.Details["hint"] = "Check that mockforge CLI is installed and the server started successfully";
        return ex;
    }

    /// <summary>
    /// Create an error for Admin API operations
    /// </summary>
    public static MockServerException AdminApiError(string operation, string message, Exception? cause = null)
    {
        var ex = new MockServerException(
            ErrorCode.AdminApiError,
            $"Admin API {operation} failed: {message}",
            cause
        );
        ex.Details["operation"] = operation;
        return ex;
    }

    /// <summary>
    /// Create an error for health check timeout
    /// </summary>
    public static MockServerException HealthCheckTimeout(int timeout, int port)
    {
        var ex = new MockServerException(
            ErrorCode.HealthCheckTimeout,
            $"Health check timed out after {timeout}ms. Could not connect to http://127.0.0.1:{port}/health"
        );
        ex.Details["timeout"] = timeout;
        ex.Details["port"] = port;
        ex.Details["hint"] = "Check that the server started successfully";
        return ex;
    }

    /// <summary>
    /// Create an error for invalid configuration
    /// </summary>
    public static MockServerException InvalidConfig(string message)
    {
        return new MockServerException(
            ErrorCode.InvalidConfig,
            $"Invalid configuration: {message}"
        );
    }

    /// <summary>
    /// Create an error for stub not found
    /// </summary>
    public static MockServerException StubNotFound(string method, string path)
    {
        var ex = new MockServerException(
            ErrorCode.StubNotFound,
            $"Stub not found: {method} {path}"
        );
        ex.Details["method"] = method;
        ex.Details["path"] = path;
        return ex;
    }

    /// <summary>
    /// Create an error for network operations
    /// </summary>
    public static MockServerException NetworkError(string message, Exception? cause = null)
    {
        return new MockServerException(
            ErrorCode.NetworkError,
            $"Network error: {message}",
            cause
        );
    }
}
