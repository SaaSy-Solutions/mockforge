//! End-to-end coverage for the three fixture-trigger executors:
//! state machines, scenarios, and relationships.
//!
//! Every test loads a YAML fixture from a tempdir, spins up a broker,
//! installs the fixture runtime, and inspects the resulting records in
//! the broker's in-memory storage. Nothing here goes over the wire —
//! the executors live above the broker's Produce handler and below the
//! wire codec, so internal assertions give the tightest coverage.

use mockforge_core::config::KafkaConfig;
use mockforge_kafka::partitions::KafkaMessage;
use mockforge_kafka::produce_codec::{DecodedRecord, PartitionProduceData, TopicProduceData};
use mockforge_kafka::KafkaMockBroker;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

fn write_fixture(dir: &TempDir, name: &str, body: &str) {
    let path = dir.path().join(name);
    std::fs::write(path, body).expect("write fixture");
}

async fn broker_with_fixtures(yaml: &str) -> (Arc<KafkaMockBroker>, TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    write_fixture(&dir, "fixture.yaml", yaml);
    let config = KafkaConfig {
        fixtures_dir: Some(dir.path().to_path_buf()),
        ..KafkaConfig::default()
    };
    let broker = Arc::new(KafkaMockBroker::new(config).await.expect("broker init"));
    (broker, dir)
}

/// Shortcut: count the records a topic has across all partitions.
async fn count_records(broker: &KafkaMockBroker, topic: &str) -> usize {
    let topics = broker.topics.read().await;
    let Some(t) = topics.get(topic) else {
        return 0;
    };
    t.partitions.iter().map(|p| p.messages.len()).sum()
}

async fn all_records(broker: &KafkaMockBroker, topic: &str) -> Vec<KafkaMessage> {
    let topics = broker.topics.read().await;
    let Some(t) = topics.get(topic) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for p in &t.partitions {
        out.extend(p.messages.iter().cloned());
    }
    out
}

// =========================================================================
// State machine executor
// =========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn state_machine_walks_graph_to_terminal_state() {
    // 3 states: start -> middle -> end. Deterministic transitions
    // (probability 1.0) and tiny delays so the test finishes quickly.
    let yaml = r#"
topics:
  - name: "orders.status"
    partitions: 1
    messages:
      - key_template: "key-{{state}}"
        value:
          stage: "{{state}}"
        auto_produce:
          enabled: true
          trigger: "state_machine"
          state_machine:
            initial_state: "start"
            states:
              - name: "start"
                next_states: ["middle"]
                probability: [1.0]
                delay_ms: [5, 10]
              - name: "middle"
                next_states: ["end"]
                probability: [1.0]
                delay_ms: [5, 10]
              - name: "end"
                next_states: []
