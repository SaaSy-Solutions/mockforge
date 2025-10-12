# Admin UI

MockForge provides a comprehensive web-based Admin UI for managing and monitoring your mock servers. The Admin UI offers real-time insights, configuration management, and debugging tools to make mock server management effortless.

## Accessing the Admin UI

The Admin UI is automatically available when you start MockForge with the `--admin` flag:

```bash
# Start with Admin UI enabled
mockforge serve --spec api-spec.json --admin --admin-port 9080 --http-port 3000
```

The Admin UI will be available at: **http://localhost:9080** (default port)

### Configuration Options

```bash
# Custom admin port
mockforge serve --admin --admin-port 9090

# Disable admin UI (default)
mockforge serve --spec api-spec.json --no-admin
```

### Environment Variables

```bash
# Enable/disable admin UI
MOCKFORGE_ADMIN_ENABLED=true

# Set admin UI port
MOCKFORGE_ADMIN_PORT=9080

# Set admin UI bind address (default: 0.0.0.0)
MOCKFORGE_ADMIN_BIND=127.0.0.1
```

## Interface Overview

The Admin UI features a clean, modern interface with the following main sections:

### Navigation Tabs

- **Dashboard** - System overview and real-time metrics
- **Routes** - API endpoint management and testing
- **Fixtures** - Recorded request/response management
- **Logs** - Request/response logging and debugging
- **Configuration** - Runtime configuration management
- **Metrics** - Performance monitoring and analytics
- **Files** - File system access for configuration files

### Status Indicators

The header displays real-time system status:
- **● Healthy** - All systems operational
- **● Warning** - Minor issues detected
- **● Error** - Critical issues requiring attention

## Dashboard

The Dashboard provides a comprehensive overview of your MockForge instance:

### System Status
- **Uptime** - How long the server has been running
- **Memory Usage** - Current memory consumption
- **CPU Usage** - Current CPU utilization
- **Active Connections** - Number of open connections

### Recent Activity
- **Latest Requests** - Most recent API calls with timestamps
- **Response Times** - Average response latency
- **Error Rate** - Percentage of failed requests

### Quick Actions
- **Restart Server** - Gracefully restart the mock server
- **Clear Logs** - Remove all accumulated logs
- **Export Configuration** - Download current config as YAML

## Routes Management

The Routes tab provides detailed API endpoint management:

### Route Listing
- View all configured API routes
- Filter by HTTP method, path pattern, or response status
- Sort by request count, response time, or error rate

### Route Details
For each route, view:
- **Request Count** - Total requests served
- **Average Response Time** - Performance metrics
- **Success/Error Rates** - Reliability statistics
- **Recent Requests** - Last 10 requests with details

### Route Testing
- **Interactive Tester** - Send test requests directly from the UI
- **Request Builder** - Construct complex requests with headers, query params, and body
- **Response Preview** - See exactly what would be returned

### Route Overrides
- **Temporary Overrides** - Modify responses without changing configuration
- **Conditional Responses** - Set up A/B testing scenarios
- **Failure Injection** - Simulate errors for testing resilience

## Fixtures Management

The Fixtures tab manages recorded request/response pairs:

### Fixture Browser
- **Search and Filter** - Find fixtures by endpoint, method, or content
- **Categorization** - Group fixtures by API version or feature
- **Tagging** - Add custom tags for organization

### Fixture Operations
- **View Details** - Inspect request/response pairs in detail
- **Edit Responses** - Modify recorded responses
- **Export/Import** - Backup and restore fixture collections
- **Bulk Operations** - Apply changes to multiple fixtures

### Recording Controls
- **Start/Stop Recording** - Control when new fixtures are captured
- **Recording Filters** - Only record specific endpoints or request types
- **Storage Management** - Configure fixture retention and cleanup

## Logging and Debugging

The Logs tab provides comprehensive request/response monitoring:

### Log Viewer
- **Real-time Updates** - See requests as they happen
- **Filtering Options** - Filter by endpoint, status code, or time range
- **Search Functionality** - Find specific requests or responses

