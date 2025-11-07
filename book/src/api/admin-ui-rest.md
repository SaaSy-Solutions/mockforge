# Admin UI REST API

This document provides comprehensive documentation for the MockForge Admin UI REST API endpoints.

## Overview

The MockForge Admin UI provides a web-based interface for managing and monitoring MockForge servers. The API is organized around the following main areas:

- **Dashboard**: System overview and real-time metrics
- **Server Management**: Control and monitor server instances
- **Configuration**: Update latency, faults, proxy, and validation settings
- **Logging**: View and filter request logs
- **Metrics**: Performance monitoring and analytics
- **Fixtures**: Manage mock data and fixtures
- **Environment**: Environment variable management

## Base URL

All API endpoints are prefixed with `/__mockforge/api` to avoid conflicts with user-defined routes.

### Standalone Mode vs Embedded Mode

The REST API works identically in both standalone and embedded modes:

**Standalone Mode (Default):**
- Admin UI runs on a separate port (default: 9080)
- REST API endpoints available at: `http://localhost:9080/__mockforge/api/*`
- Main HTTP server runs on port 3000 (or configured port)
- Example: `curl http://localhost:9080/__mockforge/api/mocks`

**Embedded Mode:**
- Admin UI mounted under HTTP server (e.g., `/admin`)
- REST API endpoints available at: `http://localhost:3000/__mockforge/api/*`
- Same endpoints, different base URL
- Example: `curl http://localhost:3000/__mockforge/api/mocks`

**Configuration via REST API (JSON over HTTP):**

The REST API supports full configuration management via JSON over HTTP, making it suitable for:
- CI/CD pipelines
- Automated testing
- Remote configuration
- Integration with external tools

All endpoints accept and return JSON, following standard REST conventions.

### Standalone Mode Examples

**Starting MockForge in Standalone Mode:**
```bash
# Start MockForge with standalone admin UI
mockforge serve --admin --admin-standalone --admin-port 9080

# Or via config file
# admin:
#   enabled: true
#   port: 9080
#   api_enabled: true
```

**Creating a Mock via REST API (Standalone Mode):**
```bash
# Create a mock using JSON over HTTP
curl -X POST http://localhost:9080/__mockforge/api/mocks \
  -H "Content-Type: application/json" \
  -d '{
    "id": "user-get",
    "name": "Get User",
    "method": "GET",
    "path": "/api/users/{id}",
    "response": {
      "body": {
        "id": "{{request.path.id}}",
        "name": "Alice",
        "email": "alice@example.com"
      },
      "headers": {
        "Content-Type": "application/json"
      }
    },
    "enabled": true,
    "status_code": 200
  }'
```

**Updating Configuration via REST API:**
```bash
# Update latency configuration
curl -X POST http://localhost:9080/__mockforge/api/config/latency \
  -H "Content-Type: application/json" \
  -d '{
    "base_ms": 100,
    "jitter_ms": 50
  }'
```

**Listing All Mocks:**
```bash
# Get all configured mocks
curl http://localhost:9080/__mockforge/api/mocks
```

**Using the SDK with Standalone Mode:**
```rust
use mockforge_sdk::AdminClient;
use mockforge_sdk::MockConfigBuilder;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to standalone admin API
    let client = AdminClient::new("http://localhost:9080");
    
    // Create a mock using the fluent builder API
    let mock = MockConfigBuilder::new("POST", "/api/users")
        .name("Create User")
        .with_header("Authorization", "Bearer.*")
        .with_query_param("role", "admin")
        .status(201)
        .body(json!({
            "id": "{{uuid}}",
            "name": "{{faker.name}}",
            "created": true
        }))
        .priority(10)
        .build();
    
    // Create the mock via REST API
    client.create_mock(mock).await?;
    
    Ok(())
}
```

## Authentication

Currently, the API does not implement authentication. In production deployments, consider adding authentication middleware.

## Response Format

All API responses follow a consistent format:

```json
{
  "success": boolean,
  "data": object | array | null,
  "error": string | null,
  "timestamp": string
}
```

