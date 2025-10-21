# MockForge Test Integration Examples

This directory contains examples showing how to integrate MockForge with popular test frameworks.

## Available Examples

### 1. [Playwright Integration](./playwright/)

Complete example showing how to use MockForge with Playwright for end-to-end testing.

**Features:**
- Automatic server startup/shutdown
- Scenario switching during tests
- Dynamic mock updates
- Authentication scenarios
- Error handling scenarios

[Read the Playwright guide →](./playwright/README.md)

### 2. [Vitest Integration](./vitest/)

Complete example showing how to use MockForge with Vitest for unit and integration testing.

**Features:**
- Global setup/teardown
- Test isolation with state resets
- Scenario management
- Comprehensive test examples

[Read the Vitest guide →](./vitest/README.md)

## Quick Start

### Prerequisites

1. Install Rust and Cargo
2. Install MockForge CLI:
   ```bash
   cargo install mockforge-cli
   ```

### Build the Test Server Binary

The examples use a helper binary that starts MockForge:

```bash
cargo build --bin mockforge-test-server
```

### Run Playwright Tests

```bash
cd playwright
npm install
npm test
```

### Run Vitest Tests

```bash
cd vitest
npm install
npm test
```

## How It Works

Both examples use the same pattern:

1. **Test Server Binary** ([src/bin/test_server.rs](./src/bin/test_server.rs))
   - Rust binary that uses `mockforge-test` crate
   - Spawns MockForge server on port 3000
   - Waits for `/health` endpoint
   - Handles graceful shutdown

2. **Test Framework Integration**
   - Playwright uses `webServer` option
   - Vitest uses `globalSetup`
   - Both automatically start/stop the server

3. **Test Code**
   - Uses MockForge management API (`/__mockforge/*`)
   - Switches scenarios as needed
   - Updates mocks dynamically
   - Resets state between tests

## MockForge Management API

All examples use these management endpoints:

### Health Check
```http
GET /health
```

Returns server health status.

### Switch Scenario
```http
POST /__mockforge/workspace/switch
Content-Type: application/json

{
  "workspace": "scenario-name"
}
```

### Update Mock
```http
POST /__mockforge/config/{endpoint}
Content-Type: application/json

{
  "response": { ... }
}
```

### Reset Mocks
```http
POST /__mockforge/reset
```

### Get Statistics
```http
GET /__mockforge/stats
```

### List Fixtures
```http
GET /__mockforge/fixtures
```

## Architecture

```
examples/test-integration/
├── Cargo.toml                    # Rust workspace for test server
├── src/
│   └── bin/
│       └── test_server.rs        # Test server binary
├── playwright/                   # Playwright example
│   ├── package.json
│   ├── playwright.config.ts
│   ├── tests/
│   │   └── example.spec.ts
│   └── README.md
└── vitest/                       # Vitest example
    ├── package.json
    ├── vitest.config.ts
    ├── tests/
    │   ├── setup.ts              # Global setup
    │   └── example.test.ts
    └── README.md
```

## Common Patterns

### Pattern 1: Scenario per Test

```typescript
test('authenticated user scenario', async () => {
  await switchScenario('user-authenticated');
  // Test authenticated behavior
});

test('unauthenticated user scenario', async () => {
  await switchScenario('user-unauthenticated');
  // Test unauthenticated behavior
});
```

### Pattern 2: Dynamic Mocks

```typescript
test('handles custom user data', async () => {
  await updateMock('/api/user/123', {
    id: 123,
    name: 'Test User',
    role: 'admin',
  });

  // Test with custom data
});
```

### Pattern 3: Error Scenarios

```typescript
test('handles server errors gracefully', async () => {
  await switchScenario('server-errors');

  // Test error handling
  const response = await fetch('/api/users');
  expect(response.status).toBe(500);
});
```

### Pattern 4: State Reset

```typescript
beforeEach(async () => {
  // Reset to clean state before each test
  await resetMocks();
});
```

## Benefits

### For Frontend Testing
- No need for real backend
- Fast, reliable tests
- Test edge cases easily
- Isolated test environments

### For Integration Testing
- Simulate complex scenarios
- Test error conditions
- Validate API contracts
- Performance testing

### For CI/CD
- No external dependencies
- Deterministic tests
- Fast execution
- Easy to reproduce

## Advanced Features

### Chaos Engineering
Test resilience with chaos scenarios:

```bash
cargo run --bin mockforge-test-server -- --chaos-scenario network_degradation
```

### Multiple Protocols
MockForge supports multiple protocols:
- HTTP/REST
- WebSocket
- gRPC
- GraphQL
- MQTT
- SMTP
- Kafka
- AMQP
- FTP

### Request Recording
Record and replay real API interactions:

```bash
cargo run --bin mockforge-test-server -- --recorder-enabled
```

## Troubleshooting

### Server Won't Start

**Problem:** Server fails to start

**Solutions:**
1. Check if port 3000 is available:
   ```bash
   lsof -i :3000
   ```

2. Verify MockForge is installed:
   ```bash
   which mockforge
   ```

3. Check logs:
   ```bash
   RUST_LOG=debug npm test
   ```

### Tests Hang

**Problem:** Tests don't complete

**Solutions:**
1. Increase timeout in test configuration
2. Check server health manually:
   ```bash
   curl http://localhost:3000/health
   ```

### Linker Errors

**Problem:** Build fails with linker errors

**Solution:** This is a system configuration issue. The code is valid, but you may need to:
1. Install required system libraries
2. Check `.cargo/config.toml` linker settings
3. Try a different linker or remove mold configuration

## Resources

- [MockForge Documentation](https://docs.mockforge.dev)
- [mockforge-test Crate](../../crates/mockforge-test/README.md)
- [Playwright Documentation](https://playwright.dev)
- [Vitest Documentation](https://vitest.dev)

## Contributing

Contributions are welcome! Please submit examples for other test frameworks:
- Jest
- Mocha
- Cypress
- WebdriverIO
- etc.

## License

These examples are part of the MockForge project and follow the same MIT/Apache-2.0 dual license.
