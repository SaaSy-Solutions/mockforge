# API Flight Recorder

MockForge's API Flight Recorder captures and stores all API interactions for analysis, debugging, replay, and testing. It provides a queryable SQLite database of requests and responses across all supported protocols (HTTP, gRPC, WebSocket, GraphQL).

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [CLI Usage](#cli-usage)
- [Management API](#management-api)
- [Recording Protocols](#recording-protocols)
- [Querying Recordings](#querying-recordings)
- [Replay Functionality](#replay-functionality)
- [HAR Export](#har-export)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

The API Flight Recorder is inspired by Java Flight Recorder and provides similar capabilities for API testing and debugging. It continuously records API traffic with minimal performance impact and stores it in a SQLite database for later analysis.

### Key Capabilities

- **Zero-Configuration Recording**: Start recording with a single CLI flag
- **Multi-Protocol Support**: HTTP, gRPC, WebSocket, and GraphQL
- **Query API**: Search recordings by protocol, method, status, duration, etc.
- **Replay Engine**: Re-execute recorded requests
- **HAR Export**: Export HTTP recordings to standard HAR format
- **Management API**: REST API for managing recordings
- **Automatic Cleanup**: Retention policies to manage database size

## Features

### Recording Features

- Capture request and response details:
  - Method, path, headers, body
  - Status code, duration
  - Client IP, trace context (trace_id, span_id)
  - Custom tags for organization
- Binary data handling (base64 encoding)
- UTF-8 text preservation
- Efficient SQLite storage with indexes

### Query Features

- Filter by:
  - Protocol (HTTP, gRPC, WebSocket, GraphQL)
  - HTTP method or gRPC service/method
  - Path patterns with wildcards
  - Status code
  - Duration range
  - Trace ID
  - Custom tags
- Pagination support
- Statistics aggregation

### Export Features

- HAR (HTTP Archive) format for HTTP requests
- Compatible with tools like Chrome DevTools, Postman, Insomnia
- Includes full request/response details
- Timing information

## Quick Start

### Enable Recording

```bash
# Start MockForge with recording enabled
mockforge serve --recorder --recorder-db ./recordings.db

# Or use environment variables
export MOCKFORGE_RECORDER_ENABLED=true
export MOCKFORGE_RECORDER_DB=./recordings.db
mockforge serve
```

### Query Recordings via Management API

```bash
# List recent requests
curl http://localhost:3000/api/recorder/requests?limit=10

# Get a specific request
curl http://localhost:3000/api/recorder/requests/{request_id}

# Export to HAR
curl http://localhost:3000/api/recorder/export/har > recordings.har

# Get statistics
curl http://localhost:3000/api/recorder/stats
```

## Configuration

### YAML Configuration

```yaml
observability:
  recorder:
    enabled: true
    database_path: "./mockforge-recordings.db"
    api_enabled: true
    api_port: null  # Use main server port
    max_requests: 10000
    retention_days: 7
    record_http: true
    record_grpc: true
    record_websocket: true
    record_graphql: true
```

### Environment Variables

```bash
MOCKFORGE_RECORDER_ENABLED=true
MOCKFORGE_RECORDER_DB=./recordings.db
MOCKFORGE_RECORDER_MAX_REQUESTS=10000
MOCKFORGE_RECORDER_RETENTION_DAYS=7
```

## CLI Usage

### Recording Flags

```bash
mockforge serve \
  --recorder \                           # Enable recording
  --recorder-db ./recordings.db \        # Database path
  --recorder-max-requests 10000 \        # Max requests to store
  --recorder-retention-days 7 \          # Auto-delete after N days
  --recorder-no-api \                    # Disable management API
  --recorder-api-port 9000               # Separate API port
```

### Examples

#### Basic Recording

```bash
# Record all protocols with defaults
mockforge serve --recorder
```

#### Production Recording

```bash
# Record with retention policy
mockforge serve \
  --recorder \
  --recorder-db /var/lib/mockforge/recordings.db \
  --recorder-max-requests 50000 \
  --recorder-retention-days 30
```

#### Development Recording

```bash
# Record with management API on separate port
mockforge serve \
  --recorder \
  --recorder-api-port 9000
```

## Management API

### Endpoints

#### Query Endpoints

**List Recent Requests**
```http
GET /api/recorder/requests?limit=100&offset=0
```

Response:
```json
{
  "total": 1523,
  "offset": 0,
  "limit": 100,
  "exchanges": [
    {
      "request": {
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "protocol": "Http",
        "timestamp": "2024-03-20T10:30:00Z",
        "method": "GET",
        "path": "/api/users/123",
        "headers": "{\"content-type\":\"application/json\"}",
        "status_code": 200,
        "duration_ms": 42,
        "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736"
      },
      "response": {
        "status_code": 200,
        "headers": "{\"content-type\":\"application/json\"}",
        "body": "{\"id\":123,\"name\":\"John Doe\"}",
        "size_bytes": 34
      }
    }
  ]
}
```

**Get Single Request**
```http
GET /api/recorder/requests/{request_id}
```

**Search Requests**
```http
POST /api/recorder/search
Content-Type: application/json

{
  "protocol": "Http",
  "method": "POST",
  "path": "/api/*",
  "status_code": 200,
  "min_duration_ms": 100,
  "max_duration_ms": 1000,
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
  "limit": 50,
  "offset": 0
}
```

#### Export Endpoints

**Export to HAR**
```http
GET /api/recorder/export/har?limit=1000
```

#### Control Endpoints

**Get Status**
```http
GET /api/recorder/status
```

Response:
```json
{
  "enabled": true
}
```

**Enable Recording**
```http
POST /api/recorder/enable
```

**Disable Recording**
```http
POST /api/recorder/disable
```

**Clear All Recordings**
```http
DELETE /api/recorder/clear
```

#### Replay Endpoints

**Replay Single Request**
```http
POST /api/recorder/replay/{request_id}
```

Response:
```json
{
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "success": true,
  "message": "Replayed Http GET request",
  "original_status": 200,
  "replay_status": 200
}
```

#### Statistics Endpoints

**Get Statistics**
```http
GET /api/recorder/stats
```

Response:
```json
{
  "total_requests": 1523,
  "by_protocol": {
    "Http": 1200,
    "Grpc": 200,
    "WebSocket": 100,
    "GraphQL": 23
  },
  "by_status_code": {
    "200": 1400,
    "404": 100,
    "500": 23
  },
  "avg_duration_ms": 85.5
}
```

## Recording Protocols

### HTTP Recording

HTTP requests and responses are automatically recorded when the HTTP server is running with recording enabled.

**Captured Data:**
- Method (GET, POST, etc.)
- Path and query parameters
- Headers
- Request/response body (UTF-8 or base64)
- Status code
- Duration
- Client IP
- Trace context (traceparent header)

**Example:**
```bash
# Make a request
curl -X POST http://localhost:3000/api/users \
  -H "Content-Type: application/json" \
  -d '{"name":"John Doe"}'

# Query the recording
curl http://localhost:3000/api/recorder/requests | jq '.exchanges[0]'
```

### gRPC Recording

gRPC calls are recorded with service/method information and protobuf messages (base64 encoded).

**Captured Data:**
- Service name
- Method name
- Metadata (headers)
- Request/response messages (base64)
- Status code (gRPC status)
- Duration

**Example Integration:**
```rust
use mockforge_recorder::protocols::grpc::record_grpc_request;

let request_id = record_grpc_request(
    &recorder,
    "helloworld.Greeter",
    "SayHello",
    &metadata,
    Some(&request_bytes),
    Some("127.0.0.1"),
    trace_id.as_deref(),
    span_id.as_deref(),
).await?;
```

### WebSocket Recording

WebSocket connections and messages are recorded with direction indicators (inbound/outbound).

**Captured Data:**
- Connection events (CONNECT, DISCONNECT)
- Message direction (inbound/outbound)
- Message content (text or binary)
- Connection duration

**Example Integration:**
```rust
use mockforge_recorder::protocols::websocket::{
    record_ws_connection,
    record_ws_message,
    record_ws_disconnection,
};

// Record connection
let conn_id = record_ws_connection(
    &recorder,
    "/ws/chat",
    &headers,
    Some("127.0.0.1"),
    trace_id.as_deref(),
    span_id.as_deref(),
).await?;

// Record messages
record_ws_message(
    &recorder,
    &conn_id,
    "inbound",
    message_bytes,
    is_binary,
    trace_id.as_deref(),
    span_id.as_deref(),
).await?;

// Record disconnection
record_ws_disconnection(
    &recorder,
    &conn_id,
    Some("Client disconnected"),
    duration_ms,
).await?;
```

### GraphQL Recording

GraphQL queries, mutations, and subscriptions are recorded with operation details.

**Captured Data:**
- Operation type (query, mutation, subscription)
- Operation name
- Query document
- Variables
- Response data and errors
- Duration

**Example Integration:**
```rust
use mockforge_recorder::protocols::graphql::{
    record_graphql_request,
    record_graphql_response,
};

let request_id = record_graphql_request(
    &recorder,
    "query",
    Some("GetUser"),
    query_string,
    Some(&variables_json),
    &headers,
    Some("127.0.0.1"),
    trace_id.as_deref(),
    span_id.as_deref(),
).await?;

record_graphql_response(
    &recorder,
    &request_id,
    &response_json,
    has_errors,
    duration_ms,
).await?;
```

## Querying Recordings

### Using the Query API

```rust
use mockforge_recorder::query::{execute_query, QueryFilter};
use mockforge_recorder::models::Protocol;

let filter = QueryFilter {
    protocol: Some(Protocol::Http),
    method: Some("POST".to_string()),
    path: Some("/api/users/*".to_string()),
    status_code: Some(201),
    min_duration_ms: Some(100),
    max_duration_ms: Some(500),
    limit: Some(100),
    offset: Some(0),
    ..Default::default()
};

let result = execute_query(&database, filter).await?;
println!("Found {} requests", result.total);
```

### Using SQL Directly

The recorder uses SQLite, so you can query the database directly:

```bash
sqlite3 mockforge-recordings.db
```

```sql
-- Find slow requests
SELECT id, protocol, method, path, duration_ms, status_code
FROM requests
WHERE duration_ms > 1000
ORDER BY duration_ms DESC
LIMIT 10;

-- Find errors by status code
SELECT status_code, COUNT(*) as count
FROM requests
WHERE status_code >= 400
GROUP BY status_code
ORDER BY count DESC;

-- Find requests by trace ID
SELECT *
FROM requests
WHERE trace_id = '4bf92f3577b34da6a3ce929d0e0e4736';

-- Get average duration by protocol
SELECT protocol, AVG(duration_ms) as avg_duration
FROM requests
WHERE duration_ms IS NOT NULL
GROUP BY protocol;
```

## Replay Functionality

The replay engine allows you to re-execute recorded requests for regression testing and debugging.

### Basic Replay

```bash
# Replay a single request
curl -X POST http://localhost:3000/api/recorder/replay/{request_id}
```

### Programmatic Replay

```rust
use mockforge_recorder::replay::ReplayEngine;

let engine = ReplayEngine::new(database.clone());

// Replay a single request
let result = engine.replay_request(&request_id).await?;
println!("Replay {}: {}",
    if result.success { "succeeded" } else { "failed" },
    result.message
);

// Replay multiple requests
let results = engine.replay_batch(10).await?;
for result in results {
    println!("{}: {}", result.request_id, result.message);
}
```

### Response Comparison

```rust
// Compare replayed response with original
let comparison = engine.compare_responses(
    &request_id,
    &replay_response_bytes,
).await?;

if comparison.matches {
    println!("Responses match!");
} else {
    println!("Differences found:");
    for diff in comparison.differences {
        println!("  {}: {} != {}",
            diff.path,
            diff.original_value,
            diff.replayed_value
        );
    }
}
```

## HAR Export

Export HTTP recordings to HAR (HTTP Archive) format for use with browser dev tools and other analysis tools.

### Export via API

```bash
# Export all HTTP requests
curl http://localhost:3000/api/recorder/export/har > recordings.har

# Export with limit
curl "http://localhost:3000/api/recorder/export/har?limit=100" > recordings.har
```

### Programmatic Export

```rust
use mockforge_recorder::har_export::export_to_har;
use mockforge_recorder::query::{execute_query, QueryFilter};

// Get HTTP requests
let filter = QueryFilter {
    protocol: Some(Protocol::Http),
    limit: Some(1000),
    ..Default::default()
};

let result = execute_query(&database, filter).await?;

// Export to HAR
let har = export_to_har(&result.exchanges)?;
let har_json = serde_json::to_string_pretty(&har)?;
std::fs::write("recordings.har", har_json)?;
```

### Using HAR Files

```bash
# View in Chrome DevTools
# 1. Open Chrome DevTools (F12)
# 2. Go to Network tab
# 3. Right-click > Import HAR file

# Analyze with jq
cat recordings.har | jq '.log.entries[] | {method: .request.method, url: .request.url, status: .response.status}'

# Import into Postman
# 1. Import > Choose Files
# 2. Select the .har file
# 3. Review and import requests
```

## Best Practices

### Production Deployment

1. **Set Retention Policies**
   ```yaml
   recorder:
     retention_days: 7  # Auto-delete old recordings
     max_requests: 50000  # Limit database size
   ```

2. **Monitor Database Size**
   ```bash
   # Check database size
   du -h mockforge-recordings.db

   # Manually clean old records
   curl -X DELETE http://localhost:3000/api/recorder/clear
   ```

3. **Separate API Port**
   ```bash
   # Run management API on different port
   mockforge serve --recorder --recorder-api-port 9000
   ```

4. **Selective Recording**
   ```yaml
   recorder:
     record_http: true
     record_grpc: false  # Disable for high-volume gRPC
     record_websocket: false
     record_graphql: true
   ```

### Development Workflow

1. **Record During Testing**
   ```bash
   # Start recording
   mockforge serve --recorder

   # Run your tests
   npm test

   # Analyze recordings
   curl http://localhost:3000/api/recorder/stats
   ```

2. **Debug Failures**
   ```bash
   # Find failed requests
   curl -X POST http://localhost:3000/api/recorder/search \
     -H "Content-Type: application/json" \
     -d '{"status_code": 500, "limit": 10}'
   ```

3. **Replay for Regression Testing**
   ```bash
   # Export recordings
   curl http://localhost:3000/api/recorder/export/har > baseline.har

   # Later, replay and compare
   curl -X POST http://localhost:3000/api/recorder/replay/{id}
   ```

### Performance Optimization

1. **Asynchronous Recording**
   - Recording happens asynchronously to minimize latency
   - Database writes are batched when possible

2. **Index Usage**
   - SQLite indexes on timestamp, protocol, status_code, trace_id
   - Use these fields in queries for best performance

3. **Database Maintenance**
   ```bash
   # Vacuum database periodically
   sqlite3 mockforge-recordings.db "VACUUM;"
   ```

## Troubleshooting

### Recording Not Working

**Issue**: Requests are not being recorded

**Solutions**:
1. Check if recording is enabled:
   ```bash
   curl http://localhost:3000/api/recorder/status
   ```

2. Enable recording:
   ```bash
   curl -X POST http://localhost:3000/api/recorder/enable
   ```

3. Check database permissions:
   ```bash
   ls -l mockforge-recordings.db
   ```

4. Check logs for errors:
   ```bash
   mockforge serve --recorder -v
   ```

### Database Errors

**Issue**: SQLite database errors

**Solutions**:
1. Check database integrity:
   ```bash
   sqlite3 mockforge-recordings.db "PRAGMA integrity_check;"
   ```

2. Rebuild database:
   ```bash
   mv mockforge-recordings.db mockforge-recordings.db.bak
   mockforge serve --recorder
   ```

3. Check disk space:
   ```bash
   df -h .
   ```

### Performance Issues

**Issue**: Recording causing slowdowns

**Solutions**:
1. Reduce retention:
   ```yaml
   recorder:
     max_requests: 5000
     retention_days: 3
   ```

2. Disable verbose recording:
   ```yaml
   recorder:
     record_websocket: false  # High-volume protocols
   ```

3. Use separate disk for database:
   ```yaml
   recorder:
     database_path: "/mnt/fast-ssd/recordings.db"
   ```

### Query Timeouts

**Issue**: Slow queries

**Solutions**:
1. Add more specific filters:
   ```json
   {
     "protocol": "Http",
     "status_code": 500,
     "limit": 100
   }
   ```

2. Use pagination:
   ```json
   {
     "limit": 100,
     "offset": 0
   }
   ```

3. Vacuum database:
   ```bash
   sqlite3 mockforge-recordings.db "VACUUM; ANALYZE;"
   ```

## Next Steps

- **Distributed Tracing**: Combine with OpenTelemetry for full observability
- **Metrics Integration**: View recording stats in Prometheus/Grafana
- **Automated Testing**: Use replay functionality in CI/CD pipelines
- **Advanced Analytics**: Export to data warehouses for long-term analysis

For more information, see:
- [OpenTelemetry Integration](./OPENTELEMETRY.md)
- [Observability Guide](./OBSERVABILITY.md)
- [API Documentation](../README.md)
