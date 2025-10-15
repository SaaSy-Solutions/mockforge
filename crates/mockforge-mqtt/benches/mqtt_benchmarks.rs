use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mockforge_core::protocol_abstraction::SpecRegistry;
use mockforge_mqtt::{
    qos::MessageState, qos::QoSHandler, MqttFixture, MqttResponse, MqttSpecRegistry, TopicTree,
};

fn benchmark_topic_matching(c: &mut Criterion) {
    let mut tree = TopicTree::new();

    // Add many subscriptions
    for i in 0..1000 {
        tree.subscribe(&format!("sensors/{}/+", i), 1, &format!("client{}", i));
    }

    c.bench_function("topic_matching_1000_subs", |b| {
        b.iter(|| {
            let matches = tree.match_topic(black_box("sensors/500/temperature"));
            black_box(matches);
        })
    });
}

fn benchmark_topic_wildcards(c: &mut Criterion) {
    let mut tree = TopicTree::new();

    // Add subscriptions with different wildcard patterns
    tree.subscribe("sensors/+/temperature", 0, "client1");
    tree.subscribe("devices/#", 1, "client2");
    tree.subscribe("alerts/+/+/status", 2, "client3");

    let mut group = c.benchmark_group("topic_wildcards");

    group.bench_function("single_level_wildcard", |b| {
        b.iter(|| {
            let matches = tree.match_topic(black_box("sensors/room1/temperature"));
            black_box(matches);
        })
    });

    group.bench_function("multi_level_wildcard", |b| {
        b.iter(|| {
            let matches = tree.match_topic(black_box("devices/room1/light1/brightness"));
            black_box(matches);
        })
    });

    group.bench_function("complex_pattern", |b| {
        b.iter(|| {
            let matches = tree.match_topic(black_box("alerts/system/cpu/status"));
            black_box(matches);
        })
    });

    group.finish();
}

fn benchmark_retained_messages(c: &mut Criterion) {
    let mut tree = TopicTree::new();

    // Store many retained messages
    for i in 0..1000 {
        tree.retain_message(&format!("retained/topic/{}", i), format!("data{}", i).into_bytes(), 1);
    }

    c.bench_function("retained_message_lookup", |b| {
        b.iter(|| {
            let retained = tree.get_retained(black_box("retained/topic/500"));
            black_box(retained);
        })
    });
}

fn benchmark_spec_registry(c: &mut Criterion) {
    let mut registry = MqttSpecRegistry::new();

    // Add many fixtures
    for i in 0..100 {
        let fixture = MqttFixture {
            identifier: format!("fixture{}", i),
            name: format!("Fixture {}", i),
            topic_pattern: format!(r"^test/topic{}/[^/]+$", i),
            qos: 1,
            retained: false,
            response: MqttResponse {
                payload: serde_json::json!({"value": i}),
            },
            auto_publish: None,
        };
        registry.add_fixture(fixture);
    }

    c.bench_function("spec_registry_operations", |b| {
        b.iter(|| {
            let operations = registry.operations();
            black_box(operations);
        })
    });
}

fn benchmark_qos_handling(c: &mut Criterion) {
    let handler = QoSHandler::new();

    let message = MessageState {
        packet_id: 1,
        topic: "test/topic".to_string(),
        payload: b"test data".to_vec(),
        qos: mockforge_mqtt::qos::QoS::AtLeastOnce,
        retained: false,
        timestamp: 1234567890,
    };

    let mut group = c.benchmark_group("qos_handling");

    group.bench_function("qos_0", |b| {
        b.iter(|| {
            let result = handler.handle_qo_s0(black_box(message.clone()));
            #[allow(unused_must_use)]
            black_box(result);
        })
    });

    group.bench_function("qos_1", |b| {
        b.iter(|| {
            let result = handler.handle_qo_s1(black_box(message.clone()), black_box("client1"));
            #[allow(unused_must_use)]
            black_box(result);
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_topic_matching,
    benchmark_topic_wildcards,
    benchmark_retained_messages,
    benchmark_spec_registry,
    benchmark_qos_handling
);
criterion_main!(benches);
