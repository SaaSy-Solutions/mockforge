# UI Builder Integration Guide

This guide explains how to integrate the UI Builder into the main MockForge server.

## Overview

The UI Builder is now integrated at the library level. You can use it in your MockForge server by calling the `management_router_with_ui_builder()` function instead of `management_router()`.

## Integration Steps

### Step 1: Update the Main Server Code

In your main MockForge server code (likely in `src/main.rs` or where the admin server is configured), replace the management router with the UI Builder-enabled version:

**Before**:
```rust
use mockforge_http::{management_router, ManagementState};

let management_state = ManagementState::new(spec, spec_path, port);
let admin_router = management_router(management_state);
```

**After**:
```rust
use mockforge_http::{management_router_with_ui_builder, ManagementState};
use mockforge_core::config::ServerConfig;

let management_state = ManagementState::new(spec, spec_path, port);
let admin_router = management_router_with_ui_builder(
    management_state,
    server_config, // Your ServerConfig instance
);
```

### Step 2: Ensure ServerConfig is Available

The UI Builder needs access to the full `ServerConfig`. Make sure you have it available when setting up the admin server:

```rust
// Load or create your ServerConfig
let server_config = mockforge_core::config::load_config_auto().await?;

// Pass it to the UI Builder-enabled router
let admin_router = management_router_with_ui_builder(
    management_state,
    server_config.clone(), // Clone if needed elsewhere
);
```

### Step 3: Build the Frontend

Build the frontend for production:

```bash
cd ui-builder/frontend
npm install
npm run build
```

This creates optimized static files in `ui-builder/frontend/dist/`.

### Step 4: Serve Frontend Static Files (Optional but Recommended)

To serve the UI from the admin server, add static file serving:

```rust
use tower_http::services::ServeDir;

// Serve UI Builder static files
let ui_static = ServeDir::new("ui-builder/frontend/dist");

let admin_router = admin_router
    .nest_service("/ui", ui_static);
```

**Note**: Add `tower-http = { version = "0.5", features = ["fs"] }` to your `Cargo.toml`.

### Step 5: Start the Server

Start MockForge with admin enabled:

```bash
mockforge serve --admin-enabled --admin-port 9080
```

### Step 6: Access the UI Builder

Open your browser to:

- **API**: `http://localhost:9080/__mockforge/ui-builder/endpoints`
- **UI** (if serving static files): `http://localhost:9080/ui/`
- **Development**: `http://localhost:5173` (via `npm run dev`)

## Complete Example

Here's a complete example of integrating the UI Builder:

```rust
use mockforge_core::config::{load_config_auto, ServerConfig};
use mockforge_http::{management_router_with_ui_builder, ManagementState};
use mockforge_core::openapi::load_spec;
use axum::Router;
use std::sync::Arc;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let server_config = load_config_auto().await?;

    // Load OpenAPI spec if configured
    let (spec, spec_path) = if let Some(path) = &server_config.http.openapi_spec {
        let spec = Arc::new(load_spec(path).await?);
        (Some(spec), Some(path.clone()))
    } else {
        (None, None)
    };

    // Create management state
    let port = server_config.admin.port;
    let management_state = ManagementState::new(spec, spec_path, port);

    // Create admin router with UI Builder
    let admin_router = management_router_with_ui_builder(
        management_state,
        server_config.clone(),
    );

    // Optionally serve UI static files
    let ui_static = ServeDir::new("ui-builder/frontend/dist");
    let admin_router = Router::new()
        .nest("/__mockforge", admin_router)
        .nest_service("/ui", ui_static);

    // Start admin server
    let addr = format!("{}:{}", server_config.admin.host, server_config.admin.port)
        .parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    println!("Admin server listening on {}", addr);
    println!("UI Builder available at http://{}/__mockforge/ui-builder/endpoints", addr);
    println!("Web UI available at http://{}/ui/", addr);

    axum::serve(listener, admin_router).await?;

    Ok(())
}
```

## API Endpoints

Once integrated, the following endpoints are available:

| Method | Path | Description |
|--------|------|-------------|
| GET | `/__mockforge/ui-builder/endpoints` | List all endpoints |
| POST | `/__mockforge/ui-builder/endpoints` | Create endpoint |
| GET | `/__mockforge/ui-builder/endpoints/:id` | Get endpoint |
| PUT | `/__mockforge/ui-builder/endpoints/:id` | Update endpoint |
| DELETE | `/__mockforge/ui-builder/endpoints/:id` | Delete endpoint |
| POST | `/__mockforge/ui-builder/endpoints/validate` | Validate endpoint |
| GET | `/__mockforge/ui-builder/config` | Get server config |
| PUT | `/__mockforge/ui-builder/config` | Update config |
| GET | `/__mockforge/ui-builder/config/export` | Export as YAML |
| POST | `/__mockforge/ui-builder/config/import` | Import YAML/JSON |

## Testing the Integration

### Test the API

```bash
# List endpoints
curl http://localhost:9080/__mockforge/ui-builder/endpoints

# Create an endpoint
curl -X POST http://localhost:9080/__mockforge/ui-builder/endpoints \
  -H "Content-Type: application/json" \
  -d '{
    "id": "",
    "protocol": "http",
    "name": "Test Endpoint",
    "enabled": true,
    "config": {
      "type": "Http",
      "method": "GET",
      "path": "/test",
      "response": {
        "status": 200,
        "body": {
          "type": "Static",
          "content": {"message": "Hello World"}
        }
      }
    }
  }'

# Get all endpoints
curl http://localhost:9080/__mockforge/ui-builder/endpoints

# Export configuration
curl http://localhost:9080/__mockforge/ui-builder/config/export > config.yaml
```

