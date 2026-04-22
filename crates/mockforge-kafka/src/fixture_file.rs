//! Nested "topology" YAML fixture format for Kafka.
//!
//! The older `KafkaFixture` shape is a flat record describing one message
//! template. Realistic fixture files want to describe a whole cluster:
//! multiple topics with partition counts, per-topic configs, and a list of
//! message templates under each topic. This module defines that richer
//! structure and a `flatten()` method that expands it into:
//!
//!   * `Vec<KafkaTopicSpec>` — one entry per topic, used by the Metadata
//!     response to advertise real topics/partitions.
//!   * `Vec<KafkaFixture>`  — one entry per message template, fed into the
//!     existing `KafkaSpecRegistry` so nothing else has to change to keep
//!     working.
//!
//! Unknown fields are silently ignored so advanced YAML sections (scenarios,
//! state_machine triggers, relationships, failure_simulation, monitoring)
//! don't break the load — they'll be implemented in later PRs.

use crate::fixtures::{AutoProduceConfig, KafkaFixture};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level shape of a Kafka fixture YAML file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KafkaFixtureFile {
    /// Human-readable metadata about the fixture file. Optional.
    #[serde(default)]
    pub fixture: Option<FixtureMeta>,
    /// Cluster-level configuration. Optional.
    #[serde(default)]
    pub cluster: Option<ClusterSpec>,
    /// Topics described by this file.
    #[serde(default)]
    pub topics: Vec<KafkaTopicSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FixtureMeta {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub protocol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClusterSpec {
    #[serde(default)]
    pub bootstrap_servers: Option<String>,
    #[serde(default)]
    pub cluster_id: Option<String>,
}

/// One topic described in the YAML file. `partitions` / `replication_factor`
/// flow into the Metadata response; `messages` flow into `KafkaFixture`s.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaTopicSpec {
    pub name: String,
    #[serde(default = "default_partitions")]
    pub partitions: i32,
    #[serde(default = "default_replication_factor")]
    pub replication_factor: i16,
    #[serde(default)]
    pub partitioning: Option<serde_json::Value>,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
    #[serde(default)]
    pub messages: Vec<KafkaMessageSpec>,
}

fn default_partitions() -> i32 {
    1
}
fn default_replication_factor() -> i16 {
    1
}

/// One message template under a topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaMessageSpec {
    #[serde(default)]
    pub key_template: Option<String>,
    pub value: serde_json::Value,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub partition: Option<i32>,
    #[serde(default)]
    pub auto_produce: Option<MessageAutoProduce>,
}

/// A superset of `AutoProduceConfig`. Simple rate-limited auto-produce fields
/// are first-class; advanced trigger configs (state_machine, scenarios) are
/// parsed but not yet executed — preserved so a later PR can wire them up
/// without another schema change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAutoProduce {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub rate_per_second: Option<u64>,
    #[serde(default)]
    pub duration_seconds: Option<u64>,
    #[serde(default)]
    pub total_count: Option<usize>,
    #[serde(default)]
    pub partition: Option<i32>,
    /// "rate" (default), "state_machine", or custom triggers. Only "rate" is
    /// honored today; other values flow through without causing a load error.
    #[serde(default)]
    pub trigger: Option<String>,
}

/// Result of expanding a `KafkaFixtureFile` for consumption downstream.
#[derive(Debug, Default)]
pub struct FlattenedFixtures {
    /// Topic definitions — used by the Metadata response.
    pub topics: Vec<KafkaTopicSpec>,
    /// Message-level fixtures — stored in `KafkaSpecRegistry` keyed by topic.
    pub fixtures: Vec<KafkaFixture>,
}

impl KafkaFixtureFile {
    /// Expand this file into `(topic specs, flat fixtures)`.
    ///
    /// Each `KafkaMessageSpec` becomes exactly one `KafkaFixture` with a
    /// synthetic identifier of the form `{topic}#{index}`. Only the
    /// rate-based auto_produce branch is forwarded; state-machine-driven
    /// messages load cleanly but don't emit an `AutoProduceConfig`.
    pub fn flatten(self) -> FlattenedFixtures {
        let mut fixtures = Vec::new();
        for topic in &self.topics {
            for (i, msg) in topic.messages.iter().enumerate() {
                fixtures.push(message_to_fixture(topic, i, msg));
            }
        }
        FlattenedFixtures {
            topics: self.topics,
            fixtures,
        }
    }
}

