# MQTT Configuration Reference

This document provides a comprehensive reference for configuring the MockForge MQTT broker. The MQTT implementation supports all standard MQTT 3.1.1 and 5.0 features with additional MockForge-specific configuration options.

## Basic Configuration

```yaml
mqtt:
  # Enable/disable MQTT broker
  enabled: true

  # Server binding
  port: 1883
  host: "0.0.0.0"

  # Connection limits
  max_connections: 1000

  # Message size limits
  max_packet_size: 1048576  # 1MB

  # Connection timeouts
  keep_alive_secs: 60
```

## Advanced Configuration

### Connection Management

```yaml
mqtt:
  # Maximum concurrent connections
  max_connections: 1000

  # Maximum packet size (bytes)
  max_packet_size: 1048576  # 1MB

  # Default keep-alive timeout (seconds)
  keep_alive_secs: 60

  # Maximum QoS 1/2 messages in flight per client
  max_inflight_messages: 20

  # Maximum queued messages per client
  max_queued_messages: 100
```

### Quality of Service (QoS)

MockForge supports all MQTT QoS levels:

- **QoS 0**: At most once delivery (fire and forget)
- **QoS 1**: At least once delivery (acknowledged)
- **QoS 2**: Exactly once delivery (assured)

QoS levels are configured per fixture and can be overridden by client requests.

### Retained Messages

```yaml
mqtt:
  # Enable retained message support
  retained_messages_enabled: true

  # Maximum retained messages per topic
  max_retained_per_topic: 1

  # Maximum total retained messages
  max_total_retained: 10000
```

### Session Management

```yaml
mqtt:
  # Enable persistent sessions
  persistent_sessions: true

  # Session expiry (seconds)
  session_expiry_secs: 3600

  # Clean session behavior
  force_clean_session: false
```

## TLS/SSL Configuration

For secure MQTT (MQTT over TLS):

```yaml
mqtt:
  # Use TLS
  tls_enabled: true
  tls_port: 8883

  # Certificate paths
  tls_cert_path: "/path/to/server.crt"
  tls_key_path: "/path/to/server.key"

  # Client certificate verification
  tls_require_client_cert: false
  tls_ca_path: "/path/to/ca.crt"
```

## Authentication and Authorization

### Basic Authentication

```yaml
mqtt:
  # Enable authentication
  auth_enabled: true

  # Authentication method
  auth_method: "basic"  # basic, jwt, oauth2

  # User database
  users:
    - username: "user1"
      password: "password1"
      permissions:
        - "publish:sensors/#"
        - "subscribe:actuators/#"
    - username: "device1"
      password: "devicepass"
      permissions:
        - "publish:devices/device1/#"
        - "subscribe:commands/device1/#"
```

### JWT Authentication

```yaml
mqtt:
  auth_method: "jwt"

  jwt:
    # JWT issuer
    issuer: "mockforge"

    # JWT audience
    audience: "mqtt-clients"

    # Secret key or public key path
    secret: "your-jwt-secret"
    # OR
    public_key_path: "/path/to/public.pem"

    # Token validation
    validate_exp: true
    validate_iat: true
    validate_nbf: true

    # Custom claims mapping
    claims_mapping:
      permissions: "perms"
      client_id: "client"
```

## Topic Authorization

```yaml
mqtt:
  # Topic access control
  topic_acl:
    # Allow anonymous access to these topics
    anonymous_topics:
      - "public/#"

    # Deny access to these topics
    denied_topics:
      - "admin/#"
      - "system/#"

    # Require authentication for these topics
    authenticated_topics:
      - "private/#"
      - "secure/#"
```

## Logging and Monitoring

```yaml
mqtt:
  # Log level
  log_level: "info"

  # Enable connection logging
  log_connections: true

  # Enable message logging (WARNING: can be verbose)
  log_messages: false

  # Metrics collection
  metrics_enabled: true

  # Prometheus metrics
  metrics_path: "/metrics"
  metrics_port: 9090
```

## Performance Tuning

```yaml
mqtt:
  # Thread pool size
  worker_threads: 4

  # Connection backlog
  connection_backlog: 1024

  # Socket options
  socket:
    # TCP_NODELAY
    no_delay: true

    # SO_KEEPALIVE
    keep_alive: true

    # Buffer sizes
    send_buffer_size: 65536
    recv_buffer_size: 65536
```

## Environment Variables

Override configuration with environment variables:

```bash
# Basic settings
export MOCKFORGE_MQTT_ENABLED=true
export MOCKFORGE_MQTT_PORT=1883
export MOCKFORGE_MQTT_HOST=0.0.0.0

# Connection limits
export MOCKFORGE_MQTT_MAX_CONNECTIONS=1000
export MOCKFORGE_MQTT_MAX_PACKET_SIZE=1048576

# TLS settings
export MOCKFORGE_MQTT_TLS_ENABLED=false
export MOCKFORGE_MQTT_TLS_CERT_PATH=/path/to/cert.pem
export MOCKFORGE_MQTT_TLS_KEY_PATH=/path/to/key.pem

# Authentication
export MOCKFORGE_MQTT_AUTH_ENABLED=true
export MOCKFORGE_MQTT_AUTH_METHOD=basic
```

## Configuration Validation

MockForge validates MQTT configuration on startup:

- **Port conflicts**: Checks if the configured port is available
- **Certificate validation**: Verifies TLS certificates exist and are valid
- **ACL consistency**: Ensures topic ACL rules don't conflict
- **Resource limits**: Validates connection and message limits are reasonable

## Configuration Examples

### Development Setup

```yaml
mqtt:
  enabled: true
  port: 1883
  host: "127.0.0.1"
  max_connections: 100
  log_connections: true
  log_messages: true
```

### Production Setup

```yaml
mqtt:
  enabled: true
  port: 1883
  host: "0.0.0.0"
  max_connections: 10000
  tls_enabled: true
  tls_port: 8883
  tls_cert_path: "/etc/ssl/certs/mqtt.crt"
  tls_key_path: "/etc/ssl/private/mqtt.key"
  auth_enabled: true
  auth_method: "jwt"
  metrics_enabled: true
```

### IoT Gateway

```yaml
mqtt:
  enabled: true
  port: 1883
  max_connections: 1000
  max_packet_size: 524288  # 512KB for sensor data
  keep_alive_secs: 300     # 5 minutes for battery-powered devices
  retained_messages_enabled: true
  max_total_retained: 5000
```

## Troubleshooting

### Common Issues

**High CPU Usage**
- Reduce `max_connections` or `worker_threads`
- Enable connection rate limiting
- Check for connection leaks

**Memory Issues**
- Lower `max_queued_messages` and `max_inflight_messages`
- Reduce `max_total_retained`
- Monitor retained message growth

**Connection Timeouts**
- Increase `keep_alive_secs`
- Check network connectivity
- Verify firewall settings

**TLS Handshake Failures**
- Verify certificate validity
- Check certificate chain
- Ensure correct certificate format (PEM)

## Next Steps

- [Getting Started](../getting-started.md) - Basic MQTT setup
- [Fixtures](fixtures.md) - Define MQTT mock scenarios
- [Examples](examples.md) - Real-world usage examples
