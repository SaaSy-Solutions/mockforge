# MockForge Admin UI v2 CLI Integration Plan

## Current Status

✅ **Completed**: React Admin UI v2 with all features
❌ **Pending**: Integration with MockForge CLI binary

## Integration Steps Required

### 1. Build Process Integration
```bash
# Add to MockForge build pipeline
cd ui-v2
npm run build
# Output: ui-v2/dist/ contains static assets
```

### 2. Rust Backend Updates
Replace the current static HTML admin with React build output:

```rust
// In crates/mockforge-ui/src/lib.rs
pub fn get_admin_html() -> &'static str {
    // Replace with React build
    include_str!("../ui-v2/dist/index.html")
}

pub fn get_admin_css() -> &'static str {
    // Include compiled CSS
    include_str!("../ui-v2/dist/assets/index-[hash].css")
}

pub fn get_admin_js() -> &'static str {
    // Include compiled JS
    include_str!("../ui-v2/dist/assets/index-[hash].js")
}
```

### 3. API Endpoint Implementation
The React app expects these endpoints (currently mocked):

```rust
// New REST endpoints needed in mockforge-ui
GET    /api/v2/services           // List services with toggle states
PUT    /api/v2/services/{id}      // Update service configuration
POST   /api/v2/services/bulk      // Bulk service operations

GET    /api/v2/fixtures           // List fixture files
PUT    /api/v2/fixtures/{id}      // Update fixture content
POST   /api/v2/fixtures/move      // Move/rename fixtures
GET    /api/v2/fixtures/{id}/diff // Generate fixture diffs

POST   /api/v2/auth/login         // JWT authentication
POST   /api/v2/auth/refresh       // Token refresh
POST   /api/v2/auth/logout        // Session management

GET    /api/v2/logs               // Get log entries with filtering
WS     /api/v2/logs/stream        // WebSocket for live logs
GET    /api/v2/metrics/latency    // Performance metrics
GET    /api/v2/metrics/failures   // Error analysis
```

### 4. WebSocket Integration
Add WebSocket support for real-time features:

```rust
// WebSocket endpoints for live updates
WS /api/v2/logs/stream           // Live log streaming
WS /api/v2/metrics/stream        // Live metrics updates
WS /api/v2/config/stream         // Configuration changes
```

### 5. Authentication System
Implement JWT-based authentication:

```rust
// Authentication middleware
pub struct AuthMiddleware {
    jwt_secret: String,
}

impl AuthMiddleware {
    pub fn verify_token(&self, token: &str) -> Result<User, AuthError> {
        // JWT token validation
    }
}
```

## Integration Effort

### Backend Development (Rust)
- **API Endpoints**: ~2-3 days to implement all REST endpoints
- **WebSocket Support**: ~1-2 days for real-time features
- **Authentication**: ~1 day for JWT implementation
- **File Operations**: ~1 day for fixture management
- **Testing**: ~1 day for integration tests

### Build Pipeline
- **Asset Embedding**: ~0.5 days to embed React build output
- **Build Scripts**: ~0.5 days to automate build process
- **CI/CD Updates**: ~0.5 days to update automation

**Total Estimated Effort**: ~6-8 days of development

## Current Workaround

For immediate testing, you can run both separately:

```bash
# Terminal 1: Start MockForge CLI
mockforge serve --admin-ui=false

# Terminal 2: Start React dev server
cd ui-v2
npm run dev
# Access at http://localhost:5173
```
