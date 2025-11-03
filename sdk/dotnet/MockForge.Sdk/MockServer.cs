using System.Diagnostics;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json;

namespace MockForge.Sdk;

/// <summary>
/// MockServer - Embedded mock server for testing
///
/// <para>This class provides a simple API for embedding MockForge mock servers
/// in .NET unit and integration tests. It manages the MockForge CLI process
/// lifecycle and provides methods for stubbing responses.</para>
///
/// <para>Example usage:</para>
/// <code>
/// var server = await MockServer.StartAsync(new MockServerConfig
/// {
///     Port = 3000
/// });
///
/// try
/// {
///     await server.StubResponseAsync("GET", "/api/users/123", new
///     {
///         id = 123,
///         name = "John Doe"
///     });
///
///     // Make requests to http://localhost:3000/api/users/123
/// }
/// finally
/// {
///     server.Dispose();
/// }
/// </code>
/// </summary>
public class MockServer : IDisposable
{
    private const int STARTUP_TIMEOUT_SECONDS = 10;
    private const int HEALTH_CHECK_INTERVAL_MS = 100;

    private readonly MockServerConfig _config;
    private Process? _process;
    private readonly int _port;
    private readonly string _host;
    private int _adminPort = 0;
    private readonly List<ResponseStub> _stubs = new();
    private readonly HttpClient _httpClient;
    private bool _disposed = false;

    /// <summary>
    /// Create a new MockServer with the given configuration
    /// </summary>
    public MockServer(MockServerConfig? config = null)
    {
        _config = config ?? new MockServerConfig();
        _port = _config.Port;
        _host = _config.Host ?? "127.0.0.1";
        _httpClient = new HttpClient
        {
            Timeout = TimeSpan.FromSeconds(1)
        };
    }

    /// <summary>
    /// Static factory method to create and start a MockServer asynchronously
    /// </summary>
    /// <param name="config">Configuration for the mock server</param>
    /// <returns>Started MockServer instance</returns>
    /// <exception cref="MockServerException">If the server fails to start</exception>
    public static async Task<MockServer> StartAsync(MockServerConfig? config = null)
    {
        var server = new MockServer(config);
        await server.StartAsync();
        return server;
    }

    /// <summary>
    /// Start the mock server
    /// </summary>
    /// <exception cref="MockServerException">If the server fails to start</exception>
    public async Task StartAsync()
    {
        var args = new List<string> { "serve" };

        if (!string.IsNullOrEmpty(_config.ConfigFile))
        {
            args.Add("--config");
            args.Add(_config.ConfigFile);
        }

        if (!string.IsNullOrEmpty(_config.OpenApiSpec))
        {
            args.Add("--spec");
            args.Add(_config.OpenApiSpec);
        }

        if (_port != 0)
        {
            args.Add("--http-port");
            args.Add(_port.ToString());
        }

        // Enable admin API for dynamic stub management
        args.Add("--admin");
        args.Add("--admin-port");
        args.Add("0"); // Auto-assign port

        var startInfo = new ProcessStartInfo
        {
            FileName = "mockforge",
            Arguments = string.Join(" ", args),
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            CreateNoWindow = true
        };

        try
        {
            _process = Process.Start(startInfo);
            if (_process == null)
            {
                throw new MockServerException("Failed to start MockForge process");
            }

            await WaitForServerAsync();
        }
        catch (Exception e)
        {
            throw new MockServerException("Failed to start MockForge process", e);
        }
    }

    /// <summary>
    /// Wait for the server to be ready by polling the health endpoint
    /// </summary>
    private async Task WaitForServerAsync()
    {
        var startTime = DateTime.UtcNow;
        var timeout = TimeSpan.FromSeconds(STARTUP_TIMEOUT_SECONDS);

        var healthUrl = $"http://{_host}:{_port}/health";

        while (DateTime.UtcNow - startTime < timeout)
        {
            try
            {
                var response = await _httpClient.GetAsync(healthUrl);
                if (response.IsSuccessStatusCode)
                {
                    // Server is ready
                    return;
                }
            }
            catch (HttpRequestException)
            {
                // Server not ready yet, continue polling
            }

            await Task.Delay(HEALTH_CHECK_INTERVAL_MS);
        }

        // If we get here, server didn't start in time
        if (_process != null && !_process.HasExited)
        {
            _process.Kill();
            _process.WaitForExit(5000);
            _process.Dispose();
            _process = null;
        }

        throw new MockServerException(
            $"Timeout waiting for server to start on {_host}:{_port}"
        );
    }

