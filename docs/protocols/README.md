# MockForge Protocol Guides

This directory contains comprehensive guides for each protocol supported by MockForge.

## Available Protocols

| Protocol | Port | Use Case |
|----------|------|----------|
| [HTTP/HTTPS](../ENVIRONMENT_VARIABLES.md#core-configuration) | 3000 | REST APIs, webhooks, web services |
| [WebSocket](./WEBSOCKET.md) | 8080 | Real-time applications, live updates |
| [gRPC](./GRPC.md) | 50051 | Microservices, high-performance RPC |
| [MQTT](./MQTT.md) | 1883/8883 | IoT devices, pub/sub messaging |
| [AMQP](./AMQP.md) | 5672 | Message queues, enterprise messaging |
| SMTP | 2525 | Email testing |

## Quick Comparison

| Feature | HTTP | WebSocket | gRPC | MQTT | AMQP |
|---------|------|-----------|------|------|------|
| Connection Type | Request/Response | Persistent | Persistent | Persistent | Persistent |
| Message Format | Any | Text/Binary | Protobuf | Binary | Binary |
| Streaming | Limited | Bidirectional | All types | Pub/Sub | Pub/Sub |
| TLS Support | Yes | Yes | Yes | Yes | Yes |
| Best For | APIs | Real-time | Microservices | IoT | Enterprise |

## Configuration Hierarchy

All protocols share a common configuration structure:

```yaml
# mockforge.yaml

# HTTP (always enabled by default)
http:
  port: 3000
  host: "0.0.0.0"

# WebSocket
websocket:
  enabled: true
  port: 8080

# gRPC
grpc:
  enabled: true
  port: 50051

# MQTT
mqtt:
  enabled: true
  port: 1883

# AMQP
amqp:
  enabled: true
  port: 5672

# SMTP
smtp:
  enabled: true
  port: 2525
```

## Common Patterns

### Multi-Protocol Testing

```yaml
# Test a microservices architecture
http:
  port: 3000

grpc:
  enabled: true
  port: 50051

amqp:
  enabled: true
  port: 5672
```

### IoT Simulation

```yaml
mqtt:
  enabled: true
  port: 1883
  tls:
    enabled: true
    port: 8883

http:
  port: 3000  # For device management API
```

### Real-Time Applications

```yaml
http:
  port: 3000

websocket:
  enabled: true
  port: 8080
  path: "/ws"
```

## CLI Commands

```bash
# Start with specific protocols
mockforge serve --http --grpc --mqtt

# List protocol status
mockforge status

# Protocol-specific commands
mockforge mqtt status
mockforge amqp queues
mockforge grpc services
mockforge ws connections
```

## See Also

- [Environment Variables Reference](../ENVIRONMENT_VARIABLES.md)
- [Getting Started Guide](../getting-started.md)
- [Configuration Guide](../configuration.md)
