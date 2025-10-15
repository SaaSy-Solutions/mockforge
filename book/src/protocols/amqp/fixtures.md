# AMQP Fixtures

Define mock AMQP message flows using YAML-based fixtures.

## Fixture Structure

```yaml
identifier: "order-processing"
name: "Order Processing Workflow"
description: "Simulates order processing message flow"

exchanges:
  - name: "orders"
    type: "topic"
    durable: true

  - name: "dlx.orders"
    type: "fanout"
    durable: true

queues:
  - name: "orders.new"
    durable: true
    properties:
      max_length: 10000
      message_ttl: 3600000
      dead_letter_exchange: "dlx.orders"
    message_template:
      order_id: "{{uuid}}"
      customer_id: "{{faker.uuid}}"
      items: []
      total: "{{faker.float 10.0 1000.0}}"
      status: "new"
      created_at: "{{now}}"

  - name: "orders.processing"
    durable: true
    message_template:
      order_id: "{{uuid}}"
      status: "processing"
      updated_at: "{{now}}"

  - name: "orders.dlq"
    durable: true

bindings:
  - exchange: "orders"
    queue: "orders.new"
    routing_key: "order.created"

  - exchange: "orders"
    queue: "orders.processing"
    routing_key: "order.processing"

  - exchange: "dlx.orders"
    queue: "orders.dlq"
    routing_key: "#"

auto_publish:
  enabled: true
  exchange: "orders"
  routing_key: "order.created"
  rate_per_second: 5
  message_template:
    event_type: "order.created"
    order_id: "{{uuid}}"
    customer_id: "{{faker.uuid}}"
    total: "{{faker.float 10.0 1000.0}}"
    timestamp: "{{now}}"
```

## Exchange Configuration

### Exchange Types

- **direct**: Routes messages to queues with exact routing key match
- **fanout**: Routes messages to all bound queues
- **topic**: Routes messages using pattern matching (* and #)
- **headers**: Routes messages based on header values

### Exchange Properties

```yaml
exchanges:
  - name: "my-exchange"
    type: "topic"
    durable: true          # Survives broker restart
    auto_delete: false     # Delete when no bindings remain
    internal: false        # Can be published to directly
```

## Queue Configuration

### Basic Queue

```yaml
queues:
  - name: "my-queue"
    durable: true
    exclusive: false
    auto_delete: false
```

### Advanced Queue Properties

```yaml
queues:
  - name: "advanced-queue"
    durable: true
    properties:
      max_length: 10000           # Maximum messages in queue
      max_length_bytes: 1048576   # Maximum queue size in bytes
      message_ttl: 3600000        # Message time-to-live (ms)
      expires: 86400000           # Queue expires after (ms)
      dead_letter_exchange: "dlx" # Dead letter exchange
      dead_letter_routing_key: "expired"  # Dead letter routing key
      max_priority: 10            # Maximum priority level
```

### Message Templates

Use template variables for dynamic content:

```yaml
message_template:
  id: "{{uuid}}"
  timestamp: "{{now}}"
  user_id: "{{faker.uuid}}"
  amount: "{{faker.float 10.0 1000.0}}"
  status: "{{choice 'pending' 'processing' 'completed'}}"
```

#### Template Functions

- `{{uuid}}` - Generate UUID
- `{{now}}` - Current timestamp
- `{{faker.uuid}}` - Faker UUID
- `{{faker.float min max}}` - Random float
- `{{faker.int min max}}` - Random integer
- `{{faker.word}}` - Random word
- `{{choice 'a' 'b' 'c'}}` - Random choice
- `{{sequence}}` - Incrementing sequence number

## Bindings

Define routing between exchanges and queues:

```yaml
bindings:
  - exchange: "orders"
    queue: "orders.new"
    routing_key: "order.created"

  # Topic pattern matching
  - exchange: "logs"
    queue: "error.logs"
    routing_key: "*.error.*"

  # Wildcard patterns
  - exchange: "events"
    queue: "all.events"
    routing_key: "#"
```

## Auto-Publish

Automatically generate messages at specified intervals:

```yaml
auto_publish:
  enabled: true
  exchange: "orders"
  routing_key: "order.created"
  rate_per_second: 2
  message_template:
    event_type: "order.created"
    order_id: "{{uuid}}"
    amount: "{{faker.float 50.0 500.0}}"
    timestamp: "{{now}}"
```

## Loading Fixtures

### From Directory

```bash
mockforge amqp fixtures load ./fixtures/amqp/
```

### From CLI

```bash
# List loaded fixtures
mockforge amqp fixtures list

# Start auto-publishing
mockforge amqp fixtures start-auto-publish

# Stop auto-publishing
mockforge amqp fixtures stop-auto-publish
```

## Example Fixtures

### E-commerce Order Flow

```yaml
identifier: "ecommerce-orders"
name: "E-commerce Order Processing"

exchanges:
  - name: "orders"
    type: "topic"
    durable: true

queues:
  - name: "order.validation"
    durable: true
    message_template:
      order_id: "{{uuid}}"
      items: "{{faker.array 1 5}}"
      total: "{{faker.float 20.0 1000.0}}"

  - name: "order.payment"
    durable: true

  - name: "order.fulfillment"
    durable: true

bindings:
  - exchange: "orders"
    queue: "order.validation"
    routing_key: "order.placed"

  - exchange: "orders"
    queue: "order.payment"
    routing_key: "order.validated"

  - exchange: "orders"
    queue: "order.fulfillment"
    routing_key: "order.paid"

auto_publish:
  enabled: true
  exchange: "orders"
  routing_key: "order.placed"
  rate_per_second: 1
```

### IoT Sensor Data

```yaml
identifier: "iot-sensors"
name: "IoT Sensor Data Stream"

exchanges:
  - name: "sensors"
    type: "topic"
    durable: true

queues:
  - name: "temperature.readings"
    durable: true

  - name: "humidity.readings"
    durable: true

bindings:
  - exchange: "sensors"
    queue: "temperature.readings"
    routing_key: "sensor.*.temperature"

  - exchange: "sensors"
    queue: "humidity.readings"
    routing_key: "sensor.*.humidity"

auto_publish:
  enabled: true
  exchange: "sensors"
  routing_key: "sensor.{{faker.int 1 100}}.temperature"
  rate_per_second: 10
  message_template:
    sensor_id: "{{faker.int 1 100}}"
    value: "{{faker.float 20.0 30.0}}"
    unit: "celsius"
    timestamp: "{{now}}"
```