fn message_to_fixture(
    topic: &KafkaTopicSpec,
    index: usize,
    msg: &KafkaMessageSpec,
) -> KafkaFixture {
    let auto = msg.auto_produce.as_ref().and_then(|ap| {
        // Only forward rate-based triggers into the existing AutoProducer.
        // Anything else is preserved at the file level but not yet honored.
        match ap.trigger.as_deref() {
            None | Some("rate") => ap.rate_per_second.map(|rate| AutoProduceConfig {
                enabled: ap.enabled,
                rate_per_second: rate,
                duration_seconds: ap.duration_seconds,
                total_count: ap.total_count,
            }),
            _ => None,
        }
    });

    KafkaFixture {
        identifier: format!("{}#{}", topic.name, index),
        name: format!("{} message {}", topic.name, index),
        topic: topic.name.clone(),
        partition: msg.partition.or_else(|| msg.auto_produce.as_ref().and_then(|a| a.partition)),
        key_pattern: msg.key_template.clone(),
        value_template: msg.value.clone(),
        headers: msg.headers.clone(),
        auto_produce: auto,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_yaml() -> &'static str {
        r#"
fixture:
  name: "E-commerce Order Events"
  description: "demo"
  protocol: kafka

cluster:
  bootstrap_servers: "localhost:9092"
  cluster_id: "mockforge-cluster"

topics:
  - name: "orders.created"
    partitions: 3
    replication_factor: 1
    partitioning:
      strategy: "key_hash"
    config:
      retention_ms: 604800000
    messages:
      - key_template: "order-{{uuid}}"
        value:
          event_type: "order.created"
          order_id: "{{uuid}}"
        headers:
          event_version: "1.0"
        auto_produce:
          enabled: true
          rate_per_second: 10
          partition: null

  - name: "orders.status-updated"
    partitions: 3
    messages:
      - key_template: "{{context.order_id}}"
        value:
          event_type: "order.status_updated"
        auto_produce:
          enabled: true
          trigger: "state_machine"
          state_machine:
            initial_state: "pending"
"#
    }

    #[test]
    fn parses_nested_topology() {
        let file: KafkaFixtureFile = serde_yaml::from_str(sample_yaml()).unwrap();
        assert_eq!(file.fixture.as_ref().unwrap().name, "E-commerce Order Events");
        assert_eq!(file.cluster.as_ref().unwrap().cluster_id.as_deref(), Some("mockforge-cluster"));
        assert_eq!(file.topics.len(), 2);
        assert_eq!(file.topics[0].name, "orders.created");
        assert_eq!(file.topics[0].partitions, 3);
        assert_eq!(file.topics[0].messages.len(), 1);
    }

    #[test]
    fn flatten_keeps_topics_and_emits_one_fixture_per_message() {
        let file: KafkaFixtureFile = serde_yaml::from_str(sample_yaml()).unwrap();
        let flat = file.flatten();
        assert_eq!(flat.topics.len(), 2);
        assert_eq!(flat.fixtures.len(), 2);
        assert_eq!(flat.fixtures[0].identifier, "orders.created#0");
        assert_eq!(flat.fixtures[0].topic, "orders.created");
        assert!(flat.fixtures[0].auto_produce.as_ref().unwrap().enabled);
        assert_eq!(flat.fixtures[0].auto_produce.as_ref().unwrap().rate_per_second, 10);
    }

    #[test]
    fn state_machine_trigger_loads_without_auto_produce() {
        // Advanced triggers should parse successfully but not emit an
        // AutoProduceConfig (the rate-based executor would misinterpret them).
        let file: KafkaFixtureFile = serde_yaml::from_str(sample_yaml()).unwrap();
        let flat = file.flatten();
        let sm = &flat.fixtures[1];
        assert_eq!(sm.topic, "orders.status-updated");
        assert!(sm.auto_produce.is_none());
    }

    #[test]
    fn missing_optional_fields_parse_with_defaults() {
        let yaml = r#"
topics:
  - name: "plain"
    messages:
      - value: { k: "v" }
"#;
        let file: KafkaFixtureFile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(file.topics[0].partitions, 1);
        assert_eq!(file.topics[0].replication_factor, 1);
        assert!(file.topics[0].messages[0].key_template.is_none());
    }

    #[test]
    fn unknown_top_level_sections_are_ignored() {
        // Real fixtures include relationships/scenarios/failure_simulation —
        // those aren't implemented yet and must not break the load.
        let yaml = r#"
topics:
  - name: "t"
    messages:
      - value: {}
scenarios:
  - name: "Ignored"
failure_simulation:
  broker_failures:
    enabled: true
monitoring:
  prometheus:
    enabled: true
"#;
        let file: KafkaFixtureFile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(file.topics.len(), 1);
    }
}
