namespace MockForge.Sdk;

/// <summary>
/// Pattern for matching requests during verification
/// </summary>
public class VerificationRequest
{
    /// <summary>
    /// HTTP method to match (e.g., "GET", "POST"). Case-insensitive. If null, matches any method.
    /// </summary>
    public string? Method { get; set; }

    /// <summary>
    /// URL path to match. Supports exact match, wildcards (*, **), and regex. If null, matches any path.
    /// </summary>
    public string? Path { get; set; }

    /// <summary>
    /// Query parameters to match (all must be present and match). If empty, query parameters are not checked.
    /// </summary>
    public Dictionary<string, string> QueryParams { get; set; } = new();

    /// <summary>
    /// Headers to match (all must be present and match). Case-insensitive header names. If empty, headers are not checked.
    /// </summary>
    public Dictionary<string, string> Headers { get; set; } = new();

    /// <summary>
    /// Request body pattern to match. Supports exact match or regex. If null, body is not checked.
    /// </summary>
    public string? BodyPattern { get; set; }
}
