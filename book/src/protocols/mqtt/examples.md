# MQTT Examples

This document provides real-world examples of using MockForge MQTT for testing IoT applications, microservices communication, and pub/sub systems.

## IoT Device Simulation

### Smart Home System

**Scenario**: Test a smart home application that controls lights, thermostats, and security sensors.

**MockForge Configuration**:

```yaml
mqtt:
  enabled: true
  port: 1883

  fixtures:
    # Smart Lights
    - identifier: "living-room-light"
      name: "Living Room Light"
      topic_pattern: "^home/lights/living_room/command$"
      qos: 1
      response:
        payload:
          device_id: "living_room_light"
          status: "success"
          brightness: "{{faker.int 0 100}}"
          timestamp: "{{now}}"

    - identifier: "kitchen-light"
      name: "Kitchen Light"
      topic_pattern: "^home/lights/kitchen/command$"
      qos: 1
      response:
        payload:
          device_id: "kitchen_light"
          status: "success"
          color_temp: "{{faker.int 2700 6500}}"
          timestamp: "{{now}}"

    # Thermostat
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
      auto_publish:
        enabled: true
        interval_ms: 30000

    # Motion Sensors
    - identifier: "motion-sensor"
      name: "Motion Sensor"
      topic_pattern: "^home/security/motion/(.+)$"
      qos: 0
      response:
        payload:
          sensor_id: "{{topic_param 1}}"
          motion_detected: "{{faker.boolean}}"
          battery_level: "{{faker.float 70.0 100.0}}"
          timestamp: "{{now}}"
      auto_publish:
        enabled: true
        interval_ms: 15000
```

**Test Code (Python)**:

```python
import paho.mqtt.client as mqtt
import json
import time

def test_smart_home_integration():
    client = mqtt.Client("test-client")
    client.connect("localhost", 1883, 60)

    # Test light control
    client.publish("home/lights/living_room/command", json.dumps({
        "action": "turn_on",
        "brightness": 80
    }), qos=1)

    # Subscribe to responses
    responses = []
    def on_message(client, userdata, msg):
        responses.append(json.loads(msg.payload.decode()))

    client.on_message = on_message
    client.subscribe("home/lights/living_room/status")
    client.loop_start()

    # Wait for response
    time.sleep(1)
    client.loop_stop()

    assert len(responses) > 0
    assert responses[0]["device_id"] == "living_room_light"
    assert responses[0]["status"] == "success"

    # Test thermostat reading
    client.subscribe("home/climate/thermostat")
    client.loop_start()
    time.sleep(2)  # Wait for auto-published message
    client.loop_stop()

    # Verify thermostat data
    thermostat_data = None
    for response in responses:
        if "temperature" in response:
            thermostat_data = response
            break

    assert thermostat_data is not None
    assert 18.0 <= thermostat_data["temperature"] <= 25.0
    assert thermostat_data["mode"] in ["heating", "cooling", "auto"]

    client.disconnect()
```

### Industrial IoT Monitoring

**Scenario**: Test an industrial monitoring system with sensors, actuators, and PLCs.

**MockForge Configuration**:

```yaml
mqtt:
  enabled: true
  port: 1883
  max_connections: 100

  fixtures:
    # Temperature Sensors
    - identifier: "temp-sensor-1"
      name: "Temperature Sensor 1"
      topic_pattern: "^factory/sensors/temp/1$"
      qos: 1
      retained: true
      response:
        payload:
          sensor_id: "temp_1"
          temperature: "{{faker.float 20.0 80.0}}"
          unit: "celsius"
          status: "operational"
          timestamp: "{{now}}"
      auto_publish:
        enabled: true
        interval_ms: 5000

    # Pressure Sensors
    - identifier: "pressure-sensor"
      name: "Pressure Sensor"
      topic_pattern: "^factory/sensors/pressure/(.+)$"
      qos: 1
      response:
        payload:
          sensor_id: "{{topic_param 1}}"
          pressure: "{{faker.float 0.5 5.0}}"
          unit: "bar"
          threshold: 3.5
          alert: "{{#if (> pressure 3.5)}}true{{else}}false{{/if}}"
          timestamp: "{{now}}"

    # Conveyor Belt Controller
    - identifier: "conveyor-controller"
      name: "Conveyor Belt Controller"
      topic_pattern: "^factory/actuators/conveyor/(.+)/command$"
      qos: 2
      response:
        payload:
          actuator_id: "{{topic_param 1}}"
          command_ack: true
          status: "executing"
          estimated_completion: "{{now + 5s}}"
          timestamp: "{{now}}"

    # Quality Control Station
    - identifier: "qc-station"
      name: "Quality Control Station"
      topic_pattern: "^factory/qc/station_(.+)/result$"
      qos: 2
      response:
        payload:
          station_id: "{{topic_param 1}}"
          product_id: "{{uuid}}"
          quality_score: "{{faker.float 85.0 100.0}}"
          defects: "{{faker.int 0 2}}"
          passed: "{{#if (> quality_score 95.0)}}true{{else}}false{{/if}}"
          timestamp: "{{now}}"
```

