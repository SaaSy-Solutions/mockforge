//! Runtime for the three fixture-trigger types defined in the
//! nested-topology YAML: **state machines**, **scenarios**, and
//! **relationships**.
//!
//! Each of these has a concrete struct in [`crate::fixture_file`], but
//! before this module existed they just sat in memory — the broker
//! deserialized them and then never looked at them. This module wires
//! them up to the broker's produce path.
//!
//! | Trigger        | Fires                                       | Runtime effect                                                                 |
//! |----------------|----------------------------------------------|--------------------------------------------------------------------------------|
//! | state_machine  | one tokio task per fixture at broker start   | walks the state graph, emits one message per visit, stops at a terminal state |
//! | scenarios      | one tokio task per scenario at broker start  | walks the sequence, sleeping between steps                                    |
//! | relationships  | synchronously on every successful Produce    | one dependent emission per source record per matching relationship            |
//!
//! The hot-path work — relationships — is cheap: an `Arc` of the
//! relationship vec is stashed on the broker, and for each produced
//! record we scan it linearly. Workloads with many relationships would
//! want an index by `from_topic`, but the current fixtures hold fewer
//! than a dozen, so linear scan wins on simplicity.

use crate::broker::KafkaMockBroker;
use crate::fixture_file::{RelationshipSpec, ScenarioSpec, StateMachineSpec, StateSpec};
use crate::fixtures::KafkaFixture;
use crate::partitions::KafkaMessage;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Produce-time fixture runtime: the subset of fixture-file data that the
/// broker needs to look at while serving requests. Shared by `Arc` — the
/// broker stores one and clones it into each produce path.
pub struct FixtureRuntime {
    /// First fixture per topic, used to render messages for scenario
    /// steps and relationship target emissions. Later fixtures for the
    /// same topic are reachable via `all_by_topic` if finer selection
    /// is needed; for now the executor uses the first.
    first_by_topic: HashMap<String, Arc<KafkaFixture>>,
    /// Fixtures keyed by identifier (`{topic}#{index}`). Used by the
    /// state-machine executor to find its template.
    by_identifier: HashMap<String, Arc<KafkaFixture>>,
    /// Relationships scanned on every produce.
    relationships: Arc<Vec<Arc<RelationshipSpec>>>,
}

impl FixtureRuntime {
    pub fn new(fixtures: &[Arc<KafkaFixture>], relationships: &[Arc<RelationshipSpec>]) -> Self {
        let mut first_by_topic: HashMap<String, Arc<KafkaFixture>> = HashMap::new();
        let mut by_identifier: HashMap<String, Arc<KafkaFixture>> = HashMap::new();
        for f in fixtures {
            first_by_topic.entry(f.topic.clone()).or_insert_with(|| f.clone());
            by_identifier.insert(f.identifier.clone(), f.clone());
        }
        Self {
            first_by_topic,
            by_identifier,
            relationships: Arc::new(relationships.to_vec()),
        }
    }

    pub fn fixture_for_topic(&self, topic: &str) -> Option<Arc<KafkaFixture>> {
        self.first_by_topic.get(topic).cloned()
    }

    pub fn fixture_by_identifier(&self, id: &str) -> Option<Arc<KafkaFixture>> {
        self.by_identifier.get(id).cloned()
    }
}

/// Install fixture triggers on a running broker.
///
/// Spawns one background task per enabled state machine and one per
/// enabled + probability-sampled scenario. Returns the runtime so the
/// broker can hand it to `on_produced_records` on the produce path.
pub async fn install(
    broker: Arc<KafkaMockBroker>,
    fixtures: &[Arc<KafkaFixture>],
    state_machines: &[(String, Arc<StateMachineSpec>)],
    scenarios: &[Arc<ScenarioSpec>],
    relationships: &[Arc<RelationshipSpec>],
) -> Arc<FixtureRuntime> {
    let runtime = Arc::new(FixtureRuntime::new(fixtures, relationships));

    for (fixture_id, spec) in state_machines {
        if let Some(fixture) = runtime.fixture_by_identifier(fixture_id) {
            let broker = Arc::clone(&broker);
            let spec = Arc::clone(spec);
            tokio::spawn(async move {
                run_state_machine(broker, fixture, spec).await;
            });
        } else {
            tracing::warn!("state machine references unknown fixture {fixture_id}; skipping");
        }
    }

    for scenario in scenarios {
        if !scenario.enabled {
            continue;
        }
        if let Some(p) = scenario.probability {
            if !sample_probability(p) {
                tracing::debug!("scenario {} skipped by probability {}", scenario.name, p);
                continue;
            }
        }
        let broker = Arc::clone(&broker);
        let runtime = Arc::clone(&runtime);
        let scenario = Arc::clone(scenario);
        tokio::spawn(async move {
            run_scenario(broker, runtime, scenario).await;
        });
    }

    runtime
}

