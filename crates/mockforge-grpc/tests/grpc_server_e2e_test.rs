//! End-to-end tests for gRPC service discovery and mock response generation
//!
//! These tests verify that the gRPC dynamic service discovery correctly parses
//! proto files and that the registry can generate mock responses.

use mockforge_core::protocol_abstraction::{
    MessagePattern, Protocol, ProtocolRequest, SpecRegistry,
};
use mockforge_grpc::dynamic::{discover_services, DynamicGrpcConfig};
use mockforge_grpc::registry::GrpcProtoRegistry;
use std::collections::HashMap;

/// Helper to get the proto directory path for tests
fn proto_dir() -> String {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set in tests");
    format!("{}/proto", manifest_dir)
}

#[tokio::test]
async fn test_grpc_service_discovery_from_proto_files() {
    let dir = proto_dir();
    assert!(std::path::Path::new(&dir).exists(), "Proto directory not found at {}", dir);

    let config = DynamicGrpcConfig {
        proto_dir: dir,
        enable_reflection: true,
        excluded_services: vec![],
        http_bridge: None,
        tls: None,
    };

    let registry = discover_services(&config).await.expect("Service discovery should succeed");

    let services = registry.service_names();
    assert!(!services.is_empty(), "Should discover at least one service from proto files");

    // Verify the Greeter service from gretter.proto is discovered
    let has_greeter = services.iter().any(|s| s.contains("Greeter"));
    assert!(has_greeter, "Should discover the Greeter service, found: {:?}", services);
}

#[tokio::test]
async fn test_grpc_service_discovery_with_exclusion() {
    let dir = proto_dir();
    assert!(std::path::Path::new(&dir).exists(), "Proto directory not found at {}", dir);

    let config = DynamicGrpcConfig {
        proto_dir: dir,
        enable_reflection: false,
        excluded_services: vec!["mockforge.greeter.Greeter".to_string()],
        http_bridge: None,
        tls: None,
    };

    let registry = discover_services(&config).await.expect("Service discovery should succeed");

    let services = registry.service_names();

    // The Greeter service should be excluded
    let has_greeter = services.iter().any(|s| s.contains("Greeter"));
    assert!(!has_greeter, "Greeter service should be excluded, found: {:?}", services);
}

#[tokio::test]
async fn test_grpc_registry_operations_from_proto() {
    let dir = proto_dir();
    assert!(std::path::Path::new(&dir).exists(), "Proto directory not found at {}", dir);

    let registry = GrpcProtoRegistry::from_directory(&dir)
        .await
        .expect("Should create registry from proto directory");

    let operations = registry.operations();
    assert!(!operations.is_empty(), "Should have operations from the Greeter service");

    // Verify SayHello unary RPC is discovered
    let say_hello = operations.iter().find(|op| op.name == "SayHello");
    assert!(say_hello.is_some(), "Should find SayHello operation");
    let say_hello = say_hello.unwrap();
    assert_eq!(say_hello.operation_type, "Unary");
    assert!(say_hello.input_schema.is_some());
    assert!(say_hello.output_schema.is_some());

    // Verify streaming RPCs are discovered with correct types
    let stream_op = operations.iter().find(|op| op.name == "SayHelloStream");
    assert!(stream_op.is_some(), "Should find SayHelloStream operation");
    assert_eq!(stream_op.unwrap().operation_type, "ServerStreaming");

    let client_stream_op = operations.iter().find(|op| op.name == "SayHelloClientStream");
    assert!(client_stream_op.is_some(), "Should find SayHelloClientStream operation");
    assert_eq!(client_stream_op.unwrap().operation_type, "ClientStreaming");

    let chat_op = operations.iter().find(|op| op.name == "Chat");
    assert!(chat_op.is_some(), "Should find Chat operation");
    assert_eq!(chat_op.unwrap().operation_type, "BidirectionalStreaming");
}

