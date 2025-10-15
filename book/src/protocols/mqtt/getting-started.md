# Getting Started with MQTT

MockForge includes a fully functional MQTT (Message Queuing Telemetry Transport) broker for testing IoT and pub/sub workflows in your applications. This guide will help you get started quickly.

## Quick Start

### 1. Enable MQTT in Configuration

Create a configuration file or modify your existing `config.yaml`:

```yaml
mqtt:
  enabled: true
  port: 1883
  host: "0.0.0.0"
  max_connections: 1000
  max_packet_size: 1048576  # 1MB
  keep_alive_secs: 60
```

### 2. Start the Server

```bash
mockforge serve --config config.yaml
```

You should see:
```
📡 MQTT broker listening on localhost:1883
```

### 3. Connect and Publish a Test Message

Using the `mosquitto` command-line tools:

```bash
# Install mosquitto clients (Ubuntu/Debian)
sudo apt install mosquitto-clients

# Or on macOS
brew install mosquitto

# Publish a test message
mosquitto_pub -h localhost -p 1883 -t "sensors/temperature" -m "25.5" -q 1

# Subscribe to receive messages
mosquitto_sub -h localhost -p 1883 -t "sensors/temperature" -q 1
```

### 4. Verify Message Handling

Messages are processed according to your fixtures configuration. Check server logs for routing information and fixture matching.

## Using Command-Line Tools

### mosquitto_pub

Publish messages to topics:

```bash
# Simple publish
mosquitto_pub -h localhost -p 1883 -t "sensors/temp/room1" -m "23.5"

# With QoS 1 (at least once delivery)
mosquitto_pub -h localhost -p 1883 -t "devices/status" -m "online" -q 1

# With retained message
mosquitto_pub -h localhost -p 1883 -t "config/max_temp" -m "30.0" -r

# JSON payload
mosquitto_pub -h localhost -p 1883 -t "sensors/data" -m '{"temperature": 22.1, "humidity": 65}'
```

### mosquitto_sub

Subscribe to topics:

```bash
# Subscribe to specific topic
mosquitto_sub -h localhost -p 1883 -t "sensors/temp/room1"

# Subscribe with wildcards
mosquitto_sub -h localhost -p 1883 -t "sensors/temp/+"
mosquitto_sub -h localhost -p 1883 -t "devices/#"

# Subscribe to all topics (for debugging)
mosquitto_sub -h localhost -p 1883 -t "#"
```

### MQTT CLI Commands

MockForge provides MQTT-specific CLI commands:

```bash
# List active topics
mockforge mqtt topics

# List connected clients
mockforge mqtt clients

# Publish a message
mockforge mqtt publish sensors/temperature 25.5 --qos 1

# Subscribe to topics
mockforge mqtt subscribe "sensors/#" --qos 0
```

## Supported MQTT Features

MockForge MQTT broker implements MQTT 3.1.1 and 5.0 specifications with the following features:

### Quality of Service (QoS) Levels
- **QoS 0** - At most once delivery (fire and forget)
- **QoS 1** - At least once delivery (acknowledged delivery)
- **QoS 2** - Exactly once delivery (assured delivery)

### Topic Management
- **Single-level wildcards** (`+`) - Match one topic level
- **Multi-level wildcards** (`#`) - Match multiple topic levels
- **Retained messages** - Store last message per topic
- **Clean sessions** - Persistent vs ephemeral subscriptions

### Connection Management
- **Keep-alive handling** - Automatic client timeout
- **Will messages** - Last-will-and-testament
- **Session persistence** - Restore subscriptions on reconnect

## Basic Configuration Options

```yaml
mqtt:
  enabled: true              # Enable/disable MQTT broker
  port: 1883                 # Port (1883 for MQTT, 8883 for MQTT over TLS)
  host: "0.0.0.0"            # Bind address
  max_connections: 1000      # Maximum concurrent connections
  max_packet_size: 1048576   # Maximum packet size (1MB)
  keep_alive_secs: 60        # Default keep-alive timeout

  # Advanced options
  max_inflight_messages: 20  # Maximum QoS 1/2 messages in flight
  max_queued_messages: 100   # Maximum queued messages per client
```

## Environment Variables

Override configuration with environment variables:

```bash
export MOCKFORGE_MQTT_ENABLED=true
export MOCKFORGE_MQTT_PORT=1883
export MOCKFORGE_MQTT_HOST=0.0.0.0
export MOCKFORGE_MQTT_MAX_CONNECTIONS=1000

mockforge serve
```

## Next Steps

- [Configuration Reference](./configuration.md) - Detailed configuration options
- [Fixtures](./fixtures.md) - Create MQTT scenarios and mock responses
- [Examples](./examples.md) - Real-world usage examples

## Troubleshooting

### Connection Refused

**Problem**: Cannot connect to MQTT broker

**Solutions**:
1. Verify MQTT is enabled: `mqtt.enabled: true`
2. Check the port isn't in use: `lsof -i :1883`
3. Ensure server is running: Look for "MQTT broker listening" in logs

### Messages Not Received

**Problem**: Messages published but not received by subscribers

**Solutions**:
1. Check topic matching patterns
2. Verify QoS levels are compatible
3. Check for retained message conflicts
4. Review server logs for routing information

### Wildcard Issues

**Problem**: Wildcard subscriptions not working as expected

**Solutions**:
1. `+` matches exactly one level: `sensors/+/temperature`
2. `#` matches multiple levels: `devices/#`
3. Wildcards only work in subscriptions, not publications

## Common Use Cases

### IoT Device Simulation

```python
# Simulate multiple IoT sensors
import paho.mqtt.client as mqtt
import time
import random

def simulate_sensor(sensor_id, topic_prefix):
    client = mqtt.Client(f"sensor_{sensor_id}")
    client.connect("localhost", 1883, 60)

    while True:
        temperature = 20 + random.uniform(-5, 5)
        payload = f'{{"sensor_id": "{sensor_id}", "temperature": {temperature:.1f}}}'

        client.publish(f"{topic_prefix}/temperature", payload, qos=1)
        time.sleep(5)

# Start multiple sensors
for i in range(3):
    simulate_sensor(f"sensor_{i}", f"sensors/room{i}")
```

### Testing MQTT Applications

```javascript
// In your test suite (Node.js with mqtt.js)
const mqtt = require('mqtt');

describe('Temperature Monitoring', () => {
  let client;

  beforeAll(() => {
    client = mqtt.connect('mqtt://localhost:1883');
  });

  afterAll(() => {
    client.end();
  });

  test('receives temperature updates', (done) => {
    client.subscribe('sensors/temperature/+', { qos: 1 });

    client.on('message', (topic, message) => {
      const data = JSON.parse(message.toString());
      expect(data).toHaveProperty('sensor_id');
      expect(data).toHaveProperty('temperature');
      expect(data.temperature).toBeGreaterThan(-50);
      expect(data.temperature).toBeLessThan(100);
      done();
    });

    // Trigger temperature reading in your app
    // Your app should publish to sensors/temperature/+
  });
});
```

### CI/CD Integration

```yaml
# .github/workflows/test.yml
- name: Start MockForge MQTT
  run: |
    mockforge serve --mqtt --mqtt-port 1883 &
    sleep 2

- name: Run MQTT tests
  env:
    MQTT_HOST: localhost
    MQTT_PORT: 1883
  run: npm test
```

## What's Next?

Now that you have a basic MQTT broker running, explore:

1. **[Fixtures](./fixtures.md)** - Define MQTT message patterns and mock responses
2. **[Configuration](./configuration.md)** - Fine-tune broker behavior
3. **[Examples](./examples.md)** - See real-world implementations
