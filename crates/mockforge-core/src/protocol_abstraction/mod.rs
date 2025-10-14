//! Protocol-agnostic abstractions for unified mocking across HTTP, GraphQL, gRPC, and WebSocket
//!
//! This module provides traits and types that abstract common patterns across different
//! protocols, enabling code reuse for spec-driven mocking, middleware, and request matching.

pub mod auth;
pub mod matcher;
pub mod middleware;
pub mod protocol_registry;

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

// Re-export middleware types
pub use auth::{AuthMiddleware, AuthResult, Claims};
pub use matcher::{FuzzyRequestMatcher, RequestFingerprint, SimpleRequestMatcher};
pub use middleware::{LatencyMiddleware, LoggingMiddleware, MetricsMiddleware};
pub use protocol_registry::{ProtocolHandler, ProtocolRegistry};

/// Protocol type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Protocol {
    /// HTTP/REST protocol
    Http,
    /// GraphQL protocol
    GraphQL,
    /// gRPC protocol
    Grpc,
    /// WebSocket protocol
    WebSocket,
    /// SMTP/Email protocol
    Smtp,
    /// MQTT protocol (IoT messaging)
    Mqtt,
    /// FTP protocol (file transfer)
    Ftp,
    /// Kafka protocol (event streaming)
    Kafka,
    /// RabbitMQ/AMQP protocol (message queuing)
    RabbitMq,
    /// AMQP protocol (advanced message queuing)
    Amqp,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::Http => write!(f, "HTTP"),
            Protocol::GraphQL => write!(f, "GraphQL"),
            Protocol::Grpc => write!(f, "gRPC"),
            Protocol::WebSocket => write!(f, "WebSocket"),
            Protocol::Smtp => write!(f, "SMTP"),
            Protocol::Mqtt => write!(f, "MQTT"),
            Protocol::Ftp => write!(f, "FTP"),
            Protocol::Kafka => write!(f, "Kafka"),
            Protocol::RabbitMq => write!(f, "RabbitMQ"),
            Protocol::Amqp => write!(f, "AMQP"),
        }
    }
}

/// Message pattern enumeration for different communication patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessagePattern {
    /// Request-Response pattern (HTTP, gRPC unary)
    RequestResponse,
    /// One-way/fire-and-forget pattern (MQTT publish, email)
    OneWay,
    /// Publish-Subscribe pattern (Kafka, RabbitMQ, MQTT)
    PubSub,
    /// Streaming pattern (gRPC streaming, WebSocket)
    Streaming,
}

impl fmt::Display for MessagePattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessagePattern::RequestResponse => write!(f, "Request-Response"),
            MessagePattern::OneWay => write!(f, "One-Way"),
            MessagePattern::PubSub => write!(f, "Pub-Sub"),
            MessagePattern::Streaming => write!(f, "Streaming"),
        }
    }
}

/// A protocol-agnostic request representation
#[derive(Debug, Clone)]
pub struct ProtocolRequest {
    /// The protocol this request uses
    pub protocol: Protocol,
    /// Message pattern for this request
    pub pattern: MessagePattern,
    /// Method or operation (e.g., "GET", "Query.users", "greeter.SayHello")
    pub operation: String,
    /// Path, query name, or service/method name
    pub path: String,
    /// Topic for pub/sub protocols (MQTT, Kafka)
    pub topic: Option<String>,
    /// Routing key for message queuing protocols (AMQP, RabbitMQ)
    pub routing_key: Option<String>,
    /// Partition for partitioned protocols (Kafka)
    pub partition: Option<i32>,
    /// Quality of Service level (MQTT: 0, 1, 2)
    pub qos: Option<u8>,
    /// Request metadata (headers, metadata, etc.)
    pub metadata: HashMap<String, String>,
    /// Request body/payload as bytes
    pub body: Option<Vec<u8>>,
    /// Client IP address if available
    pub client_ip: Option<String>,
}

impl Default for ProtocolRequest {
    fn default() -> Self {
        Self {
            protocol: Protocol::Http,
            pattern: MessagePattern::RequestResponse,
            operation: String::new(),
            path: String::new(),
            topic: None,
            routing_key: None,
            partition: None,
            qos: None,
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        }
    }
}

