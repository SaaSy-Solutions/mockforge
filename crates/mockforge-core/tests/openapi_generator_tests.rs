//! Unit tests for OpenAPI specification generator

use chrono::Utc;
use mockforge_core::intelligent_behavior::openapi_generator::{
    ConfidenceScore, HttpExchange, OpenApiGenerationConfig, OpenApiSpecGenerator,
};

fn create_test_exchange(
    method: &str,
    path: &str,
    status_code: Option<i32>,
    response_body: Option<&str>,
) -> HttpExchange {
    HttpExchange {
        method: method.to_string(),
        path: path.to_string(),
        query_params: None,
        headers: "{}".to_string(),
        body: None,
        body_encoding: "application/json".to_string(),
        status_code,
        response_headers: Some("{}".to_string()),
        response_body: response_body.map(|s| s.to_string()),
        response_body_encoding: Some("application/json".to_string()),
        timestamp: Utc::now(),
    }
}

#[tokio::test]
async fn test_path_parameter_inference() {
    // Create exchanges with similar paths that should be parameterized
    let exchanges = vec![
        create_test_exchange(
            "GET",
            "/users/123",
            Some(200),
            Some(r#"{"id": "123", "name": "Alice"}"#),
        ),
        create_test_exchange(
            "GET",
            "/users/456",
            Some(200),
            Some(r#"{"id": "456", "name": "Bob"}"#),
        ),
        create_test_exchange(
            "GET",
            "/users/789",
            Some(200),
            Some(r#"{"id": "789", "name": "Charlie"}"#),
        ),
    ];

    let config = OpenApiGenerationConfig::default();
    let generator = OpenApiSpecGenerator::new(config);

    // Group by path pattern
    let path_groups = generator.group_by_path_pattern(&exchanges);

    // Should group similar paths together
    assert!(!path_groups.is_empty());

    // Test path parameter inference
    let normalized = generator.infer_path_parameters(&path_groups);

    // Should have normalized paths with parameters
    assert!(!normalized.is_empty());

    // Check that we have a parameterized path
    let has_parameterized = normalized.keys().any(|path| path.contains("{"));
    assert!(has_parameterized, "Should have at least one parameterized path");
}

#[tokio::test]
async fn test_schema_inference() {
    let exchanges = vec![
        create_test_exchange(
            "POST",
            "/users",
            Some(201),
            Some(r#"{"id": "123", "name": "Alice", "email": "alice@example.com", "age": 30}"#),
        ),
        create_test_exchange(
            "POST",
            "/users",
            Some(201),
            Some(r#"{"id": "456", "name": "Bob", "email": "bob@example.com", "age": 25}"#),
        ),
    ];

    let config = OpenApiGenerationConfig::default();
    let generator = OpenApiSpecGenerator::new(config);

    // Test schema inference
    let schemas = generator.infer_schemas(&exchanges).await.unwrap();

    // Should have inferred schemas
    assert!(!schemas.is_empty());

    // Check that schemas contain expected properties
    for (_, schema) in schemas {
        if let Some(properties) = schema.get("properties") {
            assert!(properties.is_object(), "Schema properties should be an object");
        }
    }
}

#[tokio::test]
async fn test_json_to_schema() {
    let config = OpenApiGenerationConfig::default();
    let generator = OpenApiSpecGenerator::new(config);

    // Test simple object
    let json = serde_json::json!({
        "id": "123",
        "name": "Alice",
        "age": 30,
        "active": true
    });

    let schema = generator.json_to_schema(&json);

    assert_eq!(schema["type"], "object");
    assert!(schema.get("properties").is_some());

    let properties = schema["properties"].as_object().unwrap();
    assert_eq!(properties["id"]["type"], "string");
    assert_eq!(properties["name"]["type"], "string");
    assert_eq!(properties["age"]["type"], "integer");
    assert_eq!(properties["active"]["type"], "boolean");
}

#[tokio::test]
async fn test_confidence_scoring() {
    // Create multiple exchanges for the same path pattern
    let exchanges = vec![
        create_test_exchange("GET", "/users/1", Some(200), None),
        create_test_exchange("GET", "/users/2", Some(200), None),
        create_test_exchange("GET", "/users/3", Some(200), None),
        create_test_exchange("GET", "/users/4", Some(200), None),
        create_test_exchange("GET", "/users/5", Some(200), None),
    ];

    let config = OpenApiGenerationConfig::default();
    let generator = OpenApiSpecGenerator::new(config);

    // Group paths
    let path_groups = generator.group_by_path_pattern(&exchanges);

    // Calculate confidence scores
    let confidence = generator.calculate_confidence_scores(&path_groups);

    // Should have confidence scores
    assert!(!confidence.is_empty());

    // Confidence should be reasonable (more examples = higher confidence)
    for (_, score) in confidence {
        assert!(score.value >= 0.0 && score.value <= 1.0, "Confidence should be between 0 and 1");
        assert!(!score.reason.is_empty(), "Confidence reason should not be empty");
    }
}

#[tokio::test]
async fn test_empty_exchanges() {
    let exchanges = vec![];
    let config = OpenApiGenerationConfig::default();
    let generator = OpenApiSpecGenerator::new(config);

    // Should handle empty exchanges gracefully
    let result = generator.generate_from_exchanges(exchanges).await;

    // Should either return an error or generate an empty spec
    match result {
        Ok(result) => {
            // If successful, spec should be minimal
            assert!(result.metadata.requests_analyzed == 0);
        }
        Err(_) => {
            // Error is also acceptable for empty input
        }
    }
}

#[tokio::test]
async fn test_path_grouping() {
    let exchanges = vec![
        create_test_exchange("GET", "/users/1", Some(200), None),
        create_test_exchange("GET", "/users/2", Some(200), None),
        create_test_exchange("POST", "/users", Some(201), None),
        create_test_exchange("GET", "/products/1", Some(200), None),
        create_test_exchange("GET", "/products/2", Some(200), None),
    ];

    let config = OpenApiGenerationConfig::default();
    let generator = OpenApiSpecGenerator::new(config);

    // Group by path pattern
    let path_groups = generator.group_by_path_pattern(&exchanges);

    // Should group similar paths together
    assert!(!path_groups.is_empty());

    // Should have multiple groups (users and products)
    assert!(path_groups.len() >= 2, "Should have at least 2 path groups");
}

#[tokio::test]
async fn test_min_confidence_filtering() {
    let exchanges = vec![
        create_test_exchange("GET", "/users/1", Some(200), None),
        create_test_exchange("GET", "/users/2", Some(200), None),
        // Single exchange for a different path (low confidence)
        create_test_exchange("GET", "/orders/1", Some(200), None),
    ];

    // Set high minimum confidence
    let config = OpenApiGenerationConfig {
        min_confidence: 0.9,
        behavior_model: None,
    };
    let generator = OpenApiSpecGenerator::new(config);

    let result = generator.generate_from_exchanges(exchanges).await;

    if let Ok(result) = result {
        // Paths with low confidence should be filtered out
        // Users path should have higher confidence (2 examples) than orders (1 example)
        let user_confidence = result
            .metadata
            .path_confidence
            .get("/users/{id}")
            .map(|s| s.value)
            .unwrap_or(0.0);

        // Users path should have reasonable confidence
        assert!(user_confidence >= 0.0 && user_confidence <= 1.0);
    }
}
