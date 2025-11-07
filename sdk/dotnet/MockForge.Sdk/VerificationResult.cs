namespace MockForge.Sdk;

/// <summary>
/// Result of a verification operation
/// </summary>
public class VerificationResult
{
    /// <summary>
    /// Whether the verification passed
    /// </summary>
    public bool Matched { get; set; }

    /// <summary>
    /// Actual count of matching requests
    /// </summary>
    public int Count { get; set; }

    /// <summary>
    /// Expected count assertion
    /// </summary>
    public Dictionary<string, object> Expected { get; set; } = new();

    /// <summary>
    /// All matching request log entries (for inspection)
    /// </summary>
    public List<Dictionary<string, object>> Matches { get; set; } = new();

    /// <summary>
    /// Error message if verification failed
    /// </summary>
    public string? ErrorMessage { get; set; }
}
