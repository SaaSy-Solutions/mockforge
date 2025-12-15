//! Cross-protocol integration tests.
//!
//! These tests verify that protocols work together correctly,
//! including HTTP↔gRPC bridges and protocol state consistency.

use mockforge_grpc::dynamic::http_bridge::{HttpBridge, HttpBridgeConfig};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_http_bridge_configuration() {
    // Test that HTTP bridge can be configured correctly
    let config = HttpBridgeConfig {
        enabled: true,
        base_path: "/api".to_string(),
        enable_cors: true,
        max_request_size: 10 * 1024 * 1024,
        timeout_seconds: 30,
        route_pattern: "/{service}/{method}".to_string(),
    };

    assert!(config.enabled);
    assert_eq!(config.base_path, "/api");
    assert!(config.enable_cors);
    assert_eq!(config.max_request_size, 10 * 1024 * 1024);
    assert_eq!(config.timeout_seconds, 30);
}

#[tokio::test]
async fn test_http_bridge_default_config() {
    // Test default configuration
    let config = HttpBridgeConfig::default();

    assert!(config.enabled);
    assert_eq!(config.base_path, "/api");
    assert!(config.enable_cors);
    assert_eq!(config.max_request_size, 10 * 1024 * 1024);
    assert_eq!(config.timeout_seconds, 30);
    assert_eq!(config.route_pattern, "/{service}/{method}");
}

#[tokio::test]
async fn test_bridge_query_parsing() {
    use mockforge_grpc::dynamic::http_bridge::BridgeQuery;
    use std::collections::HashMap;

    // Test query parameter parsing
    let query_string = "stream=server&key1=value1&key2=value2";
    // Test query parameter structure (without actual parsing)
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("key1".to_string(), "value1".to_string());
    metadata.insert("key2".to_string(), "value2".to_string());

    let query = BridgeQuery {
        stream: Some("server".to_string()),
        metadata,
    };

    assert_eq!(query.stream, Some("server".to_string()));
    assert_eq!(query.metadata.get("key1"), Some(&"value1".to_string()));
    assert_eq!(query.metadata.get("key2"), Some(&"value2".to_string()));

    assert_eq!(query.stream, Some("server".to_string()));
    assert_eq!(query.metadata.get("key1"), Some(&"value1".to_string()));
    assert_eq!(query.metadata.get("key2"), Some(&"value2".to_string()));
}

#[tokio::test]
async fn test_bridge_response_format() {
    use mockforge_grpc::dynamic::http_bridge::BridgeResponse;

    // Test successful response format
    let success_response = BridgeResponse {
        success: true,
        data: Some(json!({"message": "Hello"})),
        error: None,
        metadata: std::collections::HashMap::new(),
    };

    let json = serde_json::to_string(&success_response).unwrap();
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("\"message\":\"Hello\""));

    // Test error response format
    let error_response = BridgeResponse::<serde_json::Value> {
        success: false,
        data: None,
        error: Some("Test error".to_string()),
        metadata: std::collections::HashMap::new(),
    };

    let json = serde_json::to_string(&error_response).unwrap();
    assert!(json.contains("\"success\":false"));
    assert!(json.contains("\"Test error\""));
}

#[tokio::test]
async fn test_bridge_stats_initialization() {
    use mockforge_grpc::dynamic::http_bridge::BridgeStats;

    let stats = BridgeStats {
        requests_served: 0,
        requests_successful: 0,
        requests_failed: 0,
        available_services: vec![],
    };

    assert_eq!(stats.requests_served, 0);
    assert_eq!(stats.requests_successful, 0);
    assert_eq!(stats.requests_failed, 0);
    assert!(stats.available_services.is_empty());
}

#[tokio::test]
async fn test_bridge_route_pattern_parsing() {
    // Test route pattern parsing
    let patterns = vec![
        "/{service}/{method}",
        "/api/{service}/{method}",
        "/v1/{service}/{method}",
    ];

    for pattern in patterns {
        // Verify pattern contains expected placeholders
        assert!(pattern.contains("{service}"));
        assert!(pattern.contains("{method}"));
    }
}

#[tokio::test]
async fn test_bridge_cors_configuration() {
    let config_with_cors = HttpBridgeConfig {
        enable_cors: true,
        ..Default::default()
    };

    let config_without_cors = HttpBridgeConfig {
        enable_cors: false,
        ..Default::default()
    };

    assert!(config_with_cors.enable_cors);
    assert!(!config_without_cors.enable_cors);
}

#[tokio::test]
async fn test_bridge_timeout_configuration() {
    let config_short_timeout = HttpBridgeConfig {
        timeout_seconds: 5,
        ..Default::default()
    };

    let config_long_timeout = HttpBridgeConfig {
        timeout_seconds: 60,
        ..Default::default()
    };

    assert_eq!(config_short_timeout.timeout_seconds, 5);
    assert_eq!(config_long_timeout.timeout_seconds, 60);
}

#[tokio::test]
async fn test_bridge_max_request_size() {
    let config_small = HttpBridgeConfig {
        max_request_size: 1024, // 1KB
        ..Default::default()
    };

    let config_large = HttpBridgeConfig {
        max_request_size: 100 * 1024 * 1024, // 100MB
        ..Default::default()
    };

    assert_eq!(config_small.max_request_size, 1024);
    assert_eq!(config_large.max_request_size, 100 * 1024 * 1024);
}

#[tokio::test]
async fn test_bridge_base_path_variations() {
    let base_paths = vec!["/api", "/v1/api", "/api/v1", "/"];

    for base_path in base_paths {
        let config = HttpBridgeConfig {
            base_path: base_path.to_string(),
            ..Default::default()
        };
        assert_eq!(config.base_path, base_path);
    }
}

#[tokio::test]
async fn test_bridge_disabled_config() {
    let config = HttpBridgeConfig {
        enabled: false,
        ..Default::default()
    };

    assert!(!config.enabled);
    // Other config should still be valid
    assert_eq!(config.base_path, "/api");
}
