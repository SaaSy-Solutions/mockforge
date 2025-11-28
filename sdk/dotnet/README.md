# MockForge .NET SDK

Embed MockForge mock servers directly in your .NET unit and integration tests.

## Prerequisites

**Important:** The .NET SDK requires the MockForge CLI to be installed and available in your PATH.

### Install MockForge CLI

```bash
# Via Cargo
cargo install mockforge-cli

# Or download pre-built binaries from:
# https://github.com/SaaSy-Solutions/mockforge/releases
```

Verify installation:
```bash
mockforge --version
```

## Installation

### NuGet Package Manager

```bash
Install-Package MockForge.Sdk
```

### .NET CLI

```bash
dotnet add package MockForge.Sdk
```

### PackageReference

Add to your `.csproj`:

```xml
<ItemGroup>
  <PackageReference Include="MockForge.Sdk" Version="0.1.0" />
</ItemGroup>
```

## Usage

### Basic Example

```csharp
using MockForge.Sdk;

public class UserApiTests
{
    [Fact]
    public async Task TestUserApi()
    {
        // Start a mock server
        var server = await MockServer.StartAsync(new MockServerConfig
        {
            Port = 3000
        });

        try
        {
            // Stub a response
            await server.StubResponseAsync("GET", "/api/users/123", new
            {
                id = 123,
                name = "John Doe",
                email = "john@example.com"
            });

            // Make requests to the mock
            // Use HttpClient or your HTTP client of choice
            // GET http://localhost:3000/api/users/123
        }
        finally
        {
            // Stop the server
            server.Dispose();
        }
    }
}
```

### Using IDisposable Pattern

```csharp
using (var server = await MockServer.StartAsync(new MockServerConfig
{
    Port = 3000
}))
{
    await server.StubResponseAsync("GET", "/api/users/123", new
    {
        id = 123,
        name = "John Doe"
    });

    // Your test code here
}
```

### Using MockServerBuilder

```csharp
// Fluent builder for creating and starting servers
var server = await new MockServerBuilder()
    .Port(3000)
    .Host("127.0.0.1")
    .ConfigFile("./mockforge.yaml")
    .OpenApiSpec("./api-spec.json")
    .StartAsync();
```

### Stubbing with Options

```csharp
var headers = new Dictionary<string, string>
{
    { "X-Custom-Header", "value" }
};

await server.StubResponseAsync(
    "POST",
    "/api/users",
    new { status = "created" },
    status: 201,
    headers: headers,
    latencyMs: 500 // 500ms latency
);
```

### Using StubBuilder (Fluent API)

```csharp
// Create a stub using the fluent builder pattern
var stub = new StubBuilder("GET", "/api/users/{id}")
    .Status(200)
    .Header("Content-Type", "application/json")
    .Header("X-Custom-Header", "value")
    .Body(new
    {
        id = "{{uuid}}",
        name = "{{faker.name}}",
        email = "{{faker.email}}"
    })
    .Latency(100)
    .Build();

// Register the stub
await server.StubResponseAsync(stub);
```

You can also chain multiple stubs:

```csharp
await server.StubResponseAsync(new StubBuilder("GET", "/api/users")
    .Status(200)
    .Body(new[]
    {
        new { id = 1, name = "Alice" },
        new { id = 2, name = "Bob" }
    })
    .Build());

await server.StubResponseAsync(new StubBuilder("POST", "/api/users")
    .Status(201)
    .Header("Location", "/api/users/123")
    .Body(new { id = 123, status = "created" })
    .Build());
```

### Using with xUnit

```csharp
using MockForge.Sdk;
using Xunit;

public class ApiIntegrationTests : IDisposable
{
    private MockServer? _server;

    public void Dispose()
    {
        _server?.Dispose();
    }

    [Fact]
    public async Task TestApi()
    {
        _server = await MockServer.StartAsync(new MockServerConfig
        {
            Port = 0 // Random port
        });

        await _server.StubResponseAsync("GET", "/api/endpoint", new { data = "test" });

        // Your test code here
    }
}
```

### Using with NUnit

```csharp
using MockForge.Sdk;
using NUnit.Framework;

[TestFixture]
public class ApiIntegrationTests
{
    private MockServer? _server;

    [SetUp]
    public async Task SetUp()
    {
        _server = await MockServer.StartAsync(new MockServerConfig
        {
            Port = 0
        });
    }

    [TearDown]
    public void TearDown()
    {
        _server?.Dispose();
    }

    [Test]
    public async Task TestApi()
    {
        await _server!.StubResponseAsync("GET", "/api/endpoint", new { data = "test" });
        // Your test code here
    }
}
```

