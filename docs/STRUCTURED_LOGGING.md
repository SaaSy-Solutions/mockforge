# Structured Logging and Tracing in MockForge

MockForge provides comprehensive logging and observability features including:
- **Structured JSON logging** for easy parsing and analysis
- **File output with rotation** for persistent log storage
- **OpenTelemetry distributed tracing** for request correlation
- **Multi-level logging** with configurable verbosity

## Table of Contents

- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [JSON Logging](#json-logging)
- [File Output](#file-output)
- [OpenTelemetry Integration](#opentelemetry-integration)
- [Log Levels](#log-levels)
- [Practical Examples](#practical-examples)
- [Best Practices](#best-practices)

## Quick Start

### Basic Logging (Plain Text)

```bash
# Start with default logging (plain text to console)
mockforge serve --config mockforge.yaml

# Set log level
mockforge serve -v debug --config mockforge.yaml
```

### JSON Logging

```yaml
# mockforge.yaml
logging:
  level: "info"
  json_format: true
```

```bash
# Start with JSON logging
mockforge serve --config mockforge.yaml

# Pretty-print JSON logs using jq
mockforge serve --config mockforge.yaml | jq
```

### With OpenTelemetry Tracing

```yaml
# mockforge.yaml
logging:
  level: "info"
  json_format: true

observability:
  opentelemetry:
    enabled: true
    service_name: "mockforge"
    environment: "production"
    jaeger_endpoint: "http://localhost:14268/api/traces"
    sampling_rate: 1.0
```

## Configuration

### Logging Configuration

The logging section in your configuration file controls all logging behavior:

```yaml
logging:
  # Log level: trace, debug, info, warn, error
  level: "info"

  # Enable JSON format for structured logging
  json_format: false

  # Optional: Write logs to file (in addition to stdout)
  file_path: "logs/mockforge.log"

  # Maximum log file size in MB before rotation
  max_file_size_mb: 100

  # Maximum number of rotated log files to keep
  max_files: 10
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `level` | string | `"info"` | Minimum log level to display (`trace`, `debug`, `info`, `warn`, `error`) |
| `json_format` | boolean | `false` | Enable JSON-formatted structured logging |
| `file_path` | string | `null` | Optional path to write logs to a file |
| `max_file_size_mb` | integer | `10` | Maximum file size before rotation (MB) |
| `max_files` | integer | `5` | Maximum number of rotated log files to keep |

## JSON Logging

### Enabling JSON Logging

JSON logging outputs logs as structured JSON objects, making them easy to parse, search, and analyze with tools like `jq`, Elasticsearch, or Splunk.

```yaml
logging:
  level: "info"
  json_format: true
```

### JSON Log Format

Each log entry is a JSON object with the following structure:

```json
{
  "timestamp": "2025-10-09T12:34:56.789Z",
  "level": "INFO",
  "target": "mockforge_http::middleware",
  "fields": {
    "message": "HTTP request received",
    "method": "GET",
    "path": "/api/users",
    "status": 200,
    "duration_ms": 45
  },
  "span": {
    "name": "http_request",
    "trace_id": "abc123def456...",
    "span_id": "def456ghi789..."
  }
}
```

### Fields

- **timestamp**: ISO 8601 formatted timestamp
- **level**: Log level (TRACE, DEBUG, INFO, WARN, ERROR)
- **target**: Source module/component that generated the log
- **fields**: Structured data associated with the log entry
- **span**: Current tracing span (when OpenTelemetry is enabled)

### Using JSON Logs with jq

```bash
# Pretty-print logs
mockforge serve --config config.yaml | jq

# Filter by log level
mockforge serve --config config.yaml | jq 'select(.level == "ERROR")'

# Extract specific fields
mockforge serve --config config.yaml | jq '{time: .timestamp, msg: .fields.message}'

# Filter HTTP requests with status >= 400
mockforge serve --config config.yaml | \
  jq 'select(.fields.status >= 400) | {time: .timestamp, path: .fields.path, status: .fields.status}'
```

## File Output

### Enabling File Logging

Write logs to a file in addition to stdout:

```yaml
logging:
  level: "info"
  json_format: true
  file_path: "logs/mockforge.log"
  max_file_size_mb: 100
  max_files: 10
```

### Log Rotation

MockForge automatically rotates log files when they reach the configured size:

- `mockforge.log` - Current log file
- `mockforge.log.1` - Previous log file
- `mockforge.log.2` - Older log file
- ...

Old files are deleted when the maximum number of files is reached.

### Monitoring Log Files

```bash
# Tail logs in real-time
tail -f logs/mockforge.log

# Tail and pretty-print JSON logs
tail -f logs/mockforge.log | jq

# Search logs for errors
grep -i error logs/mockforge.log

# Search JSON logs for errors using jq
jq 'select(.level == "ERROR")' logs/mockforge.log
```

## OpenTelemetry Integration

### Overview

OpenTelemetry provides distributed tracing capabilities, allowing you to:
- Track requests across multiple services
- Visualize request flows and dependencies
- Identify performance bottlenecks
- Debug complex distributed systems

### Configuration

```yaml
logging:
  level: "info"
  json_format: true

observability:
  opentelemetry:
    enabled: true
    service_name: "mockforge"
    environment: "production"
    jaeger_endpoint: "http://localhost:14268/api/traces"
    otlp_endpoint: "http://localhost:4317"
    protocol: "grpc"
    sampling_rate: 1.0
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | boolean | `false` | Enable OpenTelemetry tracing |
| `service_name` | string | `"mockforge"` | Service name for traces |
| `environment` | string | `"development"` | Deployment environment |
| `jaeger_endpoint` | string | `http://localhost:14268/api/traces` | Jaeger collector endpoint |
| `otlp_endpoint` | string | `http://localhost:4317` | OTLP collector endpoint |
| `protocol` | string | `"grpc"` | Export protocol (`grpc` or `http`) |
| `sampling_rate` | float | `1.0` | Sampling rate (0.0 to 1.0) |

### Trace Context

When OpenTelemetry is enabled, logs include trace context:

```json
{
  "timestamp": "2025-10-09T12:34:56.789Z",
  "level": "INFO",
  "fields": {
    "message": "Processing HTTP request"
  },
  "span": {
    "name": "http_request",
    "trace_id": "abc123def456...",
    "span_id": "def456ghi789...",
    "parent_span_id": "ghi789jkl012..."
  }
}
```

This allows you to:
- Correlate logs with traces
- Search logs by trace ID
- Understand the context of each log entry

### Using with Jaeger

1. **Start Jaeger**:
   ```bash
   docker run -d --name jaeger \
     -e COLLECTOR_ZIPKIN_HOST_PORT=:9411 \
     -p 5775:5775/udp \
     -p 6831:6831/udp \
     -p 6832:6832/udp \
     -p 5778:5778 \
     -p 16686:16686 \
     -p 14250:14250 \
     -p 14268:14268 \
     -p 14269:14269 \
     -p 9411:9411 \
     jaegertracing/all-in-one:latest
   ```

2. **Configure MockForge**:
   ```yaml
   observability:
     opentelemetry:
       enabled: true
       service_name: "mockforge"
       jaeger_endpoint: "http://localhost:14268/api/traces"
   ```

3. **Start MockForge**:
   ```bash
   mockforge serve --config mockforge.yaml
   ```

4. **View traces**: Open http://localhost:16686

### Using with OpenTelemetry Collector

1. **Create collector config** (`otel-collector-config.yaml`):
   ```yaml
   receivers:
     otlp:
       protocols:
         grpc:
           endpoint: 0.0.0.0:4317
         http:
           endpoint: 0.0.0.0:4318

   processors:
     batch:

   exporters:
     logging:
       loglevel: debug
     jaeger:
       endpoint: jaeger:14250
       tls:
         insecure: true

   service:
     pipelines:
       traces:
         receivers: [otlp]
         processors: [batch]
         exporters: [logging, jaeger]
   ```

2. **Start collector**:
   ```bash
   docker run -d --name otel-collector \
     -v $(pwd)/otel-collector-config.yaml:/etc/otel-collector-config.yaml \
     -p 4317:4317 \
     -p 4318:4318 \
     otel/opentelemetry-collector-contrib:latest \
     --config=/etc/otel-collector-config.yaml
   ```

3. **Configure MockForge**:
   ```yaml
   observability:
     opentelemetry:
       enabled: true
       service_name: "mockforge"
       otlp_endpoint: "http://localhost:4317"
       protocol: "grpc"
   ```

## Log Levels

### Available Levels

| Level | Description | Use Case |
|-------|-------------|----------|
| `trace` | Extremely detailed | Low-level debugging, very verbose |
| `debug` | Detailed debugging | Development, troubleshooting |
| `info` | Informational | Normal operation, key events |
| `warn` | Warnings | Potential issues, degraded performance |
| `error` | Errors | Failures, exceptions |

### Setting Log Level

**Via CLI**:
```bash
mockforge serve -v debug
mockforge serve -v trace
mockforge serve -v error
```

**Via Config**:
```yaml
logging:
  level: "debug"
```

**Via Environment**:
```bash
export RUST_LOG=debug
mockforge serve
```

### Level Filtering

Set different levels for different modules:

```bash
# Set default to info, but debug for HTTP module
export RUST_LOG="info,mockforge_http=debug"
mockforge serve
```

## Practical Examples

### Example 1: Development Setup

Simple console logging with debug level:

```yaml
logging:
  level: "debug"
  json_format: false
```

```bash
mockforge serve --config dev.yaml
```

### Example 2: Production Setup

JSON logging to file with OpenTelemetry:

```yaml
logging:
  level: "info"
  json_format: true
  file_path: "/var/log/mockforge/mockforge.log"
  max_file_size_mb: 500
  max_files: 20

observability:
  opentelemetry:
    enabled: true
    service_name: "mockforge-prod"
    environment: "production"
    otlp_endpoint: "http://otel-collector:4317"
    sampling_rate: 0.1  # Sample 10% of traces
```

### Example 3: Debugging Issues

High verbosity with trace-level logging:

```yaml
logging:
  level: "trace"
  json_format: true
  file_path: "debug.log"
```

```bash
# Filter for errors only
mockforge serve --config debug.yaml | jq 'select(.level == "ERROR")'

# Watch for specific message
mockforge serve --config debug.yaml | \
  jq 'select(.fields.message | contains("timeout"))'
```

### Example 4: Log Analysis

Aggregate and analyze logs:

```bash
# Count log entries by level
jq -s 'group_by(.level) | map({level: .[0].level, count: length})' logs/mockforge.log

# Find slow requests (>1000ms)
jq 'select(.fields.duration_ms > 1000) | {time: .timestamp, path: .fields.path, duration: .fields.duration_ms}' logs/mockforge.log

# Extract all error messages
jq 'select(.level == "ERROR") | .fields.message' logs/mockforge.log
```

## Best Practices

### 1. Use JSON Logging in Production

JSON logs are easier to parse, search, and analyze:

```yaml
logging:
  json_format: true
```

### 2. Enable File Output

Always write logs to files for persistence:

```yaml
logging:
  file_path: "/var/log/mockforge/mockforge.log"
  max_file_size_mb: 500
  max_files: 20
```

### 3. Configure Appropriate Log Levels

- **Development**: `debug` or `trace`
- **Production**: `info` or `warn`
- **Debugging**: `trace`

### 4. Use OpenTelemetry for Distributed Systems

Enable tracing when running multiple services:

```yaml
observability:
  opentelemetry:
    enabled: true
    sampling_rate: 0.1  # Adjust based on volume
```

### 5. Sample Traces in High-Volume Environments

Reduce overhead by sampling:

```yaml
observability:
  opentelemetry:
    sampling_rate: 0.1  # Sample 10% of traces
```

### 6. Monitor Log File Size

Use appropriate rotation settings:

```yaml
logging:
  max_file_size_mb: 500  # Rotate at 500MB
  max_files: 20          # Keep 20 files
```

### 7. Correlate Logs with Traces

Use trace IDs to find related logs:

```bash
# Find all logs for a specific trace
jq 'select(.span.trace_id == "abc123...")' logs/mockforge.log
```

### 8. Structured Fields

When JSON logging is enabled, structured fields are automatically included:

```json
{
  "fields": {
    "method": "GET",
    "path": "/api/users",
    "status": 200,
    "duration_ms": 45,
    "user_id": "123"
  }
}
```

## Integration with Logging Platforms

### Elasticsearch / Kibana

Ship logs to Elasticsearch using Filebeat:

```yaml
# filebeat.yml
filebeat.inputs:
  - type: log
    enabled: true
    paths:
      - /var/log/mockforge/*.log
    json.keys_under_root: true
    json.add_error_key: true

output.elasticsearch:
  hosts: ["localhost:9200"]
```

### Splunk

Forward logs to Splunk:

```bash
# Use Splunk Universal Forwarder
/opt/splunkforwarder/bin/splunk add monitor /var/log/mockforge/mockforge.log
```

### Datadog

Use Datadog agent to collect logs:

```yaml
# /etc/datadog-agent/conf.d/mockforge.d/conf.yaml
logs:
  - type: file
    path: /var/log/mockforge/mockforge.log
    service: mockforge
    source: rust
    sourcecategory: application
    tags:
      - env:production
```

## Troubleshooting

### Logs Not Appearing

1. Check log level:
   ```yaml
   logging:
     level: "debug"  # Lower level for more logs
   ```

2. Verify configuration is loaded:
   ```bash
   mockforge serve --config mockforge.yaml
   # Should see: "Loaded configuration from mockforge.yaml"
   ```

### File Logging Not Working

1. Ensure directory exists:
   ```bash
   mkdir -p /var/log/mockforge
   ```

2. Check permissions:
   ```bash
   chmod 755 /var/log/mockforge
   ```

### OpenTelemetry Not Working

1. Verify endpoint is reachable:
   ```bash
   curl http://localhost:14268/api/traces
   ```

2. Check sampling rate:
   ```yaml
   observability:
     opentelemetry:
       sampling_rate: 1.0  # Set to 1.0 for testing
   ```

3. Check logs for errors:
   ```bash
   mockforge serve --config mockforge.yaml 2>&1 | grep -i telemetry
   ```

## Further Reading

- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [Tracing in Rust](https://docs.rs/tracing/)
- [MockForge Observability Guide](./OBSERVABILITY.md)