### Success Response
```json
{
  "success": true,
  "data": { ... },
  "error": null,
  "timestamp": "2025-09-17T10:30:00Z"
}
```

### Error Response
```json
{
  "success": false,
  "data": null,
  "error": "Error message",
  "timestamp": "2025-09-17T10:30:00Z"
}
```

## API Endpoints

### Dashboard

#### GET `/__mockforge/dashboard`

Get comprehensive dashboard data including system information, server status, routes, and recent logs.

**Response:**
```json
{
  "success": true,
  "data": {
    "system": {
      "version": "0.1.0",
      "uptime_seconds": 3600,
      "memory_usage_mb": 128,
      "cpu_usage_percent": 15.5,
      "active_threads": 8,
      "total_routes": 25,
      "total_fixtures": 150
    },
    "servers": [
      {
        "server_type": "HTTP",
        "address": "127.0.0.1:3000",
        "running": true,
        "start_time": "2025-09-17T09:30:00Z",
        "uptime_seconds": 3600,
        "active_connections": 5,
        "total_requests": 1250
      }
    ],
    "routes": [
      {
        "method": "GET",
        "path": "/api/users",
        "priority": 0,
        "has_fixtures": true,
        "latency_ms": 45,
        "request_count": 125,
        "last_request": "2025-09-17T10:25:00Z",
        "error_count": 2
      }
    ],
    "recent_logs": [
      {
        "id": "log_1",
        "timestamp": "2025-09-17T10:29:00Z",
        "method": "GET",
        "path": "/api/users",
        "status_code": 200,
        "response_time_ms": 45,
        "client_ip": "127.0.0.1",
        "user_agent": "test-agent",
        "headers": {},
        "response_size_bytes": 1024,
        "error_message": null
      }
    ],
    "latency_profile": {
      "name": "default",
      "base_ms": 50,
      "jitter_ms": 20,
      "tag_overrides": {}
    },
    "fault_config": {
      "enabled": false,
      "failure_rate": 0.0,
      "status_codes": [500],
      "active_failures": 0
    },
    "proxy_config": {
      "enabled": false,
      "upstream_url": null,
      "timeout_seconds": 30,
      "requests_proxied": 0
    }
  }
}
```

### Health Check

#### GET `/__mockforge/health`

Get system health status.

**Response:**
```json
{
  "status": "healthy",
  "services": {
    "http": "healthy",
    "websocket": "healthy",
    "grpc": "healthy"
  },
  "last_check": "2025-09-17T10:30:00Z",
  "issues": []
}
```

### Server Management

#### GET `/__mockforge/server-info`

Get information about server addresses and configuration.

**Response:**
```json
{
  "success": true,
  "data": {
    "http_server": "127.0.0.1:3000",
    "ws_server": "127.0.0.1:3001",
    "grpc_server": "127.0.0.1:50051"
  }
}
```

#### POST `/__mockforge/servers/restart`

Initiate server restart.

**Request Body:**
```json
{
  "reason": "Manual restart requested"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "message": "Server restart initiated. Please wait for completion."
  }
}
```

#### GET `/__mockforge/servers/restart/status`

Get restart status.

**Response:**
```json
{
  "success": true,
  "data": {
    "in_progress": false,
    "initiated_at": null,
    "reason": null,
    "success": null
  }
}
```

### Routes

#### GET `/__mockforge/routes`

Get information about configured routes (proxied to HTTP server).

### Logs

#### GET `/__mockforge/logs`

Get request logs with optional filtering.

**Query Parameters:**
- `method` (string): Filter by HTTP method
- `path` (string): Filter by path pattern
- `status` (number): Filter by status code
- `limit` (number): Maximum number of results