#[tokio::test]
async fn test_grpc_mock_response_generation() {
    let dir = proto_dir();
    assert!(std::path::Path::new(&dir).exists(), "Proto directory not found at {}", dir);

    let registry = GrpcProtoRegistry::from_directory(&dir)
        .await
        .expect("Should create registry from proto directory");

    let operations = registry.operations();
    assert!(!operations.is_empty(), "Should have operations");

    // Find the SayHello operation to test mock generation
    let say_hello = operations
        .iter()
        .find(|op| op.name == "SayHello")
        .expect("Should find SayHello operation");

    // Generate a mock response
    let request = ProtocolRequest {
        protocol: Protocol::Grpc,
        pattern: MessagePattern::RequestResponse,
        operation: say_hello.path.clone(),
        path: say_hello.path.clone(),
        metadata: HashMap::new(),
        body: None,
        ..Default::default()
    };

    let response = registry
        .generate_mock_response(&request)
        .expect("Should generate mock response");

    assert_eq!(response.content_type, "application/grpc+json");
    assert!(!response.body.is_empty(), "Response body should not be empty");

    // Parse the response body as JSON
    let body: serde_json::Value =
        serde_json::from_slice(&response.body).expect("Response body should be valid JSON");
    assert!(body.is_object(), "Response should be a JSON object");
}

#[tokio::test]
async fn test_grpc_validate_request_known_operation() {
    let dir = proto_dir();
    assert!(std::path::Path::new(&dir).exists(), "Proto directory not found at {}", dir);

    let registry = GrpcProtoRegistry::from_directory(&dir)
        .await
        .expect("Should create registry from proto directory");

    let operations = registry.operations();
    let say_hello = operations
        .iter()
        .find(|op| op.name == "SayHello")
        .expect("Should find SayHello operation");

    // Valid request should pass validation
    let request = ProtocolRequest {
        protocol: Protocol::Grpc,
        pattern: MessagePattern::RequestResponse,
        operation: say_hello.path.clone(),
        path: say_hello.path.clone(),
        metadata: HashMap::new(),
        body: None,
        ..Default::default()
    };

    let result = registry.validate_request(&request).expect("Validation should not error");
    assert!(result.valid, "Known operation should pass validation");
}

#[tokio::test]
async fn test_grpc_validate_request_unknown_operation() {
    let dir = proto_dir();
    assert!(std::path::Path::new(&dir).exists(), "Proto directory not found at {}", dir);

    let registry = GrpcProtoRegistry::from_directory(&dir)
        .await
        .expect("Should create registry from proto directory");

    // Unknown operation should fail validation
    let request = ProtocolRequest {
        protocol: Protocol::Grpc,
        pattern: MessagePattern::RequestResponse,
        operation: "nonexistent.Service/DoSomething".to_string(),
        path: "nonexistent.Service/DoSomething".to_string(),
        metadata: HashMap::new(),
        body: None,
        ..Default::default()
    };

    let result = registry.validate_request(&request).expect("Validation should not error");
    assert!(!result.valid, "Unknown operation should fail validation");
    assert!(!result.errors.is_empty(), "Should have validation errors for unknown operation");
}

#[tokio::test]
async fn test_grpc_descriptor_pool_accessible() {
    let dir = proto_dir();
    assert!(std::path::Path::new(&dir).exists(), "Proto directory not found at {}", dir);

    let config = DynamicGrpcConfig {
        proto_dir: dir,
        enable_reflection: true,
        excluded_services: vec![],
        http_bridge: None,
        tls: None,
    };

    let registry = discover_services(&config).await.expect("Service discovery should succeed");

    let pool = registry.descriptor_pool();

    // Should be able to find the HelloReply message type in the descriptor pool
    let hello_reply = pool.get_message_by_name("mockforge.greeter.HelloReply");
    assert!(hello_reply.is_some(), "Descriptor pool should contain HelloReply message");

    let descriptor = hello_reply.unwrap();
    let field_names: Vec<String> = descriptor.fields().map(|f| f.name().to_string()).collect();
    assert!(
        field_names.iter().any(|n| n == "message"),
        "HelloReply should have a 'message' field"
    );
    assert!(
        field_names.iter().any(|n| n == "metadata"),
        "HelloReply should have a 'metadata' field"
    );
    assert!(
        field_names.iter().any(|n| n == "items"),
        "HelloReply should have an 'items' field"
    );
}
