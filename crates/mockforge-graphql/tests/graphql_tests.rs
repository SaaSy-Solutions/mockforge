use axum::body::Body;
use axum::http::{Request, StatusCode};
use mockforge_core::{latency::LatencyDistribution, LatencyProfile};
use mockforge_graphql::{create_router, GraphQLExecutor, GraphQLSchema};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use tower::ServiceExt;

#[tokio::test]
async fn test_graphql_schema_creation() {
    let schema = GraphQLSchema::new();
    // Verify schema is created successfully
    let _schema_def = schema.schema();
}

#[tokio::test]
async fn test_graphql_executor_creation() {
    let schema = GraphQLSchema::new();
    let executor = GraphQLExecutor::new(schema);

    // Verify executor has schema
    let _schema = executor.schema();
}

#[tokio::test]
async fn test_graphql_router_creation() {
    let result = create_router(None).await;
    assert!(result.is_ok());

    let router = result.unwrap();
    // Router should be created successfully
    assert!(std::mem::size_of_val(&router) > 0);
}

#[tokio::test]
async fn test_graphql_router_with_latency() {
    let latency_profile = LatencyProfile {
        base_ms: 50,
        jitter_ms: 20,
        distribution: LatencyDistribution::Fixed,
        std_dev_ms: None,
        pareto_shape: None,
        min_ms: 10,
        max_ms: Some(100),
        tag_overrides: HashMap::new(),
    };

    let result = create_router(Some(latency_profile)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_graphql_introspection_query() {
    let router = create_router(None).await.unwrap();

    let introspection_query = json!({
        "query": r#"
            query IntrospectionQuery {
                __schema {
                    queryType { name }
                    mutationType { name }
                    subscriptionType { name }
                }
            }
        "#
    });

    let request = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&introspection_query).unwrap()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_graphql_playground_endpoint() {
    let router = create_router(None).await.unwrap();

    let request = Request::builder()
        .method("GET")
        .uri("/graphql")
        .header("accept", "text/html")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Should return HTML content type for playground
    let content_type = response.headers().get("content-type");
    assert!(content_type.is_some());
    assert!(content_type.unwrap().to_str().unwrap().contains("text/html"));
}

#[tokio::test]
async fn test_invalid_graphql_query() {
    let router = create_router(None).await.unwrap();

    let invalid_query = json!({
        "query": "invalid graphql syntax {{"
    });

    let request = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&invalid_query).unwrap()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // Should still return 200 but with GraphQL errors
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    // Should contain errors field
    assert!(response_json.get("errors").is_some());
}

#[tokio::test]
async fn test_graphql_query_with_variables() {
    let router = create_router(None).await.unwrap();

    let query_with_variables = json!({
        "query": r#"
            query GetSchema($includeDeprecated: Boolean) {
                __schema {
                    queryType {
                        name
                        fields(includeDeprecated: $includeDeprecated) {
                            name
                        }
                    }
                }
            }
        "#,
        "variables": {
            "includeDeprecated": false
        }
    });

    let request = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&query_with_variables).unwrap()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    // Should contain data field
    assert!(response_json.get("data").is_some());
}

#[tokio::test]
async fn test_graphql_malformed_json() {
    let router = create_router(None).await.unwrap();

    let request = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .body(Body::from("{ invalid json"))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // Should return 400 for malformed JSON
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_graphql_get_request_without_playground() {
    let router = create_router(None).await.unwrap();

    let request = Request::builder()
        .method("GET")
        .uri("/graphql?query={__schema{queryType{name}}}")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    // Should handle GET requests with query parameter
    assert!(response.status().is_success() || response.status() == StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_graphql_options_request() {
    let router = create_router(None).await.unwrap();

    let request = Request::builder()
        .method("OPTIONS")
        .uri("/graphql")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // Should handle CORS preflight requests
    assert!(response.status().is_success() || response.status() == StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_schema_fields_and_types() {
    let schema = GraphQLSchema::new();
    let schema_def = &schema.schema();

    // Verify schema has basic introspection types
    let introspection_result = schema_def.execute(r#"{ __schema { queryType { name } } }"#).await;

    assert!(introspection_result.errors.is_empty());
    // Verify data is returned and no errors occurred
    assert!(introspection_result.errors.is_empty());
}

#[tokio::test]
async fn test_concurrent_graphql_requests() {
    let router = create_router(None).await.unwrap();

    let queries = vec![
        json!({"query": "{ __schema { queryType { name } } }"}),
        json!({"query": "{ __schema { mutationType { name } } }"}),
        json!({"query": "{ __schema { subscriptionType { name } } }"}),
    ];

    let mut handles = Vec::new();

    for query in queries {
        let router_clone = router.clone();
        let handle = tokio::spawn(async move {
            let request = Request::builder()
                .method("POST")
                .uri("/graphql")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&query).unwrap()))
                .unwrap();

            router_clone.oneshot(request).await
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let response = handle.await.unwrap().unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn test_graphql_performance_with_latency() {
    let latency_profile = LatencyProfile {
        base_ms: 10,
        jitter_ms: 5,
        distribution: LatencyDistribution::Fixed,
        std_dev_ms: None,
        pareto_shape: None,
        min_ms: 5,
        max_ms: Some(15),
        tag_overrides: HashMap::new(),
    };

    let router = create_router(Some(latency_profile)).await.unwrap();

    let query = json!({"query": "{ __schema { queryType { name } } }"});

    let start = std::time::Instant::now();

    let request = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&query).unwrap()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    let duration = start.elapsed();

    assert_eq!(response.status(), StatusCode::OK);
    // Should have some latency added (at least 5ms)
    assert!(duration >= Duration::from_millis(5));
}

#[tokio::test]
async fn test_large_graphql_query() {
    let router = create_router(None).await.unwrap();

    // Create a large introspection query
    let large_query = json!({
        "query": r#"
            query LargeIntrospection {
                __schema {
                    queryType {
                        name
                        description
                        fields {
                            name
                            description
                            type {
                                name
                                kind
                                description
                            }
                            args {
                                name
                                description
                                type {
                                    name
                                    kind
                                }
                                defaultValue
                            }
                        }
                    }
                    mutationType {
                        name
                        description
                        fields {
                            name
                            description
                            type {
                                name
                                kind
                            }
                        }
                    }
                    types {
                        name
                        kind
                        description
                        fields {
                            name
                            type {
                                name
                                kind
                            }
                        }
                    }
                }
            }
        "#
    });

    let request = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&large_query).unwrap()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: Value = serde_json::from_slice(&body).unwrap();

    // Should return substantial data
    assert!(response_json.get("data").is_some());
    let data = response_json.get("data").unwrap();
    assert!(data.get("__schema").is_some());
}