/// Called from the produce path after records have been appended to
/// topic storage. Fires any relationships whose `from_topic` matches.
pub async fn on_produced_records(
    broker: &Arc<KafkaMockBroker>,
    runtime: &Arc<FixtureRuntime>,
    source_topic: &str,
    records: &[KafkaMessage],
) {
    if runtime.relationships.is_empty() {
        return;
    }
    for rel in runtime.relationships.iter() {
        if rel.from_topic != source_topic {
            continue;
        }
        let Some(target_fixture) = runtime.fixture_for_topic(&rel.to_topic) else {
            tracing::warn!("relationship points at unknown to_topic {}; skipping", rel.to_topic);
            continue;
        };
        for record in records {
            let context = extract_context(record, &rel.key_mapping);
            if let Err(e) = emit(broker, &target_fixture, &context).await {
                tracing::warn!("relationship emission to {} failed: {}", rel.to_topic, e);
            }
        }
    }
}

// =========================================================================
// State machine runner
// =========================================================================

async fn run_state_machine(
    broker: Arc<KafkaMockBroker>,
    fixture: Arc<KafkaFixture>,
    spec: Arc<StateMachineSpec>,
) {
    let states: HashMap<&str, &StateSpec> =
        spec.states.iter().map(|s| (s.name.as_str(), s)).collect();

    let mut current = spec.initial_state.clone();
    loop {
        let Some(state) = states.get(current.as_str()) else {
            tracing::warn!(
                "state machine for {} references unknown state {}; stopping",
                fixture.identifier,
                current
            );
            break;
        };

        let mut context = HashMap::new();
        context.insert("state".to_string(), state.name.clone());
        if let Err(e) = emit(&broker, &fixture, &context).await {
            tracing::warn!("state-machine emit failed: {e}");
        }

        if state.next_states.is_empty() {
            tracing::debug!(
                "state machine for {} terminated at {}",
                fixture.identifier,
                state.name
            );
            break;
        }

        let delay = sample_delay(&state.delay_ms);
        if delay > Duration::ZERO {
            tokio::time::sleep(delay).await;
        }

        let next_idx = weighted_pick(&state.probability, state.next_states.len());
        current = state.next_states[next_idx].clone();
    }
}

// =========================================================================
// Scenario runner
// =========================================================================

async fn run_scenario(
    broker: Arc<KafkaMockBroker>,
    runtime: Arc<FixtureRuntime>,
    scenario: Arc<ScenarioSpec>,
) {
    tracing::info!("scenario {} starting ({} steps)", scenario.name, scenario.sequence.len());
    for step in &scenario.sequence {
        let delay = sample_delay(&step.delay_ms);
        if delay > Duration::ZERO {
            tokio::time::sleep(delay).await;
        }
        let Some(fixture) = runtime.fixture_for_topic(&step.topic) else {
            tracing::warn!(
                "scenario {} step points at unknown topic {}; skipping step",
                scenario.name,
                step.topic
            );
            continue;
        };
        let mut context = HashMap::new();
        context.insert("scenario".to_string(), scenario.name.clone());
        if let Some(name) = &step.message {
            context.insert("message_template".to_string(), name.clone());
        }
        if let Err(e) = emit(&broker, &fixture, &context).await {
            tracing::warn!(
                "scenario {} step for topic {} failed: {}",
                scenario.name,
                step.topic,
                e
            );
        }
    }
    tracing::info!("scenario {} finished", scenario.name);
}

// =========================================================================
// Emit helper
// =========================================================================

async fn emit(
    broker: &Arc<KafkaMockBroker>,
    fixture: &KafkaFixture,
    context: &HashMap<String, String>,
) -> mockforge_core::Result<()> {
    let message = fixture.generate_message(context)?;
    let mut topics = broker.topics.write().await;
    let topic = topics.entry(fixture.topic.clone()).or_insert_with(|| {
        crate::topics::Topic::new(fixture.topic.clone(), crate::topics::TopicConfig::default())
    });
    let partition = fixture
        .partition
        .unwrap_or_else(|| topic.assign_partition(message.key.as_deref()));
    topic.produce(partition, message).await?;
    Ok(())
}

// =========================================================================
// Extraction + sampling helpers
// =========================================================================

/// Build a template context from a source record. For each entry
/// `source_field -> target_field` in the mapping, pull `source_field`
/// out of the record's JSON-encoded value and bind it in the context
/// under `target_field`. If the value isn't valid JSON or the field is
/// absent, fall back to the raw message key (as UTF-8) for every
/// mapping entry.
fn extract_context(
    record: &KafkaMessage,
    mapping: &HashMap<String, String>,
) -> HashMap<String, String> {
    if mapping.is_empty() {
        return HashMap::new();
    }
    let key_str = record.key.as_ref().and_then(|k| std::str::from_utf8(k).ok()).unwrap_or("");
    let json: Option<serde_json::Value> = serde_json::from_slice(&record.value).ok();
    let mut context = HashMap::new();
    for (source_field, target_field) in mapping {
        let value = json
            .as_ref()
            .and_then(|v| v.get(source_field))
            .and_then(|v| match v {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Number(n) => Some(n.to_string()),
                serde_json::Value::Bool(b) => Some(b.to_string()),
                _ => None,
            })
            .unwrap_or_else(|| key_str.to_string());
        context.insert(target_field.clone(), value);
    }
    context
}

