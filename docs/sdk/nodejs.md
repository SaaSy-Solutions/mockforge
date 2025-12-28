# MockForge Node.js SDK

Embed MockForge servers directly in your JavaScript/TypeScript tests.

## Installation

```bash
npm install @mockforge/sdk
# or
yarn add @mockforge/sdk
# or
pnpm add @mockforge/sdk
```

## Prerequisites

The SDK requires the `mockforge` CLI to be installed and available in your PATH:

```bash
# Install via cargo
cargo install mockforge-cli

# Or via npm (coming soon)
npm install -g @mockforge/cli
```

## Quick Start

```typescript
import { MockServer } from '@mockforge/sdk';

// Start a mock server
const server = await MockServer.start({ port: 0 }); // port 0 = random available port

// Add response stubs
await server.stubResponse('GET', '/api/users/1', {
  id: 1,
  name: 'John Doe',
  email: 'john@example.com'
});

// Use the server
const response = await fetch(`${server.url()}/api/users/1`);
const user = await response.json();
console.log(user.name); // "John Doe"

// Stop when done
await server.stop();
```

## API Reference

### MockServer.start(config?)

Start a new mock server instance.

```typescript
interface MockServerConfig {
  port?: number;           // HTTP port (0 = random)
  host?: string;           // Bind host (default: 127.0.0.1)
  configFile?: string;     // MockForge config file path
  openApiSpec?: string;    // OpenAPI spec for validation
}

const server = await MockServer.start({
  port: 3000,
  openApiSpec: './api.yaml'
});
```

### server.stubResponse(method, path, body, options?)

Add a response stub.

```typescript
interface StubOptions {
  status?: number;                    // HTTP status code (default: 200)
  headers?: Record<string, string>;   // Response headers
  latencyMs?: number;                 // Simulated latency
}

// Simple stub
await server.stubResponse('GET', '/users', [
  { id: 1, name: 'Alice' },
  { id: 2, name: 'Bob' }
]);

// With options
await server.stubResponse('POST', '/users',
  { id: 3, name: 'Charlie' },
  {
    status: 201,
    headers: { 'Location': '/users/3' },
    latencyMs: 100
  }
);

// Error response
await server.stubResponse('GET', '/error',
  { error: 'Not Found' },
  { status: 404 }
);
```

### server.updateStub(method, path, body, options?)

Update an existing stub.

```typescript
// Initial stub
await server.stubResponse('GET', '/config', { version: 1 });

// Update it
await server.updateStub('GET', '/config', { version: 2 });
```

### server.removeStub(method, path)

Remove a specific stub.

```typescript
await server.removeStub('GET', '/api/users');
```

### server.clearStubs()

Remove all stubs.

```typescript
await server.clearStubs();
```

### server.url()

Get the server's base URL.

```typescript
const baseUrl = server.url(); // e.g., "http://127.0.0.1:3456"
```

### server.getPort()

Get the server's port number.

```typescript
const port = server.getPort(); // e.g., 3456
```

### server.isRunning()

Check if server is running.

```typescript
if (server.isRunning()) {
  // Server is active
}
```

### server.stop()

Stop the mock server.

```typescript
await server.stop();
```

## Request Verification

### server.verify(pattern, expected)

Verify requests match expectations.

```typescript
interface VerificationRequest {
  method?: string;
  path?: string;
  queryParams?: Record<string, string>;
  headers?: Record<string, string>;
  bodyPattern?: string;
}

interface VerificationCount {
  type: 'exactly' | 'at_least' | 'at_most' | 'between' | 'never';
  value?: number;
  min?: number;
  max?: number;
}

// Verify exact count
const result = await server.verify(
  { method: 'POST', path: '/orders' },
  { type: 'exactly', value: 3 }
);

if (!result.matched) {
  console.error(`Expected 3 calls, got ${result.count}`);
}
```

### server.verifyNever(pattern)

Verify no requests matched.

```typescript
const result = await server.verifyNever({
  method: 'DELETE',
  path: '/users/admin'
});

expect(result.matched).toBe(true);
```

### server.verifyAtLeast(pattern, min)

Verify at least N requests matched.

```typescript
const result = await server.verifyAtLeast(
  { method: 'GET', path: '/health' },
  5
);
```

### server.verifySequence(patterns)

Verify requests occurred in order.

```typescript
const result = await server.verifySequence([
  { method: 'POST', path: '/auth/login' },
  { method: 'GET', path: '/users/me' },
  { method: 'POST', path: '/auth/logout' }
]);
```

### server.countRequests(pattern)

Count matching requests.

```typescript
const count = await server.countRequests({
  method: 'GET',
  path: '/api/products'
});
console.log(`Products API called ${count} times`);
```

## Test Framework Integration

### Jest

