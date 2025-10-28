# MockForge + Vitest Integration Example

This example demonstrates how to use MockForge with Vitest for unit and integration testing.

## Overview

MockForge provides a powerful mock server that can be easily integrated with Vitest tests. This example shows:

- Starting MockForge server automatically with Vitest's `globalSetup`
- Switching scenarios during tests
- Updating mocks dynamically
- Testing different user scenarios (authenticated, unauthenticated, errors, etc.)
- Resetting state between tests

## Prerequisites

- Node.js 18+ and npm
- Rust and Cargo
- MockForge CLI installed (`cargo install mockforge-cli`)

## Installation

1. Install Node.js dependencies:

```bash
cd examples/test-integration/vitest
npm install
```

2. Build the test server binary:

```bash
cd ../..
cargo build --bin mockforge-test-server
```

## Running Tests

Run all tests:

```bash
npm test
```

Run tests in watch mode:

```bash
npm run test:watch
```

Run tests with UI:

```bash
npm run test:ui
```

Run tests with coverage:

```bash
npm run test:coverage
```

## How It Works

### 1. Global Setup

The `vitest.config.ts` file configures Vitest to use global setup:

```typescript
test: {
  globalSetup: './tests/setup.ts',
}
```

The setup file (`tests/setup.ts`) handles:
- Starting the MockForge server before tests
- Waiting for the server to be ready
- Stopping the server after all tests complete

### 2. Test Structure

Each test file can:
- Reset mocks between tests (in `beforeEach`)
- Switch scenarios using the management API
- Update mocks dynamically
- Validate responses

### 3. Scenario Management

Tests can switch scenarios using the MockForge management API:

```typescript
await fetch(`${BASE_URL}/__mockforge/workspace/switch`, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ workspace: 'user-authenticated' }),
});
```

## Test Structure

```
tests/
├── setup.ts          # Global setup and teardown
└── example.test.ts   # Example tests
```

### Available Test Suites

1. **MockForge Integration**: Basic functionality tests
   - Health checks
   - API calls
   - Scenario switching
   - Dynamic mocks
   - Statistics
   - Fixtures

2. **User Authentication Scenarios**: Auth-related tests
   - Authenticated access
   - Unauthenticated access (401)

3. **Error Handling Scenarios**: Error condition tests
   - Server errors (500)
   - Network timeouts
   - Slow responses

4. **Data Validation**: Request/response validation
   - Valid data acceptance
   - Invalid data rejection

## Configuration

### Environment Variables

- `MOCKFORGE_URL`: Base URL for MockForge (default: `http://localhost:3000`)
- `RUST_LOG`: Rust logging level for the server (default: `info`)

### Vitest Configuration

See [vitest.config.ts](./vitest.config.ts) for full configuration options.

## Best Practices

### 1. Reset State Between Tests

Always reset mocks in `beforeEach` to ensure test isolation:

```typescript
beforeEach(async () => {
  await fetch(`${BASE_URL}/__mockforge/reset`, { method: 'POST' });
});
```

### 2. Use Descriptive Test Names

Write clear test names that describe the behavior:

```typescript
it('authenticated user can access protected endpoint', async () => {
  // ...
});
```

### 3. Test Different Scenarios

Create separate test suites for different scenarios:

```typescript
describe('User Authentication Scenarios', () => {
  // Auth-related tests
});

describe('Error Handling Scenarios', () => {
  // Error-related tests
});
```

### 4. Validate Responses

Always validate both status codes and response data:

```typescript
const response = await fetch(`${BASE_URL}/api/users`);
expect(response.ok).toBe(true);

const users = await response.json();
expect(Array.isArray(users)).toBe(true);
```

## Advanced Usage

### Custom Workspaces

Create workspace configuration files and load them:

```typescript
const workspaceConfig = {
  name: 'custom-workspace',
  mocks: [
    { endpoint: '/api/users', response: [...] },
  ],
};

await fetch(`${BASE_URL}/__mockforge/workspace/load`, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify(workspaceConfig),
});
```

### Per-Test Scenarios

Switch scenarios in individual tests:

```typescript
it('handles specific scenario', async () => {
  await fetch(`${BASE_URL}/__mockforge/workspace/switch`, {
    method: 'POST',
    body: JSON.stringify({ workspace: 'my-scenario' }),
  });

  // Test with this scenario
  // ...
});
```

### Testing with Multiple Servers

Modify `setup.ts` to start multiple servers on different ports:

```typescript
// Start additional servers for different services
const authServer = spawn('cargo', ['run', '--bin', 'mockforge-test-server', '--', '--port', '3001']);
const paymentServer = spawn('cargo', ['run', '--bin', 'mockforge-test-server', '--', '--port', '3002']);
```

## Troubleshooting

### Server Doesn't Start

1. Ensure MockForge CLI is installed:
   ```bash
   cargo install mockforge-cli
   ```

2. Build the test server binary:
   ```bash
   cargo build --bin mockforge-test-server
   ```

3. Check the server logs:
   ```bash
   RUST_LOG=debug npm test
   ```

### Port Already in Use

If port 3000 is already in use, update the port in:
- `tests/setup.ts` (SERVER_PORT constant)
- Test files (BASE_URL)
- Test server configuration

### Tests Timing Out

Increase timeouts in `vitest.config.ts`:

```typescript
test: {
  testTimeout: 20000, // 20 seconds
}
```

### Server Doesn't Stop After Tests

The teardown function should handle this, but if issues persist:

1. Manually kill the process:
   ```bash
   pkill -f mockforge-test-server
   ```

2. Check for zombie processes:
   ```bash
   ps aux | grep mockforge
   ```

## Debugging

### Enable Verbose Logging

Set environment variables:

```bash
RUST_LOG=debug npm test
```

### Use Vitest UI

Run tests with the UI for better debugging:

```bash
npm run test:ui
```

### Inspect Server State

Query the management API during tests:

```typescript
const stats = await fetch(`${BASE_URL}/__mockforge/stats`).then(r => r.json());
console.log('Server stats:', stats);
```

## Next Steps

- Explore the [MockForge documentation](https://docs.mockforge.dev)
- Check out [mockforge-test crate documentation](../../crates/mockforge-test/README.md)
- See [Playwright integration example](../playwright/README.md)

## License

This example is part of the MockForge project and follows the same license.
