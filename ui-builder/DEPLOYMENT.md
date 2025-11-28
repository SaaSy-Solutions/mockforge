# UI Builder Deployment Guide

## Production Build Status

✅ **Build Complete**

- Frontend bundle: **352 KB** (109 KB gzipped)
- CSS bundle: **13.2 KB** (3.4 KB gzipped)
- Total: **365 KB** (~113 KB gzipped)
- Build time: **1.22s**

Built files are in: `/home/rclanan/dev/projects/work/mockforge/ui-builder/frontend/dist/`

## Quick Deploy

### Option 1: Serve from MockForge Binary (Recommended)

Add static file serving to your MockForge admin server:

```rust
use tower_http::services::ServeDir;

let ui_static = ServeDir::new("ui-builder/frontend/dist");

let admin_router = Router::new()
    .nest("/__mockforge", management_router_with_ui_builder(state, config))
    .nest_service("/ui", ui_static);
```

**Add to Cargo.toml**:
```toml
[dependencies]
tower-http = { version = "0.5", features = ["fs"] }
```

**Access**:
- Web UI: `http://localhost:9080/ui/`
- API: `http://localhost:9080/__mockforge/ui-builder/`

### Option 2: Nginx Reverse Proxy

```nginx
server {
    listen 80;
    server_name mockforge.example.com;

    # Serve UI static files
    location /ui/ {
        root /path/to/mockforge/ui-builder/frontend/dist;
        try_files $uri $uri/ /index.html;
    }

    # Proxy API requests
    location /__mockforge/ui-builder/ {
        proxy_pass http://localhost:9080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }
}
```

### Option 3: Caddy Server

```caddyfile
mockforge.example.com {
    # Serve UI
    handle /ui/* {
        root * /path/to/mockforge/ui-builder/frontend/dist
        try_files {path} /index.html
        file_server
    }

    # Proxy API
    handle /__mockforge/ui-builder/* {
        reverse_proxy localhost:9080
    }
}
```

### Option 4: Docker

```dockerfile
FROM rust:1.70 as backend-builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM node:18 as frontend-builder
WORKDIR /app
COPY ui-builder/frontend ./
RUN npm install && npm run build

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=backend-builder /app/target/release/mockforge /usr/local/bin/
COPY --from=frontend-builder /app/dist /usr/share/mockforge/ui

ENV MOCKFORGE_UI_PATH=/usr/share/mockforge/ui

EXPOSE 9080
CMD ["mockforge", "serve", "--admin-enabled", "--admin-port", "9080"]
```

Build and run:
```bash
docker build -t mockforge-with-ui .
docker run -p 9080:9080 mockforge-with-ui
```

### Option 5: Embed in Binary (Advanced)

Embed the UI directly in the binary using `include_dir`:

**Add to Cargo.toml**:
```toml
[dependencies]
include_dir = "0.7"
mime_guess = "2.0"
```

**In your code**:
```rust
use include_dir::{include_dir, Dir};
use axum::response::Response;
use axum::http::{StatusCode, header};

static UI_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/ui-builder/frontend/dist");

async fn serve_ui(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    // Try to get the file
    let file = UI_DIR.get_file(path)
        .or_else(|| UI_DIR.get_file("index.html"));

    match file {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(file.contents().into())
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Not found".into())
            .unwrap()
    }
}

// In your router
let admin_router = Router::new()
    .fallback(serve_ui);
```

## Environment Variables

Configure UI Builder behavior:

```bash
# Enable UI Builder
export MOCKFORGE_UI_BUILDER_ENABLED=true

# UI static files path
export MOCKFORGE_UI_PATH=/usr/share/mockforge/ui

# Admin server settings
export MOCKFORGE_ADMIN_ENABLED=true
export MOCKFORGE_ADMIN_PORT=9080
export MOCKFORGE_ADMIN_HOST=0.0.0.0
```

## Production Checklist

### Before Deploying

- [ ] Frontend built (`npm run build`)
- [ ] Backend compiled (`cargo build --release`)
- [ ] Environment variables configured
- [ ] Static files accessible
- [ ] Ports open in firewall
- [ ] HTTPS/TLS configured (if internet-facing)
- [ ] Authentication enabled (if needed)
- [ ] CORS configured properly

### Security

#### 1. Enable Authentication

```rust
use axum::middleware;
use axum::http::StatusCode;

async fn auth_middleware(
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Check for API key, JWT, or session
    let auth_header = headers.get("Authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(token) if verify_token(token) => Ok(next.run(request).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

let admin_router = admin_router
    .layer(middleware::from_fn(auth_middleware));
```

#### 2. Add Rate Limiting

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

#### 3. Configure CORS

```rust
use tower_http::cors::{CorsLayer, Any};

let cors = CorsLayer::new()
    .allow_origin("https://mockforge.example.com".parse::<HeaderValue>()?)
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers(Any);

let admin_router = admin_router.layer(cors);
```

#### 4. Add HTTPS

Use a reverse proxy (Nginx, Caddy) or configure TLS in Axum:

```rust
use axum_server::tls_rustls::RustlsConfig;

let config = RustlsConfig::from_pem_file(
    "cert.pem",
    "key.pem"
).await?;

axum_server::bind_rustls(addr, config)
    .serve(admin_router.into_make_service())
    .await?;
```

### Performance Optimization

#### 1. Enable Compression

```rust
use tower_http::compression::CompressionLayer;

let admin_router = admin_router
    .layer(CompressionLayer::new());
```

