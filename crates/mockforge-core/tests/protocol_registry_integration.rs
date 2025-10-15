//! Integration tests for the protocol registry system

use async_trait::async_trait;
use mockforge_core::protocol_abstraction::{
    MessagePattern, Protocol, ProtocolHandler, ProtocolRegistry, ProtocolRequest, ProtocolResponse,
    ResponseStatus, SpecRegistry,
};
use mockforge_core::Result;
use std::collections::HashMap;
use std::sync::Arc;

// Mock protocol handler for testing
struct TestProtocolHandler {
    protocol: Protocol,
    response_body: String,
}

impl TestProtocolHandler {
    fn new(protocol: Protocol, response_body: String) -> Self {
        Self {
            protocol,
            response_body,
        }
    }
}

#[async_trait]
impl ProtocolHandler for TestProtocolHandler {
    fn protocol(&self) -> Protocol {
        self.protocol
    }

    fn is_enabled(&self) -> bool {
        true
    }

    fn set_enabled(&mut self, _enabled: bool) {
        // No-op for test
    }

    fn spec_registry(&self) -> Option<&dyn SpecRegistry> {
        None
    }

    async fn handle_request(&self, _request: ProtocolRequest) -> Result<ProtocolResponse> {
        Ok(ProtocolResponse {
            status: ResponseStatus::HttpStatus(200),
            metadata: HashMap::new(),
            body: self.response_body.clone().into_bytes(),
            content_type: "text/plain".to_string(),
        })
    }

    fn validate_configuration(&self) -> Result<()> {
        Ok(())
    }

    fn get_configuration(&self) -> HashMap<String, String> {
        HashMap::new()
    }

    fn update_configuration(&mut self, _config: HashMap<String, String>) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_registry_integration() {
        let mut registry = ProtocolRegistry::new();

        // Register multiple protocol handlers
        let http_handler =
            Arc::new(TestProtocolHandler::new(Protocol::Http, "HTTP Response".to_string()));
        let grpc_handler =
            Arc::new(TestProtocolHandler::new(Protocol::Grpc, "gRPC Response".to_string()));
        let ws_handler = Arc::new(TestProtocolHandler::new(
            Protocol::WebSocket,
            "WebSocket Response".to_string(),
        ));

        registry.register_handler(http_handler).unwrap();
        registry.register_handler(grpc_handler).unwrap();
        registry.register_handler(ws_handler).unwrap();

        // Verify all protocols are registered
        let registered = registry.registered_protocols();
        assert_eq!(registered.len(), 3);
        assert!(registered.contains(&Protocol::Http));
        assert!(registered.contains(&Protocol::Grpc));
        assert!(registered.contains(&Protocol::WebSocket));

        // Verify all protocols are enabled
        let enabled = registry.enabled_protocols();
        assert_eq!(enabled.len(), 3);
        assert!(enabled.contains(&Protocol::Http));
        assert!(enabled.contains(&Protocol::Grpc));
        assert!(enabled.contains(&Protocol::WebSocket));
    }

    #[tokio::test]
    async fn test_request_handling_integration() {
        let mut registry = ProtocolRegistry::new();

        let http_handler =
            Arc::new(TestProtocolHandler::new(Protocol::Http, "HTTP Response".to_string()));
        let grpc_handler =
            Arc::new(TestProtocolHandler::new(Protocol::Grpc, "gRPC Response".to_string()));

        registry.register_handler(http_handler).unwrap();
        registry.register_handler(grpc_handler).unwrap();

        // Test HTTP request handling
        let http_request = ProtocolRequest {
            protocol: Protocol::Http,
            pattern: MessagePattern::RequestResponse,
            operation: "GET".to_string(),
            path: "/test".to_string(),
            topic: None,
            routing_key: None,
            partition: None,
            qos: None,
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        };

        let http_response = registry.handle_request(http_request).await.unwrap();
        assert_eq!(String::from_utf8(http_response.body).unwrap(), "HTTP Response");

        // Test gRPC request handling
        let grpc_request = ProtocolRequest {
            protocol: Protocol::Grpc,
            pattern: MessagePattern::RequestResponse,
            operation: "SayHello".to_string(),
            path: "/greeter.Greeter/SayHello".to_string(),
            topic: None,
            routing_key: None,
            partition: None,
            qos: None,
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        };

        let grpc_response = registry.handle_request(grpc_request).await.unwrap();
        assert_eq!(String::from_utf8(grpc_response.body).unwrap(), "gRPC Response");
    }

