# Audit Trails & Logging Guide

MockForge provides comprehensive audit logging for security, compliance, and debugging purposes. This guide covers all audit trail features.

## Overview

MockForge maintains multiple types of audit trails:

1. **Authentication Audit Logs** - Security-related authentication events
2. **Request Logs** - All API requests and responses
3. **Collaboration History** - Git-style version control for workspace changes
4. **Configuration Changes** - Changes to server configuration
5. **Plugin Activity** - Plugin execution and security events

## Authentication Audit Logs

### Features

Authentication audit logging tracks all authentication attempts:

- ✅ **Login Attempts**: Success and failure
- ✅ **Token Validation**: Token verification results
- ✅ **Method Tracking**: JWT, OAuth2, API Key, Basic Auth
- ✅ **IP Address**: Source IP for all authentication attempts
- ✅ **User Agent**: Client identification
- ✅ **Failure Reasons**: Detailed failure messages
- ✅ **Timestamp**: Precise event timestamps

### Configuration

Enable authentication audit logging:

```yaml
# config.yaml
auth:
  audit_log:
    enabled: true
    file_path: "/var/log/mockforge/auth-audit.log"
    log_success: true              # Log successful authentications
    log_failures: true             # Log failed authentications
    json_format: true              # Use JSON format for structured logging
```

### Environment Variables

```bash
export MOCKFORGE_AUTH_AUDIT_ENABLED=true
export MOCKFORGE_AUTH_AUDIT_FILE_PATH=/var/log/mockforge/auth-audit.log
export MOCKFORGE_AUTH_AUDIT_LOG_SUCCESS=true
export MOCKFORGE_AUTH_AUDIT_LOG_FAILURES=true
export MOCKFORGE_AUTH_AUDIT_JSON_FORMAT=true
```

### Log Format

#### JSON Format

```json
{
  "timestamp": "2025-01-27T10:30:00Z",
  "ip_address": "192.168.1.100",
  "user_agent": "Mozilla/5.0...",
  "auth_method": "jwt",
  "result": "success",
  "username": "admin",
  "path": "/api/workspaces",
  "http_method": "GET"
}
```

#### Text Format

```
[2025-01-27T10:30:00Z] 192.168.1.100 jwt GET /api/workspaces -> Success (user: admin, reason: None)
```

### Event Types

#### Authentication Success

```json
{
  "timestamp": "2025-01-27T10:30:00Z",
  "ip_address": "192.168.1.100",
  "auth_method": "jwt",
  "result": "success",
  "username": "admin"
}
```

#### Authentication Failure

```json
{
  "timestamp": "2025-01-27T10:30:05Z",
  "ip_address": "192.168.1.100",
  "auth_method": "jwt",
  "result": "failure",
  "failure_reason": "Token expired",
  "path": "/api/workspaces"
}
```

#### Token Expired

```json
{
  "timestamp": "2025-01-27T10:30:10Z",
  "ip_address": "192.168.1.100",
  "auth_method": "jwt",
  "result": "expired",
  "username": "admin"
}
```

### Querying Audit Logs

#### Using jq (JSON logs)

```bash
# Find all failed authentication attempts
cat /var/log/mockforge/auth-audit.log | jq 'select(.result == "failure")'

# Find all authentication attempts from specific IP
cat /var/log/mockforge/auth-audit.log | jq 'select(.ip_address == "192.168.1.100")'

# Count failures by reason
cat /var/log/mockforge/auth-audit.log | jq -r 'select(.result == "failure") | .failure_reason' | sort | uniq -c
```

#### Using grep (Text logs)

```bash
# Find all failed attempts
grep "Failure" /var/log/mockforge/auth-audit.log

# Find attempts from specific IP
grep "192.168.1.100" /var/log/mockforge/auth-audit.log

# Find attempts in last hour
grep "$(date -u -d '1 hour ago' +%Y-%m-%dT%H)" /var/log/mockforge/auth-audit.log
```

## Request Logs

### Features

Request logging captures all API interactions:

- ✅ **HTTP Requests**: All HTTP methods and paths
- ✅ **WebSocket Events**: Connection and message events
- ✅ **gRPC Calls**: Method calls and responses
- ✅ **Request/Response Pairs**: Full request and response data
- ✅ **Timing Information**: Response times and latency
- ✅ **Client Information**: IP address, user agent
- ✅ **Error Tracking**: Error messages and stack traces

