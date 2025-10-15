# Getting Started with Kafka Mock Server

MockForge provides a comprehensive Kafka mock server for testing event-driven architectures, microservices communication, and stream processing applications.

## Quick Start

### Starting the Kafka Broker

```bash
# Start Kafka broker on default port 9092
mockforge kafka serve

# Start on custom port
mockforge kafka serve --port 9093

# Start with custom configuration
mockforge kafka serve --config kafka-config.yaml
```

### Basic Topic Operations

```bash
# Create a topic
mockforge kafka topic create orders --partitions 3

# List topics
mockforge kafka topic list

# Describe a topic
mockforge kafka topic describe orders
```

### Producing and Consuming Messages

```bash
# Produce a message
mockforge kafka produce --topic orders --value '{"id": "123", "total": 99.99}'

# Produce with key
mockforge kafka produce --topic orders --key "order-123" --value '{"id": "123"}'

# Consume messages
mockforge kafka consume --topic orders --group test-group

# Consume from specific partition
mockforge kafka consume --topic orders --partition 0 --from beginning
```

## Configuration

Create a `kafka-config.yaml` file:

```yaml
server:
  kafka:
    enabled: true
    port: 9092
    host: "0.0.0.0"
    broker_id: 1
    max_connections: 1000
    fixtures_dir: "./fixtures/kafka"
    auto_create_topics: true
    default_partitions: 3
    default_replication_factor: 1
```

## Next Steps

- [Configuration Guide](configuration.md) - Detailed configuration options
- [Topics and Partitions](topics-and-partitions.md) - Working with topics
- [Producers](producers.md) - Message production
- [Consumers](consumers.md) - Message consumption
- [Fixtures](fixtures.md) - Template-based message generation
- [Testing Patterns](testing-patterns.md) - Common testing scenarios