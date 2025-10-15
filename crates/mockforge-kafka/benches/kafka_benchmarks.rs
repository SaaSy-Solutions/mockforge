use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mockforge_kafka::{partitions::KafkaMessage, topics::Topic, topics::TopicConfig};
use serde_json;

fn bench_topic_creation(c: &mut Criterion) {
    c.bench_function("topic_creation", |b| {
        b.iter(|| {
            let config = TopicConfig::default();
            let topic = Topic::new("test-topic".to_string(), config);
            black_box(topic);
        });
    });
}

fn bench_partition_append(c: &mut Criterion) {
    let mut partition = mockforge_kafka::partitions::Partition::new(0);

    c.bench_function("partition_append", |b| {
        b.iter(|| {
            let message = KafkaMessage {
                offset: 0,
                timestamp: chrono::Utc::now().timestamp_millis(),
                key: Some(b"test-key".to_vec()),
                value: b"test-value".to_vec(),
                headers: vec![],
            };
            black_box(partition.append(message));
        });
    });
}

fn bench_message_generation(c: &mut Criterion) {
    let fixture = mockforge_kafka::fixtures::KafkaFixture {
        identifier: "bench-fixture".to_string(),
        name: "Benchmark Fixture".to_string(),
        topic: "test-topic".to_string(),
        partition: Some(0),
        key_pattern: Some("test-key-{{counter}}".to_string()),
        value_template: serde_json::json!({
            "event_type": "user_action",
            "user_id": "{{uuid}}",
            "timestamp": "{{now}}",
            "data": {
                "action": "click",
                "element": "button"
            }
        }),
        headers: std::collections::HashMap::new(),
        auto_produce: None,
    };

    let mut counter = 0;
    c.bench_function("message_generation", |b| {
        b.iter(|| {
            let mut context = std::collections::HashMap::new();
            context.insert("counter".to_string(), counter.to_string());
            counter += 1;
            let message = fixture.generate_message(&context).unwrap();
            black_box(message);
        });
    });
}

fn bench_consumer_group_join(c: &mut Criterion) {
    let mut manager = mockforge_kafka::consumer_groups::ConsumerGroupManager::new();

    c.bench_function("consumer_group_join", |b| {
        b.iter(|| {
            let result = manager.join_group("test-group", "member-1", "client-1");
            black_box(result);
        });
    });
}

criterion_group!(
    benches,
    bench_topic_creation,
    bench_partition_append,
    bench_message_generation,
    bench_consumer_group_join
);
criterion_main!(benches);
