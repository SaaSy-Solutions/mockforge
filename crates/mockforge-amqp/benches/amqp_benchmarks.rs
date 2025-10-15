use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mockforge_amqp::exchanges::{ExchangeManager, ExchangeType};
use mockforge_amqp::messages::{Message, MessageProperties};
use mockforge_amqp::queues::QueueManager;
use std::collections::HashMap;

fn bench_message_creation(c: &mut Criterion) {
    c.bench_function("message_creation", |b| {
        b.iter(|| {
            let message = Message {
                properties: MessageProperties {
                    content_type: Some("application/json".to_string()),
                    delivery_mode: mockforge_amqp::messages::DeliveryMode::Persistent,
                    priority: 1,
                    headers: HashMap::from([("test".to_string(), "value".to_string())]),
                    ..MessageProperties::default()
                },
                body: black_box(b"{\"test\": \"benchmark data\"}".to_vec()),
                routing_key: "test.route".to_string(),
            };
            black_box(message);
        });
    });
}

fn bench_exchange_routing_direct(c: &mut Criterion) {
    let mut exchange_manager = ExchangeManager::new();
    exchange_manager.declare_exchange("direct-test".to_string(), ExchangeType::Direct, true, false);

    c.bench_function("exchange_routing_direct", |b| {
        b.iter(|| {
            let message = Message {
                properties: MessageProperties::default(),
                body: black_box(b"test data".to_vec()),
                routing_key: "test.key".to_string(),
            };
            let exchange = exchange_manager.get_exchange("direct-test").unwrap();
            let routes = exchange.route_message(&message, "test.key");
            black_box(routes);
        });
    });
}

fn bench_exchange_routing_topic(c: &mut Criterion) {
    let mut exchange_manager = ExchangeManager::new();
    exchange_manager.declare_exchange("topic-test".to_string(), ExchangeType::Topic, true, false);

    c.bench_function("exchange_routing_topic", |b| {
        b.iter(|| {
            let message = Message {
                properties: MessageProperties::default(),
                body: black_box(b"test data".to_vec()),
                routing_key: "order.created.user.123".to_string(),
            };
            let exchange = exchange_manager.get_exchange("topic-test").unwrap();
            let routes = exchange.route_message(&message, "order.created.user.123");
            black_box(routes);
        });
    });
}

fn bench_exchange_routing_fanout(c: &mut Criterion) {
    let mut exchange_manager = ExchangeManager::new();
    exchange_manager.declare_exchange("fanout-test".to_string(), ExchangeType::Fanout, true, false);

    c.bench_function("exchange_routing_fanout", |b| {
        b.iter(|| {
            let message = Message {
                properties: MessageProperties::default(),
                body: black_box(b"test data".to_vec()),
                routing_key: "any.key".to_string(),
            };
            let exchange = exchange_manager.get_exchange("fanout-test").unwrap();
            let routes = exchange.route_message(&message, "any.key");
            black_box(routes);
        });
    });
}

fn bench_queue_operations(c: &mut Criterion) {
    let mut queue_manager = QueueManager::new();
    queue_manager.declare_queue("test-queue".to_string(), true, false, false);

    c.bench_function("queue_enqueue_dequeue", |b| {
        b.iter(|| {
            let queue = queue_manager.get_queue_mut("test-queue").unwrap();
            let message = Message {
                properties: MessageProperties::default(),
                body: black_box(b"benchmark message".to_vec()),
                routing_key: "test".to_string(),
            };
            let queued_message = mockforge_amqp::messages::QueuedMessage::new(message);
            queue.enqueue(queued_message).unwrap();
            let dequeued = queue.dequeue();
            black_box(dequeued);
        });
    });
}

fn bench_topic_pattern_matching(c: &mut Criterion) {
    c.bench_function("topic_pattern_matching", |b| {
        b.iter(|| {
            let routing_parts = black_box(vec!["order", "created", "user", "123"]);
            let pattern_parts = black_box(vec!["order", "*", "user", "#"]);
            let result = mockforge_amqp::exchanges::Exchange::matches_topic_pattern(
                &routing_parts,
                &pattern_parts,
            );
            black_box(result);
        });
    });
}

criterion_group!(
    benches,
    bench_message_creation,
    bench_exchange_routing_direct,
    bench_exchange_routing_topic,
    bench_exchange_routing_fanout,
    bench_queue_operations,
    bench_topic_pattern_matching
);
criterion_main!(benches);
