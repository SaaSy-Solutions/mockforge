use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_amqp_routing(c: &mut Criterion) {
    // TODO: Implement AMQP routing benchmarks
    c.bench_function("amqp_routing", |b| {
        b.iter(|| {
            black_box(());
        });
    });
}

criterion_group!(benches, bench_amqp_routing);
criterion_main!(benches);