**Examples:**
```
GET /__mockforge/logs?method=GET&limit=50
GET /__mockforge/logs?path=/api/users&status=200
```

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": "log_1",
      "timestamp": "2025-09-17T10:29:00Z",
      "method": "GET",
      "path": "/api/users",
      "status_code": 200,
      "response_time_ms": 45,
      "client_ip": "127.0.0.1",
      "user_agent": "test-agent",
      "headers": {},
      "response_size_bytes": 1024,
      "error_message": null
    }
  ]
}
```

#### POST `/__mockforge/logs/clear`

Clear all request logs.

**Response:**
```json
{
  "success": true,
  "data": {
    "message": "Logs cleared"
  }
}
```

### Metrics

#### GET `/__mockforge/metrics`

Get performance metrics and analytics.

**Response:**
```json
{
  "success": true,
  "data": {
    "requests_by_endpoint": {
      "GET /api/users": 125,
      "POST /api/users": 45
    },
    "response_time_percentiles": {
      "p50": 45,
      "p95": 120,
      "p99": 250
    },
    "error_rate_by_endpoint": {
      "GET /api/users": 0.02,
      "POST /api/users": 0.0
    },
    "memory_usage_over_time": [
      ["2025-09-17T10:25:00Z", 120],
      ["2025-09-17T10:26:00Z", 125]
    ],
    "cpu_usage_over_time": [
      ["2025-09-17T10:25:00Z", 12.5],
      ["2025-09-17T10:26:00Z", 15.2]
    ]
  }
}
```

### Configuration

#### GET `/__mockforge/config`

Get current configuration settings.

**Response:**
```json
{
  "success": true,
  "data": {
    "latency": {
      "enabled": true,
      "base_ms": 50,
      "jitter_ms": 20,
      "tag_overrides": {}
    },
    "faults": {
      "enabled": false,
      "failure_rate": 0.0,
      "status_codes": [500, 502, 503]
    },
    "proxy": {
      "enabled": false,
      "upstream_url": null,
      "timeout_seconds": 30
    },
    "validation": {
      "mode": "enforce",
      "aggregate_errors": true,
      "validate_responses": false,
      "overrides": {}
    }
  }
}
```

#### POST `/__mockforge/config/latency`

Update latency configuration.

**Request Body:**
```json
{
  "config_type": "latency",
  "data": {
    "base_ms": 100,
    "jitter_ms": 50,
    "tag_overrides": {
      "auth": 200
    }
  }
}
```

#### POST `/__mockforge/config/faults`

Update fault injection configuration.

**Request Body:**
```json
{
  "config_type": "faults",
  "data": {
    "enabled": true,
    "failure_rate": 0.1,
    "status_codes": [500, 502, 503]
  }
}
```

#### POST `/__mockforge/config/proxy`

Update proxy configuration.

**Request Body:**
```json
{
  "config_type": "proxy",
  "data": {
    "enabled": true,
    "upstream_url": "http://api.example.com",
    "timeout_seconds": 60
  }
}
```

#### POST `/__mockforge/validation`

Update validation settings.

**Request Body:**
```json
{
  "mode": "warn",
  "aggregate_errors": false,
  "validate_responses": true,
  "overrides": {
    "GET /health": "off"
  }
}
```

### Environment Variables

#### GET `/__mockforge/env`

Get relevant environment variables.

**Response:**
```json
{
  "success": true,
  "data": {
    "MOCKFORGE_LATENCY_ENABLED": "true",
    "MOCKFORGE_HTTP_PORT": "3000",
    "RUST_LOG": "info"
  }
}
```

#### POST `/__mockforge/env`

Update an environment variable (runtime only).

**Request Body:**
```json
{
  "key": "MOCKFORGE_LOG_LEVEL",
  "value": "debug"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "message": "Environment variable MOCKFORGE_LOG_LEVEL updated to 'debug'. Note: This change is not persisted and will be lost on restart."
  }
}
```

### Files

#### POST `/__mockforge/files/content`

Get file content.

**Request Body:**
```json
{
  "file_path": "config.yaml",
  "file_type": "yaml"
}
```

**Response:**
```json
{
  "success": true,
  "data": "http:\n  request_validation: \"enforce\"\n  aggregate_validation_errors: true\n"
}
```

#### POST `/__mockforge/files/save`

Save file content.

**Request Body:**
```json
{
  "file_path": "config.yaml",
  "content": "http:\n  port: 9080\n"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "message": "File saved successfully"
  }
}
```

### Fixtures

#### GET `/__mockforge/fixtures`

Get all fixtures with metadata.

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": "fixture_123",
      "protocol": "http",
      "method": "GET",
      "path": "/api/users",
      "saved_at": "2025-09-17T09:00:00Z",
      "file_size": 2048,
      "file_path": "http/get/api_users_123.json",
      "fingerprint": "abc123",
      "metadata": { ... }
    }
  ]
}
```

