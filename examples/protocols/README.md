# Protocol Examples

This directory contains example fixture files for MockForge's protocol support, including upcoming protocol implementations.

## Current Protocols

### HTTP/REST
See `../http/` for HTTP examples. MockForge has comprehensive HTTP support with OpenAPI-driven mocking.

**Quick Start:**
```bash
mockforge serve --spec examples/openapi-demo.json
curl http://localhost:3000/ping
```

### gRPC
See `../grpc/` for gRPC examples with .proto files.

**Quick Start:**
```bash
mockforge serve --grpc --proto-dir examples/grpc/proto
grpcurl -plaintext localhost:50051 list
```

### WebSocket
See `../ws/` for WebSocket replay examples.

**Quick Start:**
```bash
mockforge serve --ws --ws-replay-file examples/ws-demo.jsonl
# Connect with websocat, wscat, or browser
```

### GraphQL
See `../graphql/` for GraphQL schema examples.

**Quick Start:**
```bash
mockforge serve --graphql --schema examples/graphql/schema.graphql
curl -X POST http://localhost:3000/graphql -d '{"query": "{ hello }"}'
```

## Upcoming Protocols

The following examples demonstrate the fixture format for protocols currently in development.

### SMTP - Email Server Mocking

**Status:** 🚧 In Development
**Example:** [`smtp/welcome-email.yaml`](./smtp/welcome-email.yaml)

Mock SMTP server for testing email sending and receiving.

**Use Cases:**
- Test email notifications
- Verify email formatting
- Test auto-reply systems
- Validate email templates

**Example Configuration:**
```yaml
# config.yaml
smtp:
  enabled: true
  port: 1025
  host: "0.0.0.0"
  fixtures_dir: "./examples/protocols/smtp"
```

**Usage (when available):**
```bash
# Start SMTP server
mockforge serve --smtp

# Send test email
echo "Subject: Test\n\nHello World" | \
  mail -s "Test" -S smtp=localhost:1025 user@example.com

# View received emails
mockforge mailbox list
mockforge mailbox show <id>
```

**Features:**
- Auto-reply configuration
- Template-based email generation
- Mailbox storage (in-memory/disk)
- Email validation
- Attachment handling

---

### MQTT - IoT Protocol Mocking

**Status:** 🚧 In Development
**Example:** [`mqtt/iot-sensors.yaml`](./mqtt/iot-sensors.yaml)

Mock MQTT broker for testing IoT devices and pub/sub systems.

**Use Cases:**
- Test IoT device communication
- Simulate sensor data streams
- Test MQTT client applications
- Validate pub/sub patterns

**Example Configuration:**
```yaml
# config.yaml
mqtt:
  enabled: true
  port: 1883
  host: "0.0.0.0"
  fixtures_dir: "./examples/protocols/mqtt"
```

**Usage (when available):**
```bash
# Start MQTT broker
mockforge serve --mqtt

# Subscribe to topics
mockforge mqtt subscribe --topic "sensors/#"

# Publish message
mockforge mqtt publish \
  --topic "sensors/temperature/room1" \
  --payload '{"temp": 22.5, "unit": "celsius"}'

# Load fixtures (auto-publish)
mockforge mqtt load-fixtures ./examples/protocols/mqtt/
```

**Features:**
- QoS levels 0, 1, 2
- Retained messages
- Last Will and Testament (LWT)
- Topic wildcards
- Auto-publish scenarios
- Data drift simulation
- MQTT 5.0 support

---

### FTP - File Transfer Mocking

**Status:** 🚧 In Development
**Example:** [`ftp/file-server.yaml`](./ftp/file-server.yaml)

Mock FTP server with virtual file system for testing file transfers.

**Use Cases:**
- Test FTP client applications
- Validate file upload/download logic
- Test directory operations
- Simulate large file transfers

**Example Configuration:**
```yaml
# config.yaml
ftp:
  enabled: true
  port: 2121
  host: "0.0.0.0"
  fixtures_dir: "./examples/protocols/ftp"
```

**Usage (when available):**
```bash
# Start FTP server
mockforge serve --ftp

# Connect with FTP client
ftp localhost 2121
# Username: testuser
# Password: testpass

# Or use command-line tools
curl -u testuser:testpass ftp://localhost:2121/downloads/readme.txt

# List virtual files
mockforge ftp ls /
mockforge ftp cat /downloads/readme.txt
```

**Features:**
- Virtual file system
- Template-based file generation
- Dynamic file creation
- Upload validation
- Bandwidth throttling
- Quota management
- Anonymous and authenticated access

---

### Kafka - Event Streaming Mocking

**Status:** 🚧 In Development
**Example:** [`kafka/order-events.yaml`](./kafka/order-events.yaml)

Mock Kafka broker for testing event-driven architectures.

**Use Cases:**
- Test Kafka consumers/producers
- Validate event processing logic
- Simulate consumer lag
- Test stream processing applications

**Example Configuration:**
```yaml
# config.yaml
kafka:
  enabled: true
  port: 9092
  host: "0.0.0.0"
  fixtures_dir: "./examples/protocols/kafka"
```