**Test Code (JavaScript/Node.js)**:

```javascript
const mqtt = require('mqtt');

describe('Industrial IoT System', () => {
  let client;

  beforeAll(() => {
    client = mqtt.connect('mqtt://localhost:1883');
  });

  afterAll(() => {
    client.end();
  });

  test('sensor data collection', (done) => {
    const sensorData = [];

    client.subscribe('factory/sensors/temp/1');
    client.subscribe('factory/sensors/pressure/1');

    client.on('message', (topic, message) => {
      const data = JSON.parse(message.toString());
      sensorData.push({ topic, data });

      if (sensorData.length >= 2) {
        // Verify temperature sensor
        const tempSensor = sensorData.find(s => s.topic === 'factory/sensors/temp/1');
        expect(tempSensor.data.temperature).toBeGreaterThanOrEqual(20);
        expect(tempSensor.data.temperature).toBeLessThanOrEqual(80);
        expect(tempSensor.data.unit).toBe('celsius');

        // Verify pressure sensor
        const pressureSensor = sensorData.find(s => s.topic === 'factory/sensors/pressure/1');
        expect(pressureSensor.data.pressure).toBeGreaterThanOrEqual(0.5);
        expect(pressureSensor.data.pressure).toBeLessThanOrEqual(5.0);
        expect(pressureSensor.data.unit).toBe('bar');

        client.unsubscribe(['factory/sensors/temp/1', 'factory/sensors/pressure/1']);
        done();
      }
    });

    // Trigger sensor readings
    client.publish('factory/sensors/temp/1/trigger', 'read');
    client.publish('factory/sensors/pressure/1/trigger', 'read');
  });

  test('actuator control', (done) => {
    client.subscribe('factory/actuators/conveyor/1/status');

    client.on('message', (topic, message) => {
      if (topic === 'factory/actuators/conveyor/1/status') {
        const status = JSON.parse(message.toString());
        expect(status.actuator_id).toBe('1');
        expect(status.command_ack).toBe(true);
        expect(status.status).toBe('executing');

        client.unsubscribe('factory/actuators/conveyor/1/status');
        done();
      }
    });

    // Send control command
    client.publish('factory/actuators/conveyor/1/command', JSON.stringify({
      action: 'start',
      speed: 50
    }), { qos: 2 });
  });

  test('quality control workflow', (done) => {
    client.subscribe('factory/qc/station_1/result');

    client.on('message', (topic, message) => {
      const result = JSON.parse(message.toString());
      expect(result.station_id).toBe('1');
      expect(result.quality_score).toBeGreaterThanOrEqual(85);
      expect(result.quality_score).toBeLessThanOrEqual(100);
      expect(typeof result.defects).toBe('number');
      expect(typeof result.passed).toBe('boolean');

      client.unsubscribe('factory/qc/station_1/result');
      done();
    });

    // Trigger quality check
    client.publish('factory/qc/station_1/check', JSON.stringify({
      product_id: 'PROD-001',
      batch_id: 'BATCH-2024'
    }));
  });
});
```

## Microservices Communication

### Event-Driven Architecture

**Scenario**: Test microservices communicating via MQTT events.

**MockForge Configuration**:

```yaml
mqtt:
  enabled: true
  port: 1883

  fixtures:
    # User Service Events
    - identifier: "user-registered"
      name: "User Registration Event"
      topic_pattern: "^events/user/registered$"
      qos: 1
      response:
        payload:
          event_type: "user_registered"
          user_id: "{{uuid}}"
          email: "{{faker.email}}"
          timestamp: "{{now}}"
          source: "user-service"

    # Order Service Events
    - identifier: "order-created"
      name: "Order Created Event"
      topic_pattern: "^events/order/created$"
      qos: 1
      response:
        payload:
          event_type: "order_created"
          order_id: "{{uuid}}"
          user_id: "{{uuid}}"
          amount: "{{faker.float 10.0 500.0}}"
          currency: "USD"
          items: "{{faker.int 1 10}}"
          timestamp: "{{now}}"
          source: "order-service"

    # Payment Service Events
    - identifier: "payment-processed"
      name: "Payment Processed Event"
      topic_pattern: "^events/payment/processed$"
      qos: 2
      response:
        payload:
          event_type: "payment_processed"
          payment_id: "{{uuid}}"
          order_id: "{{uuid}}"
          amount: "{{faker.float 10.0 500.0}}"
          currency: "USD"
          status: "{{faker.random_element completed failed pending}}"
          method: "{{faker.random_element credit_card paypal bank_transfer}}"
          timestamp: "{{now}}"
          source: "payment-service"

    # Notification Service
    - identifier: "email-notification"
      name: "Email Notification"
      topic_pattern: "^commands/notification/email$"
      qos: 1
      response:
        payload:
          command_type: "send_email"
          notification_id: "{{uuid}}"
          recipient: "{{faker.email}}"
          subject: "Order Confirmation"
          template: "order_confirmation"
          status: "queued"
          timestamp: "{{now}}"
```

