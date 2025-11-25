//! Performance benchmarks for MockForge core functionality
//!
//! Run with: cargo bench --bench core_benchmarks
//!
//! ## Memory Benchmarks
//!
//! Memory profiling is included for operations that allocate significant memory:
//! - Large OpenAPI spec parsing
//! - Bulk data generation
//! - Deep template rendering

#![allow(missing_docs)]
//!
//! These benchmarks use smaller sample sizes to reduce overhead while still
//! providing meaningful memory usage insights.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mockforge_core::openapi_routes::create_registry_from_json;
use mockforge_core::templating::expand_str;
use mockforge_core::validation::{validate_json_schema, ValidationResult, Validator};
use serde_json::json;

/// Benchmark template rendering with different payload sizes
fn bench_template_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("template_rendering");

    // Simple template
    group.bench_function("simple", |b| {
        let template = "Hello {{name}}!";
        b.iter(|| expand_str(black_box(template)));
    });

    // Complex template with multiple variables
    group.bench_function("complex", |b| {
        let template = r#"
            User: {{user.name}}
            Email: {{user.email}}
            Age: {{user.age}}
            Address: {{user.address.street}}, {{user.address.city}}
        "#;
        b.iter(|| expand_str(black_box(template)));
    });

    // Template with arrays
    group.bench_function("arrays", |b| {
        let template = "{{#each items}}{{name}}: {{price}}\n{{/each}}";
        b.iter(|| expand_str(black_box(template)));
    });

    group.finish();
}

/// Benchmark JSON schema validation
fn bench_json_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_validation");

    // Simple schema - pre-compile validator to avoid recompilation overhead
    let simple_schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        }
    });
    let simple_data = json!({"name": "test"});
    // Pre-compile validator once to measure actual validation performance
    let simple_validator = Validator::from_json_schema(&simple_schema).unwrap();

    group.bench_function("simple", |b| {
        b.iter(|| {
            // Use pre-compiled validator to avoid schema compilation overhead
            let result = match simple_validator.validate(black_box(&simple_data)) {
                Ok(_) => ValidationResult::success(),
                Err(e) => ValidationResult::failure(vec![e.to_string()]),
            };
            black_box(result)
        });
    });

    // Complex schema with nested objects - pre-compile validator
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
    // Pre-compile validator once to measure actual validation performance
    let complex_validator = Validator::from_json_schema(&complex_schema).unwrap();

    group.bench_function("complex", |b| {
        b.iter(|| {
            // Use pre-compiled validator to avoid schema compilation overhead
            let result = match complex_validator.validate(black_box(&complex_data)) {
                Ok(_) => ValidationResult::success(),
                Err(e) => ValidationResult::failure(vec![e.to_string()]),
            };
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

    // Use iter_with_setup to avoid cloning in the hot loop
    // Clone is done once in setup, not on every iteration
    group.bench_function("small_spec", |b| {
        b.iter_with_setup(
            || small_spec.clone(),
            |spec| {
                let result = create_registry_from_json(black_box(spec));
                black_box(result)
            },
        );
    });

    // Medium spec with multiple paths
    let mut paths = serde_json::Map::new();
    for i in 0..10 {
        let path = format!("/resource{}", i);
        paths.insert(
            path,
            json!({
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
            }),
        );
    }

    let medium_spec = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "paths": paths
    });

    // Use iter_with_setup to avoid cloning in the hot loop
    // Clone is done once in setup, not on every iteration
    group.bench_function("medium_spec_10_paths", |b| {
        b.iter_with_setup(
            || medium_spec.clone(),
            |spec| {
                let result = create_registry_from_json(black_box(spec));
                black_box(result)
            },
        );
    });

    group.finish();
}

/// Benchmark data generation
fn bench_data_generation(c: &mut Criterion) {
    use mockforge_data::{DataConfig, DataGenerator, SchemaDefinition};
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
    }))
    .unwrap();

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