### Configuration

```yaml
# config.yaml
logging:
  request_log:
    enabled: true
    max_entries: 1000              # Keep last 1000 requests in memory
    retention_days: 30             # Keep logs for 30 days
    log_file: "/var/log/mockforge/requests.log"
    log_level: "info"              # info, debug, warn, error
    include_headers: true          # Include request/response headers
    include_body: false            # Include request/response bodies (can be large)
    filter_sensitive: true         # Filter sensitive headers (Authorization, etc.)
```

### Log Entry Structure

```json
{
  "id": "req-123456",
  "timestamp": "2025-01-27T10:30:00Z",
  "server_type": "HTTP",
  "method": "GET",
  "path": "/api/users",
  "status_code": 200,
  "response_time_ms": 45,
  "client_ip": "192.168.1.100",
  "user_agent": "curl/7.68.0",
  "headers": {
    "Content-Type": "application/json",
    "Accept": "application/json"
  },
  "response_size_bytes": 1024,
  "error_message": null,
  "metadata": {
    "mock_id": "mock-789",
    "scenario": "default"
  }
}
```

### Querying Request Logs

#### Via Admin API

```bash
# Get recent requests
curl http://localhost:9080/api/logs/requests?limit=100

# Filter by status code
curl http://localhost:9080/api/logs/requests?status_code=500

# Filter by path
curl http://localhost:9080/api/logs/requests?path=/api/users

# Filter by time range
curl "http://localhost:9080/api/logs/requests?since=2025-01-27T00:00:00Z&until=2025-01-27T23:59:59Z"
```

#### Via Admin UI

The Admin UI provides:
- Real-time log streaming via Server-Sent Events (SSE)
- Search and filtering interface
- Log export functionality
- Visual timeline of requests

## Collaboration History

### Features

Collaboration history provides Git-style version control:

- ✅ **Commit History**: Every change creates a commit
- ✅ **Full Snapshots**: Complete workspace state in each commit
- ✅ **Author Attribution**: User ID and timestamp for all changes
- ✅ **Named Snapshots**: Tag important versions
- ✅ **Diff Viewing**: Compare any two versions
- ✅ **Restore Capability**: Restore to any previous commit

### Accessing History

#### Via API

```bash
# Get commit history
GET /api/workspaces/{id}/history?limit=50

# Get specific commit
GET /api/workspaces/{id}/history/{commit_id}

# Create snapshot
POST /api/workspaces/{id}/snapshots
{
  "name": "v1.0.0",
  "description": "Production release"
}

# Restore from snapshot
POST /api/workspaces/{id}/restore
{
  "snapshot_name": "v1.0.0"
}

# Compare versions
GET /api/workspaces/{id}/diff?from={commit1}&to={commit2}
```

#### Commit Structure

```json
{
  "id": "commit-abc123",
  "workspace_id": "workspace-xyz",
  "author_id": "user-789",
  "author_name": "John Doe",
  "timestamp": "2025-01-27T10:30:00Z",
  "message": "Added user endpoint",
  "parent_id": "commit-def456",
  "snapshot": {
    "mocks": [...],
    "config": {...}
  }
}
```

## Configuration Change Tracking

### Features

Track all configuration changes:

- ✅ **Startup Config**: Log configuration on server startup
- ✅ **Runtime Changes**: Track configuration updates via API
- ✅ **Environment Variables**: Log environment variable usage
- ✅ **Validation Errors**: Track configuration validation failures

### Logging Configuration Changes

```yaml
# config.yaml
logging:
  config_changes:
    enabled: true
    log_file: "/var/log/mockforge/config-changes.log"
    include_full_config: false    # Include full config in logs (can be large)
```

## Plugin Activity Logs

### Features

Track plugin execution and security events:

- ✅ **Plugin Loading**: Track when plugins are loaded
- ✅ **Plugin Execution**: Log plugin method calls
- ✅ **Security Violations**: Track security policy violations
- ✅ **Resource Usage**: Monitor plugin resource consumption

### Configuration