**Test Code (Go)**:

```go
package main

import (
    "encoding/json"
    "testing"
    "time"

    mqtt "github.com/eclipse/paho.mqtt.golang"
)

func TestEventDrivenWorkflow(t *testing.T) {
    opts := mqtt.NewClientOptions().AddBroker("tcp://localhost:1883")
    client := mqtt.NewClient(opts)

    if token := client.Connect(); token.Wait() && token.Error() != nil {
        t.Fatalf("Failed to connect: %v", token.Error())
    }
    defer client.Disconnect(250)

    // Test user registration -> order creation -> payment -> notification flow
    events := make(chan map[string]interface{}, 10)

    // Subscribe to all events
    client.Subscribe("events/#", 1, func(client mqtt.Client, msg mqtt.Message) {
        var event map[string]interface{}
        json.Unmarshal(msg.Payload(), &event)
        events <- event
    })

    // Trigger user registration
    userEvent := map[string]interface{}{
        "user_id": "user-123",
        "email": "user@example.com",
    }
    payload, _ := json.Marshal(userEvent)
    client.Publish("events/user/registered", 1, false, payload)

    // Wait for events
    timeout := time.After(5 * time.Second)
    receivedEvents := make(map[string]int)

    for {
        select {
        case event := <-events:
            eventType := event["event_type"].(string)
            receivedEvents[eventType]++

            // Verify event structure
            switch eventType {
            case "user_registered":
                if event["user_id"] == nil || event["email"] == nil {
                    t.Errorf("Invalid user_registered event: %v", event)
                }
            case "order_created":
                if event["order_id"] == nil || event["amount"] == nil {
                    t.Errorf("Invalid order_created event: %v", event)
                }
            case "payment_processed":
                if event["payment_id"] == nil || event["status"] == nil {
                    t.Errorf("Invalid payment_processed event: %v", event)
                }
            }
        case <-timeout:
            // Check that we received expected events
            if receivedEvents["user_registered"] == 0 {
                t.Error("Expected user_registered event")
            }
            if receivedEvents["order_created"] == 0 {
                t.Error("Expected order_created event")
            }
            if receivedEvents["payment_processed"] == 0 {
                t.Error("Expected payment_processed event")
            }
            return
        }
    }
}
```

## Real-Time Data Streaming

### Live Dashboard Testing

**Scenario**: Test a real-time dashboard that displays sensor data and alerts.

**MockForge Configuration**:

```yaml
mqtt:
  enabled: true
  port: 1883

  fixtures:
    # Environmental Sensors
    - identifier: "env-sensor-cluster"
      name: "Environmental Sensor Cluster"
      topic_pattern: "^sensors/env/(.+)/(.+)$"
      qos: 0
      response:
        payload:
          sensor_type: "{{topic_param 2}}"
          location: "{{topic_param 1}}"
          value: "{{#switch topic_param.2}}
                     {{#case 'temperature'}}{{faker.float 15.0 35.0}}{{/case}}
                     {{#case 'humidity'}}{{faker.float 30.0 90.0}}{{/case}}
                     {{#case 'co2'}}{{faker.float 400.0 2000.0}}{{/case}}
                     {{#default}}0{{/default}}
                   {{/switch}}"
          unit: "{{#switch topic_param.2}}
                   {{#case 'temperature'}}celsius{{/case}}
                   {{#case 'humidity'}}percent{{/case}}
                   {{#case 'co2'}}ppm{{/case}}
                   {{#default}}unit{{/default}}
                 {{/switch}}"
          timestamp: "{{now}}"
      auto_publish:
        enabled: true
        interval_ms: 2000

    # System Alerts
    - identifier: "system-alerts"
      name: "System Alerts"
      topic_pattern: "^alerts/system/(.+)$"
      qos: 1
      response:
        payload:
          alert_type: "{{topic_param 1}}"
          severity: "{{faker.random_element info warning error critical}}"
          message: "{{#switch topic_param.1}}
                      {{#case 'temperature'}}High temperature detected{{/case}}
                      {{#case 'power'}}Power supply issue{{/case}}
                      {{#case 'network'}}Network connectivity lost{{/case}}
                      {{#default}}System alert{{/default}}
                    {{/switch}}"
          sensor_id: "{{uuid}}"
          timestamp: "{{now}}"
      auto_publish:
        enabled: true
        interval_ms: 30000
```

