namespace MockForge.Sdk;

/// <summary>
/// Exception thrown by MockServer operations
/// </summary>
public class MockServerException : Exception
{
    public MockServerException(string message) : base(message)
    {
    }

    public MockServerException(string message, Exception innerException)
        : base(message, innerException)
    {
    }
}
