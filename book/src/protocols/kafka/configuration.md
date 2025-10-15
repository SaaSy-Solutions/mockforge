# Kafka Configuration

The Kafka mock server supports extensive configuration options to simulate various Kafka cluster behaviors.

## Server Configuration

```yaml
server:
  kafka:
    # Enable/disable Kafka server
    enabled: true

    # Network settings
    port: 9092
    host: "0.0.0.0"

    # Broker identity
    broker_id: 1

    # Connection limits
    max_connections: 1000

    # Topic management
    auto_create_topics: true
    default_partitions: 3
    default_replication_factor: 1

    # Data retention (simulated)
    log_retention_ms: 604800000  # 7 days
    log_segment_bytes: 1073741824  # 1GB

    # Fixture directory
    fixtures_dir: "./fixtures/kafka"
```

## Environment Variables

All configuration options can be overridden with environment variables:

```bash
export MOCKFORGE_KAFKA_PORT=9093
export MOCKFORGE_KAFKA_HOST=127.0.0.1
export MOCKFORGE_KAFKA_MAX_CONNECTIONS=500
```

## Advanced Configuration

### High Availability Simulation

```yaml
server:
  kafka:
    # Simulate multiple brokers
    broker_id: 1
    cluster_id: "mockforge-cluster"

    # Controller settings
    controller_id: 1
```

### Performance Tuning

```yaml
server:
  kafka:
    # Connection pooling
    max_connections: 10000

    # Buffer sizes
    log_segment_bytes: 536870912  # 512MB

    # Retention policies
    log_retention_ms: 86400000  # 24 hours
```

### Security Settings

```yaml
server:
  kafka:
    # Authentication (future feature)
    # sasl_enabled: false
    # ssl_enabled: false

    # Authorization
    # acl_enabled: false
```

## Configuration Validation

MockForge validates your Kafka configuration on startup:

```bash
mockforge kafka serve --config kafka-config.yaml
```

Common validation errors:
- Invalid port numbers
- Missing required directories
- Incompatible settings