/// A protocol-agnostic response representation
#[derive(Debug, Clone)]
pub struct ProtocolResponse {
    /// Status code or success indicator (HTTP: 200, gRPC: OK, GraphQL: no errors)
    pub status: ResponseStatus,
    /// Response metadata (headers, metadata, etc.)
    pub metadata: HashMap<String, String>,
    /// Response body/payload
    pub body: Vec<u8>,
    /// Content type or serialization format
    pub content_type: String,
}

/// Response status abstraction across protocols
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseStatus {
    /// HTTP status code
    HttpStatus(u16),
    /// gRPC status code
    GrpcStatus(i32),
    /// GraphQL success (true) or error (false)
    GraphQLStatus(bool),
    /// WebSocket status
    WebSocketStatus(bool),
    /// SMTP status code (2xx = success, 4xx/5xx = error)
    SmtpStatus(u16),
    /// MQTT status (true = success, false = error)
    MqttStatus(bool),
    /// Kafka status code (0 = success, non-zero = error)
    KafkaStatus(i16),
    /// AMQP/RabbitMQ status code
    AmqpStatus(u16),
    /// FTP status code
    FtpStatus(u16),
}

impl ResponseStatus {
    /// Check if the response is successful
    pub fn is_success(&self) -> bool {
        match self {
            ResponseStatus::HttpStatus(code) => (200..300).contains(code),
            ResponseStatus::GrpcStatus(code) => *code == 0, // gRPC OK = 0
            ResponseStatus::GraphQLStatus(success) => *success,
            ResponseStatus::WebSocketStatus(success) => *success,
            ResponseStatus::SmtpStatus(code) => (200..300).contains(code), // 2xx codes are success
            ResponseStatus::MqttStatus(success) => *success,
            ResponseStatus::KafkaStatus(code) => *code == 0, // Kafka OK = 0
            ResponseStatus::AmqpStatus(code) => (200..300).contains(code), // AMQP success codes
            ResponseStatus::FtpStatus(code) => (200..300).contains(code), // FTP success codes
        }
    }

    /// Get numeric representation if applicable
    pub fn as_code(&self) -> Option<i32> {
        match self {
            ResponseStatus::HttpStatus(code) => Some(*code as i32),
            ResponseStatus::GrpcStatus(code) => Some(*code),
            ResponseStatus::SmtpStatus(code) => Some(*code as i32),
            ResponseStatus::KafkaStatus(code) => Some(*code as i32),
            ResponseStatus::AmqpStatus(code) => Some(*code as i32),
            ResponseStatus::FtpStatus(code) => Some(*code as i32),
            ResponseStatus::GraphQLStatus(_)
            | ResponseStatus::WebSocketStatus(_)
            | ResponseStatus::MqttStatus(_) => None,
        }
    }
}

/// Trait for spec-driven mocking registries (OpenAPI, GraphQL schema, Proto files)
pub trait SpecRegistry: Send + Sync {
    /// Get the protocol this registry handles
    fn protocol(&self) -> Protocol;

    /// Get all available operations/routes in this spec
    fn operations(&self) -> Vec<SpecOperation>;

    /// Find an operation by path/name
    fn find_operation(&self, operation: &str, path: &str) -> Option<SpecOperation>;

    /// Validate a request against the spec
    fn validate_request(&self, request: &ProtocolRequest) -> Result<ValidationResult>;

    /// Generate a mock response for a request
    fn generate_mock_response(&self, request: &ProtocolRequest) -> Result<ProtocolResponse>;
}

/// Represents a single operation in a spec (endpoint, query, RPC method)
#[derive(Debug, Clone)]
pub struct SpecOperation {
    /// Operation name or identifier
    pub name: String,
    /// Path or fully qualified name
    pub path: String,
    /// Operation type (GET, POST, Query, Mutation, RPC)
    pub operation_type: String,
    /// Input schema/type information
    pub input_schema: Option<String>,
    /// Output schema/type information
    pub output_schema: Option<String>,
    /// Metadata from spec
    pub metadata: HashMap<String, String>,
}