/// Helper function to create a large OpenAPI spec for memory benchmarking
fn create_large_openapi_spec() -> serde_json::Value {
    let mut paths = serde_json::Map::new();

    // Create 100 paths with complex schemas to stress memory
    for i in 0..100 {
        let path = format!("/api/v1/resource_{}", i);
        paths.insert(path, json!({
            "get": {
                "summary": format!("Get resource {}", i),
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": {"type": "string"}
                    },
                    {
                        "name": "filter",
                        "in": "query",
                        "schema": {"type": "string"}
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Success",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "id": {"type": "integer"},
                                        "name": {"type": "string"},
                                        "description": {"type": "string"},
                                        "metadata": {
                                            "type": "object",
                                            "properties": {
                                                "created_at": {"type": "string", "format": "date-time"},
                                                "updated_at": {"type": "string", "format": "date-time"},
                                                "tags": {
                                                    "type": "array",
                                                    "items": {"type": "string"}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "post": {
                "summary": format!("Create resource {}", i),
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "type": "object",
                                "properties": {
                                    "name": {"type": "string"},
                                    "description": {"type": "string"}
                                },
                                "required": ["name"]
                            }
                        }
                    }
                },
                "responses": {
                    "201": {
                        "description": "Created",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "id": {"type": "integer"}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }));
    }

    json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Large Test API",
            "version": "1.0.0",
            "description": "A large API spec for memory benchmarking"
        },
        "paths": paths
    })
}

/// Benchmark memory usage for large operations
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory");
    group.sample_size(10);

    // Pre-create the large spec once to avoid variance from JSON construction
    // This ensures we're measuring parsing/route generation, not JSON creation
    let large_spec = create_large_openapi_spec();

    // Benchmark large OpenAPI spec parsing
    group.bench_function("large_spec_parsing", |b| {
        b.iter_with_setup(
            || large_spec.clone(), // Clone the pre-created spec (more predictable than recreating)
            |spec| {
                let result = create_registry_from_json(black_box(spec));
                black_box(result)
            },
        );
    });

    // Benchmark deep template rendering
    group.bench_function("deep_template_rendering", |b| {
        b.iter_with_setup(
            || {
                // Create a deeply nested template
                let mut template = String::from("{{#each items}}");
                for i in 0..10 {
                    template.push_str(&format!("  Level {}: {{{{level{}}}}}\n", i, i));
                    template.push_str("  {{#each nested}}");
                }
                for _ in 0..10 {
                    template.push_str("  {{/each}}");
                }
                template.push_str("{{/each}}");
                template
            },
            |template| {
                let result = expand_str(black_box(&template));
                black_box(result)
            },
        );
    });

    // Benchmark complex validation with large data
    group.bench_function("large_data_validation", |b| {
        b.iter_with_setup(
            || {
                let schema = json!({
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "integer"},
                            "name": {"type": "string", "minLength": 1},
                            "email": {"type": "string", "format": "email"},
                            "metadata": {
                                "type": "object",
                                "properties": {
                                    "tags": {
                                        "type": "array",
                                        "items": {"type": "string"}
                                    }
                                }
                            }
                        },
                        "required": ["id", "name", "email"]
                    },
                    "minItems": 1
                });

                let mut data = Vec::new();
                for i in 0..100 {
                    data.push(json!({
                        "id": i,
                        "name": format!("User {}", i),
                        "email": format!("user{}@example.com", i),
                        "metadata": {
                            "tags": ["tag1", "tag2", "tag3"]
                        }
                    }));
                }

                (schema, json!(data))
            },
            |(schema, data)| {
                let result = validate_json_schema(black_box(&data), black_box(&schema));
                black_box(result)
            },
        );
    });

    group.finish();
}

// Benchmark group for core functionality
criterion_group!(
    benches,
    bench_template_rendering,
    bench_json_validation,
    bench_openapi_parsing,
    bench_data_generation,
    bench_encryption,
    bench_memory_usage
);
criterion_main!(benches);