**Test Code (Rust)**:

```rust
use paho_mqtt as mqtt;
use std::time::Duration;

#[tokio::test]
async fn test_realtime_dashboard() {
    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri("tcp://localhost:1883")
        .client_id("dashboard-test")
        .finalize();

    let mut client = mqtt::AsyncClient::new(create_opts).unwrap();
    let conn_opts = mqtt::ConnectOptions::new();
    client.connect(conn_opts).await.unwrap();

    // Subscribe to sensor data
    client.subscribe("sensors/env/+/temperature", mqtt::QOS_0).await.unwrap();
    client.subscribe("sensors/env/+/humidity", mqtt::QOS_0).await.unwrap();
    client.subscribe("alerts/system/+", mqtt::QOS_1).await.unwrap();

    let mut receiver = client.get_stream(100);
    let mut message_count = 0;
    let mut alerts_received = 0;

    // Collect messages for 10 seconds
    let start_time = std::time::Instant::now();
    while start_time.elapsed() < Duration::from_secs(10) {
        if let Ok(Some(msg)) = tokio::time::timeout(Duration::from_millis(100), receiver.recv()).await {
            message_count += 1;

            let payload: serde_json::Value = serde_json::from_str(&msg.payload_str()).unwrap();

            // Verify sensor data structure
            if msg.topic().contains("sensors/env") {
                assert!(payload.get("sensor_type").is_some());
                assert!(payload.get("location").is_some());
                assert!(payload.get("value").is_some());
                assert!(payload.get("unit").is_some());
                assert!(payload.get("timestamp").is_some());
            }

            // Count alerts
            if msg.topic().contains("alerts/system") {
                alerts_received += 1;
                assert!(payload.get("alert_type").is_some());
                assert!(payload.get("severity").is_some());
                assert!(payload.get("message").is_some());
            }
        }
    }

    // Verify we received data
    assert!(message_count > 0, "No messages received");
    assert!(alerts_received > 0, "No alerts received");

    client.disconnect(None).await.unwrap();
}
```

## CI/CD Integration

### Automated Testing Pipeline

```yaml
# .github/workflows/mqtt-tests.yml
name: MQTT Integration Tests

on: [push, pull_request]

jobs:
  mqtt-tests:
    runs-on: ubuntu-latest

    services:
      mockforge:
        image: mockforge:latest
        ports:
          - 1883:1883
        env:
          MOCKFORGE_MQTT_ENABLED: true
          MOCKFORGE_MQTT_FIXTURES: ./test-fixtures/mqtt/

    steps:
      - uses: actions/checkout@v3

      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '18'

      - name: Install dependencies
        run: npm ci

      - name: Wait for MockForge
        run: |
          timeout 30 bash -c 'until nc -z localhost 1883; do sleep 1; done'

      - name: Run MQTT tests
        run: npm test -- --testPathPattern=mqtt
        env:
          MQTT_BROKER: localhost:1883
```

## Performance Testing

### Load Testing MQTT Broker

```yaml
mqtt:
  enabled: true
  port: 1883
  max_connections: 1000

  fixtures:
    - identifier: "load-test-sensor"
      name: "Load Test Sensor"
      topic_pattern: "^loadtest/sensor/(.+)$"
      qos: 0
      response:
        payload:
          sensor_id: "{{topic_param 1}}"
          value: "{{faker.float 0.0 100.0}}"
          timestamp: "{{now}}"
```

**Load Test Script (Python)**:

```python
import paho.mqtt.client as mqtt
import threading
import time
import json

def create_publisher(client_id, num_messages):
    client = mqtt.Client(f"publisher-{client_id}")
    client.connect("localhost", 1883, 60)

    for i in range(num_messages):
        payload = {
            "sensor_id": f"sensor_{client_id}_{i}",
            "value": i * 1.5,
            "timestamp": time.time()
        }
        client.publish(f"loadtest/sensor/{client_id}", json.dumps(payload), qos=0)

    client.disconnect()

def load_test():
    num_publishers = 50
    messages_per_publisher = 100

    start_time = time.time()

    threads = []
    for i in range(num_publishers):
        thread = threading.Thread(target=create_publisher, args=(i, messages_per_publisher))
        threads.append(thread)
        thread.start()

    for thread in threads:
        thread.join()

    end_time = time.time()
    total_messages = num_publishers * messages_per_publisher
    duration = end_time - start_time

    print(f"Published {total_messages} messages in {duration:.2f} seconds")
    print(f"Throughput: {total_messages / duration:.0f} messages/second")

if __name__ == "__main__":
    load_test()
```

## Next Steps

- [Getting Started](../getting-started.md) - Basic MQTT setup
- [Configuration](configuration.md) - Detailed configuration options
- [Fixtures](fixtures.md) - Define MQTT mock scenarios
