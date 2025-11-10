# Tunnel Testing Requirements

## Current Status: Partial Implementation ✅

The tunneling feature is **structurally complete** but **functionally incomplete** for end-to-end testing. Here's what you have and what you need:

## ✅ What Works Now

1. **Tunnel Client** - Fully implemented
   - Can create tunnel configurations
   - Can connect to tunnel servers
   - CLI commands work (`start`, `stop`, `list`, `status`)
   - All validation and error handling works

2. **Test Tunnel Server** - Partially implemented
   - REST API for tunnel management (`POST /api/tunnels`, `GET /api/tunnels`, etc.)
   - Health check endpoint
   - In-memory tunnel storage
   - Subdomain management

3. **Unit Tests** - All passing
   - Configuration tests
   - Manager tests
   - Error handling tests

## ❌ What's Missing for Full Testing

### 1. HTTP Request Forwarding (CRITICAL)

**Problem**: The tunnel server can create/manage tunnels, but it doesn't actually forward HTTP requests.

**What's needed**:
- A route handler that receives HTTP requests on the public URL
- Looks up the tunnel by subdomain (from Host header)
- Forwards the request to the `local_url`
- Returns the response from the local server

**Impact**: Without this, you can create tunnels but can't actually use them to receive requests.

### 2. End-to-End Integration Test

**Problem**: No test verifies the complete flow:
```
External Request → Tunnel Server → Local Server → Response → Tunnel Server → External Client
```

**What's needed**:
- Test that starts a local HTTP server with known responses
- Creates a tunnel pointing to that local server
- Makes HTTP request to tunnel's public URL
- Verifies response matches local server response

### 3. Subdomain-Based Routing

**Problem**: The tunnel server doesn't route requests based on subdomains.

**What's needed**:
- Parse `Host` header from incoming requests
- Extract subdomain (e.g., `my-api.tunnel.example.com` → `my-api`)
- Look up tunnel by subdomain
- Route request to that tunnel's local URL

## How to Test What Exists Now

### Step 1: Build and Start Test Server

```bash
# Build the tunnel server
cargo build --package mockforge-tunnel --features server --bin tunnel-server

# Start it
./target/debug/tunnel-server
# Server runs on http://127.0.0.1:4040
```

### Step 2: Test Tunnel Management API

```bash
# Create a tunnel
curl -X POST http://localhost:4040/api/tunnels \
  -H "Content-Type: application/json" \
  -d '{
    "local_url": "http://localhost:3000",
    "subdomain": "my-api"
  }'
# Returns: {"tunnel_id": "...", "public_url": "https://my-api.tunnel.mockforge.test", ...}

# List tunnels
curl http://localhost:4040/api/tunnels

# Get tunnel status
curl http://localhost:4040/api/tunnels/<tunnel-id>

# Delete tunnel
curl -X DELETE http://localhost:4040/api/tunnels/<tunnel-id>
```

### Step 3: Test CLI Commands

```bash
# Start tunnel (will fail to connect without real server, but validates correctly)
mockforge tunnel start \
  --local-url http://localhost:3000 \
  --server-url http://localhost:4040 \
  --subdomain test-api

# List tunnels (requires server running)
mockforge tunnel list --server-url http://localhost:4040
```

## What You Need to Do for Full Testing

### Priority 1: Implement HTTP Proxy Handler

Add to `crates/mockforge-tunnel/src/server.rs`:

```rust
/// Proxy handler that forwards requests to local servers
async fn proxy_handler(
    State(store): State<TunnelStore>,
    headers: HeaderMap,
    req: Request<Body>,
) -> Result<Response<Body>, StatusCode> {
    // 1. Extract subdomain from Host header
    let host = headers.get("host")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let subdomain = extract_subdomain(host)?;

    // 2. Look up tunnel
    let tunnel = store.get_tunnel_by_subdomain(&subdomain).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // 3. Forward request to local_url
    let local_url = tunnel.local_url.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let target_url = format!("{}{}", local_url, req.uri().path_and_query().map(|pq| pq.as_str()).unwrap_or(""));

    // 4. Make request to local server
    let client = reqwest::Client::new();
    let response = client
        .request(req.method().clone(), &target_url)
        .headers(req.headers().clone())
        .body(req.into_body())
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    // 5. Convert and return response
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.bytes().await.unwrap();

    let mut response_builder = Response::builder().status(status);
    for (name, value) in headers.iter() {
        response_builder = response_builder.header(name, value);
    }

    response_builder.body(Body::from(body)).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
```

### Priority 2: Add Subdomain Lookup

Add to `TunnelStore`:

```rust
impl TunnelStore {
    pub async fn get_tunnel_by_subdomain(&self, subdomain: &str) -> Result<TunnelStatus> {
        let subdomains = self.subdomains.read().await;
        let tunnel_id = subdomains.get(subdomain)
            .ok_or_else(|| crate::TunnelError::NotFound(format!("Subdomain not found: {}", subdomain)))?
            .clone();
        drop(subdomains);

        self.get_tunnel(&tunnel_id).await
    }
}
```

### Priority 3: Add Wildcard Route

Add to router (this is tricky - axum doesn't support wildcard subdomain routing easily):

```rust
// Need to use a middleware or custom routing logic
// Or handle subdomain routing at a lower level
```

### Priority 4: Write Integration Test

```rust
#[tokio::test]
async fn test_full_tunnel_proxy() {
    // 1. Start test tunnel server
    let server_addr = start_test_server(0).await.unwrap();

    // 2. Start local test server
    let local_server = tokio::spawn(async {
        // Simple HTTP server that returns "Hello from local"
    });

    // 3. Create tunnel
    let client = reqwest::Client::new();
    let tunnel_resp = client
        .post(format!("http://{}/api/tunnels", server_addr))
        .json(&serde_json::json!({
            "local_url": "http://localhost:3000",
            "subdomain": "test"
        }))
        .send()
        .await
        .unwrap();

    let tunnel: TunnelStatus = tunnel_resp.json().await.unwrap();

    // 4. Make request to public URL
    // NOTE: This won't work until subdomain routing is implemented
    // Would need to configure DNS or use Host header
    let response = client
        .get(&tunnel.public_url)
        .header("Host", "test.tunnel.mockforge.test")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "Hello from local");
}
```

## Alternative: Simpler Testing Approach

For initial testing, you could:

1. **Skip subdomain routing** - Use path-based routing instead:
   ```
   POST /tunnel/<tunnel-id>/<path> → forward to local_url/<path>
   ```

2. **Use direct IP/port** - Test without DNS:
   ```
   http://localhost:4040/tunnel/<tunnel-id>/api/mocks
   ```

This is simpler but less realistic than subdomain-based routing.

## Summary

**You can test:**
- ✅ Tunnel creation/deletion via API
- ✅ CLI command parsing and validation
- ✅ Configuration management
- ✅ Error handling

**You cannot test yet:**
- ❌ Actual HTTP request forwarding
- ❌ End-to-end request/response flow
- ❌ WebSocket tunneling
- ❌ Real-world usage scenarios

**Next steps:**
1. Implement HTTP proxy handler in tunnel server
2. Add subdomain lookup functionality
3. Write integration test
4. Test with real MockForge instance

The foundation is solid - you just need to add the actual request forwarding layer to make it functional!
