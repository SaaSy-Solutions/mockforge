# CORS Enhancement for Browser Access

## Current Status

The MockForge HTTP server accepts a `HttpCorsConfig` parameter in `build_router_with_multi_tenant()` but does not actually apply CORS middleware. The parameter is currently unused (prefixed with `_`).

## Required Enhancement

To enable browser-based access from ForgeConnect, CORS middleware needs to be applied to the HTTP router.

### Location

File: `crates/mockforge-http/src/lib.rs`

### Implementation

Add CORS middleware using `tower_http::cors::CorsLayer` based on the `HttpCorsConfig`:

```rust
use tower_http::cors::{CorsLayer, Any};

// In build_router_with_multi_tenant(), after creating the router:
if let Some(cors_config) = cors_config {
    if cors_config.enabled {
        let mut cors_layer = CorsLayer::new();

        // Configure allowed origins
        if cors_config.allowed_origins.contains(&"*".to_string()) {
            cors_layer = cors_layer.allow_origin(Any);
        } else {
            for origin in &cors_config.allowed_origins {
                cors_layer = cors_layer.allow_origin(origin.parse().unwrap_or_else(|_| Any));
            }
        }

        // Configure allowed methods
        if !cors_config.allowed_methods.is_empty() {
            let methods: Vec<_> = cors_config.allowed_methods
                .iter()
                .filter_map(|m| m.parse().ok())
                .collect();
            if !methods.is_empty() {
                cors_layer = cors_layer.allow_methods(methods);
            }
        }

        // Configure allowed headers
        if !cors_config.allowed_headers.is_empty() {
            let headers: Vec<_> = cors_config.allowed_headers
                .iter()
                .filter_map(|h| h.parse().ok())
                .collect();
            if !headers.is_empty() {
                cors_layer = cors_layer.allow_headers(headers);
            }
        }

        app = app.layer(cors_layer);
    }
} else {
    // Default: permissive CORS for development
    app = app.layer(CorsLayer::permissive());
}
```

### Default Configuration

The default `HttpCorsConfig` in `mockforge-core/src/config.rs` already allows:
- All origins (`*`)
- Common HTTP methods (GET, POST, PUT, DELETE, PATCH, OPTIONS)
- Common headers (Content-Type, Authorization)

This should work for browser access out of the box once CORS middleware is applied.

## Testing

After implementation, test with:

```bash
# Start MockForge
mockforge serve --http-port 3000

# Test CORS from browser console
fetch('http://localhost:3000/mocks', {
  method: 'GET',
  headers: { 'Accept': 'application/json' }
})
  .then(r => r.json())
  .then(console.log);
```

## Priority

**High** - Required for ForgeConnect browser SDK to function properly.
