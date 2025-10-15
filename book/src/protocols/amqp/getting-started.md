# Getting Started with AMQP

MockForge provides comprehensive AMQP 0.9.1 protocol support, allowing you to mock RabbitMQ-compatible message brokers for testing and development.

## Quick Start

### 1. Enable AMQP in Configuration

Add AMQP configuration to your `mockforge.yaml`:

```yaml
amqp:
  enabled: true
  port: 5672
  host: "127.0.0.1"
  fixtures_dir: "./fixtures/amqp"
```

### 2. Start the AMQP Broker

```bash
mockforge amqp serve --port 5672
```

### 3. Connect with Your Application

Use any AMQP 0.9.1 compatible client:

```python
# Python with pika
import pika

connection = pika.BlockingConnection(pika.ConnectionParameters('localhost', 5672))
channel = connection.channel()

# Declare exchange and queue
channel.exchange_declare(exchange='orders', exchange_type='topic')
channel.queue_declare(queue='orders.new')

# Publish a message
channel.basic_publish(
    exchange='orders',
    routing_key='order.created',
    body='{"order_id": "123", "amount": 99.99}'
)

connection.close()
```

## Supported Features

- **Exchange Types**: Direct, Fanout, Topic, Headers
- **Queue Management**: Durable queues, TTL, dead letter exchanges
- **Message Properties**: Content type, delivery mode, priority, headers
- **Topic Routing**: Wildcard patterns (* and #)
- **Fixture System**: YAML-based mock data and auto-publishing

## Example Workflow

1. **Define Fixtures**: Create YAML files describing your message flows
2. **Load Fixtures**: MockForge automatically sets up exchanges, queues, and bindings
3. **Auto-Publish**: Configure automatic message generation
4. **Test Integration**: Connect your application and verify behavior

See the [Configuration](configuration.md) and [Fixtures](fixtures.md) guides for detailed setup.