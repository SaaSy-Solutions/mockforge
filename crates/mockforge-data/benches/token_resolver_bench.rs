//! Benchmarks for token resolver and domain generators
//!
//! This benchmark ensures that token resolution and data generation
//! meet the <200ms response time requirement.

#![allow(missing_docs)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use mockforge_data::{resolve_tokens, Domain, DomainGenerator, TokenResolver};
use serde_json::json;
use tokio::runtime::Runtime;

fn bench_token_resolution(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("resolve_simple_tokens", |b| {
        let value = json!({
            "id": "$random.uuid",
            "name": "$faker.name",
            "email": "$faker.email"
        });

        b.iter(|| rt.block_on(async { black_box(resolve_tokens(&value).await.unwrap()) }));
    });

    c.bench_function("resolve_nested_tokens", |b| {
        let value = json!({
            "user": {
                "id": "$random.uuid",
                "profile": {
                    "name": "$faker.name",
                    "contact": {
                        "email": "$faker.email",
                        "phone": "$faker.phone"
                    }
                }
            }
        });

        b.iter(|| rt.block_on(async { black_box(resolve_tokens(&value).await.unwrap()) }));
    });

    c.bench_function("resolve_array_tokens", |b| {
        let value = json!({
            "users": [
                {"id": "$random.uuid", "name": "$faker.name"},
                {"id": "$random.uuid", "name": "$faker.name"},
                {"id": "$random.uuid", "name": "$faker.name"},
                {"id": "$random.uuid", "name": "$faker.name"},
                {"id": "$random.uuid", "name": "$faker.name"}
            ]
        });

        b.iter(|| rt.block_on(async { black_box(resolve_tokens(&value).await.unwrap()) }));
    });

    c.bench_function("resolve_large_object", |b| {
        let value = json!({
            "id": "$random.uuid",
            "name": "$faker.name",
            "email": "$faker.email",
            "phone": "$faker.phone",
            "address": "$faker.address",
            "company": "$faker.company",
            "website": "$faker.url",
            "created_at": "$faker.datetime",
            "updated_at": "$faker.datetime",
            "status": "$random.choice"
        });

        b.iter(|| rt.block_on(async { black_box(resolve_tokens(&value).await.unwrap()) }));
    });
}

fn bench_domain_generators(c: &mut Criterion) {
    let mut group = c.benchmark_group("domain_generators");

    // Finance domain
    let finance_gen = DomainGenerator::new(Domain::Finance);
    group.bench_function("finance_account_number", |b| {
        b.iter(|| black_box(finance_gen.generate("account_number").unwrap()));
    });
    group.bench_function("finance_transaction", |b| {
        b.iter(|| black_box(finance_gen.generate("transaction_id").unwrap()));
    });

    // IoT domain
    let iot_gen = DomainGenerator::new(Domain::Iot);
    group.bench_function("iot_device_id", |b| {
        b.iter(|| black_box(iot_gen.generate("device_id").unwrap()));
    });
    group.bench_function("iot_temperature", |b| {
        b.iter(|| black_box(iot_gen.generate("temperature").unwrap()));
    });

    // Healthcare domain
    let healthcare_gen = DomainGenerator::new(Domain::Healthcare);
    group.bench_function("healthcare_patient_id", |b| {
        b.iter(|| black_box(healthcare_gen.generate("patient_id").unwrap()));
    });
    group.bench_function("healthcare_blood_pressure", |b| {
        b.iter(|| black_box(healthcare_gen.generate("blood_pressure").unwrap()));
    });

    group.finish();
}

fn bench_mixed_domain_response(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("mixed_finance_response", |b| {
        let value = json!({
            "account": {
                "id": "$random.uuid",
                "number": "$faker.name",  // Would be replaced with domain generator
                "balance": "$random.float",
                "currency": "$random.choice",
                "created_at": "$faker.datetime"
            },
            "transactions": [
                {
                    "id": "$random.uuid",
                    "amount": "$random.float",
                    "timestamp": "$faker.datetime"
                },
                {
                    "id": "$random.uuid",
                    "amount": "$random.float",
                    "timestamp": "$faker.datetime"
                },
                {
                    "id": "$random.uuid",
                    "amount": "$random.float",
                    "timestamp": "$faker.datetime"
                }
            ]
        });

        b.iter(|| rt.block_on(async { black_box(resolve_tokens(&value).await.unwrap()) }));
    });
}

fn bench_real_world_scenario(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("real_world_scenario");

    // E-commerce order response
    group.bench_function("ecommerce_order", |b| {
        let value = json!({
            "order_id": "$random.uuid",
            "customer": {
                "id": "$random.uuid",
                "name": "$faker.name",
                "email": "$faker.email",
                "phone": "$faker.phone"
            },
            "items": [
                {
                    "id": "$random.uuid",
                    "name": "$faker.word",
                    "price": "$random.float",
                    "quantity": "$random.int.small"
                },
                {
                    "id": "$random.uuid",
                    "name": "$faker.word",
                    "price": "$random.float",
                    "quantity": "$random.int.small"
                }
            ],
            "total": "$random.float",
            "status": "$random.choice",
            "created_at": "$faker.datetime",
            "updated_at": "$faker.datetime"
        });

        b.iter(|| rt.block_on(async { black_box(resolve_tokens(&value).await.unwrap()) }));
    });

    // IoT sensor data
    group.bench_function("iot_sensor_reading", |b| {
        let value = json!({
            "device_id": "$random.uuid",
            "sensor_id": "$random.uuid",
            "readings": [
                {
                    "temperature": "$random.float",
                    "humidity": "$random.float",
                    "pressure": "$random.float",
                    "timestamp": "$faker.datetime"
                },
                {
                    "temperature": "$random.float",
                    "humidity": "$random.float",
                    "pressure": "$random.float",
                    "timestamp": "$faker.datetime"
                },
                {
                    "temperature": "$random.float",
                    "humidity": "$random.float",
                    "pressure": "$random.float",
                    "timestamp": "$faker.datetime"
                }
            ],
            "location": {
                "latitude": "$random.float",
                "longitude": "$random.float"
            },
            "status": "$random.choice"
        });

        b.iter(|| rt.block_on(async { black_box(resolve_tokens(&value).await.unwrap()) }));
    });

    group.finish();
}

// Benchmark group for token resolver functionality
criterion_group!(
    benches,
    bench_token_resolution,
    bench_domain_generators,
    bench_mixed_domain_response,
    bench_real_world_scenario
);
criterion_main!(benches);