    /// <summary>
    /// Stub a response
    /// </summary>
    /// <param name="method">HTTP method (GET, POST, etc.)</param>
    /// <param name="path">Request path</param>
    /// <param name="body">Response body</param>
    public async Task StubResponseAsync(string method, string path, object? body)
    {
        await StubResponseAsync(method, path, body, 200, null, null);
    }

    /// <summary>
    /// Stub a response with options
    /// </summary>
    /// <param name="method">HTTP method</param>
    /// <param name="path">Request path</param>
    /// <param name="body">Response body</param>
    /// <param name="status">HTTP status code</param>
    /// <param name="headers">Response headers</param>
    /// <param name="latencyMs">Latency in milliseconds</param>
    public async Task StubResponseAsync(
        string method,
        string path,
        object? body,
        int status = 200,
        Dictionary<string, string>? headers = null,
        int? latencyMs = null
    )
    {
        var stub = new ResponseStub
        {
            Method = method.ToUpperInvariant(),
            Path = path,
            Body = body,
            Status = status,
            Headers = headers ?? new Dictionary<string, string>(),
            LatencyMs = latencyMs
        };

        _stubs.Add(stub);

        // If admin API is available, use it to add the stub dynamically
        if (_adminPort != 0)
        {
            try
            {
                var stubJson = JsonSerializer.Serialize(stub);
                var content = new StringContent(
                    stubJson,
                    Encoding.UTF8,
                    "application/json"
                );

                var response = await _httpClient.PostAsync(
                    $"http://{_host}:{_adminPort}/api/stubs",
                    content
                );

                if (!response.IsSuccessStatusCode)
                {
                    // Log warning but continue - stub is stored locally
                    Console.Error.WriteLine("Warning: Failed to add stub via admin API");
                }
            }
            catch (HttpRequestException)
            {
                // Silently fail - stub is stored locally as fallback
            }
        }
    }

    /// <summary>
    /// Clear all stubs
    /// </summary>
    public async Task ClearStubsAsync()
    {
        _stubs.Clear();

        if (_adminPort != 0)
        {
            try
            {
                await _httpClient.DeleteAsync($"http://{_host}:{_adminPort}/api/stubs");
            }
            catch (HttpRequestException)
            {
                // Silently fail
            }
        }
    }

    /// <summary>
    /// Get the server URL
    /// </summary>
    /// <returns>Server URL (e.g., "http://127.0.0.1:3000")</returns>
    public string GetUrl()
    {
        return $"http://{_host}:{_port}";
    }

    /// <summary>
    /// Get the server port
    /// </summary>
    /// <returns>Server port</returns>
    public int GetPort()
    {
        return _port;
    }

    /// <summary>
    /// Check if the server is running
    /// </summary>
    /// <returns>true if server is running, false otherwise</returns>
    public bool IsRunning()
    {
        return _process != null && !_process.HasExited;
    }

    /// <summary>
    /// Stop the mock server
    /// </summary>
    public void Stop()
    {
        if (_process != null && !_process.HasExited)
        {
            _process.Kill();

            // Wait for process to exit (with timeout)
            if (!_process.WaitForExit(5000))
            {
                // Force kill if it didn't exit gracefully
                _process.Kill();
            }

            _process.Dispose();
            _process = null;
        }
    }

    /// <summary>
    /// Dispose resources
    /// </summary>
    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        Stop();
        _httpClient.Dispose();
        _disposed = true;
        GC.SuppressFinalize(this);
    }
}
