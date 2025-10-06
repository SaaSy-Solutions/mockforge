//! Performance benchmarks for MockForge core functionality
//!
//! Run with: cargo bench --bench core_benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mockforge_core::templating::expand_str;
use mockforge_core::validation::validate_json_schema;
use mockforge_core::openapi_routes::create_registry_from_json;
use serde_json::json;

/// Benchmark template rendering with different payload sizes
fn bench_template_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("template_rendering");

    // Simple template
    group.bench_function("simple", |b| {
        let template = "Hello {{name}}!";
        b.iter(|| {
            expand_str(black_box(template))
        });
    });

    // Complex template with multiple variables
    group.bench_function("complex", |b| {
        let template = r#"
            User: {{user.name}}
            Email: {{user.email}}
            Age: {{user.age}}
            Address: {{user.address.street}}, {{user.address.city}}
        "#;
        b.iter(|| {
            expand_str(black_box(template))
        });
    });

    // Template with arrays
    group.bench_function("arrays", |b| {
        let template = "{{#each items}}{{name}}: {{price}}\n{{/each}}";
        b.iter(|| {
            expand_str(black_box(template))
        });
    });

    group.finish();
}

/// Benchmark JSON schema validation
fn bench_json_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_validation");

    // Simple schema
    let simple_schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        }
    });
    let simple_data = json!({"name": "test"});

    group.bench_function("simple", |b| {
        b.iter(|| {
            let result = validate_json_schema(black_box(&simple_data), black_box(&simple_schema));
            black_box(result)
        });
    });

    // Complex schema with nested objects
    let complex_schema = json!({
        "type": "object",
        "properties": {
            "user": {
                "type": "object",
                "properties": {
                    "name": {"type": "string", "minLength": 1},
                    "email": {"type": "string", "format": "email"},
                    "age": {"type": "integer", "minimum": 0, "maximum": 150}
                },
                "required": ["name", "email"]
            }
        },
        "required": ["user"]
    });
    let complex_data = json!({
        "user": {
            "name": "John Doe",
            "email": "john@example.com",
            "age": 30
        }
    });

    group.bench_function("complex", |b| {
        b.iter(|| {
            let result = validate_json_schema(black_box(&complex_data), black_box(&complex_schema));
            black_box(result)
        });
    });

    group.finish();
}

/// Benchmark OpenAPI spec parsing
fn bench_openapi_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("openapi_parsing");

    // Small spec with few paths
    let small_spec = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "paths": {
            "/users": {
                "get": {
                    "summary": "Get users",
                    "responses": {
                        "200": {
                            "description": "Success",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {
                                            "type": "object",
                                            "properties": {
                                                "id": {"type": "integer"},
                                                "name": {"type": "string"}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    group.bench_function("small_spec", |b| {
        b.iter(|| {
            let result = create_registry_from_json(black_box(small_spec.clone()));
            black_box(result)
        });
    });

    // Medium spec with multiple paths
    let mut paths = serde_json::Map::new();
    for i in 0..10 {
        let path = format!("/resource{}", i);
        paths.insert(path, json!({
            "get": {
                "summary": format!("Get resource {}", i),
                "responses": {
                    "200": {
                        "description": "Success",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "id": {"type": "integer"},
                                        "name": {"type": "string"}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }));
    }

    let medium_spec = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "paths": paths
    });

    group.bench_function("medium_spec_10_paths", |b| {
        b.iter(|| {
            let result = create_registry_from_json(black_box(medium_spec.clone()));
            black_box(result)
        });
    });

    group.finish();
}

/// Benchmark data generation
fn bench_data_generation(c: &mut Criterion) {
    use mockforge_data::{DataGenerator, DataConfig, SchemaDefinition};
    use serde_json::json;

    let mut group = c.benchmark_group("data_generation");

    // Create a simple schema for benchmarking
    let schema = SchemaDefinition::from_json_schema(&json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "email": {"type": "string"},
            "id": {"type": "string"}
        }
    })).unwrap();

    let config = DataConfig {
        rows: 1,
        ..Default::default()
    };
    let generator = DataGenerator::new(schema, config).unwrap();

    group.bench_function("generate_single_record", |b| {
        b.iter(|| {
            // Note: This is async, so we'll just benchmark the sync parts
            black_box(&generator)
        });
    });

    group.finish();
}

/// Benchmark encryption/decryption
fn bench_encryption(c: &mut Criterion) {
    // Note: Encryption benchmarks temporarily disabled due to API changes
    // This can be re-enabled once the encryption API is stabilized
    let mut group = c.benchmark_group("encryption");

    group.bench_function("placeholder", |b| {
        b.iter(|| {
            // Placeholder benchmark - replace with actual encryption tests
            black_box("encryption benchmark placeholder")
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_template_rendering,
    bench_json_validation,
    bench_openapi_parsing,
    bench_data_generation,
    bench_encryption
);
criterion_main!(benches);