### Log Details
For each log entry:
- **Full Request** - Headers, body, and metadata
- **Full Response** - Status, headers, and body
- **Timing Information** - Request/response duration
- **Error Details** - Stack traces and error context

### Log Management
- **Export Logs** - Download logs in various formats
- **Log Rotation** - Automatic cleanup of old logs
- **Log Levels** - Adjust verbosity for debugging

## Configuration Management

The Configuration tab allows runtime configuration changes:

### Current Configuration
- **View Active Config** - See all current settings
- **Configuration Sources** - Understand precedence (CLI > Env > File)
- **Validation Status** - Check configuration validity

### Configuration Editor
- **Live Editing** - Modify settings without restart
- **Validation** - Real-time syntax and semantic validation
- **Change History** - Track configuration modifications

### Configuration Templates
- **Save/Load Templates** - Reuse common configurations
- **Environment Profiles** - Different configs for dev/staging/prod
- **Backup/Restore** - Version control for configurations

### Validation Management
- **Validation Mode Toggle** - Switch between off/warn/enforce modes
- **Per-Route Overrides** - Set custom validation rules for specific endpoints
- **Real-time Updates** - Apply validation changes without server restart
- **Validation Statistics** - View validation errors and success rates

## Metrics and Analytics

The Metrics tab provides detailed performance analytics:

### Performance Metrics
- **Response Time Distribution** - P50, P95, P99 latencies
- **Throughput** - Requests per second over time
- **Error Rate Trends** - Track reliability over time

### Endpoint Analytics
- **Top Endpoints** - Most frequently called routes
- **Slowest Endpoints** - Performance bottlenecks
- **Error-prone Endpoints** - Routes with high failure rates

### System Metrics
- **Resource Usage** - CPU, memory, disk over time
- **Connection Pool** - Database connection utilization
- **Cache Hit Rates** - Effectiveness of response caching

## File System Access

The Files tab provides access to configuration and data files:

### File Browser
- **Navigate Directory Structure** - Browse the file system
- **File Type Detection** - Syntax highlighting for different file types
- **Quick Access** - Bookmarks for frequently used directories

### File Editor
- **In-browser Editing** - Edit configuration files directly
- **Syntax Validation** - Catch errors before saving
- **Version Control Integration** - Commit changes with Git

### File Operations
- **Upload/Download** - Transfer files to/from the server
- **Backup Operations** - Create and restore backups
- **Permission Management** - Control file access

## Advanced Features

### Auto-Refresh
- **Configurable Intervals** - Set refresh rates from 1 second to 5 minutes
- **Smart Updates** - Only refresh when data has changed
- **Background Updates** - Continue working while data refreshes

### Keyboard Shortcuts
- **Navigation** - Tab switching with keyboard shortcuts
- **Actions** - Quick access to common operations
- **Search** - Global search across all tabs

### Themes and Customization
- **Light/Dark Mode** - Choose your preferred theme
- **Layout Options** - Customize dashboard layout
- **Color Schemes** - Personalize the interface

## Security Considerations

### Access Control
- **Authentication** - Optional login requirements
- **Authorization** - Role-based access control
- **IP Restrictions** - Limit access to specific networks

### Data Protection
- **Sensitive Data Masking** - Hide passwords and tokens in logs
- **Encryption** - Secure data transmission
- **Audit Logging** - Track all administrative actions

## Troubleshooting

### Common Issues

**Admin UI not loading**: Check that `--admin` flag is used and port 9080 is accessible

**Slow performance**: Reduce auto-refresh interval or disable real-time updates

**Missing data**: Ensure proper permissions for file system access

**Configuration not applying**: Some changes may require server restart

### Debug Tools

- **Network Inspector** - Monitor all HTTP requests
- **Console Logs** - JavaScript debugging information
- **Performance Profiler** - Identify UI performance bottlenecks

### Getting Help

- **Built-in Help** - Press `?` for keyboard shortcuts
- **Tooltips** - Hover over UI elements for explanations
- **Context Help** - Right-click for contextual help menus

The Admin UI transforms MockForge from a simple mock server into a powerful development and testing platform, providing the visibility and control needed for professional API mocking workflows.