## API Reference

### `MockServerBuilder`

Fluent builder for creating and starting mock servers.

**Methods:**
- `Port(int port)` - Set the HTTP port (0 for random)
- `Host(string host)` - Set the host address
- `ConfigFile(string path)` - Load configuration from YAML file
- `OpenApiSpec(string path)` - Load routes from OpenAPI spec
- `StartAsync()` - Build and start the MockServer asynchronously
- `Build()` - Build MockServerConfig without starting

**Example:**
```csharp
var server = await new MockServerBuilder()
    .Port(3000)
    .OpenApiSpec("./api.json")
    .StartAsync();
```

### `StubBuilder`

Fluent builder for creating response stubs.

**Methods:**
- `Status(int code)` - Set HTTP status code (default: 200)
- `Header(string key, string value)` - Add a response header
- `Headers(Dictionary<string, string> headers)` - Set multiple headers
- `Body(object body)` - Set response body (required)
- `Latency(int ms)` - Set response latency in milliseconds
- `Build()` - Build the ResponseStub

**Example:**
```csharp
var stub = new StubBuilder("GET", "/api/users/{id}")
    .Status(200)
    .Header("Content-Type", "application/json")
    .Body(new { id = 123, name = "John" })
    .Latency(100)
    .Build();
```

### `MockServer.StartAsync(config)`

Starts a mock server with the given configuration asynchronously.

**Parameters:**
- `config` - `MockServerConfig?` - Server configuration (optional)

**Returns:** `Task<MockServer>` - Started server instance

**Throws:** `MockServerException` - If server fails to start

### `MockServer` Methods

#### `StubResponseAsync(method, path, body)`
Add a response stub with default options (status 200).

**Parameters:**
- `method` - `string` - HTTP method (GET, POST, etc.)
- `path` - `string` - Request path
- `body` - `object?` - Response body (will be serialized to JSON)

#### `StubResponseAsync(method, path, body, status, headers, latencyMs)`
Add a response stub with custom options.

**Parameters:**
- `method` - `string` - HTTP method
- `path` - `string` - Request path
- `body` - `object?` - Response body
- `status` - `int` - HTTP status code (default: 200)
- `headers` - `Dictionary<string, string>?` - Response headers
- `latencyMs` - `int?` - Latency in milliseconds (null for no delay)

#### `StubResponseAsync(ResponseStub stub)`
Add a response stub using a ResponseStub object (created with StubBuilder).

**Parameters:**
- `stub` - `ResponseStub` - Response stub instance

#### `ClearStubsAsync()`
Remove all stubs asynchronously.

#### `Stop()`
Stop the server synchronously.

#### `Dispose()`
Stop the server and cleanup resources (implements `IDisposable`).

#### `GetUrl()`
Get the server URL (e.g., "http://127.0.0.1:3000").

#### `GetPort()`
Get the server port.

#### `IsRunning()`
Check if the server is running.

### `MockServerConfig`

```csharp
var config = new MockServerConfig
{
    Port = 3000,                    // Port (default: 0 = random)
    Host = "127.0.0.1",             // Host (default: 127.0.0.1)
    ConfigFile = "./config.yaml",   // Config file (optional)
    OpenApiSpec = "./api.json"       // OpenAPI spec (optional)
};
```

## Advanced Features

### Template Support

All stubs support MockForge's template syntax for dynamic responses:

```csharp
await server.StubResponseAsync("GET", "/api/users/{id}", new
{
    id = "{{uuid}}",
    name = "{{faker.name}}",
    email = "{{faker.email}}",
    created_at = "{{now}}"
});
```

### Using with OpenAPI Specs

Load routes from an OpenAPI specification:

```csharp
var config = new MockServerConfig
{
    OpenApiSpec = "./api-spec.yaml"
};

var server = await MockServer.StartAsync(config);
// All routes from the OpenAPI spec are now available
```

### Async/Await Support

All operations are fully asynchronous:

```csharp
var server = await MockServer.StartAsync(config);
await server.StubResponseAsync("GET", "/api/test", new { data = "test" });
await server.ClearStubsAsync();
```

## Requirements

- .NET 6.0 or higher (or .NET Standard 2.1+)
- MockForge CLI installed and available in PATH

## Dependencies

- **System.Text.Json** - JSON serialization (built-in)

## Examples

See the `examples/` directory for complete working examples:

- [Basic Example](../examples/sdk-dotnet/)
- [xUnit Example](../examples/sdk-dotnet/)
- [NUnit Example](../examples/sdk-dotnet/)

## License

MIT License - see [LICENSE](../../LICENSE-MIT) for details.