#### POST `/__mockforge/fixtures/delete`

Delete a fixture.

**Request Body:**
```json
{
  "fixture_id": "fixture_123"
}
```

#### POST `/__mockforge/fixtures/delete-bulk`

Delete multiple fixtures.

**Request Body:**
```json
{
  "fixture_ids": ["fixture_123", "fixture_456"]
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "deleted_count": 2,
    "total_requested": 2,
    "errors": []
  }
}
```

#### GET `/__mockforge/fixtures/download?id=fixture_123`

Download a fixture file.

**Response:** Binary file download

### Smoke Tests

#### GET `/__mockforge/smoke`

Get smoke test results.

#### GET `/__mockforge/smoke/run`

Run smoke tests against fixtures.

**Response:**
```json
{
  "success": true,
  "data": {
    "message": "Smoke tests started. Check results in the smoke tests section."
  }
}
```

## Error Codes

### HTTP Status Codes

- `200 OK`: Success
- `400 Bad Request`: Invalid request parameters
- `404 Not Found`: Endpoint or resource not found
- `500 Internal Server Error`: Server error

### Common Error Messages

- `"Invalid config type"`: Configuration update with invalid type
- `"Failed to load fixtures"`: Error reading fixture files
- `"Path traversal detected"`: Security violation in file paths
- `"Server restart already in progress"`: Attempted restart while one is running

## Rate Limiting

Currently, no rate limiting is implemented. Consider adding rate limiting for production deployments.

## CORS

The API includes CORS middleware allowing cross-origin requests from web applications.

## WebSocket Support

The admin UI supports real-time updates through WebSocket connections for live monitoring of metrics and logs.

## Examples

### Complete Dashboard Fetch
```javascript
const response = await fetch('/__mockforge/dashboard');
const data = await response.json();

if (data.success) {
  console.log('System uptime:', data.data.system.uptime_seconds);
  console.log('Active servers:', data.data.servers.length);
}
```

### Update Latency Configuration
```javascript
const response = await fetch('/__mockforge/config/latency', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    config_type: 'latency',
    data: {
      base_ms: 100,
      jitter_ms: 25
    }
  })
});

const result = await response.json();
console.log(result.data.message);
```

### Filter Logs
```javascript
const response = await fetch('/__mockforge/logs?method=GET&status=200&limit=100');
const data = await response.json();

data.data.forEach(log => {
  console.log(`${log.method} ${log.path} - ${log.status_code} (${log.response_time_ms}ms)`);
});
```

## Development

### Running Tests
```bash
# Run all tests
cargo test --package mockforge-ui

# Run integration tests
cargo test --package mockforge-ui --test integration

# Run smoke tests
cargo test --package mockforge-ui --test smoke
```

### Building Documentation
```bash
# Generate API documentation
cargo doc --package mockforge-ui --open
```

## Security Considerations

1. **Input Validation**: All inputs should be validated
2. **Path Traversal**: File operations prevent directory traversal
3. **Rate Limiting**: Consider implementing rate limiting
4. **Authentication**: Add authentication for production use
5. **HTTPS**: Use HTTPS in production
6. **CORS**: Properly configure CORS policies

## Contributing

When adding new API endpoints:
1. Follow the established response format
2. Add comprehensive error handling
3. Include integration tests
4. Update this documentation
5. Ensure proper CORS and security measures
