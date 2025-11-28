# Tunnel Production Hardening

**Date**: 2025-01-27
**Status**: ✅ **Implemented**

## Overview

The tunnel service has been enhanced with production-ready features including persistent storage, rate limiting, TLS support, and comprehensive audit logging.

## Features Implemented

### 1. Persistent Storage

**Location**: `crates/mockforge-tunnel/src/storage.rs`

**Features**:
- SQLite-based persistent storage for tunnel data
- Tunnels survive server restarts
- Automatic schema initialization
- Indexes for performance
- WAL mode for better concurrency
- Automatic cleanup of expired tunnels

**Usage**:
```bash
# Use persistent storage (default)
TUNNEL_DATABASE_PATH=/var/lib/mockforge/tunnels.db tunnel-server

# Use in-memory storage (testing only)
TUNNEL_USE_IN_MEMORY_STORAGE=true tunnel-server
```

**Database Schema**:
```sql
CREATE TABLE tunnels (
    tunnel_id TEXT PRIMARY KEY,
    subdomain TEXT UNIQUE,
    public_url TEXT NOT NULL,
    local_url TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1,
    request_count INTEGER NOT NULL DEFAULT 0,
    bytes_transferred INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    expires_at TEXT,
    custom_domain TEXT,
    protocol TEXT NOT NULL DEFAULT 'http',
    websocket_enabled INTEGER NOT NULL DEFAULT 1,
    http2_enabled INTEGER NOT NULL DEFAULT 1,
    updated_at TEXT NOT NULL
);
```

### 2. Rate Limiting

**Location**: `crates/mockforge-tunnel/src/rate_limit.rs`

**Features**:
- Global rate limiting (configurable requests per minute)
- Per-IP rate limiting
- Configurable burst capacity
- Uses `governor` crate for efficient rate limiting
- Automatic cleanup of old IP limiters

**Configuration**:
```bash
# Enable rate limiting (default: enabled)
TUNNEL_RATE_LIMIT_ENABLED=true

# Set global rate limit (default: 1000 RPM)
TUNNEL_RATE_LIMIT_RPM=1000

# Per-IP rate limit (default: 100 RPM)
TUNNEL_RATE_LIMIT_PER_IP_RPM=100
```

**Response**: When rate limit is exceeded, server returns `429 Too Many Requests`.

### 3. TLS Support

**Location**: `crates/mockforge-tunnel/src/bin/tunnel-server.rs`

**Features**:
- TLS/HTTPS support using RustLS
- Certificate and key file configuration
- Automatic TLS termination

**Configuration**:
```bash
# Enable TLS
TUNNEL_TLS_CERT=/path/to/cert.pem
TUNNEL_TLS_KEY=/path/to/key.pem
```

**Generating Certificates**:
```bash
# Self-signed certificate (testing)
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

# Production: Use certificates from Let's Encrypt or your CA
```

### 4. Audit Logging

**Location**: `crates/mockforge-tunnel/src/audit.rs`

**Features**:
- Structured audit logging for all operations
- JSON-formatted audit events
- Tracks: tunnel creation, deletion, access, errors
- Includes: timestamps, client IPs, principals, actions
- Integrates with tracing for centralized logging

**Event Types**:
- `TunnelCreated`: Tunnel was created
- `TunnelDeleted`: Tunnel was deleted
- `TunnelAccessed`: Tunnel endpoint was accessed
- `TunnelStatusChecked`: Tunnel status was queried
- `RateLimitExceeded`: Rate limit was exceeded
- `AuthenticationFailed`: Authentication failed
- `AuthorizationFailed`: Authorization failed
- `Error`: Error occurred
- `ConfigChanged`: Configuration changed

**Configuration**:
```bash
# Enable audit logging (default: enabled)
TUNNEL_AUDIT_LOG_ENABLED=true

# Optional: Custom audit log file path
TUNNEL_AUDIT_LOG_PATH=/var/log/mockforge/audit.log
```

**Example Audit Event**:
```json
{
  "timestamp": "2025-01-27T12:00:00Z",
  "event_type": "tunnel_created",
  "tunnel_id": "abc123",
  "client_ip": "192.168.1.1",
  "principal": null,
  "action": "create_tunnel",
  "resource": "tunnel:abc123",
  "success": true,
  "error_message": null,
  "metadata": {
    "local_url": "http://localhost:3000",
    "public_url": "https://tunnel-abc123.tunnel.mockforge.test"
  }
}
```

