//! Regression coverage for nested-topology fixture loading against the
//! bundled example file. If this test fails, a Kafka docker run pointed at
//! `/app/examples/protocols/kafka` would silently start with zero topics
//! (or worse, fail to bind entirely) — exactly the symptom this PR fixes.

use mockforge_core::config::KafkaConfig;
use mockforge_kafka::fixtures::load_kafka_fixtures_from_dir;
use mockforge_kafka::KafkaSpecRegistry;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

fn bundled_fixtures_dir() -> PathBuf {
    // Walk up from CARGO_MANIFEST_DIR (crates/mockforge-kafka) to the repo
    // root so this works from either cargo-test in-place or a worktree.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap().parent().unwrap().join("examples/protocols/kafka")
}

#[test]
fn loads_bundled_order_events_yaml() {
    let flat = load_kafka_fixtures_from_dir(&bundled_fixtures_dir())
        .expect("bundled examples/protocols/kafka should load cleanly");

    let topic_names: Vec<_> = flat.topics.iter().map(|t| t.name.as_str()).collect();
    assert!(topic_names.contains(&"orders.created"), "got {topic_names:?}");
    assert!(topic_names.contains(&"orders.status-updated"), "got {topic_names:?}");
    assert!(topic_names.contains(&"payments.processed"), "got {topic_names:?}");
    assert!(topic_names.contains(&"inventory.updated"), "got {topic_names:?}");

    // inventory.updated is declared with 5 partitions in the YAML — check the
    // partition count actually made it through the parser.
    let inventory = flat
        .topics
        .iter()
        .find(|t| t.name == "inventory.updated")
        .expect("inventory topic");
    assert_eq!(inventory.partitions, 5);

    // Every topic has at least one message template in the example file.
    assert_eq!(flat.fixtures.len(), flat.topics.len());

    // orders.created is the only one with rate-based auto_produce; the
    // state-machine-driven topics load but don't emit an AutoProduceConfig.
    let orders = flat
        .fixtures
        .iter()
        .find(|f| f.topic == "orders.created")
        .expect("orders.created fixture");
    let ap = orders.auto_produce.as_ref().expect("rate auto_produce");
    assert!(ap.enabled);
    assert_eq!(ap.rate_per_second, 10);

    let status = flat
        .fixtures
        .iter()
        .find(|f| f.topic == "orders.status-updated")
        .expect("orders.status-updated fixture");
    assert!(
        status.auto_produce.is_none(),
        "state_machine-triggered messages must not emit a rate-based AutoProduceConfig"
    );
}

#[tokio::test]
async fn spec_registry_preregisters_topics_from_bundled_yaml() {
    // Also exercise the KafkaSpecRegistry path — when the registry is built
    // with the bundled fixtures_dir, the topic store should have all four
    // topics with their declared partition counts, so subsequent Metadata
    // handling (future PR) has real state to serialize.
    let config = KafkaConfig {
        fixtures_dir: Some(bundled_fixtures_dir()),
        ..KafkaConfig::default()
    };

    let topic_map = Arc::new(RwLock::new(HashMap::new()));
    let registry = KafkaSpecRegistry::new(config, topic_map.clone())
        .await
        .expect("registry should build against bundled fixture");

    let topics = topic_map.read().await;
    assert!(topics.contains_key("orders.created"));
    assert!(topics.contains_key("orders.status-updated"));
    assert!(topics.contains_key("payments.processed"));
    let inventory = topics.get("inventory.updated").expect("inventory topic pre-registered");
    assert_eq!(inventory.partitions.len(), 5);

    // The registry's accessor should expose the same topic specs the
    // Metadata handler will consume.
    assert!(registry
        .topic_specs()
        .iter()
        .any(|t| t.name == "inventory.updated" && t.partitions == 5));
}
