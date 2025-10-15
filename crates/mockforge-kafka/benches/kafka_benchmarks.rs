use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mockforge_kafka::{KafkaMockBroker, topics::Topic, topics::TopicConfig, partitions::KafkaMessage};
use mockforge_core::config::KafkaConfig;
use std::collections::HashMap;

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
    // TODO: Implement with actual fixture
    c.bench_function("message_generation", |b| {
        b.iter(|| {
            // Simulate message generation
            let message = KafkaMessage {
                offset: 0,
                timestamp: chrono::Utc::now().timestamp_millis(),
                key: Some(b"key".to_vec()),
                value: br#"{"test": "data"}"#.to_vec(),
                headers: vec![],
            };
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

criterion_group!(benches,
    bench_topic_creation,
    bench_partition_append,
    bench_message_generation,
    bench_consumer_group_join
);
criterion_main!(benches);
