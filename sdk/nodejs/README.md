# MockForge Node.js SDK

Embed MockForge mock servers directly in your Node.js/TypeScript tests.

## Prerequisites

The Node.js SDK requires the MockForge CLI to be installed and available in your PATH:

```bash
# Via Cargo
cargo install mockforge-cli

# Or download pre-built binaries from:
# https://github.com/SaaSy-Solutions/mockforge/releases
```

## Installation

```bash
npm install @mockforge/sdk
```

## Usage

### Basic Example

```typescript
import { MockServer } from '@mockforge/sdk';

describe('API Tests', () => {
  let server: MockServer;

  beforeEach(async () => {
    server = await MockServer.start({ port: 3000 });
  });

  afterEach(async () => {
    await server.stop();
  });

  it('should mock user API', async () => {
    await server.stubResponse('GET', '/api/users/123', {
      id: 123,
      name: 'John Doe',
      email: 'john@example.com'
    });

    const response = await fetch('http://localhost:3000/api/users/123');
    const data = await response.json();

    expect(data.id).toBe(123);
    expect(data.name).toBe('John Doe');
  });
});
```

### With OpenAPI Specification

```typescript
import { MockServer } from '@mockforge/sdk';

const server = await MockServer.start({
  port: 3000,
  openApiSpec: './openapi.yaml'
});
```

### With Custom Configuration

```typescript
const server = await MockServer.start({
  port: 3000,
  host: '127.0.0.1',
  configFile: './mockforge.yaml'
});
```

## API Reference

### `MockServer.start(config)`

Starts a mock server.

**Config Options:**
| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `port` | `number` | random | Port to listen on |
| `host` | `string` | `127.0.0.1` | Host to bind to |
| `configFile` | `string` | - | Path to MockForge config file |
| `openApiSpec` | `string` | - | Path to OpenAPI specification |

### Instance Methods

| Method | Description |
|--------|-------------|
| `stubResponse(method, path, body, options?)` | Add a response stub |
| `clearStubs()` | Remove all stubs |
| `stop()` | Stop the server |
| `url()` | Get the server URL |
| `getPort()` | Get the server port |
| `isRunning()` | Check if server is running |

### Stub Options

```typescript
await server.stubResponse('GET', '/api/users', { users: [] }, {
  status: 200,
  headers: { 'X-Custom-Header': 'value' },
  latencyMs: 100
});
```

## Jest Integration

```typescript
// jest.setup.ts
import { MockServer } from '@mockforge/sdk';

let server: MockServer;

beforeAll(async () => {
  server = await MockServer.start({ port: 3000 });
});

afterAll(async () => {
  await server.stop();
});

beforeEach(async () => {
  await server.clearStubs();
});
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `MOCKFORGE_CLI_PATH` | Custom path to MockForge CLI binary |
| `MOCKFORGE_LOG_LEVEL` | Log level (debug, info, warn, error) |

## License

Apache-2.0 OR MIT
