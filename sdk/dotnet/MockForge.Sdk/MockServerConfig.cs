namespace MockForge.Sdk;

/// <summary>
/// Configuration for MockServer
/// </summary>
public class MockServerConfig
{
    /// <summary>
    /// Port to listen on (default: 0 = random available port)
    /// </summary>
    public int Port { get; set; } = 0;

    /// <summary>
    /// Host to bind to (default: 127.0.0.1)
    /// </summary>
    public string Host { get; set; } = "127.0.0.1";

    /// <summary>
    /// Path to MockForge configuration file
    /// </summary>
    public string? ConfigFile { get; set; }

    /// <summary>
    /// Path to OpenAPI specification
    /// </summary>
    public string? OpenApiSpec { get; set; }
}
