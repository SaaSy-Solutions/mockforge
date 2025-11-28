namespace MockForge.Sdk;

/// <summary>
/// Builder for creating and configuring MockServer instances
///
/// <para>This builder provides a fluent API for configuring mock servers before starting them.
/// It follows the builder pattern for better ergonomics and type safety.</para>
///
/// <para>Example usage:</para>
/// <code>
/// var server = await new MockServerBuilder()
///     .Port(3000)
///     .Host("127.0.0.1")
///     .ConfigFile("./mockforge.yaml")
///     .OpenApiSpec("./api-spec.json")
///     .StartAsync();
/// </code>
/// </summary>
public class MockServerBuilder
{
    private int? _port;
    private string? _host;
    private string? _configFile;
    private string? _openApiSpec;

    /// <summary>
    /// Create a new MockServerBuilder
    /// </summary>
    public MockServerBuilder()
    {
        // Default values
        _host = "127.0.0.1";
    }

    /// <summary>
    /// Set the HTTP port
    /// </summary>
    /// <param name="port">Port number (0 for random available port)</param>
    /// <returns>This builder for method chaining</returns>
    public MockServerBuilder Port(int port)
    {
        _port = port;
        return this;
    }

    /// <summary>
    /// Set the host address
    /// </summary>
    /// <param name="host">Host address (default: "127.0.0.1")</param>
    /// <returns>This builder for method chaining</returns>
    public MockServerBuilder Host(string host)
    {
        _host = host;
        return this;
    }

    /// <summary>
    /// Load configuration from a YAML file
    /// </summary>
    /// <param name="configFile">Path to MockForge configuration file</param>
    /// <returns>This builder for method chaining</returns>
    public MockServerBuilder ConfigFile(string configFile)
    {
        _configFile = configFile;
        return this;
    }

    /// <summary>
    /// Load routes from an OpenAPI specification
    /// </summary>
    /// <param name="openApiSpec">Path to OpenAPI specification file</param>
    /// <returns>This builder for method chaining</returns>
    public MockServerBuilder OpenApiSpec(string openApiSpec)
    {
        _openApiSpec = openApiSpec;
        return this;
    }

    /// <summary>
    /// Build and start the MockServer asynchronously
    /// </summary>
    /// <returns>Started MockServer instance</returns>
    /// <exception cref="MockServerException">If the server fails to start</exception>
    public async Task<MockServer> StartAsync()
    {
        var config = new MockServerConfig
        {
            Port = _port ?? 0,
            Host = _host ?? "127.0.0.1",
            ConfigFile = _configFile,
            OpenApiSpec = _openApiSpec
        };

        return await MockServer.StartAsync(config);
    }

    /// <summary>
    /// Build the MockServerConfig without starting the server
    /// </summary>
    /// <returns>MockServerConfig instance</returns>
    public MockServerConfig Build()
    {
        return new MockServerConfig
        {
            Port = _port ?? 0,
            Host = _host ?? "127.0.0.1",
            ConfigFile = _configFile,
            OpenApiSpec = _openApiSpec
        };
    }
}
