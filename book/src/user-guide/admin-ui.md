# Admin UI

![MockForge Logo](../../assets/mockforge-logo.png)

MockForge Admin UI is a modern React-based dashboard that provides comprehensive administrative capabilities for your MockForge instances. Built with Shadcn UI components and designed for power users, it eliminates the need for manual file editing while providing enhanced functionality and user experience.

## Overview

The Admin UI replaces the legacy static HTML interface with a rich, interactive React application that offers:

- **Service Management**: Enable/disable services and routes with granular control
- **Fixture Management**: Visual editing, diffing, and organization of mock data
- **Live Monitoring**: Real-time logs and performance metrics
- **Authentication**: Secure role-based access control
- **Advanced Search**: Full-text search across services, fixtures, and logs
- **Bulk Operations**: Manage multiple services simultaneously

## Getting Started

### Enabling the Admin UI

The Admin UI is enabled by default when starting MockForge with the admin interface:

```bash
mockforge serve --admin-ui
```

Access the interface at `http://localhost:9080/admin` (or your configured admin port).

### Authentication

The Admin UI includes secure authentication with three built-in roles:

#### Admin Role
- **Username**: `admin`
- **Password**: `admin123`
- **Permissions**: Full access to all features

#### Editor Role
- **Username**: `editor`
- **Password**: `editor123`
- **Permissions**: Create/edit fixtures, routes, and configuration (no user
  management)

#### Viewer Role
- **Username**: `viewer`
- **Password**: `viewer123`
- **Permissions**: Read-only access to dashboard, logs, and metrics

### First Login

1. Navigate to the admin URL
2. Enter your credentials or click "Demo Admin" for quick access
3. The interface will load with role-appropriate navigation

## Core Features

### Dashboard

The dashboard provides an overview of your MockForge instance:

- **System Status**: CPU, memory usage, uptime, and active threads
- **Server Status**: HTTP, WebSocket, and gRPC server health
- **Recent Requests**: Latest API calls with response times and status codes
- **Quick Stats**: Total routes, fixtures, and active connections

### Service Management

Manage your mock services without editing configuration files:

#### Service Controls
- **Service Toggle**: Enable/disable entire services
- **Route Toggle**: Granular control over individual endpoints
- **Bulk Operations**: Enable/disable multiple services at once
- **Tag Filtering**: Filter services by tags for organized management

#### Service Information
- Request counts and error rates per route
- Response time averages
- HTTP method indicators (GET, POST, PUT, DELETE)
- gRPC service paths

```typescript
// Example: Toggle a service programmatically
const { updateService } = useServiceStore();
updateService('user-service', { enabled: false });
```

### Fixture Management

Complete fixture lifecycle management through the web interface:

#### File Operations
- **Tree View**: Hierarchical organization of fixture files
- **Drag & Drop**: Move fixtures between folders
- **Inline Rename**: Click to edit fixture names
- **Rich Editor**: Monaco-style editing with syntax highlighting

#### Content Management
- **Real-time Editing**: Live preview of fixture content
- **Version Control**: Track changes with version numbers
- **Auto-save**: Ctrl+S keyboard shortcut for quick saves
- **File Metadata**: Size, modification dates, and route associations

#### Visual Diff
- **Change Detection**: Automatic diff generation on content changes
- **Side-by-side View**: Color-coded comparison of old vs new content
- **Change Statistics**: Count of added, removed, and modified lines
- **Diff History**: Review previous changes with timestamps

### Live Logs

Monitor your MockForge instance in real-time:

#### Log Streaming
- **Real-time Updates**: Live log feed with configurable refresh intervals
- **Auto-scroll**: Smart scrolling with pause/resume controls
- **Connection Status**: Visual indicators for WebSocket health

#### Advanced Filtering
- **Method Filter**: Filter by HTTP methods (GET, POST, etc.)
- **Status Code Filter**: Focus on specific response codes
- **Path Search**: Full-text search across request paths
- **Time Range**: Filter logs by time windows (1h, 6h, 24h, 7d)

#### Log Details
- **Request Inspection**: Click any log entry for detailed view
- **Headers & Timing**: Complete request/response metadata
- **Error Analysis**: Detailed error messages and stack traces
- **Export Options**: Download filtered logs for analysis

### Performance Metrics

Comprehensive performance monitoring and analysis:

#### Latency Analysis
- **Histogram Visualization**: Response time distribution across buckets
- **Percentile Metrics**: P50, P95, and P99 latency measurements
- **Service Comparison**: Compare performance across different services
- **Color-coded Buckets**: Visual indicators for fast (green), medium (yellow), and slow (red) responses

#### Failure Analysis
- **Success/Failure Ratios**: Pie chart visualization of request outcomes
- **Status Code Distribution**: Bar chart of HTTP response codes
- **Error Rate Tracking**: Percentage of failed requests over time
- **SLA Monitoring**: Visual indicators for SLA compliance

#### Real-time Updates
- **Auto-refresh**: Metrics update every 30 seconds
- **Manual Refresh**: Force immediate data refresh
- **Performance Alerts**: Automatic warnings for high error rates or latency

## Advanced Features

The Admin UI provides access to many advanced MockForge features:

- **Chaos Lab**: Interactive network condition simulation with real-time latency visualization
- **Reality Slider**: Unified control for adjusting mock environment realism
- **Scenario State Machine Editor**: Visual flow editor for creating state machines
- **Time Travel Controls**: Virtual clock controls for temporal simulation
- **Contract Diff Dashboard**: Visualize and analyze API contract mismatches
- **Voice Interface**: Create APIs using natural language commands

For detailed documentation on these features, see the [Advanced Features](advanced-features.md) section.

### Authentication & Authorization

#### JWT-based Security
- **Token Authentication**: Secure JWT tokens with automatic refresh
- **Session Persistence**: Login state survives browser refresh
- **Auto-logout**: Automatic logout on token expiration

#### Role-based Access Control
- **Admin Features**: Full read/write access to all functionality
- **Viewer Restrictions**: Read-only access to monitoring features
- **Navigation Adaptation**: Menu items adjust based on user role
- **Permission Guards**: Graceful handling of unauthorized access

### Search & Filtering

#### Global Search
- **Service Search**: Find services by name, route paths, or tags
- **Fixture Search**: Search fixture names, paths, and content
- **Log Search**: Full-text search across log messages and metadata

#### Advanced Filters
- **Tag-based Filtering**: Group services by functional tags
- **Time-based Filtering**: Filter data by time ranges
- **Status Filtering**: Focus on specific response codes or error states
- **Persistent Filters**: Maintain filter state across navigation

### Bulk Operations

#### Service Management
```bash
# Enable all services in a tag group
services.filter(s => s.tags.includes('api'))
  .forEach(s => updateService(s.id, { enabled: true }));
```

#### Fixture Operations
- **Batch Selection**: Select multiple fixtures for operations
- **Bulk Rename**: Apply naming patterns to multiple files
- **Mass Delete**: Remove multiple fixtures with confirmation

### Validation Management

The Admin UI provides comprehensive validation controls for OpenAPI request validation:

#### Validation Mode Control
- **Global Mode Toggle**: Switch between `off`, `warn`, and `enforce` validation modes
- **Per-Route Overrides**: Set custom validation rules for specific endpoints
- **Real-time Application**: Changes take effect immediately without server restart

#### Validation Monitoring
- **Error Statistics**: View validation failure rates and error types
- **Route-specific Metrics**: See which endpoints are failing validation
- **Error Details**: Inspect detailed validation error messages

#### Advanced Validation Features
- **Aggregate Error Reporting**: Combine multiple validation errors into single responses
- **Response Validation**: Validate response payloads against OpenAPI schemas
- **Admin Route Exclusion**: Skip validation for admin UI routes when configured

```typescript
// Example: Update validation mode programmatically
const { updateValidation } = useValidationStore();
updateValidation({
  mode: 'warn',
  aggregate_errors: true,
  overrides: {
    'GET /health': 'off',
    'POST /api/users': 'enforce'
  }
});
```

## Configuration

### Environment Variables

```bash
# Admin UI port (default: 9080)
MOCKFORGE_ADMIN_PORT=9080

# Enable the Admin UI
MOCKFORGE_ADMIN_ENABLED=true
```

Auth, session timeout, JWT secrets, and resource limits are YAML-only —
configure under `admin.*` in `mockforge.yaml`.

### Custom Authentication

Replace the default authentication with your own system:

```rust
// Custom auth provider
pub struct CustomAuthProvider {
    // Your authentication implementation
}

impl AuthProvider for CustomAuthProvider {
    fn authenticate(&self, username: &str, password: &str) -> Result<User> {
        // Your authentication logic
    }
}
```

### Theming

The Admin UI supports light and dark themes with CSS custom properties:

```css
:root {
  --background: 0 0% 100%;
  --foreground: 222.2 84% 4.9%;
  --primary: 221.2 83.2% 53.3%;
  /* ... additional theme variables */
}

.dark {
  --background: 222.2 84% 4.9%;
  --foreground: 210 40% 98%;
  /* ... dark theme overrides */
}
```

## API Integration

### REST Endpoints

The Admin UI communicates with MockForge through RESTful APIs:

The embedded admin server exposes its API under `/__mockforge/*` (the
hosted cloud registry uses a different `/api/v1/*` prefix; see the Cloud
Workspaces guide for that surface).

```http
# Service & route listing
GET    /__mockforge/routes
GET    /__mockforge/server-info

# Fixture management
GET    /__mockforge/fixtures
POST   /__mockforge/fixtures
PUT    /__mockforge/fixtures/{id}
DELETE /__mockforge/fixtures/{id}

# Authentication
POST   /__mockforge/auth/login
POST   /__mockforge/auth/refresh
POST   /__mockforge/auth/logout
GET    /__mockforge/auth/me

# Logs and metrics
GET    /__mockforge/logs
GET    /__mockforge/metrics
```

### Server-Sent Events

Real-time features stream over SSE:

```http
# Live log streaming
GET /__mockforge/logs/sse
```

## Troubleshooting

### Common Issues

#### Authentication Problems
```bash
# Verify admin credentials against the embedded admin server
curl -X POST http://localhost:9080/__mockforge/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}'
```

If the curl above returns a token but the browser login page still fails,
the UI bundle was built in *cloud* mode. The OSS docker image must ship a
*local* bundle — see `crates/mockforge-ui/ui/.env.production.example`;
`.env.production` is gitignored on purpose.

#### Log Stream Connection Issues
```bash
# Check the log SSE endpoint
curl -N http://localhost:9080/__mockforge/logs/sse

# Verify proxy configuration if behind a reverse proxy
ProxyPass /__mockforge/ http://localhost:9080/__mockforge/
```

#### Performance Issues

For large fixture sets / high-traffic dashboards, enable Prometheus
metrics with `--metrics --metrics-port 9090` to track admin server
overhead. Memory tuning is YAML-only (`admin.memory_limit_mb`).

### Debug Mode

Enable debug logging for troubleshooting:

```bash
MOCKFORGE_LOG_LEVEL=debug mockforge serve --admin-ui
```

### Browser Compatibility

The Admin UI requires modern browsers with support for:
- ES2020 features
- WebSocket API
- CSS Grid and Flexbox
- Local Storage

## Best Practices

### Security
- Change default admin credentials in production
- Use HTTPS for admin interface in production
- Configure appropriate session timeouts
- Regularly rotate JWT secrets

### Performance
- Use filtering to limit large datasets
- Enable auto-scroll only when monitoring actively
- Clear old logs periodically to improve performance
- Monitor memory usage with large fixture files

### Organization
- Use descriptive service and fixture names
- Organize fixtures in logical folder structures
- Apply consistent tagging to services
- Document fixture purposes in comments

## Examples

### Service Management Workflow

```typescript
// 1. Filter services by tag
const apiServices = services.filter(s => s.tags.includes('api'));

// 2. Enable all API services
apiServices.forEach(service => {
  updateService(service.id, { enabled: true });
});

// 3. Disable specific routes within services
apiServices.forEach(service => {
  service.routes
    .filter(route => route.path.includes('/internal'))
    .forEach(route => {
      const routeId = `${route.method}-${route.path}`;
      toggleRoute(service.id, routeId, false);
    });
});
```

### Fixture Management Workflow

```typescript
// 1. Create new fixture
const newFixture = {
  id: 'user-profile-success',
  name: 'user-profile.json',
  path: 'http/get/users/profile/user-profile.json',
  content: JSON.stringify({
    id: '{{uuid}}',
    name: '{{faker.name.fullName}}',
    email: '{{faker.internet.email}}',
    created_at: '{{now}}'
  }, null, 2)
};

// 2. Add to store
addFixture(newFixture);

// 3. Associate with route
updateFixture(newFixture.id, {
  ...newFixture.content,
  route_path: '/api/users/profile',
  method: 'GET'
});
```

This comprehensive guide covers all aspects of the MockForge Admin UI, from basic usage to advanced configuration and troubleshooting. The interface provides a complete administrative solution that eliminates the need for manual file editing while offering enhanced functionality and user experience.
