# MockForge SDK API Standardization

This document defines the standard API that all MockForge SDKs must implement to ensure consistency across languages.

## Core Principles

1. **Consistent Method Names**: Same functionality should have the same name across all SDKs
2. **Unified Error Types**: All SDKs should use the same error codes and error structure
3. **Feature Parity**: All SDKs should support the same features
4. **Language Idioms**: While maintaining consistency, respect language-specific conventions

## Standard API Methods

### Initialization

#### Constructor
```typescript
// Node.js/TypeScript
new MockServer(config?: MockServerConfig)

// Python
MockServer(**kwargs)

// Go
NewMockServer(config MockServerConfig) *MockServer

// Java
new MockServer(config: MockServerConfig)

// .NET
new MockServer(config?: MockServerConfig)
```

#### Static Factory (Start)
```typescript
// Node.js/TypeScript
MockServer.start(config?: MockServerConfig): Promise<MockServer>

// Python
MockServer(port=3000).start() -> MockServer  # Context manager

// Go
NewMockServer(config).Start() error

// Java
MockServer.start(config: MockServerConfig): MockServer

// .NET
MockServer.StartAsync(config?: MockServerConfig): Task<MockServer>
```

### Stub Management

#### Add Stub
```typescript
// All SDKs
stubResponse(method: string, path: string, body: any, options?: StubOptions): Promise<void> | void
```

#### Update Stub
```typescript
// All SDKs
updateStub(method: string, path: string, body: any, options?: StubOptions): Promise<void> | void
```

#### Remove Stub
```typescript
// All SDKs
removeStub(method: string, path: string): Promise<void> | void
```

#### Clear All Stubs
```typescript
// All SDKs
clearStubs(): Promise<void> | void
```

### Server Control

#### Stop Server
```typescript
// Node.js/TypeScript
stop(): Promise<void>

// Python
stop(): None

// Go
Stop() error

// Java
stop(): void

// .NET
Dispose() / StopAsync(): Task
```

#### Get Server Info
```typescript
// All SDKs
url(): string
getPort(): number
isRunning(): boolean
```

### Verification

All SDKs should support:
- `verify(pattern, expected)`
- `verifyNever(pattern)`
- `verifyAtLeast(pattern, min)`
- `verifySequence(patterns)`
- `countRequests(pattern)`

## Standard Error Types

### Error Codes

```typescript
enum MockServerErrorCode {
  CLI_NOT_FOUND = "CLI_NOT_FOUND",
  SERVER_START_FAILED = "SERVER_START_FAILED",
  PORT_DETECTION_FAILED = "PORT_DETECTION_FAILED",
  ADMIN_API_ERROR = "ADMIN_API_ERROR",
  HEALTH_CHECK_TIMEOUT = "HEALTH_CHECK_TIMEOUT",
  INVALID_CONFIG = "INVALID_CONFIG",
  STUB_NOT_FOUND = "STUB_NOT_FOUND",
  NETWORK_ERROR = "NETWORK_ERROR",
  UNKNOWN_ERROR = "UNKNOWN_ERROR"
}
```

### Error Structure

```typescript
interface MockServerError {
  code: MockServerErrorCode;
  message: string;
  cause?: Error;
  details?: Record<string, any>;
}
```

## Implementation by Language

### Node.js/TypeScript
```typescript
export class MockServerError extends Error {
  constructor(
    public code: string,
    message: string,
    public cause?: Error,
    public details?: Record<string, any>
  ) {
    super(message);
    this.name = 'MockServerError';
  }
}
```

### Python
```python
class MockServerError(Exception):
    def __init__(self, code: str, message: str, cause: Optional[Exception] = None, details: Optional[Dict[str, Any]] = None):
        self.code = code
        self.cause = cause
        self.details = details
        super().__init__(message)
```

### Go
```go
type MockServerError struct {
    Code    string
    Message string
    Cause   error
    Details map[string]interface{}
}

func (e *MockServerError) Error() string {
    return e.Message
}
```

### Java
```java
public class MockServerException extends Exception {
    private final String code;
    private final Map<String, Object> details;

    public MockServerException(String code, String message) {
        super(message);
        this.code = code;
        this.details = new HashMap<>();
    }

    public MockServerException(String code, String message, Throwable cause) {
        super(message, cause);
        this.code = code;
        this.details = new HashMap<>();
    }

    public String getCode() { return code; }
    public Map<String, Object> getDetails() { return details; }
}
```

### .NET
```csharp
public class MockServerException : Exception
{
    public string Code { get; }
    public Dictionary<string, object> Details { get; }

    public MockServerException(string code, string message)
        : base(message)
    {
        Code = code;
        Details = new Dictionary<string, object>();
    }

    public MockServerException(string code, string message, Exception innerException)
        : base(message, innerException)
    {
        Code = code;
        Details = new Dictionary<string, object>();
    }
}
```

## Configuration Standardization

### MockServerConfig

All SDKs should support:
- `port?: number` (default: 0 for random)
- `host?: string` (default: "127.0.0.1")
- `configFile?: string`
- `openApiSpec?: string`

### StubOptions

All SDKs should support:
- `status?: number` (default: 200)
- `headers?: Record<string, string>`
- `latencyMs?: number`

## Migration Guide

### For Existing Code

1. Replace generic exceptions with `MockServerError`/`MockServerException`
2. Use error codes for programmatic error handling
3. Update method names to match standard API
4. Add missing methods (updateStub, removeStub)

## Testing Requirements

All SDKs must have:
1. Unit tests for error handling
2. Integration tests for all API methods
3. Error code validation tests
4. Cross-language compatibility tests