#### 2. Add Caching Headers

```rust
use tower_http::set_header::SetResponseHeaderLayer;

let admin_router = admin_router
    .nest_service("/ui", ServeDir::new("dist"))
    .layer(SetResponseHeaderLayer::if_not_present(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000"),
    ));
```

#### 3. Use CDN (Optional)

Upload `dist/` folder to a CDN:

```bash
# AWS S3 + CloudFront
aws s3 sync ui-builder/frontend/dist/ s3://your-bucket/ui/
aws cloudfront create-invalidation --distribution-id XXXXX --paths "/*"

# Update API base URL in frontend
# Set VITE_API_URL environment variable before building
```

## Monitoring

### Health Check

Add a health endpoint:

```rust
async fn health() -> &'static str {
    "OK"
}

let admin_router = admin_router
    .route("/health", get(health));
```

Test:
```bash
curl http://localhost:9080/__mockforge/health
```

### Metrics

The UI Builder automatically tracks:
- Endpoint CRUD operations
- Configuration exports/imports
- API response times

Access metrics at your Prometheus endpoint (if configured in MockForge).

### Logging

UI Builder uses `tracing` for logging. Set log level:

```bash
export RUST_LOG=mockforge_http::ui_builder=debug
```

View logs:
```bash
# With timestamps
mockforge serve --admin-enabled 2>&1 | ts

# Filter UI Builder logs
mockforge serve --admin-enabled 2>&1 | grep ui_builder
```

## Testing Deployment

### 1. Test API Endpoints

```bash
# Health check
curl http://localhost:9080/__mockforge/health

# List endpoints
curl http://localhost:9080/__mockforge/ui-builder/endpoints

# Create test endpoint
curl -X POST http://localhost:9080/__mockforge/ui-builder/endpoints \
  -H "Content-Type: application/json" \
  -d '{"id":"","protocol":"http","name":"Test","enabled":true,"config":{"type":"Http","method":"GET","path":"/test","response":{"status":200,"body":{"type":"Static","content":{"test":true}}}}}'
```

### 2. Test Web UI

1. Open browser: `http://localhost:9080/ui/`
2. Should see MockForge UI Builder dashboard
3. Click "New Endpoint"
4. Create a test endpoint
5. Verify it appears in the list

### 3. Load Testing

```bash
# Install Apache Bench
sudo apt-get install apache2-utils

# Test API
ab -n 1000 -c 10 http://localhost:9080/__mockforge/ui-builder/endpoints

# Test UI static files
ab -n 1000 -c 10 http://localhost:9080/ui/
```

## Troubleshooting

### UI shows blank page

1. Check browser console for errors
2. Verify API base URL is correct
3. Check CORS configuration
4. Clear browser cache (Ctrl+Shift+R)

### API returns 404

1. Verify admin server is running:
   ```bash
   curl http://localhost:9080/__mockforge/health
   ```
2. Check router mounting:
   - Is `management_router_with_ui_builder` used?
   - Is it nested under `/__mockforge`?

### Static files not loading

1. Check path exists:
   ```bash
   ls -la ui-builder/frontend/dist/
   ```
2. Verify permissions:
   ```bash
   chmod -R 755 ui-builder/frontend/dist/
   ```
3. Check server logs for errors

### Build fails

1. Clear node_modules and reinstall:
   ```bash
   rm -rf node_modules package-lock.json
   npm install
   ```
2. Check Node.js version: `node --version` (need 18+)
3. Check for TypeScript errors: `npm run type-check`

## Rollback

If something goes wrong:

1. **Stop the server**
   ```bash
   pkill -f mockforge
   ```

2. **Restore previous version**
   ```bash
   git checkout main
   cargo build --release
   ```

3. **Use old management router**
   ```rust
   let admin_router = management_router(state); // Without UI Builder
   ```

4. **Restart**
   ```bash
   mockforge serve --admin-enabled
   ```

## Upgrading

When upgrading the UI Builder:

1. **Pull latest changes**
   ```bash
   git pull origin main
   ```

2. **Rebuild frontend**
   ```bash
   cd ui-builder/frontend
   npm install
   npm run build
   ```

3. **Rebuild backend**
   ```bash
   cargo build --release
   ```

4. **Restart server**
   ```bash
   systemctl restart mockforge  # If using systemd
   ```

## Systemd Service (Linux)

Create `/etc/systemd/system/mockforge.service`:

```ini
[Unit]
Description=MockForge Server with UI Builder
After=network.target

[Service]
Type=simple
User=mockforge
Group=mockforge
WorkingDirectory=/opt/mockforge
Environment="RUST_LOG=info"
Environment="MOCKFORGE_ADMIN_ENABLED=true"
Environment="MOCKFORGE_UI_BUILDER_ENABLED=true"
ExecStart=/usr/local/bin/mockforge serve --admin-enabled
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable mockforge
sudo systemctl start mockforge
sudo systemctl status mockforge
```

View logs:
```bash
sudo journalctl -u mockforge -f
```

## Support

- Documentation: [README.md](README.md)
- Integration: [INTEGRATION_GUIDE.md](INTEGRATION_GUIDE.md)
- Architecture: [ARCHITECTURE.md](ARCHITECTURE.md)
- Issues: [GitHub Issues](https://github.com/mockforge/mockforge/issues)

---

**Deployment Status**: ✅ Ready for Production
**Last Updated**: October 2025
