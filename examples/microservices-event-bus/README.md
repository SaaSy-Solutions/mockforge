# Microservices Event-Driven Architecture Example

This example demonstrates a complete microservices architecture using **Kafka**, **MQTT**, and **AMQP** protocols for different messaging patterns.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    E-Commerce System                             │
└─────────────────────────────────────────────────────────────────┘

┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│ Order Service│────▶│ Kafka Broker │────▶│  Inventory   │
│  (HTTP API)  │     │   (Events)   │     │   Service    │
└──────────────┘     └──────────────┘     └──────────────┘
                            │                      │
                            ├──────────────────────┤
                            ▼                      ▼
                     ┌──────────────┐     ┌──────────────┐
                     │ Notification │     │  Analytics   │
                     │   Service    │     │   Service    │
                     └──────────────┘     └──────────────┘
                            │
                            ▼
                     ┌──────────────┐
                     │ AMQP Broker  │
                     │  (Routing)   │
                     └──────────────┘
                            │
                    ┌───────┴───────┐
                    ▼               ▼
            ┌──────────────┐ ┌──────────────┐
            │ Email Queue  │ │  SMS Queue   │
            └──────────────┘ └──────────────┘

┌──────────────┐     ┌──────────────┐
│ IoT Devices  │────▶│ MQTT Broker  │
│  (Sensors)   │     │ (Real-time)  │
└──────────────┘     └──────────────┘
                            │
                            ▼
                     ┌──────────────┐
                     │  Dashboard   │
                     │   Service    │
                     └──────────────┘
```

## Protocols Used

### Kafka - Event Streaming
- **Purpose**: Core event bus for domain events
- **Topics**:
  - `orders.created` - New order events
  - `orders.status-updated` - Order status changes
  - `inventory.updated` - Stock level changes
  - `payments.processed` - Payment confirmations

### MQTT - Real-Time Pub/Sub
- **Purpose**: IoT device telemetry and real-time updates
- **Topics**:
  - `warehouse/sensors/temperature`
  - `warehouse/sensors/humidity`
  - `alerts/critical`
  - `dashboard/updates`

### AMQP - Task Queues & Routing
- **Purpose**: Notification routing and task distribution
- **Exchanges**:
  - `notifications` (topic exchange)
  - `tasks` (direct exchange)
- **Queues**:
  - `email.queue`
  - `sms.queue`
  - `push.queue`

## Quick Start

### 1. Start MockForge with All Protocols

```bash
# Start MockForge
mockforge serve \
  --config mockforge-config.yaml \
  --kafka-port 9092 \
  --mqtt-port 1883 \
  --amqp-port 5672

# Or using Docker
docker-compose up -d
```

### 2. Configure Services

See `mockforge-config.yaml` for complete configuration.

### 3. Load Fixtures

```bash
# Kafka fixtures
mockforge kafka fixtures load ./fixtures/kafka/

# MQTT fixtures
mockforge mqtt fixtures load ./fixtures/mqtt/

# AMQP fixtures
mockforge amqp fixtures load ./fixtures/amqp/
```

## Use Cases

### Use Case 1: Order Processing Flow (Kafka)

**Scenario**: Customer places an order

1. **Order Service** publishes `order.created` event to Kafka
2. **Inventory Service** consumes event and reserves stock
3. **Payment Service** processes payment and publishes `payment.processed`
4. **Notification Service** sends confirmation email
5. **Analytics Service** tracks order metrics

**Try it:**

```bash
# Produce an order event
mockforge kafka produce \
  --topic orders.created \
  --key "order-12345" \
  --value '{
    "order_id": "12345",
    "customer_id": "cust-789",
    "items": [
      {"product_id": "prod-1", "quantity": 2}
    ],
    "total": 99.99,
    "status": "pending"
  }'

# Consume from inventory service topic
mockforge kafka consume \
  --topic inventory.updated \
  --group inventory-service
```

### Use Case 2: Warehouse Monitoring (MQTT)

**Scenario**: Temperature sensors in warehouse

1. **Sensors** publish temperature readings to MQTT
2. **Alert System** subscribes and triggers critical alerts
3. **Dashboard** displays real-time metrics

**Try it:**

```bash
# Publish sensor data
mockforge mqtt publish \
  --topic "warehouse/sensors/temperature/zone-1" \
  --payload '{"temp": 25.5, "unit": "celsius"}' \
  --qos 1

# Subscribe to all sensor data
mockforge mqtt subscribe --topic "warehouse/sensors/#" -v
```

### Use Case 3: Notification Routing (AMQP)

**Scenario**: Send order confirmation via multiple channels

1. **Notification Service** publishes to `notifications` exchange
2. **Routing Key** determines delivery method (email, sms, push)
3. **Workers** consume from specific queues

**Try it:**

```bash
# Declare exchange and queues
mockforge amqp exchange declare \
  --name notifications \
  --type topic

mockforge amqp queue declare --name email.queue
mockforge amqp queue bind \
  --exchange notifications \
  --queue email.queue \
  --routing-key "notification.email.*"

# Publish notification
mockforge amqp publish \
  --exchange notifications \
  --routing-key "notification.email.order" \
  --body '{
    "type": "order_confirmation",
    "recipient": "customer@example.com",
    "order_id": "12345"
  }'

