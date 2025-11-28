# MockForge Rust SDK Example

This example demonstrates how to use the MockForge Rust SDK in your tests.

## Running the Example

```bash
cargo test
```

## What it Does

The example shows:

1. Starting an embedded mock server
2. Stubbing HTTP responses
3. Making requests to the mock server
4. Verifying responses
5. Stopping the server

## Key Features

- **Template Support**: Dynamic data with `{{uuid}}`, `{{faker.name}}`, etc.
- **Multiple Stubs**: Handle multiple endpoints
- **Clean Lifecycle**: Start/stop in test setup/teardown
- **Type-Safe**: Full Rust type safety
