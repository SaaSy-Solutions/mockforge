namespace MockForge.Sdk;

/// <summary>
/// Count assertion for verification
/// </summary>
public class VerificationCount
{
    /// <summary>
    /// Request must be made exactly N times
    /// </summary>
    public static Dictionary<string, object> Exactly(int n)
    {
        return new Dictionary<string, object>
        {
            { "type", "exactly" },
            { "value", n }
        };
    }

    /// <summary>
    /// Request must be made at least N times
    /// </summary>
    public static Dictionary<string, object> AtLeast(int n)
    {
        return new Dictionary<string, object>
        {
            { "type", "at_least" },
            { "value", n }
        };
    }

    /// <summary>
    /// Request must be made at most N times
    /// </summary>
    public static Dictionary<string, object> AtMost(int n)
    {
        return new Dictionary<string, object>
        {
            { "type", "at_most" },
            { "value", n }
        };
    }

    /// <summary>
    /// Request must never be made (count must be 0)
    /// </summary>
    public static Dictionary<string, object> Never()
    {
        return new Dictionary<string, object>
        {
            { "type", "never" }
        };
    }

    /// <summary>
    /// Request must be made at least once (count >= 1)
    /// </summary>
    public static Dictionary<string, object> AtLeastOnce()
    {
        return new Dictionary<string, object>
        {
            { "type", "at_least_once" }
        };
    }
}