### Test the UI

1. Start the development server:
```bash
cd ui-builder/frontend
npm run dev
```

2. Open `http://localhost:5173` in your browser

3. Create a test endpoint and verify it appears in the dashboard

## Configuration

### Enable/Disable UI Builder

You can conditionally enable the UI Builder:

```rust
let admin_router = if server_config.admin.ui_builder_enabled {
    management_router_with_ui_builder(management_state, server_config)
} else {
    management_router(management_state)
};
```

Add to your `ServerConfig`:

```rust
#[derive(Serialize, Deserialize)]
pub struct AdminConfig {
    // ... existing fields
    pub ui_builder_enabled: bool,
}
```

And in your YAML config:

```yaml
admin:
  enabled: true
  port: 9080
  ui_builder_enabled: true  # Add this
```

### Environment Variables

Support environment variable configuration:

```bash
export MOCKFORGE_UI_BUILDER_ENABLED=true
export MOCKFORGE_UI_BUILDER_PATH=/ui-builder
```

## Development Workflow

### Frontend Development

1. Start the backend:
```bash
cargo run -- serve --admin-enabled
```

2. Start the frontend in dev mode:
```bash
cd ui-builder/frontend
npm run dev
```

3. Make changes to frontend code - they'll hot reload automatically

4. Frontend will proxy API requests to `localhost:9080`

### Backend Development

1. Make changes to `ui_builder.rs`

2. Restart the server:
```bash
cargo run -- serve --admin-enabled
```

3. Test API changes with curl or the frontend

### Full Stack Development

Use two terminals:

**Terminal 1** (Backend):
```bash
cargo watch -x "run -- serve --admin-enabled"
```

**Terminal 2** (Frontend):
```bash
cd ui-builder/frontend
npm run dev
```

## Production Deployment

### Building

1. Build the frontend:
```bash
cd ui-builder/frontend
npm run build
```

2. Build the backend:
```bash
cargo build --release
```

### Deploying

#### Option 1: Serve Static Files from Binary

Embed the frontend in your binary using `include_dir` or similar:

```toml
[dependencies]
include_dir = "0.7"
```

```rust
use include_dir::{include_dir, Dir};

static UI_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/ui-builder/frontend/dist");

// Then serve it with axum
```

#### Option 2: Separate Static File Server

Serve the frontend from Nginx, Caddy, or a CDN:

```nginx
# Nginx configuration
location /ui/ {
    root /path/to/ui-builder/frontend/dist;
    try_files $uri $uri/ /index.html;
}

location /__mockforge/ui-builder/ {
    proxy_pass http://localhost:9080;
}
```

#### Option 3: Docker

```dockerfile
FROM rust:1.70 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM node:18 AS frontend
WORKDIR /app
COPY ui-builder/frontend .
RUN npm install && npm run build

FROM debian:bullseye-slim
COPY --from=builder /app/target/release/mockforge /usr/local/bin/
COPY --from=frontend /app/dist /usr/share/mockforge/ui
EXPOSE 9080
CMD ["mockforge", "serve", "--admin-enabled"]
```

## Troubleshooting

### Issue: API requests fail with 404

**Solution**: Make sure the admin server is running and the UI Builder router is mounted:
```bash
curl http://localhost:9080/__mockforge/ui-builder/endpoints
```

### Issue: CORS errors in browser

**Solution**: The frontend dev server (Vite) has a proxy configured. If deploying to production, add CORS headers:

```rust
use tower_http::cors::{CorsLayer, Any};

let admin_router = admin_router.layer(
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
);
```

### Issue: Frontend shows blank page

**Solution**:
1. Check browser console for errors
2. Verify API is accessible
3. Ensure `index.html` is being served correctly
4. Check network tab for failed requests

### Issue: Changes not reflected

**Solution**:
- Frontend: Hard refresh (Ctrl+Shift+R)
- Backend: Restart the server
- Clear browser cache

## Security Considerations

### Authentication

The UI Builder inherits authentication from the admin server. Add authentication middleware:

```rust
use axum::middleware;

async fn auth_middleware(/* ... */) -> Result<(), StatusCode> {
    // Check JWT, session, or API key
}

let admin_router = admin_router
    .layer(middleware::from_fn(auth_middleware));
```

### Rate Limiting

Add rate limiting to prevent abuse:

```rust
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

let governor_conf = GovernorConfigBuilder::default()
    .per_second(10)
    .burst_size(20)
    .finish()
    .unwrap();

let admin_router = admin_router
    .layer(GovernorLayer { config: Box::leak(Box::new(governor_conf)) });
```

### Input Validation

The UI Builder validates all inputs on both frontend and backend. Additional validation can be added in the route handlers.

## Next Steps

1. ✅ Integration complete - UI Builder API is mounted
2. ⏳ Add authentication if needed
3. ⏳ Build and bundle frontend for production
4. ⏳ Add to main MockForge CLI
5. ⏳ Write integration tests
6. ⏳ Update user documentation

## Support

- Issues: [GitHub Issues](https://github.com/mockforge/mockforge/issues)
- Docs: [UI Builder README](README.md)
- Architecture: [ARCHITECTURE.md](ARCHITECTURE.md)

---

**Last Updated**: October 2025
**Integration Status**: ✅ Complete - Ready to use