### 5. Server Configuration

**Location**: `crates/mockforge-tunnel/src/server_config.rs`

**Features**:
- Centralized configuration management
- Environment variable support
- Sensible defaults
- TLS configuration
- Rate limiting configuration

**Configuration Options**:
```bash
# Server settings
TUNNEL_SERVER_PORT=4040
TUNNEL_SERVER_BIND=0.0.0.0

# Storage
TUNNEL_DATABASE_PATH=/var/lib/mockforge/tunnels.db
TUNNEL_USE_IN_MEMORY_STORAGE=false

# TLS
TUNNEL_TLS_CERT=/path/to/cert.pem
TUNNEL_TLS_KEY=/path/to/key.pem

# Rate limiting
TUNNEL_RATE_LIMIT_ENABLED=true
TUNNEL_RATE_LIMIT_RPM=1000

# Audit logging
TUNNEL_AUDIT_LOG_ENABLED=true
TUNNEL_AUDIT_LOG_PATH=/var/log/mockforge/audit.log
```

## Production Deployment

### 1. System Requirements

- **Storage**: At least 1GB for database (grows with tunnel count)
- **Memory**: 512MB minimum, 1GB recommended
- **CPU**: 2 cores minimum
- **Network**: Sufficient bandwidth for tunneled traffic

### 2. Security Considerations

1. **TLS Certificates**: Use valid certificates from a trusted CA
2. **Database Permissions**: Restrict database file permissions
3. **Audit Logs**: Store audit logs securely, rotate regularly
4. **Rate Limiting**: Tune rate limits based on expected load
5. **Authentication**: Implement authentication (TODO: future enhancement)

### 3. Deployment Steps

```bash
# 1. Build the server
cargo build --release --package mockforge-tunnel --features server --bin tunnel-server

# 2. Create directories
sudo mkdir -p /var/lib/mockforge
sudo mkdir -p /var/log/mockforge
sudo chown mockforge:mockforge /var/lib/mockforge /var/log/mockforge

# 3. Set up TLS certificates
sudo cp cert.pem /etc/mockforge/tunnel-cert.pem
sudo cp key.pem /etc/mockforge/tunnel-key.pem
sudo chmod 600 /etc/mockforge/tunnel-key.pem

# 4. Configure environment
cat > /etc/mockforge/tunnel-server.env << EOF
TUNNEL_SERVER_PORT=443
TUNNEL_SERVER_BIND=0.0.0.0
TUNNEL_DATABASE_PATH=/var/lib/mockforge/tunnels.db
TUNNEL_TLS_CERT=/etc/mockforge/tunnel-cert.pem
TUNNEL_TLS_KEY=/etc/mockforge/tunnel-key.pem
TUNNEL_RATE_LIMIT_ENABLED=true
TUNNEL_RATE_LIMIT_RPM=1000
TUNNEL_AUDIT_LOG_ENABLED=true
TUNNEL_AUDIT_LOG_PATH=/var/log/mockforge/audit.log
EOF

# 5. Create systemd service
cat > /etc/systemd/system/mockforge-tunnel.service << EOF
[Unit]
Description=MockForge Tunnel Server
After=network.target

[Service]
Type=simple
User=mockforge
Group=mockforge
EnvironmentFile=/etc/mockforge/tunnel-server.env
ExecStart=/usr/local/bin/tunnel-server
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# 6. Start service
sudo systemctl daemon-reload
sudo systemctl enable mockforge-tunnel
sudo systemctl start mockforge-tunnel
```

### 4. Monitoring

**Health Check**:
```bash
curl http://localhost:4040/health
```

**Check Logs**:
```bash
# Service logs
sudo journalctl -u mockforge-tunnel -f

# Audit logs
tail -f /var/log/mockforge/audit.log
```

**Database Maintenance**:
```bash
# Check database size
du -h /var/lib/mockforge/tunnels.db

# Backup database
cp /var/lib/mockforge/tunnels.db /backup/tunnels-$(date +%Y%m%d).db

# Vacuum database (optimize)
sqlite3 /var/lib/mockforge/tunnels.db "VACUUM;"
```

## Performance Tuning

### Rate Limiting

