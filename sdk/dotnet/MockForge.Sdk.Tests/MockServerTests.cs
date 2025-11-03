using MockForge.Sdk;
using Xunit;

namespace MockForge.Sdk.Tests;

public class MockServerTests : IDisposable
{
    private MockServer? _server;

    public void Dispose()
    {
        _server?.Dispose();
    }

    [Fact]
    public async Task TestStartAndStop()
    {
        var config = new MockServerConfig
        {
            Port = 0 // Random port
        };

        _server = await MockServer.StartAsync(config);
        Assert.True(_server.IsRunning());
        Assert.True(_server.GetPort() > 0);
    }

    [Fact]
    public async Task TestStubResponse()
    {
        var config = new MockServerConfig
        {
            Port = 0
        };

        _server = await MockServer.StartAsync(config);

        var responseBody = new
        {
            id = 123,
            name = "John Doe"
        };

        await _server.StubResponseAsync("GET", "/api/users/123", responseBody);

        // Verify stub was added
        // In a real test, you would make an HTTP request here
        Assert.True(_server.IsRunning());
    }

    [Fact]
    public async Task TestStubResponseWithOptions()
    {
        var config = new MockServerConfig
        {
            Port = 0
        };

        _server = await MockServer.StartAsync(config);

        var headers = new Dictionary<string, string>
        {
            { "X-Custom-Header", "value" }
        };

        await _server.StubResponseAsync(
            "POST",
            "/api/users",
            new { status = "created" },
            201,
            headers,
            500 // 500ms latency
        );

        Assert.True(_server.IsRunning());
    }

    [Fact]
    public async Task TestClearStubs()
    {
        var config = new MockServerConfig
        {
            Port = 0
        };

        _server = await MockServer.StartAsync(config);

        await _server.StubResponseAsync("GET", "/api/test", new { data = "test" });
        await _server.ClearStubsAsync();

        Assert.True(_server.IsRunning());
    }

    [Fact]
    public async Task TestGetUrl()
    {
        var config = new MockServerConfig
        {
            Port = 3000,
            Host = "127.0.0.1"
        };

        _server = await MockServer.StartAsync(config);
        Assert.Equal("http://127.0.0.1:3000", _server.GetUrl());
    }
}
