namespace MockForge.Sdk;

/// <summary>
/// Response stub configuration
/// </summary>
public class ResponseStub
{
    /// <summary>
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    /// </summary>
    public string Method { get; set; } = string.Empty;

    /// <summary>
    /// Path pattern (supports path parameters)
    /// </summary>
    public string Path { get; set; } = string.Empty;

    /// <summary>
    /// HTTP status code (default: 200)
    /// </summary>
    public int Status { get; set; } = 200;

    /// <summary>
    /// Response headers
    /// </summary>
    public Dictionary<string, string> Headers { get; set; } = new();

    /// <summary>
    /// Response body
    /// </summary>
    public object? Body { get; set; }

    /// <summary>
    /// Latency in milliseconds
    /// </summary>
    public int? LatencyMs { get; set; }
}