Adjust rate limits based on your traffic patterns:

```bash
# High-traffic environment
TUNNEL_RATE_LIMIT_RPM=5000
TUNNEL_RATE_LIMIT_PER_IP_RPM=500

# Low-traffic environment
TUNNEL_RATE_LIMIT_RPM=100
TUNNEL_RATE_LIMIT_PER_IP_RPM=10
```

### Database Optimization

```sql
-- Analyze and optimize
ANALYZE tunnels;

-- Check index usage
EXPLAIN QUERY PLAN SELECT * FROM tunnels WHERE subdomain = ?;
```

### Connection Pooling

The SQLite connection pool is configured with:
- Max connections: 10
- WAL mode enabled for better concurrency

## Troubleshooting

### Database Locked

**Problem**: SQLite database is locked

**Solutions**:
1. Check for other processes accessing the database
2. Ensure WAL mode is enabled
3. Increase connection pool size if needed

### Rate Limit Too Aggressive

**Problem**: Legitimate requests are being rate limited

**Solutions**:
1. Increase `TUNNEL_RATE_LIMIT_RPM`
2. Increase `TUNNEL_RATE_LIMIT_PER_IP_RPM`
3. Review audit logs for patterns

### TLS Certificate Issues

**Problem**: TLS handshake fails

**Solutions**:
1. Verify certificate and key file paths
2. Check file permissions (key should be 600)
3. Verify certificate is not expired
4. Check certificate format (should be PEM)

## Future Enhancements

Potential improvements:
1. **Authentication**: JWT-based authentication for tunnel management
2. **Authorization**: Role-based access control (RBAC)
3. **Distributed Storage**: Redis for shared state across instances
4. **Metrics**: Prometheus metrics export
5. **WebSocket Rate Limiting**: Separate rate limiting for WebSocket connections
6. **Geo-blocking**: Restrict access by geographic location
7. **DDoS Protection**: Advanced DDoS mitigation

## Files Created/Modified

1. **`crates/mockforge-tunnel/src/storage.rs`** (NEW)
   - Persistent SQLite storage for tunnels

2. **`crates/mockforge-tunnel/src/rate_limit.rs`** (NEW)
   - Rate limiting middleware and configuration

3. **`crates/mockforge-tunnel/src/audit.rs`** (NEW)
   - Audit logging for all operations

4. **`crates/mockforge-tunnel/src/server_config.rs`** (NEW)
   - Server configuration management

5. **`crates/mockforge-tunnel/src/bin/tunnel-server.rs`** (MODIFIED)
   - Enhanced with TLS, persistent storage, rate limiting, audit logging

6. **`crates/mockforge-tunnel/Cargo.toml`** (MODIFIED)
   - Added dependencies: sqlx, governor, rustls

7. **`crates/mockforge-tunnel/src/lib.rs`** (MODIFIED)
   - Exported new modules

## Testing

### Unit Tests

```bash
cargo test --package mockforge-tunnel --features server
```

### Integration Tests

```bash
# Test with persistent storage
TUNNEL_DATABASE_PATH=test.db cargo test --package mockforge-tunnel --features server

# Test with in-memory storage
TUNNEL_USE_IN_MEMORY_STORAGE=true cargo test --package mockforge-tunnel --features server
```

### Manual Testing

```bash
# Start server with all features
TUNNEL_SERVER_PORT=4040 \
TUNNEL_DATABASE_PATH=tunnels.db \
TUNNEL_RATE_LIMIT_ENABLED=true \
TUNNEL_AUDIT_LOG_ENABLED=true \
cargo run --release --package mockforge-tunnel --features server --bin tunnel-server

# Create tunnel
curl -X POST http://localhost:4040/api/tunnels \
  -H "Content-Type: application/json" \
  -d '{"local_url": "http://localhost:3000"}'

# Access tunnel
curl http://localhost:4040/tunnel/<tunnel_id>/health
```

## Summary

The tunnel service is now production-ready with:
- ✅ Persistent storage (SQLite)
- ✅ Rate limiting (global and per-IP)
- ✅ TLS support
- ✅ Comprehensive audit logging
- ✅ Configuration management
- ✅ Graceful shutdown
- ✅ Automatic cleanup of expired tunnels

All features are optional and can be enabled/disabled via environment variables, making the tunnel server suitable for both development and production environments.