/// Sample a delay from a `[min_ms, max_ms]` pair. Returns `Duration::ZERO`
/// when the vec is empty, or a single fixed delay when only one value is
/// given. Everything else samples uniformly.
fn sample_delay(delay_ms: &[u64]) -> Duration {
    match delay_ms {
        [] => Duration::ZERO,
        [fixed] => Duration::from_millis(*fixed),
        [min, max] => {
            let (lo, hi) = if min <= max {
                (*min, *max)
            } else {
                (*max, *min)
            };
            if lo == hi {
                Duration::from_millis(lo)
            } else {
                let sampled = rand::random_range(lo..=hi);
                Duration::from_millis(sampled)
            }
        }
        other => {
            // Unexpected shape — use the first value as a fixed delay.
            Duration::from_millis(other[0])
        }
    }
}

fn sample_probability(p: f64) -> bool {
    if p <= 0.0 {
        return false;
    }
    if p >= 1.0 {
        return true;
    }
    rand::random::<f64>() < p
}

/// Pick an index in `[0, len)`. If `weights` has the same length, sample
/// proportional to them (normalized — summing to > 1 is fine). Otherwise
/// pick uniformly.
fn weighted_pick(weights: &[f64], len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    if weights.len() != len {
        return rand::random_range(0..len);
    }
    let total: f64 = weights.iter().sum();
    if total <= 0.0 {
        return rand::random_range(0..len);
    }
    let r = rand::random::<f64>() * total;
    let mut acc = 0.0;
    for (i, w) in weights.iter().enumerate() {
        acc += w;
        if r < acc {
            return i;
        }
    }
    len - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_pick_respects_weights() {
        // Heavily weight index 2; ~1000 samples should land there most often.
        let weights = vec![0.0, 0.0, 1.0];
        let mut counts = [0usize; 3];
        for _ in 0..500 {
            counts[weighted_pick(&weights, 3)] += 1;
        }
        assert!(counts[2] > 400, "index 2 should dominate: {counts:?}");
        assert_eq!(counts[0], 0);
        assert_eq!(counts[1], 0);
    }

    #[test]
    fn weighted_pick_falls_back_to_uniform_on_mismatch() {
        // Wrong-length weights → uniform. Not a statistical test; just
        // verifies no panic + stays in range.
        for _ in 0..50 {
            let idx = weighted_pick(&[0.5], 4);
            assert!(idx < 4);
        }
    }

    #[test]
    fn sample_delay_shapes() {
        assert_eq!(sample_delay(&[]), Duration::ZERO);
        assert_eq!(sample_delay(&[5]), Duration::from_millis(5));
        for _ in 0..20 {
            let d = sample_delay(&[10, 20]);
            assert!(d.as_millis() >= 10);
            assert!(d.as_millis() <= 20);
        }
        // Reversed bounds still yield a value inside the interval.
        for _ in 0..20 {
            let d = sample_delay(&[20, 10]);
            assert!(d.as_millis() >= 10);
            assert!(d.as_millis() <= 20);
        }
    }

    #[test]
    fn extract_context_from_json_value() {
        let record = KafkaMessage {
            offset: 0,
            timestamp: 0,
            key: None,
            value: br#"{"order_id":"order-42","total":17.5}"#.to_vec(),
            headers: vec![],
        };
        let mut mapping = HashMap::new();
        mapping.insert("order_id".to_string(), "order_id".to_string());
        mapping.insert("total".to_string(), "order_total".to_string());
        let ctx = extract_context(&record, &mapping);
        assert_eq!(ctx.get("order_id").map(String::as_str), Some("order-42"));
        assert_eq!(ctx.get("order_total").map(String::as_str), Some("17.5"));
    }

    #[test]
    fn extract_context_falls_back_to_key_when_value_not_json() {
        let record = KafkaMessage {
            offset: 0,
            timestamp: 0,
            key: Some(b"fallback-key".to_vec()),
            value: b"not json".to_vec(),
            headers: vec![],
        };
        let mut mapping = HashMap::new();
        mapping.insert("order_id".to_string(), "order_id".to_string());
        let ctx = extract_context(&record, &mapping);
        assert_eq!(ctx.get("order_id").map(String::as_str), Some("fallback-key"));
    }

    #[test]
    fn sample_probability_bounds() {
        assert!(!sample_probability(0.0));
        assert!(sample_probability(1.0));
    }
}
