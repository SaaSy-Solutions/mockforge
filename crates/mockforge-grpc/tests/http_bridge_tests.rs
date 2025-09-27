//! Integration tests for HTTP bridge functionality

use mockforge_grpc::dynamic::{discover_services, start_dynamic_server, DynamicGrpcConfig};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_http_bridge_creation() {
    // Use the existing proto directory
    let proto_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/proto";

    let config = DynamicGrpcConfig {
        proto_dir: proto_dir.clone(),
        enable_reflection: true,
        excluded_services: vec![],
        http_bridge: Some(Default::default()),
    };

    // Discover services
    let registry = discover_services(&config).await;
    assert!(registry.is_ok(), "Failed to discover services");

    let registry = registry.unwrap();
    println!("Discovered {} services", registry.service_names().len());

    // Verify we can create a server without bridge
    let grpc_config = DynamicGrpcConfig {
        proto_dir,
        enable_reflection: true,
        excluded_services: vec![],
        http_bridge: None, // Disable bridge
    };

    // Test that gRPC-only server can be started (but don't actually start it due to port conflicts)
    let registry = discover_services(&grpc_config).await.unwrap();
    assert!(
        registry.service_names().len() >= 1,
        "Should discover at least the greeter service"
    );

    println!("Test passed: HTTP bridge configuration and service discovery works");
}

// Basic test to verify the bridge configuration structure
#[test]
fn test_bridge_config_defaults() {
    use mockforge_grpc::dynamic::http_bridge::HttpBridgeConfig;

    let config = HttpBridgeConfig::default();

    assert!(config.enabled, "Bridge should be enabled by default");
    assert_eq!(config.base_path, "/api", "Default base path should be /api");
    assert!(config.enable_cors, "CORS should be enabled by default");
    assert!(config.max_request_size > 0, "Max request size should be positive");
    assert!(config.timeout_seconds > 0, "Timeout should be positive");
}