/// Result of request validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,
    /// Validation errors if any
    pub errors: Vec<ValidationError>,
    /// Validation warnings
    pub warnings: Vec<String>,
}

/// A validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error message
    pub message: String,
    /// Path to the error (e.g., "body.user.email")
    pub path: Option<String>,
    /// Error code
    pub code: Option<String>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn success() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create a failed validation result with errors
    pub fn failure(errors: Vec<ValidationError>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// Add a warning to the result
    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }
}

/// Trait for protocol-agnostic middleware
#[async_trait::async_trait]
pub trait ProtocolMiddleware: Send + Sync {
    /// Get the name of this middleware
    fn name(&self) -> &str;

    /// Process a request before it reaches the handler
    async fn process_request(&self, request: &mut ProtocolRequest) -> Result<()>;

    /// Process a response before it's returned to the client
    async fn process_response(
        &self,
        request: &ProtocolRequest,
        response: &mut ProtocolResponse,
    ) -> Result<()>;

    /// Check if this middleware should run for a given protocol
    fn supports_protocol(&self, protocol: Protocol) -> bool;
}

/// Trait for request matching across protocols
pub trait RequestMatcher: Send + Sync {
    /// Match a request and return a score (higher = better match)
    fn match_score(&self, request: &ProtocolRequest) -> f64;

    /// Get the protocol this matcher handles
    fn protocol(&self) -> Protocol;
}

/// Unified fixture format supporting all protocols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedFixture {
    /// Unique identifier for this fixture
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Description of what this fixture does
    #[serde(default)]
    pub description: String,

    /// Protocol this fixture applies to
    pub protocol: Protocol,

    /// Request matching criteria
    pub request: FixtureRequest,

    /// Response configuration
    pub response: FixtureResponse,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,

    /// Whether this fixture is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Priority for matching (higher = matched first)
    #[serde(default)]
    pub priority: i32,

    /// Tags for organization
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Request matching criteria for fixtures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureRequest {
    /// Message pattern to match
    #[serde(default)]
    pub pattern: Option<MessagePattern>,

    /// Operation/method to match (exact or regex)
    pub operation: Option<String>,

    /// Path/route to match (exact or regex)
    pub path: Option<String>,

    /// Topic to match (for pub/sub protocols)
    pub topic: Option<String>,

    /// Routing key to match (for message queuing)
    pub routing_key: Option<String>,

    /// Partition to match
    pub partition: Option<i32>,

    /// QoS level to match
    pub qos: Option<u8>,

    /// Headers/metadata to match (key-value pairs)
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Request body pattern (regex for text, or exact match)
    pub body_pattern: Option<String>,

    /// Custom matching logic (script or expression)
    pub custom_matcher: Option<String>,
}

/// Response configuration for fixtures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureResponse {
    /// Response status
    pub status: FixtureStatus,

    /// Response headers
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Response body (can be string, JSON, or base64-encoded binary)
    pub body: Option<serde_json::Value>,

    /// Content type
    pub content_type: Option<String>,

    /// Response delay in milliseconds
    #[serde(default)]
    pub delay_ms: u64,

    /// Template variables for dynamic responses
    #[serde(default)]
    pub template_vars: HashMap<String, serde_json::Value>,
}

/// Status representation for fixtures
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FixtureStatus {
    /// HTTP status code
    Http(u16),
    /// gRPC status code
    Grpc(i32),
    /// Generic success/failure
    Generic(bool),
    /// Custom status with code and message
    Custom { code: i32, message: String },
}

fn default_true() -> bool {
    true
}

