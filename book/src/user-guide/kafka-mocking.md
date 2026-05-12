# Kafka Mocking

MockForge ships a wire-compatible Kafka mock broker so your producer
and consumer code can run against a fake broker that speaks the real
Kafka protocol — no JVM, no testcontainers, no real Kafka deployment.
The mock is good for the cases where you want to know whether your
client config works, whether your consumer handles the messages it
gets, and whether your retry policies engage. It is not a substitute
for performance testing or for verifying log compaction / replica
mechanics.

## Overview

The Kafka mock implements:

- **Produce / Fetch** — full request/response with batches, keys,
  headers, and offsets
- **Metadata** — broker discovery, leader assignment, advertised host
  override for hosted deployments
- **Group coordination** — `FindCoordinator`, `JoinGroup`, `SyncGroup`,
  `Heartbeat`, offset commit/fetch
- **CreateTopics / DeleteTopics / DescribeConfigs** — dynamic topic
  management
- **API version negotiation** — so modern clients auto-negotiate to a
  supported version

What it doesn't implement:

- Log compaction or retention enforcement (records stay in memory for
  the broker's lifetime)
- Inter-broker replication (the mock is a single broker advertising
  itself as leader of every partition)
- Realistic performance characteristics — don't use it for benchmarks

## Quick Start

Bring up a Kafka listener with topic auto-creation enabled. Your tests
produce or consume topics on demand without pre-declaring them:

```yaml
# mockforge.yaml
kafka:
  enabled: true
  port: 9092
  auto_create_topics: true
  default_partitions: 3
  default_replication_factor: 1
```

```bash
$ mockforge serve --config mockforge.yaml
[kafka] Listening on 0.0.0.0:9092 (auto_create_topics: true)
```

Or run the broker by itself with the dedicated CLI:

```bash
$ mockforge kafka serve --port 9092
```

## Pointing a Client at the Broker

Use whatever Kafka client your project already has. The mock advertises
`localhost:9092` (or your configured `advertised_host`/`port`) through
its metadata response so the client doesn't notice it's not a real
broker.

### Python (confluent-kafka)

```python
from confluent_kafka import Producer

p = Producer({"bootstrap.servers": "localhost:9092"})
p.produce("orders.created", key="order-001", value='{"order_id":"order-001"}')
p.flush()
```

### Go (sarama)

```go
config := sarama.NewConfig()
config.Producer.Return.Successes = true
producer, _ := sarama.NewSyncProducer([]string{"localhost:9092"}, config)
producer.SendMessage(&sarama.ProducerMessage{
    Topic: "orders.created",
    Key:   sarama.StringEncoder("order-001"),
    Value: sarama.StringEncoder(`{"order_id":"order-001"}`),
})
```

### Java (kafka-clients)

```java
Properties props = new Properties();
props.put("bootstrap.servers", "localhost:9092");
props.put("key.serializer", "org.apache.kafka.common.serialization.StringSerializer");
props.put("value.serializer", "org.apache.kafka.common.serialization.StringSerializer");
Producer<String,String> p = new KafkaProducer<>(props);
p.send(new ProducerRecord<>("orders.created", "order-001", "{\"order_id\":\"order-001\"}"));
p.flush();
```

## Seeding Messages at Startup

For tests that need messages already in the topic before the consumer
runs, declare seed messages in config. The broker injects them into
the topic log before accepting any client connections:

```yaml
kafka:
  enabled: true
  port: 9092
  default_partitions: 3
  seed_messages:
    orders.created:
      - key: "order-001"
        value: '{"order_id":"order-001","total":4299}'
      - key: "order-002"
        value: '{"order_id":"order-002","total":1599}'
        headers:
          source: "seed"
    orders.shipped:
      - key: "order-001"
        value: '{"order_id":"order-001","carrier":"UPS"}'
```

Seeded records land at offset 0+, so a consumer reading from the
beginning of the topic sees them immediately. Partition assignment
uses Kafka's hash-on-key strategy, so seeded records with the same key
always land on the same partition (the same way real produced records
would).

## Fault Injection

The reason to use a mock broker over a real one for tests isn't speed
— it's that you can deterministically reproduce specific failure
modes that are nearly impossible to reproduce on a real cluster.
MockForge supports three fault kinds today:

```yaml
kafka:
  enabled: true
  port: 9092
  faults:
    # Sleep 2s before processing produce; exercises client retry/backoff
    - topic: orders.created
      partition: 1
      kind: produce_throttle
      delay_ms: 2000

    # 5% of fetches on this topic return OFFSET_OUT_OF_RANGE; consumer
    # hits its auto.offset.reset policy
    - topic: orders.shipped
      kind: offset_out_of_range
      probability: 0.05

    # Every produce returns NOT_LEADER_OR_FOLLOWER; client re-fetches
    # metadata and retries
    - topic: critical-pipeline
      kind: produce_not_leader
```

| Kind | Behavior | Kafka error code |
|---|---|---|
| `produce_throttle` | Sleeps `delay_ms` before processing the produce. Client's request timeout / retry path engages. | n/a (succeeds after delay) |
| `produce_not_leader` | Returns NOT_LEADER_OR_FOLLOWER. | 6 |
| `offset_out_of_range` | Returns OFFSET_OUT_OF_RANGE on fetch, regardless of actual offset. | 1 |

### Match rules

- `topic` is required and matched exactly
- `partition` is optional; omit to target every partition of the topic
- `kind` selects which request type the rule applies to
- `probability` is in `0.0..=1.0`; `None` or `1.0` means the rule fires
  every time (deterministic, test-friendly), `0.0` means never. When
  set to an intermediate value, each request rolls
  `rand::random::<f64>()` to decide
- Rules are evaluated in declaration order; the first one that matches
  a given (topic, partition, kind) wins

### When to reach for each kind

| Test goal | Fault to use |
|---|---|
| Does my producer correctly back off and retry under throttling? | `produce_throttle` |
| Does my consumer reset offsets the way I expect? | `offset_out_of_range` |
| Does my producer recover from a leader election? | `produce_not_leader` |

## CI Patterns

GitHub Actions snippet. Background the mock, wait for the port, run
the tests:

```yaml
- run: cargo install mockforge-cli
- name: Start mock Kafka
  run: |
    mockforge serve --config mockforge.yaml &
    timeout 30 bash -c 'until nc -z localhost 9092; do sleep 0.5; done'
- run: pytest tests/integration/
```

The wait-on-port loop is important: starting Kafka clients before the
broker's TCP listener is up gives confusing "broker not available"
errors that look like real bugs.

## Configuration Reference

All fields are documented in the [Configuration page](../protocols/kafka/configuration.md).
The most commonly tweaked:

| Field | Default | Notes |
|---|---|---|
| `enabled` | `false` | Set to `true` (or pass `--kafka-port` on the CLI) to enable |
| `port` | `9092` | Standard Kafka port |
| `auto_create_topics` | `true` | When true, produce to a previously unknown topic creates it on demand |
| `default_partitions` | `3` | Used for auto-created topics |
| `default_replication_factor` | `1` | Advertised in metadata; the mock is a single broker |
| `advertised_host` | None | Override the host returned in metadata responses (useful for hosted deployments) |
| `seed_messages` | `{}` | Topic → list of records to inject at startup |
| `faults` | `[]` | List of fault-injection rules |

## When This Isn't Enough

Wire-compatible mocks don't replicate the actual log mechanics — log
compaction, replica leadership transitions, performance under load.
If your test depends on those, you want a real broker via
testcontainers and you're going to pay the startup tax.

For the "does my producer config work, does my consumer handle bad
data, do my retry policies engage" class of tests — which is most of
them — a wire-compatible mock turns a flaky 30-second CI step into a
reliable one-second one.

## Further Reading

- [Kafka Configuration](../protocols/kafka/configuration.md) — full
  config reference
- [Kafka Fixtures](../protocols/kafka/fixtures.md) — fixture-driven
  request/response shaping
- [Kafka Testing Patterns](../protocols/kafka/testing-patterns.md) —
  end-to-end recipes
- [Chaos Engineering](chaos-engineering.md) — broader fault-injection
  story across protocols
