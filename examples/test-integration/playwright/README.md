# MockForge + Playwright Integration Example

This example demonstrates how to use MockForge with Playwright for end-to-end testing.

## Overview

MockForge provides a powerful mock server that can be easily integrated with Playwright tests. This example shows:

- Starting MockForge server automatically with Playwright's `webServer` option
- Switching scenarios during tests
- Updating mocks dynamically
- Testing different user scenarios (authenticated, unauthenticated, errors, etc.)

## Prerequisites

- Node.js 18+ and npm
- Rust and Cargo
- MockForge CLI installed (`cargo install mockforge-cli`)

## Installation

1. Install Node.js dependencies:

```bash
cd examples/test-integration/playwright
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

Run tests in UI mode:

```bash
npm run test:ui
```

Run tests in debug mode:

```bash
npm run test:debug
```

## How It Works

### 1. Automatic Server Startup

The `playwright.config.ts` file configures Playwright to automatically start the MockForge server:

```typescript
webServer: {
  command: 'cargo run --bin mockforge-test-server',
  url: 'http://localhost:3000/health',
  reuseExistingServer: !process.env.CI,
  timeout: 30 * 1000,
}
```

This ensures:
- MockForge starts before tests run
- Server is ready (waits for `/health` endpoint)
- Server is reused in development mode
- Server is cleaned up after tests

### 2. Scenario Switching

Tests can switch scenarios using the MockForge management API:

```typescript
await request.post('/__mockforge/workspace/switch', {
  data: { workspace: 'user-authenticated' },
});
```

### 3. Dynamic Mock Updates

Tests can update mocks on the fly:

```typescript
await request.post('/__mockforge/config/api/users/1', {
  data: {
    id: 1,
    name: 'Test User',
    email: 'test@example.com',
  },
});
```

## Test Structure

```
tests/
└── example.spec.ts    # Example tests demonstrating various features
```

### Available Test Scenarios

1. **Health Check**: Verifies server is running
2. **Basic API Calls**: Tests fetching mock data
3. **Scenario Switching**: Tests changing between scenarios
4. **Dynamic Mocks**: Tests updating mocks during test execution
5. **Authentication Scenarios**: Tests authenticated and unauthenticated states
6. **Error Scenarios**: Tests server error responses
7. **Performance Scenarios**: Tests slow responses

## Configuration

### Environment Variables

- `MOCKFORGE_URL`: Base URL for MockForge (default: `http://localhost:3000`)
- `CI`: Set to `true` for CI mode (affects retries and parallelism)

### Playwright Configuration

See [playwright.config.ts](./playwright.config.ts) for full configuration options.

## Advanced Usage

### Custom Workspaces

Create workspace configuration files and load them:

```typescript
// Load a workspace
await request.post('/__mockforge/workspace/load', {
  data: await fs.readFile('workspace.json', 'utf-8'),
});
```

### Multiple Servers

Run multiple MockForge instances on different ports:

```typescript
// In playwright.config.ts
webServer: [
  {
    command: 'cargo run --bin mockforge-test-server -- --port 3000',
    url: 'http://localhost:3000/health',
  },
  {
    command: 'cargo run --bin mockforge-test-server -- --port 3001',
    url: 'http://localhost:3001/health',
  },
],
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

If port 3000 is already in use, either:

1. Stop the process using port 3000
2. Change the port in `playwright.config.ts` and test server configuration

### Tests Timing Out

Increase the timeout in `playwright.config.ts`:

```typescript
webServer: {
  timeout: 60 * 1000, // 60 seconds
}
```

## Next Steps

- Explore the [MockForge documentation](https://docs.mockforge.dev)
- Check out [mockforge-test crate documentation](../../crates/mockforge-test/README.md)
- See [Vitest integration example](../vitest/README.md)

## License

This example is part of the MockForge project and follows the same license.