impl UnifiedFixture {
    /// Check if this fixture matches the given protocol request
    pub fn matches(&self, request: &ProtocolRequest) -> bool {
        // Check protocol
        if request.protocol != self.protocol {
            return false;
        }

        // Check pattern
        if let Some(pattern) = &self.request.pattern {
            if request.pattern != *pattern {
                return false;
            }
        }

        // Check operation
        if let Some(operation) = &self.request.operation {
            if !self.matches_pattern(&request.operation, operation) {
                return false;
            }
        }

        // Check path
        if let Some(path) = &self.request.path {
            if !self.matches_pattern(&request.path, path) {
                return false;
            }
        }

        // Check topic
        if let Some(topic) = &self.request.topic {
            if !self.matches_pattern(request.topic.as_ref().unwrap_or(&String::new()), topic) {
                return false;
            }
        }

        // Check routing key
        if let Some(routing_key) = &self.request.routing_key {
            if !self.matches_pattern(
                request.routing_key.as_ref().unwrap_or(&String::new()),
                routing_key,
            ) {
                return false;
            }
        }

        // Check partition
        if let Some(partition) = self.request.partition {
            if request.partition != Some(partition) {
                return false;
            }
        }

        // Check QoS
        if let Some(qos) = self.request.qos {
            if request.qos != Some(qos) {
                return false;
            }
        }

        // Check headers
        for (key, expected_value) in &self.request.headers {
            if let Some(actual_value) = request.metadata.get(key) {
                if !self.matches_pattern(actual_value, expected_value) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check body pattern
        if let Some(pattern) = &self.request.body_pattern {
            if let Some(body) = &request.body {
                let body_str = String::from_utf8_lossy(body);
                if !self.matches_pattern(&body_str, pattern) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // TODO: Implement custom matcher logic

        true
    }

    /// Helper method to match patterns (supports regex and exact match)
    fn matches_pattern(&self, value: &str, pattern: &str) -> bool {
        use regex::Regex;

        // Try regex first
        if let Ok(re) = Regex::new(pattern) {
            re.is_match(value)
        } else {
            // Fall back to exact match
            value == pattern
        }
    }

    /// Convert fixture response to ProtocolResponse
    pub fn to_protocol_response(&self) -> Result<ProtocolResponse> {
        let status = match &self.response.status {
            FixtureStatus::Http(code) => ResponseStatus::HttpStatus(*code),
            FixtureStatus::Grpc(code) => ResponseStatus::GrpcStatus(*code),
            FixtureStatus::Generic(success) => ResponseStatus::GraphQLStatus(*success), // Using GraphQL as generic
            FixtureStatus::Custom { code, .. } => ResponseStatus::GrpcStatus(*code), // Using gRPC as custom
        };

        let body = match &self.response.body {
            Some(serde_json::Value::String(s)) => s.clone().into_bytes(),
            Some(value) => serde_json::to_string(value)?.into_bytes(),
            None => Vec::new(),
        };

        let content_type = self
            .response
            .content_type
            .clone()
            .unwrap_or_else(|| "application/json".to_string());

        Ok(ProtocolResponse {
            status,
            metadata: self.response.headers.clone(),
            body,
            content_type,
        })
    }
}

/// Middleware chain for composing multiple middleware
pub struct MiddlewareChain {
    middleware: Vec<Arc<dyn ProtocolMiddleware>>,
}

impl MiddlewareChain {
    /// Create a new middleware chain
    pub fn new() -> Self {
        Self {
            middleware: Vec::new(),
        }
    }

    /// Add middleware to the chain
    pub fn with_middleware(mut self, middleware: Arc<dyn ProtocolMiddleware>) -> Self {
        self.middleware.push(middleware);
        self
    }

    /// Process a request through all middleware
    pub async fn process_request(&self, request: &mut ProtocolRequest) -> Result<()> {
        for middleware in &self.middleware {
            if middleware.supports_protocol(request.protocol) {
                middleware.process_request(request).await?;
            }
        }
        Ok(())
    }

    /// Process a response through all middleware (in reverse order)
    pub async fn process_response(
        &self,
        request: &ProtocolRequest,
        response: &mut ProtocolResponse,
    ) -> Result<()> {
        for middleware in self.middleware.iter().rev() {
            if middleware.supports_protocol(request.protocol) {
                middleware.process_response(request, response).await?;
            }
        }
        Ok(())
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_display() {
        assert_eq!(Protocol::Http.to_string(), "HTTP");
        assert_eq!(Protocol::GraphQL.to_string(), "GraphQL");
        assert_eq!(Protocol::Grpc.to_string(), "gRPC");
        assert_eq!(Protocol::WebSocket.to_string(), "WebSocket");
        assert_eq!(Protocol::Smtp.to_string(), "SMTP");
        assert_eq!(Protocol::Mqtt.to_string(), "MQTT");
        assert_eq!(Protocol::Ftp.to_string(), "FTP");
        assert_eq!(Protocol::Kafka.to_string(), "Kafka");
        assert_eq!(Protocol::RabbitMq.to_string(), "RabbitMQ");
        assert_eq!(Protocol::Amqp.to_string(), "AMQP");
    }

    #[test]
    fn test_response_status_is_success() {
        assert!(ResponseStatus::HttpStatus(200).is_success());
        assert!(ResponseStatus::HttpStatus(204).is_success());
        assert!(!ResponseStatus::HttpStatus(404).is_success());
        assert!(!ResponseStatus::HttpStatus(500).is_success());

        assert!(ResponseStatus::GrpcStatus(0).is_success());
        assert!(!ResponseStatus::GrpcStatus(2).is_success());

        assert!(ResponseStatus::GraphQLStatus(true).is_success());
        assert!(!ResponseStatus::GraphQLStatus(false).is_success());
    }

    #[test]
    fn test_response_status_as_code() {
        assert_eq!(ResponseStatus::HttpStatus(200).as_code(), Some(200));
        assert_eq!(ResponseStatus::GrpcStatus(0).as_code(), Some(0));
        assert_eq!(ResponseStatus::GraphQLStatus(true).as_code(), None);
    }

    #[test]
    fn test_validation_result_success() {
        let result = ValidationResult::success();
        assert!(result.valid);
        assert_eq!(result.errors.len(), 0);
        assert_eq!(result.warnings.len(), 0);
    }

    #[test]
    fn test_validation_result_failure() {
        let errors = vec![ValidationError {
            message: "Invalid field".to_string(),
            path: Some("body.field".to_string()),
            code: Some("INVALID_FIELD".to_string()),
        }];
        let result = ValidationResult::failure(errors);
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_validation_result_with_warning() {
        let result = ValidationResult::success().with_warning("Deprecated field used".to_string());
        assert!(result.valid);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_middleware_chain_creation() {
        let chain = MiddlewareChain::new();
        assert_eq!(chain.middleware.len(), 0);
    }

    #[test]
    fn test_protocol_request_creation() {
        let request = ProtocolRequest {
            protocol: Protocol::Http,
            operation: "GET".to_string(),
            path: "/users".to_string(),
            client_ip: Some("127.0.0.1".to_string()),
            ..Default::default()
        };
        assert_eq!(request.protocol, Protocol::Http);
        assert_eq!(request.pattern, MessagePattern::RequestResponse);
        assert_eq!(request.operation, "GET");
        assert_eq!(request.path, "/users");
    }

    #[test]
    fn test_protocol_response_creation() {
        let response = ProtocolResponse {
            status: ResponseStatus::HttpStatus(200),
            metadata: HashMap::new(),
            body: b"{}".to_vec(),
            content_type: "application/json".to_string(),
        };
        assert!(response.status.is_success());
        assert_eq!(response.content_type, "application/json");
    }

    #[test]
    fn test_unified_fixture_matching() {
        let fixture = UnifiedFixture {
            id: "test-fixture".to_string(),
            name: "Test Fixture".to_string(),
            description: "A test fixture".to_string(),
            protocol: Protocol::Http,
            request: FixtureRequest {
                pattern: Some(MessagePattern::RequestResponse),
                operation: Some("GET".to_string()),
                path: Some("/api/users".to_string()),
                topic: None,
                routing_key: None,
                partition: None,
                qos: None,
                headers: HashMap::new(),
                body_pattern: None,
                custom_matcher: None,
            },
            response: FixtureResponse {
                status: FixtureStatus::Http(200),
                headers: HashMap::new(),
                body: Some(serde_json::json!({"users": ["john", "jane"]})),
                content_type: Some("application/json".to_string()),
                delay_ms: 0,
                template_vars: HashMap::new(),
            },
            metadata: HashMap::new(),
            enabled: true,
            priority: 0,
            tags: vec![],
        };

        let matching_request = ProtocolRequest {
            protocol: Protocol::Http,
            pattern: MessagePattern::RequestResponse,
            operation: "GET".to_string(),
            path: "/api/users".to_string(),
            topic: None,
            routing_key: None,
            partition: None,
            qos: None,
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        };

        let non_matching_request = ProtocolRequest {
            protocol: Protocol::Http,
            pattern: MessagePattern::RequestResponse,
            operation: "POST".to_string(),
            path: "/api/users".to_string(),
            topic: None,
            routing_key: None,
            partition: None,
            qos: None,
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        };

        assert!(fixture.matches(&matching_request));
        assert!(!fixture.matches(&non_matching_request));
    }

    #[test]
    fn test_fixture_to_protocol_response() {
        let fixture = UnifiedFixture {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "".to_string(),
            protocol: Protocol::Http,
            request: FixtureRequest {
                pattern: None,
                operation: None,
                path: None,
                topic: None,
                routing_key: None,
                partition: None,
                qos: None,
                headers: HashMap::new(),
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
                body: Some(serde_json::json!({"message": "ok"})),
                content_type: Some("application/json".to_string()),
                delay_ms: 0,
                template_vars: HashMap::new(),
            },
            metadata: HashMap::new(),
            enabled: true,
            priority: 0,
            tags: vec![],
        };

        let response = fixture.to_protocol_response().unwrap();
        assert!(response.status.is_success());
        assert_eq!(response.content_type, "application/json");
        assert_eq!(response.metadata.get("content-type"), Some(&"application/json".to_string()));
    }

    #[test]
    fn test_fixture_status_serialization() {
        // Test HTTP status
        let status = FixtureStatus::Http(404);
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "404");

        // Test gRPC status
        let status = FixtureStatus::Grpc(5);
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "5");

        // Test generic status
        let status = FixtureStatus::Generic(true);
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "true");

        // Test custom status
        let status = FixtureStatus::Custom {
            code: 500,
            message: "Internal Error".to_string(),
        };
        let serialized = serde_json::to_string(&status).unwrap();
        let expected: serde_json::Value =
            serde_json::json!({"code": 500, "message": "Internal Error"});
        assert_eq!(serde_json::from_str::<serde_json::Value>(&serialized).unwrap(), expected);
    }

    #[test]
    fn test_fixture_pattern_matching() {
        let fixture = UnifiedFixture {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "".to_string(),
            protocol: Protocol::Http,
            request: FixtureRequest {
                pattern: Some(MessagePattern::RequestResponse),
                operation: Some("GET".to_string()),
                path: Some("/api/.*".to_string()),
                topic: None,
                routing_key: None,
                partition: None,
                qos: None,
                headers: HashMap::new(),
                body_pattern: None,
                custom_matcher: None,
            },
            response: FixtureResponse {
                status: FixtureStatus::Http(200),
                headers: HashMap::new(),
                body: None,
                content_type: None,
                delay_ms: 0,
                template_vars: HashMap::new(),
            },
            metadata: HashMap::new(),
            enabled: true,
            priority: 0,
            tags: vec![],
        };

        // Test matching request
        let request = ProtocolRequest {
            protocol: Protocol::Http,
            pattern: MessagePattern::RequestResponse,
            operation: "GET".to_string(),
            path: "/api/users".to_string(),
            topic: None,
            routing_key: None,
            partition: None,
            qos: None,
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        };
        assert!(fixture.matches(&request));

        // Test non-matching protocol
        let grpc_request = ProtocolRequest {
            protocol: Protocol::Grpc,
            pattern: MessagePattern::RequestResponse,
            operation: "GET".to_string(),
            path: "/api/users".to_string(),
            topic: None,
            routing_key: None,
            partition: None,
            qos: None,
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        };
        assert!(!fixture.matches(&grpc_request));

        // Test non-matching operation
        let post_request = ProtocolRequest {
            protocol: Protocol::Http,
            pattern: MessagePattern::RequestResponse,
            operation: "POST".to_string(),
            path: "/api/users".to_string(),
            topic: None,
            routing_key: None,
            partition: None,
            qos: None,
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        };
        assert!(!fixture.matches(&post_request));
    }
}
