# Kafka Fixtures

Fixtures enable template-based message generation for testing Kafka-based applications.

## Fixture Structure

Create fixture files in your `fixtures_dir` (default: `./fixtures/kafka/`):

```yaml
# fixtures/kafka/order-events.yaml
identifier: "order-created-events"
name: "Order Created Events"
description: "Generates order creation events"

topic: "orders.created"
partition: null  # Distribute across all partitions

key_pattern: "order-{{uuid}}"

value_template:
  event_type: "order.created"
  event_id: "{{uuid}}"
  timestamp: "{{now}}"
  order_id: "{{uuid}}"
  customer_id: "{{faker.uuid}}"
  items:
    - product_id: "{{faker.uuid}}"
      quantity: "{{faker.int 1 10}}"
      price: "{{faker.float 10.0 1000.0}}"
  total: "{{faker.float 10.0 5000.0}}"
  status: "pending"
  created_at: "{{now}}"

headers:
  event_type: "order.created"
  version: "1.0"
  source: "mockforge"

auto_produce:
  enabled: true
  rate_per_second: 10
  duration_seconds: null  # Infinite
  total_count: null       # Infinite
```

## Template Variables

### Built-in Variables

- `{{uuid}}` - Random UUID
- `{{now}}` - Current timestamp (ISO 8601)
- `{{timestamp}}` - Current timestamp (milliseconds)
- `{{random.int min max}}` - Random integer
- `{{random.float min max}}` - Random float

### Faker Variables

- `{{faker.uuid}}` - Faker UUID
- `{{faker.name}}` - Random name
- `{{faker.email}}` - Random email
- `{{faker.address}}` - Random address
- `{{faker.company}}` - Random company
- `{{faker.int min max}}` - Faker integer
- `{{faker.float min max}}` - Faker float

## Auto-Produce Configuration

### Continuous Production

```yaml
auto_produce:
  enabled: true
  rate_per_second: 5
  duration_seconds: null  # Run indefinitely
  total_count: null       # Unlimited messages
```

### Limited Production

```yaml
auto_produce:
  enabled: true
  rate_per_second: 10
  duration_seconds: 3600  # 1 hour
  total_count: 1000       # Max 1000 messages
```

## Fixture Management

### Loading Fixtures

```bash
# Load all fixtures from directory
mockforge kafka fixtures load ./fixtures/kafka/

# List loaded fixtures
mockforge kafka fixtures list
```

### Auto-Produce Control

```bash
# Start auto-producing messages
mockforge kafka fixtures start-auto-produce

# Stop auto-producing messages
mockforge kafka fixtures stop-auto-produce
```

## Advanced Templates

### Conditional Logic

```yaml
value_template:
  order_type: "{{#if (eq status 'premium')}}premium{{else}}standard{{/if}}"
  priority: "{{#if (gt total 1000)}}high{{else}}normal{{/if}}"
```

### Arrays and Objects

```yaml
value_template:
  items: "{{#each (range 1 5)}}{{faker.product}} {{/each}}"
  metadata:
    source: "ecommerce"
    version: "2.1"
    tags: ["order", "created", "test"]
```

### Custom Functions

```yaml
value_template:
  order_id: "ORD-{{pad (random.int 1000 9999) 4 '0'}}"
  tracking_id: "TRK-{{uppercase (uuid)}}"
```

## Testing with Fixtures

### Consumer Lag Simulation

```yaml
# fixtures/kafka/slow-consumer-test.yaml
identifier: "slow-consumer-simulation"
name: "Simulate Consumer Lag"
description: "Tests lag monitoring"

topic: "orders.created"

auto_produce:
  enabled: true
  rate_per_second: 100  # Fast production

# Simulate slow consumer
consumer_groups:
  - group_id: "order-processor"
    simulated_lag_messages: 1000
    simulated_processing_delay_ms: 500
```

### Error Scenario Testing

```yaml
# fixtures/kafka/error-events.yaml
identifier: "order-failed-events"
name: "Order Failed Events"

topic: "orders.failed"

value_template:
  event_type: "order.failed"
  order_id: "{{uuid}}"
  error_code: "{{random.pick 'INSUFFICIENT_FUNDS' 'INVALID_PAYMENT' 'OUT_OF_STOCK'}}"
  error_message: "{{faker.sentence}}"
  retry_count: "{{random.int 0 3}}"
```

## Best Practices

1. **Organize by Domain**: Group fixtures by business domain
2. **Use Descriptive Names**: Clear fixture identifiers and names
3. **Version Templates**: Include version information in headers
4. **Test Edge Cases**: Include fixtures for error conditions
5. **Monitor Performance**: Adjust auto-produce rates based on testing needs