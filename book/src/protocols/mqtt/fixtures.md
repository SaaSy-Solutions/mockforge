# MQTT Fixtures

MQTT fixtures in MockForge define mock responses for MQTT topics. Unlike HTTP fixtures that respond to requests, MQTT fixtures define what messages should be published when clients publish to specific topics.

## Basic Fixture Structure

```yaml
mqtt:
  fixtures:
    - identifier: "temperature-sensor"
      name: "Temperature Sensor Mock"
      topic_pattern: "^sensors/temperature/[^/]+$"
      qos: 1
      retained: false
      response:
        payload:
          sensor_id: "{{topic_param 2}}"
          temperature: "{{faker.float 15.0 35.0}}"
          unit: "celsius"
          timestamp: "{{now}}"
      auto_publish:
        enabled: false
        interval_ms: 1000
        count: 10
```

## Topic Patterns

MQTT fixtures use regex patterns to match topics:

```yaml
# Match specific topic
topic_pattern: "^sensors/temperature/room1$"

# Match topic hierarchy with wildcards
topic_pattern: "^sensors/temperature/[^/]+$"

# Match multiple levels
topic_pattern: "^devices/.+/status$"

# Complex patterns
topic_pattern: "^([^/]+)/([^/]+)/(.+)$"
```

## Response Configuration

### Static Responses

```yaml
response:
  payload:
    status: "online"
    version: "1.2.3"
    uptime: 3600
```

### Dynamic Responses with Templates

```yaml
response:
  payload:
    sensor_id: "{{topic_param 1}}"
    temperature: "{{faker.float 20.0 30.0}}"
    humidity: "{{faker.float 40.0 80.0}}"
    timestamp: "{{now}}"
    random_id: "{{uuid}}"
```

### Template Variables

MockForge supports extensive templating for MQTT responses:

#### Topic Parameters
- `{{topic}}` - Full topic string
- `{{topic_param N}}` - Nth segment of topic (0-indexed)

#### Random Data
- `{{uuid}}` - Random UUID
- `{{faker.float min max}}` - Random float between min and max
- `{{faker.int min max}}` - Random integer between min and max
- `{{rand.float}}` - Random float 0.0-1.0
- `{{rand.int}}` - Random integer

#### Time and Dates
- `{{now}}` - Current timestamp (RFC3339)
- `{{now + 1h}}` - Future timestamp
- `{{now - 30m}}` - Past timestamp

#### Environment Variables
- `{{env VAR_NAME}}` - Environment variable value

## Quality of Service (QoS)

```yaml
# QoS 0 - At most once (fire and forget)
qos: 0

# QoS 1 - At least once (acknowledged)
qos: 1

# QoS 2 - Exactly once (assured)
qos: 2
```

## Retained Messages

```yaml
# Message is retained on the broker
retained: true

# Message is not retained
retained: false
```

## Auto-Publish Configuration

Automatically publish messages at regular intervals:

```yaml
auto_publish:
  enabled: true
  interval_ms: 5000    # Publish every 5 seconds
  count: 100          # Publish 100 messages, then stop (optional)
```

## Advanced Fixtures

### Conditional Responses

```yaml
fixtures:
  - identifier: "smart-sensor"
    name: "Smart Temperature Sensor"
    topic_pattern: "^sensors/temp/(.+)$"
    response:
      payload: |
        {
          "sensor_id": "{{topic_param 1}}",
          "temperature": {{faker.float 15.0 35.0}},
          "status": "{{#if (> temperature 30.0)}}critical{{else}}normal{{/if}}",
          "timestamp": "{{now}}"
        }
    conditions:
      - variable: "temperature"
        operator: ">"
        value: 30.0
        response:
          payload:
            sensor_id: "{{topic_param 1}}"
            temperature: "{{temperature}}"
            status: "critical"
            alert: true
```

### Sequence Responses

```yaml
fixtures:
  - identifier: "sequence-demo"
    name: "Sequence Response Demo"
    topic_pattern: "^demo/sequence$"
    sequence:
      - payload:
          step: 1
          message: "Starting sequence"
      - payload:
          step: 2
          message: "Processing..."
      - payload:
          step: 3
          message: "Complete"
    sequence_reset: "manual"  # auto, manual, time
```

### Error Simulation

```yaml
fixtures:
  - identifier: "faulty-sensor"
    name: "Faulty Sensor"
    topic_pattern: "^sensors/faulty/(.+)$"
    error_simulation:
      enabled: true
      error_rate: 0.1  # 10% of messages fail
      error_responses:
        - payload:
            error: "Sensor malfunction"
            code: "SENSOR_ERROR"
        - payload:
            error: "Communication timeout"
            code: "TIMEOUT"
```