"#;
    let (broker, _dir) = broker_with_fixtures(yaml).await;

    // Call install_fixture_runtime directly so the test doesn't have to
    // bind a TCP listener.
    broker.install_fixture_runtime_for_tests().await;

    // Poll until the terminal state has been emitted or we time out.
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        let records = all_records(&broker, "orders.status").await;
        if records.len() >= 3 {
            let stages: Vec<String> = records
                .iter()
                .filter_map(|m| {
                    serde_json::from_slice::<serde_json::Value>(&m.value)
                        .ok()
                        .and_then(|v| v.get("stage").and_then(|s| s.as_str()).map(String::from))
                })
                .collect();
            assert_eq!(stages, vec!["start", "middle", "end"]);
            return;
        }
        if std::time::Instant::now() >= deadline {
            panic!(
                "state machine did not emit 3 records in time, got {} in storage",
                records.len()
            );
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

// =========================================================================
// Scenario executor
// =========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_walks_sequence_in_order() {
    // One scenario with three steps — each on a different topic.
    let yaml = r#"
topics:
  - name: "t1"
    messages:
      - key_template: "k1"
        value:
          n: 1
  - name: "t2"
    messages:
      - key_template: "k2"
        value:
          n: 2
  - name: "t3"
    messages:
      - key_template: "k3"
        value:
          n: 3
scenarios:
  - name: "linear"
    enabled: true
    probability: 1.0
    sequence:
      - topic: "t1"
      - topic: "t2"
        delay_ms: [5, 10]
      - topic: "t3"
        delay_ms: [5, 10]
"#;
    let (broker, _dir) = broker_with_fixtures(yaml).await;
    broker.install_fixture_runtime_for_tests().await;

    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        if count_records(&broker, "t1").await >= 1
            && count_records(&broker, "t2").await >= 1
            && count_records(&broker, "t3").await >= 1
        {
            return;
        }
        if std::time::Instant::now() >= deadline {
            panic!(
                "scenario did not emit to all topics: t1={} t2={} t3={}",
                count_records(&broker, "t1").await,
                count_records(&broker, "t2").await,
                count_records(&broker, "t3").await,
            );
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_with_zero_probability_does_not_run() {
    let yaml = r#"
topics:
  - name: "t1"
    messages:
      - key_template: "k1"
        value:
          n: 1
scenarios:
  - name: "never"
    enabled: true
    probability: 0.0
    sequence:
      - topic: "t1"
"#;
    let (broker, _dir) = broker_with_fixtures(yaml).await;
    broker.install_fixture_runtime_for_tests().await;

    // Give it a moment to not do anything.
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(count_records(&broker, "t1").await, 0);
}

// =========================================================================
// Relationships executor
// =========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn relationship_fires_on_produce() {
    // orders.created → payments.processed with order_id correlated.
    let yaml = r#"
topics:
  - name: "orders.created"
    partitions: 1
    messages:
      - key_template: "order-{{order_id}}"
        value:
          order_id: "{{order_id}}"
  - name: "payments.processed"
    partitions: 1
    messages:
      - key_template: "payment-{{order_id}}"
        value:
          order_id: "{{order_id}}"
          status: "success"
relationships:
  - from_topic: "orders.created"
    to_topic: "payments.processed"
    relationship: "one_to_one"
    key_mapping:
      order_id: "order_id"
"#;
    let (broker, _dir) = broker_with_fixtures(yaml).await;
    broker.install_fixture_runtime_for_tests().await;

    // Directly drive handle_produce via the broker's test API: emulate a
    // client produce on `orders.created` with a single record.
    broker
        .test_produce_one(
            "orders.created",
            Some(b"order-ABC".to_vec()),
            br#"{"order_id":"ABC"}"#.to_vec(),
        )
        .await;

    // Poll briefly for the dependent record to land.
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        let records = all_records(&broker, "payments.processed").await;
        if !records.is_empty() {
            let value: serde_json::Value = serde_json::from_slice(&records[0].value).unwrap();
            assert_eq!(value["order_id"].as_str(), Some("ABC"));
            assert_eq!(
                records[0].key.as_ref().map(|k| String::from_utf8_lossy(k).into_owned()),
                Some("payment-ABC".to_string())
            );
            return;
        }
        if std::time::Instant::now() >= deadline {
            panic!("relationship did not fire; payments.processed is empty");
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

// =========================================================================
// Test-only broker helpers
// =========================================================================
//
// These aren't exported from the crate; we pull them into scope via
// trait extension so the tests above stay readable.

#[allow(async_fn_in_trait)]
trait TestOnlyBrokerExt {
    async fn install_fixture_runtime_for_tests(&self);
    async fn test_produce_one(&self, topic: &str, key: Option<Vec<u8>>, value: Vec<u8>);
}

impl TestOnlyBrokerExt for KafkaMockBroker {
    async fn install_fixture_runtime_for_tests(&self) {
        // `install_fixture_runtime` is the same code `start()` would call
        // after its TCP bind; invoking it directly skips the bind so the
        // test doesn't need to pick a free port.
        self.install_fixture_runtime().await;
    }

    async fn test_produce_one(&self, topic: &str, key: Option<Vec<u8>>, value: Vec<u8>) {
        // Replicate the work `handle_produce` does for a single record
        // so we can exercise the relationships fan-out without building
        // a raw Produce v9 frame.
        use mockforge_kafka::fixture_executor;

        // Append to storage, exactly like handle_produce does.
        {
            let mut topics = self.topics.write().await;
            let t = topics.entry(topic.to_string()).or_insert_with(|| {
                mockforge_kafka::Topic::new(
                    topic.to_string(),
                    mockforge_kafka::TopicConfig::default(),
                )
            });
            let partition = t.assign_partition(key.as_deref());
            let msg = KafkaMessage {
                offset: 0,
                timestamp: chrono::Utc::now().timestamp_millis(),
                key: key.clone(),
                value: value.clone(),
                headers: vec![],
            };
            let _ = t.produce(partition, msg).await;
        }

        // Fire the runtime's relationship hook the same way handle_produce
        // does — this is the behavior under test.
        if let Some(runtime) = self.fixture_runtime() {
            let me = Arc::new(self.clone());
            let accepted = vec![KafkaMessage {
                offset: 0,
                timestamp: 0,
                key,
                value,
                headers: vec![],
            }];
            fixture_executor::on_produced_records(&me, &runtime, topic, &accepted).await;
        }

        // Silence unused imports when the trait is not instantiated.
        let _ = std::mem::size_of::<DecodedRecord>();
        let _ = std::mem::size_of::<PartitionProduceData>();
        let _ = std::mem::size_of::<TopicProduceData>();
    }
}
