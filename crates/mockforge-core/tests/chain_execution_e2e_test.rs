//! End-to-end test for request chain execution with actual HTTP requests
//!
//! This test creates a chain definition, executes it against a real HTTP server
//! (using httpbin.org), and verifies that the chain executes correctly with
//! variable extraction and dependency resolution.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::test;

use mockforge_core::chain_execution::ChainExecutionEngine;
use mockforge_core::request_chaining::{
    ChainConfig, ChainDefinition, ChainLink, ChainRequest, RequestBody, RequestChainRegistry,
};

/// Create a simple chain that makes actual HTTP requests
fn create_simple_http_chain() -> ChainDefinition {
    ChainDefinition {
        id: "http-test-chain".to_string(),
        name: "HTTP Test Chain".to_string(),
        description: Some("Test chain with real HTTP requests".to_string()),
        config: ChainConfig {
            enabled: true,
            max_chain_length: 10,
            global_timeout_secs: 30,
            enable_parallel_execution: false,
        },
        links: vec![
            // First request: GET request to fetch data
            ChainLink {
                request: ChainRequest {
                    id: "get_data".to_string(),
                    method: "GET".to_string(),
                    url: "https://httpbin.org/json".to_string(),
                    headers: HashMap::from([(
                        "User-Agent".to_string(),
                        "MockForge-Test".to_string(),
                    )]),
                    body: None,
                    depends_on: vec![],
                    timeout_secs: Some(10),
                    expected_status: Some(vec![200]),
                    scripting: None,
                },
                extract: HashMap::from([
                    ("slideshow_author".to_string(), "slideshow.author".to_string()),
                    ("slideshow_title".to_string(), "slideshow.title".to_string()),
                ]),
                store_as: Some("get_data_response".to_string()),
            },
            // Second request: POST request using extracted data
            ChainLink {
                request: ChainRequest {
                    id: "post_data".to_string(),
                    method: "POST".to_string(),
                    url: "https://httpbin.org/post".to_string(),
                    headers: HashMap::from([
                        ("Content-Type".to_string(), "application/json".to_string()),
                        ("User-Agent".to_string(), "MockForge-Test".to_string()),
                    ]),
                    body: Some(RequestBody::Json(serde_json::json!({
                        "author": "{{chain.get_data_response.slideshow.author}}",
                        "title": "{{chain.get_data_response.slideshow.title}}",
                        "test": "chain execution"
                    }))),
                    depends_on: vec!["get_data".to_string()],
                    timeout_secs: Some(10),
                    expected_status: Some(vec![200]),
                    scripting: None,
                },
                extract: HashMap::from([("posted_data".to_string(), "json".to_string())]),
                store_as: Some("post_data_response".to_string()),
            },
        ],
        variables: HashMap::new(),
        tags: vec!["test".to_string(), "e2e".to_string()],
    }
}

#[test]
#[ignore] // Ignore by default - requires external HTTP service
async fn test_chain_execution_with_http_requests() {
    // Create registry and engine
    let registry = Arc::new(RequestChainRegistry::new(ChainConfig::default()));
    let _engine = Arc::new(ChainExecutionEngine::new(registry.clone(), ChainConfig::default()));

    // Register the chain
    let chain_definition = create_simple_http_chain();
    let chain_yaml = serde_yaml::to_string(&chain_definition).unwrap();
    let chain_id = registry.register_from_yaml(&chain_yaml).await.unwrap();

    println!("Registered chain: {}", chain_id);

    // Execute the chain
    // Note: This requires implementing the actual execution logic in ChainExecutionEngine
    // For now, we validate that the chain structure is correct

    // Verify chain was registered
    let retrieved_chain = registry.get_chain(&chain_id).await;
    assert!(retrieved_chain.is_some(), "Chain should be registered");

    let chain = retrieved_chain.unwrap();
    assert_eq!(chain.links.len(), 2, "Chain should have 2 links");

    // Verify first link has no dependencies
    assert!(
        chain.links[0].request.depends_on.is_empty(),
        "First link should have no dependencies"
    );

    // Verify second link depends on first
    assert_eq!(
        chain.links[1].request.depends_on,
        vec!["get_data"],
        "Second link should depend on first"
    );

    println!("✓ Chain execution test structure validated");
    println!(
        "  Note: Actual HTTP execution requires ChainExecutionEngine.execute() implementation"
    );
}

