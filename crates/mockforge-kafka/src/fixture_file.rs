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
//! Unknown fields are silently ignored so advanced YAML sections
//! (failure_simulation, monitoring) don't break the load. The three
//! trigger sections the issue calls for — state_machine, scenarios, and
//! relationships — are now first-class: they deserialize into concrete
//! structs that `fixture_executor` consumes at broker startup.

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
    /// Scenario-based sequences of topic emissions. Each scenario fires
    /// once on startup if `enabled` and survives random sampling against
    /// `probability`.
    #[serde(default)]
    pub scenarios: Vec<ScenarioSpec>,
    /// Causal links — producing to `from_topic` triggers a dependent
    /// emission on `to_topic` with correlated keys.
    #[serde(default)]
    pub relationships: Vec<RelationshipSpec>,
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

/// A superset of `AutoProduceConfig`. Simple rate-limited auto-produce
/// fields are first-class; `state_machine` drives the probabilistic state
/// graph executor when `trigger == "state_machine"`.
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
    /// `"rate"` (default) drives the rate-based `AutoProducer`.
    /// `"state_machine"` drives `fixture_executor::StateMachineExecutor`.
    /// Other values deserialize cleanly but don't hook anything up.
    #[serde(default)]
    pub trigger: Option<String>,
    /// Graph definition used by the `"state_machine"` trigger.
    #[serde(default)]
    pub state_machine: Option<StateMachineSpec>,
}

/// A probabilistic state graph that emits one message per state visit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachineSpec {
    pub initial_state: String,
    #[serde(default)]
    pub states: Vec<StateSpec>,
}

/// One node in a `StateMachineSpec`.
///
/// Terminal states have `next_states` empty — the executor stops when it
/// reaches one. When `next_states` has entries, `probability` (same length)
/// selects which one to visit next, and `delay_ms` (a `[min, max]` pair)
/// controls how long before the transition fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSpec {
    pub name: String,
    #[serde(default)]
    pub next_states: Vec<String>,
    #[serde(default)]
    pub probability: Vec<f64>,
    /// `[min_ms, max_ms]` — sampled uniformly. Absent = fire immediately.
    #[serde(default)]
    pub delay_ms: Vec<u64>,
}

/// One top-level scenario: a sequence of topic emissions fired once when
/// the broker starts, gated by `enabled` and `probability`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSpec {
    pub name: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    /// 0.0–1.0 chance the scenario actually runs. Absent = always run.
    #[serde(default)]
    pub probability: Option<f64>,
    #[serde(default)]
    pub sequence: Vec<ScenarioStep>,
}

fn yes() -> bool {
    true
}

/// One step inside a scenario: emit a message on `topic` after
/// `delay_ms[0]..=delay_ms[1]` ms (absent = fire immediately).
///
/// `message` is a free-form identifier in the fixture file (e.g.
/// `"order_created_template"`). Today it's informational — the executor
/// emits the first known message template for the referenced topic. A
/// later PR can make `message` select among the topic's templates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioStep {
    pub topic: String,
    #[serde(default)]
    pub message: Option<String>,
    /// `[min_ms, max_ms]` — sampled uniformly. Absent = fire immediately.
    #[serde(default)]
    pub delay_ms: Vec<u64>,
}

/// A causal link: when a record lands on `from_topic`, emit a record on
/// `to_topic` using `key_mapping` to correlate identifiers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSpec {
    pub from_topic: String,
    pub to_topic: String,
    /// `"one_to_one"` / `"one_to_many"` — informational; the executor
    /// always emits exactly one `to_topic` record per `from_topic` record,
    /// and 1-to-many fan-out is expressed through multiple relationships
    /// or through the rate/state-machine triggers on `to_topic`.
    #[serde(default)]
    pub relationship: Option<String>,
    /// Map of `source_field -> target_field`. For each entry, the
    /// executor pulls `source_field` out of the source record's JSON
    /// value and puts it in the rendering context under `target_field`.
    /// If the source value isn't valid JSON, the raw message key is used
    /// as the value for every mapping entry.
    #[serde(default)]
    pub key_mapping: HashMap<String, String>,
}

