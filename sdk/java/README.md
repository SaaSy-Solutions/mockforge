# MockForge Java SDK

Embed MockForge mock servers directly in your Java unit and integration tests.

## Prerequisites

**Important:** The Java SDK requires the MockForge CLI to be installed and available in your PATH.

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

### Maven

Add to your `pom.xml`:

```xml
<dependency>
    <groupId>com.mockforge</groupId>
    <artifactId>mockforge-sdk</artifactId>
    <version>0.1.0</version>
    <scope>test</scope>
</dependency>
```

### Gradle

Add to your `build.gradle`:

```gradle
testImplementation 'com.mockforge:mockforge-sdk:0.1.0'
```

## Usage

### Basic Example

```java
import com.mockforge.sdk.MockServer;
import com.mockforge.sdk.MockServerConfig;
import com.mockforge.sdk.MockServerException;

public class UserApiTest {
    @Test
    public void testUserApi() throws MockServerException {
        // Start a mock server
        MockServer server = MockServer.start(MockServerConfig.builder()
            .port(3000)
            .build());

        try {
            // Stub a response
            Map<String, Object> responseBody = new HashMap<>();
            responseBody.put("id", 123);
            responseBody.put("name", "John Doe");
            responseBody.put("email", "john@example.com");

            server.stubResponse("GET", "/api/users/123", responseBody);

            // Make requests to the mock
            // Use your HTTP client of choice (OkHttp, HttpClient, etc.)
            // GET http://localhost:3000/api/users/123

        } finally {
            // Stop the server
            server.stop();
        }
    }
}
```

### Using Builder Pattern

```java
MockServerConfig config = MockServerConfig.builder()
    .port(3000)
    .host("127.0.0.1")
    .configFile("./mockforge.yaml")
    .openApiSpec("./api-spec.json")
    .build();

MockServer server = MockServer.start(config);
```

### Stubbing with Options

```java
Map<String, String> headers = new HashMap<>();
headers.put("X-Custom-Header", "value");

server.stubResponse(
    "POST",
    "/api/users",
    Map.of("status", "created"),
    201,                    // Status code
    headers,                // Headers
    500                     // Latency in milliseconds
);
```

### Using with JUnit 5

```java
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.AfterEach;
import com.mockforge.sdk.*;

public class ApiIntegrationTest {
    private MockServer server;

    @AfterEach
    void tearDown() {
        if (server != null) {
            server.stop();
        }
    }

    @Test
    void testApi() throws MockServerException {
        server = MockServer.start(MockServerConfig.builder()
            .port(0) // Random port
            .build());

        server.stubResponse("GET", "/api/endpoint", Map.of("data", "test"));

        // Your test code here
    }
}
```

## API Reference

### `MockServer.start(config)`

Starts a mock server with the given configuration.

**Parameters:**
- `config` - `MockServerConfig` - Server configuration

**Returns:** `MockServer` - Started server instance

**Throws:** `MockServerException` - If server fails to start

### `MockServer` Methods

#### `stubResponse(method, path, body)`
Add a response stub with default options (status 200).

**Parameters:**
- `method` - `String` - HTTP method (GET, POST, etc.)
- `path` - `String` - Request path
- `body` - `Object` - Response body (will be serialized to JSON)

#### `stubResponse(method, path, body, status, headers, latencyMs)`
Add a response stub with custom options.

**Parameters:**
- `method` - `String` - HTTP method
- `path` - `String` - Request path
- `body` - `Object` - Response body
- `status` - `int` - HTTP status code
- `headers` - `Map<String, String>` - Response headers
- `latencyMs` - `Integer` - Latency in milliseconds (null for no delay)

#### `clearStubs()`
Remove all stubs.

#### `stop()`
Stop the server and cleanup resources.

#### `getUrl()`
Get the server URL (e.g., "http://127.0.0.1:3000").

#### `getPort()`
Get the server port.

#### `isRunning()`
Check if the server is running.

### `MockServerConfig` Builder

```java
MockServerConfig config = MockServerConfig.builder()
    .port(3000)                    // Port (default: 0 = random)
    .host("127.0.0.1")             // Host (default: 127.0.0.1)
    .configFile("./config.yaml")   // Config file (optional)
    .openApiSpec("./api.json")      // OpenAPI spec (optional)
    .build();
```

## Advanced Features

### Template Support

All stubs support MockForge's template syntax for dynamic responses:

```java
Map<String, Object> responseBody = new HashMap<>();
responseBody.put("id", "{{uuid}}");
responseBody.put("name", "{{faker.name}}");
responseBody.put("email", "{{faker.email}}");
responseBody.put("created_at", "{{now}}");

server.stubResponse("GET", "/api/users/{id}", responseBody);
```

### Using with OpenAPI Specs

Load routes from an OpenAPI specification:

```java
MockServerConfig config = MockServerConfig.builder()
    .openApiSpec("./api-spec.yaml")
    .build();

MockServer server = MockServer.start(config);
// All routes from the OpenAPI spec are now available
```

## Requirements

- Java 11 or higher
- MockForge CLI installed and available in PATH
- Maven or Gradle for dependency management

## Dependencies

- **OkHttp** - HTTP client for health checks and admin API
- **Gson** - JSON serialization

## Examples

See the `examples/` directory for complete working examples:

- [Basic Example](../examples/sdk-java/)
- [JUnit 5 Example](../examples/sdk-java/)
- [Spring Boot Integration](../examples/sdk-java/)

## License

MIT License - see [LICENSE](../../LICENSE-MIT) for details.