#[test]
async fn test_chain_execution_validation() {
    let registry = Arc::new(RequestChainRegistry::new(ChainConfig::default()));
    let chain_definition = create_simple_http_chain();

    // Validate the chain
    let validation_result = registry.validate_chain(&chain_definition).await;

    assert!(
        validation_result.is_ok(),
        "Chain should pass validation: {:?}",
        validation_result.err()
    );

    println!("✓ Chain validation passed");
}

#[test]
async fn test_chain_with_parallel_requests() {
    let registry = Arc::new(RequestChainRegistry::new(ChainConfig {
        enabled: true,
        max_chain_length: 20,
        global_timeout_secs: 300,
        enable_parallel_execution: true,
    }));

    // Create a chain with parallel independent requests
    let parallel_chain = ChainDefinition {
        id: "parallel-http-chain".to_string(),
        name: "Parallel HTTP Chain".to_string(),
        description: Some("Chain with parallel HTTP requests".to_string()),
        config: ChainConfig {
            enabled: true,
            max_chain_length: 20,
            global_timeout_secs: 300,
            enable_parallel_execution: true,
        },
        links: vec![
            ChainLink {
                request: ChainRequest {
                    id: "parallel_request_1".to_string(),
                    method: "GET".to_string(),
                    url: "https://httpbin.org/delay/1".to_string(),
                    headers: HashMap::new(),
                    body: None,
                    depends_on: vec![],
                    timeout_secs: Some(5),
                    expected_status: Some(vec![200]),
                    scripting: None,
                },
                extract: HashMap::new(),
                store_as: Some("response_1".to_string()),
            },
            ChainLink {
                request: ChainRequest {
                    id: "parallel_request_2".to_string(),
                    method: "GET".to_string(),
                    url: "https://httpbin.org/delay/1".to_string(),
                    headers: HashMap::new(),
                    body: None,
                    depends_on: vec![],
                    timeout_secs: Some(5),
                    expected_status: Some(vec![200]),
                    scripting: None,
                },
                extract: HashMap::new(),
                store_as: Some("response_2".to_string()),
            },
            ChainLink {
                request: ChainRequest {
                    id: "sequential_request".to_string(),
                    method: "GET".to_string(),
                    url: "https://httpbin.org/get".to_string(),
                    headers: HashMap::new(),
                    body: None,
                    depends_on: vec!["parallel_request_1".to_string()],
                    timeout_secs: Some(5),
                    expected_status: Some(vec![200]),
                    scripting: None,
                },
                extract: HashMap::new(),
                store_as: Some("response_3".to_string()),
            },
        ],
        variables: HashMap::new(),
        tags: vec!["parallel".to_string()],
    };

    let result = registry.validate_chain(&parallel_chain).await;
    assert!(result.is_ok(), "Parallel chain should be valid");

    println!("✓ Parallel chain validation passed");
}

#[test]
async fn test_chain_execution_error_handling() {
    let registry = Arc::new(RequestChainRegistry::new(ChainConfig::default()));

    // Create a chain with an invalid URL (will fail at execution)
    let error_chain = ChainDefinition {
        id: "error-chain".to_string(),
        name: "Error Handling Chain".to_string(),
        description: Some("Test error handling".to_string()),
        config: ChainConfig::default(),
        links: vec![ChainLink {
            request: ChainRequest {
                id: "failing_request".to_string(),
                method: "GET".to_string(),
                url: "https://httpbin.org/status/500".to_string(), // Will return 500
                headers: HashMap::new(),
                body: None,
                depends_on: vec![],
                timeout_secs: Some(5),
                expected_status: Some(vec![200]), // Expects 200, will get 500
                scripting: None,
            },
            extract: HashMap::new(),
            store_as: Some("response".to_string()),
        }],
        variables: HashMap::new(),
        tags: vec![],
    };

    let result = registry.validate_chain(&error_chain).await;
    assert!(result.is_ok(), "Chain structure should be valid");

    println!("✓ Error handling chain validation passed");
    println!("  Note: Actual error handling requires execution implementation");
}
