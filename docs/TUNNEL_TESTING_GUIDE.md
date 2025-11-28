# Tunnel Testing Guide

This guide explains what's needed to fully test the MockForge tunneling feature.

## Current Status

The tunneling feature is **partially implemented**. Here's what exists and what's needed:

### ✅ What's Already Implemented

1. **Tunnel Client Library** (`mockforge-tunnel` crate)
   - Client code to connect to tunnel servers
   - Configuration management
   - Provider abstraction (self-hosted, Cloud, Cloudflare, etc.)
   - Tunnel manager for lifecycle management

2. **Test Tunnel Server** (with `--features server`)
   - Basic HTTP REST API for tunnel management
   - In-memory tunnel store
   - Subdomain management
   - Health check endpoint

3. **CLI Commands**
   - `mockforge tunnel start` - Start a tunnel
   - `mockforge tunnel stop` - Stop a tunnel
   - `mockforge tunnel list` - List active tunnels
   - `mockforge tunnel status` - Get tunnel status

4. **Unit Tests**
   - Basic configuration tests
   - Tunnel store tests

### ❌ What's Missing for Full Testing

1. **Actual Request Forwarding**
   - The tunnel server API exists, but there's no HTTP proxy layer
   - Need to implement: receive request → forward to local URL → return response
   - WebSocket forwarding not yet implemented

2. **End-to-End Integration**
   - No complete integration test that:
     - Starts test server
     - Creates tunnel
     - Makes HTTP request to public URL
     - Verifies request is forwarded to local server
     - Verifies response is returned

3. **Real Tunnel Server**
   - The test server only manages tunnel metadata (create/list/delete)
   - Doesn't actually proxy HTTP requests
   - Need a production-ready tunnel server implementation

## How to Test Currently

### 1. Build the Test Tunnel Server

```bash
cargo build --package mockforge-tunnel --features server --bin tunnel-server
```

### 2. Start the Test Tunnel Server

```bash
# Terminal 1: Start tunnel server
./target/debug/tunnel-server
# Or set port:
TUNNEL_SERVER_PORT=4040 ./target/debug/tunnel-server
```

The server will start on `http://127.0.0.1:4040` (or your specified port).

### 3. Test API Endpoints

You can manually test the REST API:

```bash
# Health check
curl http://localhost:4040/health

# Create a tunnel
curl -X POST http://localhost:4040/api/tunnels \
  -H "Content-Type: application/json" \
  -d '{
    "local_url": "http://localhost:3000",
    "subdomain": "my-test-api"
  }'

# List tunnels
curl http://localhost:4040/api/tunnels

# Get tunnel status
curl http://localhost:4040/api/tunnels/<tunnel-id>

# Delete tunnel
curl -X DELETE http://localhost:4040/api/tunnels/<tunnel-id>
```

### 4. Test CLI Commands

```bash
# Start a tunnel
mockforge tunnel start \
  --local-url http://localhost:3000 \
  --server-url http://localhost:4040 \
  --subdomain my-dev-api

# List tunnels
mockforge tunnel list

# Get status
mockforge tunnel status

# Stop tunnel (need tunnel ID from list/status)
mockforge tunnel stop <tunnel-id>
```

### 5. Run Unit Tests

```bash
# Test tunnel library (without server features)
cargo test --package mockforge-tunnel

# Test with server features (includes integration tests)
cargo test --package mockforge-tunnel --features server
```

## What Needs to be Built for Full Testing

### 1. HTTP Request Proxy Layer

Add to `crates/mockforge-tunnel/src/server.rs`:

```rust
// Add a route handler that proxies requests to local URLs
async fn proxy_handler(
    State(store): State<TunnelStore>,
    req: Request<Body>,
) -> Result<Response, StatusCode> {
    // 1. Extract subdomain from Host header
    // 2. Look up tunnel by subdomain
    // 3. Forward request to tunnel.local_url
    // 4. Return response
}
```

### 2. Integration Test Setup

Create a complete integration test that:

```rust
#[tokio::test]
async fn test_end_to_end_tunnel() {
    // 1. Start test tunnel server
    let server_addr = start_test_server(0).await?;

    // 2. Start a real local HTTP server (mockforge or simple test server)
    let local_server = start_local_test_server(3000).await?;

    // 3. Create tunnel via API
    let tunnel = create_tunnel("http://localhost:3000").await?;

    // 4. Make request to public URL
    let response = reqwest::get(&tunnel.public_url).await?;

    // 5. Verify request reached local server
    // 6. Verify response matches expected
}
```

### 3. WebSocket Proxy (Future)

For WebSocket support, need to:
- Upgrade HTTP connection to WebSocket
- Maintain bidirectional connection between client ↔ tunnel server ↔ local server
- Forward frames in both directions

### 4. Production Tunnel Server

The current test server is minimal. A production server needs:

- **Subdomain routing**: Map public subdomains to tunnels
- **Connection pooling**: Manage multiple concurrent tunnels
- **Load balancing**: Distribute requests across tunnel instances
- **Authentication**: API key management
- **Rate limiting**: Prevent abuse
- **Metrics**: Track usage, latency, errors
- **Persistent storage**: Store tunnel configs in database
- **TLS termination**: Handle HTTPS for public URLs

## Recommended Testing Approach

### Phase 1: Basic HTTP Proxy (Current Priority)

1. Implement HTTP request forwarding in test server
2. Write integration test that:
   - Starts local test server with known responses
   - Creates tunnel
   - Makes HTTP request to public URL
   - Verifies response matches local server response

### Phase 2: CLI Integration

1. Test `mockforge tunnel start` with real server
2. Verify tunnel persists and handles requests
3. Test `mockforge tunnel stop` properly cleans up

### Phase 3: Error Handling

1. Test connection failures (local server down)
2. Test timeout handling
3. Test invalid subdomain requests
4. Test concurrent requests

### Phase 4: WebSocket Support

1. Implement WebSocket proxy
2. Test bidirectional communication
3. Test connection lifecycle

### Phase 5: Production Readiness

1. Deploy tunnel server to cloud
2. Test with real domain and DNS
3. Load testing
4. Security audit

## Next Steps

To fully test the tunneling feature, you should:

1. **Implement HTTP proxy in test server** - Add request forwarding logic
2. **Write end-to-end integration test** - Verify full request/response cycle
3. **Test with real MockForge instance** - Start `mockforge serve` and tunnel to it
4. **Test with external services** - Try webhook callbacks from real services

## Example: Manual End-to-End Test

```bash
# Terminal 1: Start tunnel server
cargo run --package mockforge-tunnel --features server --bin tunnel-server

# Terminal 2: Start MockForge server
mockforge serve --http-port 3000

# Terminal 3: Create tunnel and test
# Create tunnel
TUNNEL_ID=$(curl -s -X POST http://localhost:4040/api/tunnels \
  -H "Content-Type: application/json" \
  -d '{"local_url": "http://localhost:3000", "subdomain": "test"}' \
  | jq -r '.tunnel_id')

# Get public URL
PUBLIC_URL=$(curl -s http://localhost:4040/api/tunnels/$TUNNEL_ID | jq -r '.public_url')

# Make request (this will fail until proxy is implemented)
curl $PUBLIC_URL/api/mocks
```

## Summary

The tunneling feature has a solid foundation but needs the **HTTP proxy layer** to be fully functional. Once that's implemented, you can run complete end-to-end tests. The current code supports:

- ✅ Tunnel creation/management
- ✅ Configuration and CLI
- ✅ Provider abstraction
- ❌ Actual request forwarding (needs implementation)
- ❌ End-to-end testing (blocked by missing proxy)