    #[test]
    fn test_protocol_enable_disable_integration() {
        let mut registry = ProtocolRegistry::new();

        let http_handler =
            Arc::new(TestProtocolHandler::new(Protocol::Http, "HTTP Response".to_string()));
        let grpc_handler =
            Arc::new(TestProtocolHandler::new(Protocol::Grpc, "gRPC Response".to_string()));

        registry.register_handler(http_handler).unwrap();
        registry.register_handler(grpc_handler).unwrap();

        // Initially both should be enabled
        assert!(registry.is_protocol_enabled(Protocol::Http));
        assert!(registry.is_protocol_enabled(Protocol::Grpc));

        // Disable HTTP
        registry.disable_protocol(Protocol::Http).unwrap();
        assert!(!registry.is_protocol_enabled(Protocol::Http));
        assert!(registry.is_protocol_enabled(Protocol::Grpc));

        // Re-enable HTTP
        registry.enable_protocol(Protocol::Http).unwrap();
        assert!(registry.is_protocol_enabled(Protocol::Http));
        assert!(registry.is_protocol_enabled(Protocol::Grpc));
    }

    #[tokio::test]
    async fn test_disabled_protocol_request_handling() {
        let mut registry = ProtocolRegistry::new();

        let http_handler =
            Arc::new(TestProtocolHandler::new(Protocol::Http, "HTTP Response".to_string()));
        registry.register_handler(http_handler).unwrap();

        // Disable HTTP protocol
        registry.disable_protocol(Protocol::Http).unwrap();

        let request = ProtocolRequest {
            protocol: Protocol::Http,
            pattern: MessagePattern::RequestResponse,
            operation: "GET".to_string(),
            path: "/test".to_string(),
            topic: None,
            routing_key: None,
            partition: None,
            qos: None,
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        };

        // Request should fail for disabled protocol
        let result = registry.handle_request(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Protocol disabled"));
    }

    #[test]
    fn test_unified_fixture_integration() {
        use mockforge_core::protocol_abstraction::{
            FixtureRequest, FixtureResponse, FixtureStatus, UnifiedFixture,
        };

        let fixture = UnifiedFixture {
            id: "integration-test".to_string(),
            name: "Integration Test Fixture".to_string(),
            description: "Test fixture for integration testing".to_string(),
            protocol: Protocol::Http,
            request: FixtureRequest {
                pattern: Some(MessagePattern::RequestResponse),
                operation: Some("GET".to_string()),
                path: Some("/api/test".to_string()),
                topic: None,
                routing_key: None,
                partition: None,
                qos: None,
                headers: {
                    let mut h = HashMap::new();
                    h.insert("content-type".to_string(), "application/json".to_string());
                    h
                },
                body_pattern: None,
                custom_matcher: None,
            },
            response: FixtureResponse {
                status: FixtureStatus::Http(200),
                headers: {
                    let mut h = HashMap::new();
                    h.insert("content-type".to_string(), "application/json".to_string());
                    h
                },
                body: Some(serde_json::json!({"status": "ok", "data": [1, 2, 3]})),
                content_type: Some("application/json".to_string()),
                delay_ms: 0,
                template_vars: HashMap::new(),
            },
            metadata: {
                let mut m = HashMap::new();
                m.insert("test".to_string(), serde_json::json!(true));
                m
            },
            enabled: true,
            priority: 1,
            tags: vec!["integration".to_string(), "api".to_string()],
        };

        // Test fixture matching
        let matching_request = ProtocolRequest {
            protocol: Protocol::Http,
            pattern: MessagePattern::RequestResponse,
            operation: "GET".to_string(),
            path: "/api/test".to_string(),
            topic: None,
            routing_key: None,
            partition: None,
            qos: None,
            metadata: {
                let mut h = HashMap::new();
                h.insert("content-type".to_string(), "application/json".to_string());
                h
            },
            body: None,
            client_ip: None,
        };

        assert!(fixture.matches(&matching_request));

        // Test fixture response generation
        let response = fixture.to_protocol_response().unwrap();
        assert!(response.status.is_success());
        assert_eq!(response.content_type, "application/json");
        assert!(response.metadata.contains_key("content-type"));
        assert!(!response.body.is_empty());
    }
}