```typescript
import { MockServer } from '@mockforge/sdk';

describe('User Service', () => {
  let server: MockServer;

  beforeAll(async () => {
    server = await MockServer.start();
  });

  afterAll(async () => {
    await server.stop();
  });

  beforeEach(async () => {
    await server.clearStubs();
  });

  it('fetches user profile', async () => {
    await server.stubResponse('GET', '/users/me', {
      id: 'user-1',
      name: 'Test User'
    });

    const user = await userService.getProfile(server.url());

    expect(user.name).toBe('Test User');
  });

  it('handles errors', async () => {
    await server.stubResponse('GET', '/users/me',
      { error: 'Unauthorized' },
      { status: 401 }
    );

    await expect(userService.getProfile(server.url()))
      .rejects.toThrow('Unauthorized');
  });
});
```

### Vitest

```typescript
import { describe, it, beforeAll, afterAll, beforeEach, expect } from 'vitest';
import { MockServer } from '@mockforge/sdk';

describe('API Client', () => {
  let server: MockServer;

  beforeAll(async () => {
    server = await MockServer.start();
  });

  afterAll(async () => {
    await server.stop();
  });

  beforeEach(async () => {
    await server.clearStubs();
  });

  it('retries on 503', async () => {
    let callCount = 0;

    // First call returns 503, second returns 200
    await server.stubResponse('GET', '/flaky',
      { error: 'Service Unavailable' },
      { status: 503 }
    );

    // Make request with retry logic
    const client = new APIClient(server.url(), { retries: 3 });

    // Update stub after first call
    setTimeout(async () => {
      await server.updateStub('GET', '/flaky', { success: true });
    }, 100);

    const result = await client.get('/flaky');
    expect(result.success).toBe(true);
  });
});
```

### Mocha

```typescript
import { MockServer } from '@mockforge/sdk';
import { expect } from 'chai';

describe('Payment Gateway', function() {
  let server: MockServer;

  before(async function() {
    server = await MockServer.start();
  });

  after(async function() {
    await server.stop();
  });

  it('processes payment', async function() {
    await server.stubResponse('POST', '/payments', {
      id: 'pay_123',
      status: 'succeeded'
    }, { status: 201 });

    const result = await gateway.charge({
      amount: 1000,
      currency: 'usd'
    });

    expect(result.status).to.equal('succeeded');
  });
});
```

## Advanced Usage

### OpenAPI Validation

```typescript
const server = await MockServer.start({
  openApiSpec: './openapi.yaml'
});

// Requests are validated against the spec
await server.stubResponse('POST', '/users', { id: 1, name: 'Alice' });

// Invalid request body returns 400
const res = await fetch(`${server.url()}/users`, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ invalid: 'schema' })
});

expect(res.status).toBe(400);
```

### Simulating Latency

```typescript
// Individual stub latency
await server.stubResponse('GET', '/slow-api',
  { data: 'result' },
  { latencyMs: 2000 }
);

// Test timeout handling
await expect(
  fetch(`${server.url()}/slow-api`, {
    signal: AbortSignal.timeout(1000)
  })
).rejects.toThrow();
```

### Testing Webhooks

```typescript
const server = await MockServer.start();

// Stub webhook endpoint
await server.stubResponse('POST', '/webhook', { received: true });

// Trigger action that sends webhook
await triggerWebhook(server.url() + '/webhook');

// Verify webhook was called with expected payload
const result = await server.verify(
  {
    method: 'POST',
    path: '/webhook',
    bodyPattern: '{"event":"user.created"}'
  },
  { type: 'exactly', value: 1 }
);

expect(result.matched).toBe(true);
```

### Multiple Servers

```typescript
const authServer = await MockServer.start({ port: 3001 });
const apiServer = await MockServer.start({ port: 3002 });

await authServer.stubResponse('POST', '/oauth/token', {
  access_token: 'test-token',
  expires_in: 3600
});

await apiServer.stubResponse('GET', '/protected', { secret: 'data' });

// Test with both servers
process.env.AUTH_URL = authServer.url();
process.env.API_URL = apiServer.url();

// Run tests...

await authServer.stop();
await apiServer.stop();
```

## Error Handling

```typescript
import { MockServer, MockServerError, MockServerErrorCode } from '@mockforge/sdk';

try {
  const server = await MockServer.start();
} catch (error) {
  if (error instanceof MockServerError) {
    switch (error.code) {
      case MockServerErrorCode.CLI_NOT_FOUND:
        console.error('MockForge CLI not installed');
        break;
      case MockServerErrorCode.PORT_DETECTION_FAILED:
        console.error('Could not detect server port');
        break;
      case MockServerErrorCode.HEALTH_CHECK_TIMEOUT:
        console.error('Server failed to start');
        break;
    }
  }
}
```

## TypeScript Support

Full TypeScript definitions included:

```typescript
import type {
  MockServerConfig,
  ResponseStub,
  StubOptions,
  VerificationRequest,
  VerificationCount,
  VerificationResult
} from '@mockforge/sdk';
```

## See Also

- [SDK Overview](./README.md)
- [Python SDK](./python.md)
- [Go SDK](./go.md)
