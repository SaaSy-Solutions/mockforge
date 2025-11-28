namespace MockForge.Sdk;

/// <summary>
/// Fluent builder for creating response stubs
///
/// <para>This builder provides a fluent API for constructing ResponseStub objects
/// with method chaining for better readability.</para>
///
/// <para>Example usage:</para>
/// <code>
/// var stub = new StubBuilder("GET", "/api/users/{id}")
///     .Status(200)
///     .Header("Content-Type", "application/json")
///     .Body(new { id = 123, name = "John Doe" })
///     .Latency(100)
///     .Build();
///
/// await server.StubResponseAsync(stub);
/// </code>
/// </summary>
public class StubBuilder
{
    private readonly string _method;
    private readonly string _path;
    private int _status = 200;
    private readonly Dictionary<string, string> _headers = new();
    private object? _body;
    private int? _latencyMs;

    /// <summary>
    /// Create a new StubBuilder
    /// </summary>
    /// <param name="method">HTTP method (GET, POST, PUT, DELETE, etc.)</param>
    /// <param name="path">Request path pattern (supports path parameters like {id})</param>
    public StubBuilder(string method, string path)
    {
        _method = method?.ToUpperInvariant() ?? "GET";
        _path = path ?? throw new ArgumentNullException(nameof(path));
    }

    /// <summary>
    /// Set the HTTP status code
    /// </summary>
    /// <param name="status">HTTP status code (default: 200)</param>
    /// <returns>This builder for method chaining</returns>
    public StubBuilder Status(int status)
    {
        _status = status;
        return this;
    }

    /// <summary>
    /// Add a response header
    /// </summary>
    /// <param name="key">Header name</param>
    /// <param name="value">Header value</param>
    /// <returns>This builder for method chaining</returns>
    public StubBuilder Header(string key, string value)
    {
        if (key != null && value != null)
        {
            _headers[key] = value;
        }
        return this;
    }

    /// <summary>
    /// Set multiple response headers at once
    /// </summary>
    /// <param name="headers">Dictionary of header names to values</param>
    /// <returns>This builder for method chaining</returns>
    public StubBuilder Headers(Dictionary<string, string> headers)
    {
        if (headers != null)
        {
            foreach (var kvp in headers)
            {
                _headers[kvp.Key] = kvp.Value;
            }
        }
        return this;
    }

    /// <summary>
    /// Set the response body
    ///
    /// <para>The body will be serialized to JSON when the stub is registered.
    /// Supports MockForge template syntax like {{uuid}}, {{faker.name}}, etc.</para>
    /// </summary>
    /// <param name="body">Response body (will be serialized to JSON)</param>
    /// <returns>This builder for method chaining</returns>
    public StubBuilder Body(object? body)
    {
        _body = body;
        return this;
    }

    /// <summary>
    /// Set response latency in milliseconds
    /// </summary>
    /// <param name="ms">Latency in milliseconds</param>
    /// <returns>This builder for method chaining</returns>
    public StubBuilder Latency(int ms)
    {
        _latencyMs = ms;
        return this;
    }

    /// <summary>
    /// Build the ResponseStub
    /// </summary>
    /// <returns>ResponseStub instance</returns>
    /// <exception cref="InvalidOperationException">If body is not set</exception>
    public ResponseStub Build()
    {
        if (_body == null)
        {
            throw new InvalidOperationException("Response body is required");
        }

        return new ResponseStub
        {
            Method = _method,
            Path = _path,
            Status = _status,
            Headers = new Dictionary<string, string>(_headers),
            Body = _body,
            LatencyMs = _latencyMs
        };
    }
}