```yaml
# config.yaml
plugins:
  audit_log:
    enabled: true
    log_file: "/var/log/mockforge/plugins.log"
    log_execution: true
    log_security_events: true
    log_resource_usage: false    # Can be verbose
```

## Log Retention & Management

### Retention Policies

Configure log retention:

```yaml
# config.yaml
logging:
  retention:
    enabled: true
    retention_days: 90           # Keep logs for 90 days
    cleanup_interval_hours: 24   # Run cleanup every 24 hours
    archive_enabled: true         # Archive old logs
    archive_path: "/var/log/mockforge/archive"
    compression: true             # Compress archived logs
```

### Manual Cleanup

```bash
# Clean logs older than 30 days
mockforge logs cleanup --days 30

# Archive logs
mockforge logs archive --output ./archive

# Export logs for analysis
mockforge logs export --format json --output logs.json
```

## Compliance & Security

### Audit Log Requirements

MockForge audit logs support compliance requirements:

- ✅ **SOC 2**: Comprehensive audit logging
- ✅ **ISO 27001**: Security event logging
- ✅ **HIPAA**: Audit controls for healthcare data
- ✅ **GDPR**: Data access and modification tracking

### Best Practices

1. **Secure Storage**: Store audit logs in secure, encrypted locations
2. **Access Control**: Restrict access to audit logs
3. **Regular Review**: Review audit logs regularly for anomalies
4. **Retention**: Follow regulatory retention requirements
5. **Monitoring**: Set up alerts for suspicious activity
6. **Backup**: Regularly backup audit logs

### Example Compliance Configuration

```yaml
# config.yaml
logging:
  # Comprehensive audit logging
  auth_audit:
    enabled: true
    log_success: true
    log_failures: true
    json_format: true

  request_log:
    enabled: true
    retention_days: 365           # 1 year retention
    include_headers: true
    filter_sensitive: true

  # Secure storage
  storage:
    path: "/secure/audit-logs"
    permissions: "0600"           # Owner read/write only
    encryption: true               # Encrypt log files

  # Alerting
  alerts:
    enabled: true
    failed_auth_threshold: 5      # Alert after 5 failed attempts
    error_rate_threshold: 0.1     # Alert if error rate > 10%
```

## Integration with Monitoring

### Prometheus Metrics

Audit events are exposed as Prometheus metrics:

```prometheus
# Authentication attempts
mockforge_auth_attempts_total{method="jwt", result="success"} 150
mockforge_auth_attempts_total{method="jwt", result="failure"} 5

# Request counts
mockforge_requests_total{method="GET", status="200"} 1000
mockforge_requests_total{method="POST", status="500"} 10
```

### Grafana Dashboards

Import audit log dashboards for visualization:

- Authentication success/failure rates
- Request patterns and trends
- Error rate monitoring
- User activity timelines

## Troubleshooting

### Common Issues

#### Logs Not Being Written

**Problem:** Audit logs not appearing in log files

**Solutions:**
1. Check file permissions: `ls -l /var/log/mockforge/`
2. Verify directory exists: `mkdir -p /var/log/mockforge`
3. Check disk space: `df -h`
4. Verify configuration: Check `enabled: true` in config

#### Log Files Too Large

**Problem:** Log files growing too large

**Solutions:**
1. Enable log rotation:
```yaml
logging:
  rotation:
    enabled: true
    max_size_mb: 100
    max_files: 10
```

2. Reduce retention period
3. Enable compression
4. Filter less important events

#### Missing Audit Events

**Problem:** Some events not being logged

**Solutions:**
1. Check log level: Ensure level includes desired events
2. Verify filters: Check if events are being filtered
3. Check configuration: Verify audit logging is enabled for all modules

## Related Documentation

- [RBAC Guide](RBAC_GUIDE.md) - Role-based access control
- [Security Guide](../book/src/user-guide/security.md) - Security features
- [Compliance Checklist](COMPLIANCE_AUDIT_CHECKLIST.md) - Compliance requirements

## Support

For issues or questions:
- [GitHub Issues](https://github.com/SaaSy-Solutions/mockforge/issues)
- [GitHub Discussions](https://github.com/SaaSy-Solutions/mockforge/discussions)
- [Discord](https://discord.gg/2FxXqKpa)

---

**Last Updated:** 2025-01-27