**Usage (when available):**
```bash
# Start Kafka mock broker
mockforge serve --kafka

# List topics
mockforge kafka topic list

# Create topic
mockforge kafka topic create orders --partitions 3

# Produce message
mockforge kafka produce \
  --topic orders \
  --key "order-1" \
  --value '{"id": 1, "total": 99.99}'

# Consume messages
mockforge kafka consume \
  --topic orders \
  --group test-consumer

# Load fixtures (auto-produce scenarios)
mockforge kafka load-fixtures ./examples/protocols/kafka/

# Simulate consumer lag
mockforge kafka simulate-lag \
  --group test-consumer \
  --topic orders \
  --lag 1000
```

**Features:**
- Multiple topics and partitions
- Consumer groups
- Message keys and headers
- State machine-based scenarios
- Consumer lag simulation
- Broker failure simulation
- Event relationships
- Metrics and monitoring

---

## Fixture Format Overview

All protocol fixtures follow a consistent YAML structure:

```yaml
fixture:
  name: "Fixture Name"
  description: "Description of what this fixture does"
  protocol: protocol_name  # smtp, mqtt, ftp, kafka, etc.

# Protocol-specific configuration
{protocol_config}

# Common sections across protocols:

# Response/message configuration
response:
  # Static or template-based responses
  template: "..."

# Behavior simulation
behavior:
  delay_ms: 100
  failure_rate: 0.01

# Validation rules
validation:
  # Protocol-specific validation

# Monitoring and logging
monitoring:
  enabled: true
```

## Template Engine Support

All fixtures support MockForge's template engine for dynamic data generation:

```yaml
# Basic templates
message: "Hello, {{uuid}}"

# Faker integration
email: "{{faker.email}}"
name: "{{faker.name}}"
temperature: "{{faker.float 15.0 30.0}}"

# Date/time functions
timestamp: "{{now}}"
future: "{{now+1h}}"
past: "{{now-7d}}"

# Conditionals and loops
{{#each (range 1 101)}}
  id: {{this}}
  name: "{{faker.name}}"
{{/each}}

# Random selection
category: "{{faker.randomChoice ['A', 'B', 'C']}}"
```

See [Template Documentation](../../docs/TEMPLATING.md) for full reference.

## Protocol Comparison

| Feature | SMTP | MQTT | FTP | Kafka |
|---------|------|------|-----|-------|
| **Complexity** | Low | Medium | Medium | High |
| **Message Pattern** | Request-Response | Pub/Sub | Request-Response | Pub/Sub |
| **State Management** | Minimal | Topics | File System | Topics + Partitions |
| **Auto-Generation** | ✅ | ✅ | ✅ | ✅ |
| **Relationships** | - | - | File hierarchy | Event chains |
| **Failure Simulation** | ✅ | ✅ | ✅ | ✅ |
| **Metrics** | ✅ | ✅ | ✅ | ✅ |

## Testing Workflow

### 1. Start MockForge with Protocol

```bash
# Single protocol
mockforge serve --mqtt --mqtt-port 1883

# Multiple protocols
mockforge serve \
  --http \
  --mqtt \
  --kafka \
  --config config.yaml
```

### 2. Load Fixtures

```bash
# Load protocol-specific fixtures
mockforge {protocol} load-fixtures ./examples/protocols/{protocol}/

# Or specify in config.yaml
{protocol}:
  fixtures_dir: "./examples/protocols/{protocol}"
```

### 3. Test Your Application

```bash
# Your application connects to MockForge protocols
# Example: MQTT client connects to localhost:1883
# Example: Kafka consumer connects to localhost:9092
```

### 4. Verify and Monitor

```bash
# View logs
mockforge logs --protocol mqtt

# Check metrics
mockforge metrics --protocol kafka

# View captured data
mockforge {protocol} inspect
```

## Development Status

### ✅ Completed Protocols
- HTTP/REST
- gRPC
- WebSocket
- GraphQL

### 🚧 In Development
- SMTP (Expected: Q2 2025)
- MQTT (Expected: Q3 2025)
- FTP (Expected: Q3 2025)
- Kafka (Expected: Q4 2025)

### 🔮 Planned
- RabbitMQ/AMQP
- Redis Protocol
- PostgreSQL Wire Protocol
- MongoDB Wire Protocol

## Contributing

Want to implement a new protocol? See:
- [Protocol Expansion Roadmap](../../docs/PROTOCOL_EXPANSION_ROADMAP.md)
- [Protocol Implementation Guide](../../docs/PROTOCOL_IMPLEMENTATION_GUIDE.md)
- [Contributing Guidelines](../../CONTRIBUTING.md)

## Documentation

- [Protocol Abstraction Layer](../../docs/PROTOCOL_ABSTRACTION.md)
- [Template Engine Reference](../../docs/TEMPLATING.md)
- [Configuration Guide](../../docs/CONFIGURATION.md)
- [Plugin Development](../../docs/plugins/development-guide.md)

## Questions or Issues?

- 📖 [Documentation](https://docs.mockforge.dev)
- 💬 [GitHub Discussions](https://github.com/SaaSy-Solutions/mockforge/discussions)
- 🐛 [Report Issues](https://github.com/SaaSy-Solutions/mockforge/issues)
- 📧 [Contact](mailto:talksaas@saasysolutionsllc.com)