## Fixture Management

### Loading Fixtures

```bash
# Load fixtures from file
mockforge mqtt fixtures load ./fixtures/mqtt.yaml

# Load fixtures from directory
mockforge mqtt fixtures load ./fixtures/mqtt/
```

### Auto-Publish Control

```bash
# Start auto-publishing for all fixtures
mockforge mqtt fixtures start-auto-publish

# Stop auto-publishing
mockforge mqtt fixtures stop-auto-publish

# Start specific fixture
mockforge mqtt fixtures start-auto-publish temperature-sensor
```

### Fixture Validation

MockForge validates fixtures on load:

- **Topic pattern syntax** - Valid regex patterns
- **Template variables** - Available variables and functions
- **QoS levels** - Valid QoS values (0, 1, 2)
- **JSON structure** - Valid JSON payloads

## Examples

### IoT Sensor Network

```yaml
mqtt:
  fixtures:
    - identifier: "temp-sensor-room1"
      name: "Room 1 Temperature Sensor"
      topic_pattern: "^sensors/temperature/room1$"
      qos: 1
      retained: true
      response:
        payload:
          sensor_id: "room1"
          temperature: "{{faker.float 20.0 25.0}}"
          humidity: "{{faker.float 40.0 60.0}}"
          battery_level: "{{faker.float 80.0 100.0}}"
          timestamp: "{{now}}"

    - identifier: "motion-sensor"
      name: "Motion Sensor"
      topic_pattern: "^sensors/motion/(.+)$"
      qos: 0
      retained: false
      response:
        payload:
          sensor_id: "{{topic_param 1}}"
          motion_detected: "{{faker.boolean}}"
          timestamp: "{{now}}"
      auto_publish:
        enabled: true
        interval_ms: 30000  # Every 30 seconds
```

### Smart Home Devices

```yaml
mqtt:
  fixtures:
    - identifier: "smart-light"
      name: "Smart Light Controller"
      topic_pattern: "^home/lights/(.+)/command$"
      qos: 1
      response:
        payload:
          device_id: "{{topic_param 1}}"
          command: "ack"
          status: "success"
          timestamp: "{{now}}"

    - identifier: "thermostat"
      name: "Smart Thermostat"
      topic_pattern: "^home/climate/thermostat$"
      qos: 2
      retained: true
      response:
        payload:
          temperature: "{{faker.float 18.0 25.0}}"
          humidity: "{{faker.float 35.0 65.0}}"
          mode: "{{faker.random_element heating cooling auto}}"
          setpoint: "{{faker.float 19.0 23.0}}"
          timestamp: "{{now}}"
```

### Industrial IoT

```yaml
mqtt:
  fixtures:
    - identifier: "conveyor-belt"
      name: "Conveyor Belt Monitor"
      topic_pattern: "^factory/conveyor/(.+)/status$"
      qos: 1
      retained: true
      response:
        payload:
          conveyor_id: "{{topic_param 1}}"
          status: "{{faker.random_element running stopped maintenance}}"
          speed_rpm: "{{faker.float 50.0 150.0}}"
          temperature: "{{faker.float 25.0 45.0}}"
          vibration: "{{faker.float 0.1 2.0}}"
          timestamp: "{{now}}"
      auto_publish:
        enabled: true
        interval_ms: 5000

    - identifier: "quality-control"
      name: "Quality Control Station"
      topic_pattern: "^factory/qc/(.+)/result$"
      qos: 2
      response:
        payload:
          station_id: "{{topic_param 1}}"
          product_id: "{{uuid}}"
          quality_score: "{{faker.float 85.0 100.0}}"
          defects_found: "{{faker.int 0 3}}"
          passed: "{{#if (> quality_score 90.0)}}true{{else}}false{{/if}}"
          timestamp: "{{now}}"
```

## Best Practices

### Topic Design
- Use hierarchical topics: `building/floor/room/device`
- Include device IDs: `sensors/temp/sensor_001`
- Use consistent naming conventions

### QoS Selection
- **QoS 0**: Sensor data, non-critical updates
- **QoS 1**: Important status updates, commands
- **QoS 2**: Critical control messages, financial data

### Retained Messages
- Use for current state: `device/status`, `sensor/last_reading`
- Avoid for event data: `sensor/trigger`, `button/press`

### Auto-Publish
- Reasonable intervals: 1-60 seconds for sensors
- Consider battery life for IoT devices
- Use for simulation, not production data

## Next Steps

- [Getting Started](../getting-started.md) - Basic MQTT setup
- [Configuration](configuration.md) - Detailed configuration options
- [Examples](examples.md) - Real-world usage examples