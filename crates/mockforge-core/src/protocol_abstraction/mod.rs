//! Protocol-agnostic abstractions for unified mocking across HTTP, GraphQL, gRPC, and WebSocket
//!
//! This module provides traits and types that abstract common patterns across different
//! protocols, enabling code reuse for spec-driven mocking, middleware, and request matching.

pub mod auth;
pub mod matcher;
pub mod middleware;

use crate::Result;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

// Re-export middleware types
pub use auth::{AuthMiddleware, AuthResult, Claims};
pub use matcher::{FuzzyRequestMatcher, RequestFingerprint, SimpleRequestMatcher};
pub use middleware::{LatencyMiddleware, LoggingMiddleware, MetricsMiddleware};

/// Protocol type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    /// HTTP/REST protocol
    Http,
    /// GraphQL protocol
    GraphQL,
    /// gRPC protocol
    Grpc,
    /// WebSocket protocol
    WebSocket,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::Http => write!(f, "HTTP"),
            Protocol::GraphQL => write!(f, "GraphQL"),
            Protocol::Grpc => write!(f, "gRPC"),
            Protocol::WebSocket => write!(f, "WebSocket"),
        }
    }
}

/// A protocol-agnostic request representation
#[derive(Debug, Clone)]
pub struct ProtocolRequest {
    /// The protocol this request uses
    pub protocol: Protocol,
    /// Method or operation (e.g., "GET", "Query.users", "greeter.SayHello")
    pub operation: String,
    /// Path, query name, or service/method name
    pub path: String,
    /// Request metadata (headers, metadata, etc.)
    pub metadata: HashMap<String, String>,
    /// Request body/payload as bytes
    pub body: Option<Vec<u8>>,
    /// Client IP address if available
    pub client_ip: Option<String>,
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
}

impl ResponseStatus {
    /// Check if the response is successful
    pub fn is_success(&self) -> bool {
        match self {
            ResponseStatus::HttpStatus(code) => (200..300).contains(code),
            ResponseStatus::GrpcStatus(code) => *code == 0, // gRPC OK = 0
            ResponseStatus::GraphQLStatus(success) => *success,
            ResponseStatus::WebSocketStatus(success) => *success,
        }
    }

    /// Get numeric representation if applicable
    pub fn as_code(&self) -> Option<i32> {
        match self {
            ResponseStatus::HttpStatus(code) => Some(*code as i32),
            ResponseStatus::GrpcStatus(code) => Some(*code),
            ResponseStatus::GraphQLStatus(_) | ResponseStatus::WebSocketStatus(_) => None,
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
            metadata: HashMap::new(),
            body: None,
            client_ip: Some("127.0.0.1".to_string()),
        };
        assert_eq!(request.protocol, Protocol::Http);
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
}
