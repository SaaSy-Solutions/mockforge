use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mockforge_core::protocol_abstraction::{
    MessagePattern, Protocol, ProtocolRequest, UnifiedFixture,
};

fn bench_protocol_request_creation(c: &mut Criterion) {
    c.bench_function("protocol_request_creation", |b| {
        b.iter(|| {
            let request = ProtocolRequest {
                protocol: Protocol::Http,
                pattern: MessagePattern::RequestResponse,
                operation: black_box("GET".to_string()),
                path: black_box("/api/users".to_string()),
                topic: None,
                routing_key: None,
                partition: None,
                qos: None,
                metadata: black_box(std::collections::HashMap::new()),
                body: None,
                client_ip: None,
            };
            black_box(request);
        });
    });
}

fn bench_fixture_matching(c: &mut Criterion) {
    let fixture = UnifiedFixture {
        id: "bench-fixture".to_string(),
        name: "Benchmark Fixture".to_string(),
        description: "Fixture for benchmarking".to_string(),
        protocol: Protocol::Http,
        request: mockforge_core::protocol_abstraction::FixtureRequest {
            pattern: Some(MessagePattern::RequestResponse),
            operation: Some("GET".to_string()),
            path: Some("/api/users".to_string()),
            topic: None,
            routing_key: None,
            partition: None,
            qos: None,
            headers: std::collections::HashMap::new(),
            body_pattern: None,
            custom_matcher: None,
        },
        response: mockforge_core::protocol_abstraction::FixtureResponse {
            status: mockforge_core::protocol_abstraction::FixtureStatus::Http(200),
            headers: std::collections::HashMap::new(),
            body: Some(serde_json::json!({"users": ["test"]})),
            content_type: Some("application/json".to_string()),
            delay_ms: 0,
            template_vars: std::collections::HashMap::new(),
        },
        metadata: std::collections::HashMap::new(),
        enabled: true,
        priority: 0,
        tags: vec![],
    };

    let request = ProtocolRequest {
        protocol: Protocol::Http,
        pattern: MessagePattern::RequestResponse,
        operation: "GET".to_string(),
        path: "/api/users".to_string(),
        topic: None,
        routing_key: None,
        partition: None,
        qos: None,
        metadata: std::collections::HashMap::new(),
        body: None,
        client_ip: None,
    };

    c.bench_function("fixture_matching", |b| {
        b.iter(|| {
            let result = fixture.matches(black_box(&request));
            black_box(result);
        });
    });
}

criterion_group!(benches, bench_protocol_request_creation, bench_fixture_matching);
criterion_main!(benches);