# Consume from email queue
mockforge amqp consume --queue email.queue
```

## Testing Scenarios

### Scenario 1: Happy Path - Complete Order Flow

```bash
# Run the complete flow
./scripts/test-happy-path.sh
```

**Expected Flow:**
1. Order created → Kafka `orders.created`
2. Inventory reserved → Kafka `inventory.updated`
3. Payment processed → Kafka `payments.processed`
4. Email sent → AMQP `email.queue`
5. Analytics updated → Kafka consumer group

### Scenario 2: Failure Handling - Out of Stock

```bash
# Run out-of-stock scenario
./scripts/test-out-of-stock.sh
```

**Expected Flow:**
1. Order created → Kafka
2. Inventory check fails
3. Order cancelled → Kafka `orders.status-updated`
4. Refund processed → Kafka `payments.refunded`
5. Notification sent → AMQP

### Scenario 3: Real-Time Monitoring - Critical Alert

```bash
# Simulate temperature spike
./scripts/test-temperature-alert.sh
```

**Expected Flow:**
1. Sensor publishes high temp → MQTT
2. Alert triggered → MQTT `alerts/critical`
3. Email sent → AMQP `email.queue`
4. Dashboard updated → MQTT `dashboard/updates`

## Monitoring & Metrics

### View All Metrics

```bash
curl http://localhost:9080/__mockforge/metrics

# Output includes:
# kafka_messages_produced_total{topic="orders.created"} 1234
# mqtt_messages_published_total{topic="warehouse/sensors/temperature"} 567
# amqp_messages_published_total{exchange="notifications"} 890
```

### Kafka Consumer Lag

```bash
mockforge kafka consumer-groups describe \
  --group inventory-service

# Shows lag per partition
```

### MQTT Client Connections

```bash
mockforge mqtt stats

# Shows:
# - Active connections
# - Subscription count
# - Message rates
```

## Configuration Files

### `mockforge-config.yaml`

See the main configuration file for:
- Protocol port settings
- Auto-produce configurations
- Fixture directories
- Metrics settings

### Fixture Files

- `fixtures/kafka/` - Kafka message templates
- `fixtures/mqtt/` - MQTT topics and payloads
- `fixtures/amqp/` - AMQP exchanges and queues

## Client Code Examples

### Python - Kafka Consumer

```python
from confluent_kafka import Consumer

consumer = Consumer({
    'bootstrap.servers': 'localhost:9092',
    'group.id': 'inventory-service',
    'auto.offset.reset': 'earliest'
})

consumer.subscribe(['orders.created'])

for msg in consumer:
    order = json.loads(msg.value().decode())
    print(f"Processing order {order['order_id']}")
    # Reserve inventory logic here
```

### JavaScript - MQTT Subscriber

```javascript
const mqtt = require('mqtt');
const client = mqtt.connect('mqtt://localhost:1883');

client.subscribe('warehouse/sensors/#');

client.on('message', (topic, message) => {
  const data = JSON.parse(message.toString());
  if (data.temp > 30) {
    console.log(`ALERT: High temperature in ${topic}`);
  }
});
```

### Python - AMQP Consumer

```python
import pika

connection = pika.BlockingConnection(
    pika.ConnectionParameters('localhost')
)
channel = connection.channel()

def callback(ch, method, properties, body):
    notification = json.loads(body)
    print(f"Sending email to {notification['recipient']}")
    # Email sending logic here
    ch.basic_ack(delivery_tag=method.delivery_tag)

channel.basic_consume(
    queue='email.queue',
    on_message_callback=callback
)

channel.start_consuming()
```

## Performance Testing

### Load Test - High Throughput

```bash
# Generate 10,000 orders/second
./scripts/load-test.sh --rate 10000 --duration 60

# Results:
# - Kafka throughput
# - MQTT latency
# - AMQP queue depth
```

### Stress Test - Connection Limits

```bash
# Test max connections
./scripts/stress-test.sh --connections 5000
```

## Troubleshooting

### Kafka Not Connecting

```bash
# Check broker status
mockforge kafka topics list

# Verify network
telnet localhost 9092
```

### MQTT Connection Refused

```bash
# Test with mosquitto client
mosquitto_pub -h localhost -p 1883 -t test -m "hello"
```

### AMQP Queue Not Receiving

```bash
# Check queue bindings
mockforge amqp queue list
mockforge amqp bindings list --queue email.queue
```

## Next Steps

1. **Scale Up**: Run multiple instances of MockForge for high availability
2. **Add Chaos**: Introduce network latency, broker failures
3. **Custom Fixtures**: Create domain-specific message templates
4. **Integration Tests**: Write automated E2E tests using real clients
5. **Grafana Dashboards**: Visualize metrics from Prometheus

## Resources

- [Kafka Documentation](../../ASYNC_PROTOCOLS.md#kafka-mock-broker)
- [MQTT Documentation](../../ASYNC_PROTOCOLS.md#mqtt-broker)
- [AMQP Documentation](../../ASYNC_PROTOCOLS.md#amqp-broker)
- [Docker Compose Setup](../../docker-compose.yml)