/// Result of expanding a `KafkaFixtureFile` for consumption downstream.
#[derive(Debug, Default)]
pub struct FlattenedFixtures {
    /// Topic definitions — used by the Metadata response.
    pub topics: Vec<KafkaTopicSpec>,
    /// Message-level fixtures — stored in `KafkaSpecRegistry` keyed by topic.
    pub fixtures: Vec<KafkaFixture>,
    /// Scenarios aggregated from every fixture file. Executed once on
    /// broker startup by `fixture_executor`.
    pub scenarios: Vec<ScenarioSpec>,
    /// Relationships aggregated from every fixture file. Fire on every
    /// successful produce.
    pub relationships: Vec<RelationshipSpec>,
    /// State-machine definitions keyed by the fixture identifier
    /// (`{topic}#{index}`) that owns them. Drives the state-machine
    /// executor.
    pub state_machines: Vec<(String, StateMachineSpec)>,
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
        let mut state_machines = Vec::new();
        for topic in &self.topics {
            for (i, msg) in topic.messages.iter().enumerate() {
                fixtures.push(message_to_fixture(topic, i, msg));
                if let Some(ap) = &msg.auto_produce {
                    if ap.enabled && ap.trigger.as_deref() == Some("state_machine") {
                        if let Some(sm) = &ap.state_machine {
                            state_machines.push((format!("{}#{}", topic.name, i), sm.clone()));
                        }
                    }
                }
            }
        }
        FlattenedFixtures {
            topics: self.topics,
            fixtures,
            scenarios: self.scenarios,
            relationships: self.relationships,
            state_machines,
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
    fn flattens_state_machine_spec_into_index() {
        let yaml = r#"
topics:
  - name: "orders.status-updated"
    messages:
      - value: { event_type: "order.status_updated" }
        auto_produce:
          enabled: true
          trigger: "state_machine"
          state_machine:
            initial_state: "pending"
            states:
              - name: "pending"
                next_states: ["processing"]
                probability: [1.0]
                delay_ms: [1000, 2000]
              - name: "processing"
                next_states: ["shipped", "cancelled"]
                probability: [0.9, 0.1]
                delay_ms: [2000, 5000]
              - name: "shipped"
                next_states: []
              - name: "cancelled"
                next_states: []
"#;
        let file: KafkaFixtureFile = serde_yaml::from_str(yaml).unwrap();
        let flat = file.flatten();
        assert_eq!(flat.state_machines.len(), 1);
        let (id, sm) = &flat.state_machines[0];
        assert_eq!(id, "orders.status-updated#0");
        assert_eq!(sm.initial_state, "pending");
        assert_eq!(sm.states.len(), 4);
        assert_eq!(sm.states[1].next_states, vec!["shipped", "cancelled"]);
        assert_eq!(sm.states[1].probability, vec![0.9, 0.1]);
        assert_eq!(sm.states[2].next_states, Vec::<String>::new());
    }

    #[test]
    fn parses_scenarios_and_relationships_sections() {
        let yaml = r#"
topics:
  - name: "orders.created"
    messages:
      - value: { k: "v" }
scenarios:
  - name: "Successful Order"
    enabled: true
    probability: 0.85
    sequence:
      - topic: "orders.created"
      - topic: "payments.processed"
        delay_ms: [1000, 3000]
relationships:
  - from_topic: "orders.created"
    to_topic: "payments.processed"
    relationship: "one_to_one"
    key_mapping:
      order_id: "order_id"
"#;
        let file: KafkaFixtureFile = serde_yaml::from_str(yaml).unwrap();
        let flat = file.flatten();
        assert_eq!(flat.scenarios.len(), 1);
        assert_eq!(flat.scenarios[0].name, "Successful Order");
        assert_eq!(flat.scenarios[0].probability, Some(0.85));
        assert_eq!(flat.scenarios[0].sequence.len(), 2);
        assert_eq!(flat.scenarios[0].sequence[1].topic, "payments.processed");
        assert_eq!(flat.scenarios[0].sequence[1].delay_ms, vec![1000, 3000]);

        assert_eq!(flat.relationships.len(), 1);
        assert_eq!(flat.relationships[0].from_topic, "orders.created");
        assert_eq!(flat.relationships[0].to_topic, "payments.processed");
        assert_eq!(
            flat.relationships[0].key_mapping.get("order_id"),
            Some(&"order_id".to_string())
        );
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
