# AMQP Configuration

Configure MockForge's AMQP broker to match your testing requirements.

## Basic Configuration

```yaml
amqp:
  enabled: true
  port: 5672
  host: "127.0.0.1"
  fixtures_dir: "./fixtures/amqp"
  connections:
    max_connections: 100
    connection_timeout: 30
  fixtures:
    auto_load: true
    watch_changes: true
```

## Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | boolean | `false` | Enable AMQP protocol support |
| `port` | number | `5672` | Port to listen on (standard AMQP port) |
| `host` | string | `"127.0.0.1"` | Host address to bind to |
| `fixtures_dir` | string | `"./fixtures/amqp"` | Directory containing fixture files |
| `max_connections` | number | `100` | Maximum concurrent connections |
| `connection_timeout` | number | `30` | Connection timeout in seconds |

## Environment Variables

You can override configuration using environment variables:

```bash
export MOCKFORGE_AMQP_PORT=5673
export MOCKFORGE_AMQP_HOST=0.0.0.0
export MOCKFORGE_AMQP_FIXTURES_DIR=./custom/fixtures
```

## Advanced Configuration

### Connection Limits

```yaml
amqp:
  connections:
    max_connections: 1000
    max_channels_per_connection: 10
    heartbeat_interval: 60
    connection_timeout: 30
```

### Queue Settings

```yaml
amqp:
  queues:
    default_max_length: 10000
    default_message_ttl: 3600000  # 1 hour
    default_expires: 86400000     # 24 hours
```

### Exchange Settings

```yaml
amqp:
  exchanges:
    max_exchanges: 100
    default_auto_delete: false
```

## Command Line Usage

Override configuration from command line:

```bash
# Start with custom port
mockforge amqp serve --port 5673

# Use custom config file
mockforge amqp serve --config custom-amqp.yaml

# Bind to all interfaces
mockforge amqp serve --host 0.0.0.0
```

## Docker Configuration

When running in Docker, use appropriate host settings:

```yaml
amqp:
  host: "0.0.0.0"  # Listen on all interfaces
  port: 5672
```

And map the port in docker-compose:

```yaml
services:
  mockforge:
    ports:
      - "5672:5672"
```