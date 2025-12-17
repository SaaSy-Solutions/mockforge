//! Data models for recorded requests and responses

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "lowercase")]
pub enum Protocol {
    #[sqlx(rename = "http")]
    Http,
    #[sqlx(rename = "grpc")]
    Grpc,
    #[sqlx(rename = "websocket")]
    WebSocket,
    #[sqlx(rename = "graphql")]
    GraphQL,
}

impl Protocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::Http => "http",
            Protocol::Grpc => "grpc",
            Protocol::WebSocket => "websocket",
            Protocol::GraphQL => "graphql",
        }
    }
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Tracing context for requests (OpenTelemetry)
#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    /// Client IP address
    pub client_ip: Option<String>,
    /// Trace ID (from OpenTelemetry)
    pub trace_id: Option<String>,
    /// Span ID (from OpenTelemetry)
    pub span_id: Option<String>,
}

impl RequestContext {
    /// Create a new request context
    pub fn new(client_ip: Option<&str>, trace_id: Option<&str>, span_id: Option<&str>) -> Self {
        Self {
            client_ip: client_ip.map(|s| s.to_string()),
            trace_id: trace_id.map(|s| s.to_string()),
            span_id: span_id.map(|s| s.to_string()),
        }
    }
}

/// Recorded HTTP/API request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedRequest {
    /// Unique request ID
    pub id: String,
    /// Protocol type
    pub protocol: Protocol,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// HTTP method or gRPC method name
    pub method: String,
    /// Request path or endpoint
    pub path: String,
    /// Query parameters (for HTTP)
    pub query_params: Option<String>,
    /// Request headers (JSON)
    pub headers: String,
    /// Request body (may be base64 encoded for binary)
    pub body: Option<String>,
    /// Body encoding (utf8, base64)
    pub body_encoding: String,
    /// Client IP address
    pub client_ip: Option<String>,
    /// Trace ID (from OpenTelemetry)
    pub trace_id: Option<String>,
    /// Span ID (from OpenTelemetry)
    pub span_id: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: Option<i64>,
    /// Response status code
    pub status_code: Option<i32>,
    /// Tags for categorization (JSON array)
    pub tags: Option<String>,
}

impl RecordedRequest {
    /// Parse headers from JSON string
    pub fn headers_map(&self) -> HashMap<String, String> {
        serde_json::from_str(&self.headers).unwrap_or_default()
    }

    /// Parse query parameters
    pub fn query_params_map(&self) -> HashMap<String, String> {
        self.query_params
            .as_ref()
            .and_then(|q| serde_json::from_str(q).ok())
            .unwrap_or_default()
    }

    /// Parse tags
    pub fn tags_vec(&self) -> Vec<String> {
        self.tags
            .as_ref()
            .and_then(|t| serde_json::from_str(t).ok())
            .unwrap_or_default()
    }

    /// Decode body based on encoding
    pub fn decoded_body(&self) -> Option<Vec<u8>> {
        self.body.as_ref().map(|body| {
            if self.body_encoding == "base64" {
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, body)
                    .unwrap_or_else(|_| body.as_bytes().to_vec())
            } else {
                body.as_bytes().to_vec()
            }
        })
    }
}

/// Recorded response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedResponse {
    /// Response ID (same as request ID)
    pub request_id: String,
    /// Response status code
    pub status_code: i32,
    /// Response headers (JSON)
    pub headers: String,
    /// Response body (may be base64 encoded for binary)
    pub body: Option<String>,
    /// Body encoding (utf8, base64)
    pub body_encoding: String,
    /// Response size in bytes
    pub size_bytes: i64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl RecordedResponse {
    /// Parse headers from JSON string
    pub fn headers_map(&self) -> HashMap<String, String> {
        serde_json::from_str(&self.headers).unwrap_or_default()
    }

    /// Decode body based on encoding
    pub fn decoded_body(&self) -> Option<Vec<u8>> {
        self.body.as_ref().map(|body| {
            if self.body_encoding == "base64" {
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, body)
                    .unwrap_or_else(|_| body.as_bytes().to_vec())
            } else {
                body.as_bytes().to_vec()
            }
        })
    }
}

/// Request/Response pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedExchange {
    pub request: RecordedRequest,
    pub response: Option<RecordedResponse>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Protocol Tests ====================

    #[test]
    fn test_protocol_display() {
        assert_eq!(Protocol::Http.to_string(), "http");
        assert_eq!(Protocol::Grpc.to_string(), "grpc");
        assert_eq!(Protocol::WebSocket.to_string(), "websocket");
        assert_eq!(Protocol::GraphQL.to_string(), "graphql");
    }

    #[test]
    fn test_protocol_as_str() {
        assert_eq!(Protocol::Http.as_str(), "http");
        assert_eq!(Protocol::Grpc.as_str(), "grpc");
        assert_eq!(Protocol::WebSocket.as_str(), "websocket");
        assert_eq!(Protocol::GraphQL.as_str(), "graphql");
    }

    #[test]
    fn test_protocol_equality() {
        assert_eq!(Protocol::Http, Protocol::Http);
        assert_ne!(Protocol::Http, Protocol::Grpc);
    }

    #[test]
    fn test_protocol_clone() {
        let proto = Protocol::Http;
        let cloned = proto.clone();
        assert_eq!(proto, cloned);
    }

    #[test]
    fn test_protocol_serialize() {
        let proto = Protocol::Http;
        let json = serde_json::to_string(&proto).unwrap();
        assert_eq!(json, "\"Http\"");
    }

    #[test]
    fn test_protocol_deserialize() {
        let json = "\"Grpc\"";
        let proto: Protocol = serde_json::from_str(json).unwrap();
        assert_eq!(proto, Protocol::Grpc);
    }

    // ==================== RequestContext Tests ====================

    #[test]
    fn test_request_context_new() {
        let ctx = RequestContext::new(Some("192.168.1.1"), Some("trace-123"), Some("span-456"));
        assert_eq!(ctx.client_ip, Some("192.168.1.1".to_string()));
        assert_eq!(ctx.trace_id, Some("trace-123".to_string()));
        assert_eq!(ctx.span_id, Some("span-456".to_string()));
    }

    #[test]
    fn test_request_context_new_with_nones() {
        let ctx = RequestContext::new(None, None, None);
        assert!(ctx.client_ip.is_none());
        assert!(ctx.trace_id.is_none());
        assert!(ctx.span_id.is_none());
    }

    #[test]
    fn test_request_context_default() {
        let ctx = RequestContext::default();
        assert!(ctx.client_ip.is_none());
        assert!(ctx.trace_id.is_none());
        assert!(ctx.span_id.is_none());
    }

    #[test]
    fn test_request_context_clone() {
        let ctx = RequestContext::new(Some("127.0.0.1"), Some("trace"), Some("span"));
        let cloned = ctx.clone();
        assert_eq!(ctx.client_ip, cloned.client_ip);
        assert_eq!(ctx.trace_id, cloned.trace_id);
        assert_eq!(ctx.span_id, cloned.span_id);
    }

    // ==================== RecordedRequest Tests ====================

    fn create_test_request() -> RecordedRequest {
        RecordedRequest {
            id: "test-123".to_string(),
            protocol: Protocol::Http,
            timestamp: Utc::now(),
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            query_params: Some(r#"{"page":"1","limit":"10"}"#.to_string()),
            headers: r#"{"content-type":"application/json","authorization":"Bearer token"}"#
                .to_string(),
            body: Some("hello world".to_string()),
            body_encoding: "utf8".to_string(),
            client_ip: Some("192.168.1.1".to_string()),
            trace_id: Some("trace-abc".to_string()),
            span_id: Some("span-xyz".to_string()),
            duration_ms: Some(150),
            status_code: Some(200),
            tags: Some(r#"["api","users","test"]"#.to_string()),
        }
    }

    #[test]
    fn test_headers_parsing() {
        let request = RecordedRequest {
            id: "test".to_string(),
            protocol: Protocol::Http,
            timestamp: Utc::now(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            query_params: None,
            headers: r#"{"content-type":"application/json"}"#.to_string(),
            body: None,
            body_encoding: "utf8".to_string(),
            client_ip: None,
            trace_id: None,
            span_id: None,
            duration_ms: None,
            status_code: None,
            tags: Some(r#"["test","api"]"#.to_string()),
        };

        let headers = request.headers_map();
        assert_eq!(headers.get("content-type"), Some(&"application/json".to_string()));

        let tags = request.tags_vec();
        assert_eq!(tags, vec!["test", "api"]);
    }

    #[test]
    fn test_recorded_request_headers_map() {
        let request = create_test_request();
        let headers = request.headers_map();
        assert_eq!(headers.get("content-type"), Some(&"application/json".to_string()));
        assert_eq!(headers.get("authorization"), Some(&"Bearer token".to_string()));
    }

    #[test]
    fn test_recorded_request_headers_map_invalid_json() {
        let mut request = create_test_request();
        request.headers = "invalid json".to_string();
        let headers = request.headers_map();
        assert!(headers.is_empty());
    }

    #[test]
    fn test_recorded_request_query_params_map() {
        let request = create_test_request();
        let params = request.query_params_map();
        assert_eq!(params.get("page"), Some(&"1".to_string()));
        assert_eq!(params.get("limit"), Some(&"10".to_string()));
    }

    #[test]
    fn test_recorded_request_query_params_map_none() {
        let mut request = create_test_request();
        request.query_params = None;
        let params = request.query_params_map();
        assert!(params.is_empty());
    }

    #[test]
    fn test_recorded_request_tags_vec() {
        let request = create_test_request();
        let tags = request.tags_vec();
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"api".to_string()));
        assert!(tags.contains(&"users".to_string()));
        assert!(tags.contains(&"test".to_string()));
    }

    #[test]
    fn test_recorded_request_tags_vec_none() {
        let mut request = create_test_request();
        request.tags = None;
        let tags = request.tags_vec();
        assert!(tags.is_empty());
    }

    #[test]
    fn test_recorded_request_decoded_body_utf8() {
        let request = create_test_request();
        let body = request.decoded_body();
        assert!(body.is_some());
        assert_eq!(body.unwrap(), b"hello world".to_vec());
    }

    #[test]
    fn test_recorded_request_decoded_body_base64() {
        let mut request = create_test_request();
        request.body = Some("aGVsbG8gd29ybGQ=".to_string()); // "hello world" in base64
        request.body_encoding = "base64".to_string();
        let body = request.decoded_body();
        assert!(body.is_some());
        assert_eq!(body.unwrap(), b"hello world".to_vec());
    }

    #[test]
    fn test_recorded_request_decoded_body_none() {
        let mut request = create_test_request();
        request.body = None;
        let body = request.decoded_body();
        assert!(body.is_none());
    }

    #[test]
    fn test_recorded_request_serialize() {
        let request = create_test_request();
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test-123"));
        assert!(json.contains("GET"));
        assert!(json.contains("/api/users"));
    }

    #[test]
    fn test_recorded_request_clone() {
        let request = create_test_request();
        let cloned = request.clone();
        assert_eq!(request.id, cloned.id);
        assert_eq!(request.method, cloned.method);
        assert_eq!(request.path, cloned.path);
    }

    // ==================== RecordedResponse Tests ====================

    fn create_test_response() -> RecordedResponse {
        RecordedResponse {
            request_id: "test-123".to_string(),
            status_code: 200,
            headers: r#"{"content-type":"application/json"}"#.to_string(),
            body: Some(r#"{"status":"ok"}"#.to_string()),
            body_encoding: "utf8".to_string(),
            size_bytes: 15,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_recorded_response_headers_map() {
        let response = create_test_response();
        let headers = response.headers_map();
        assert_eq!(headers.get("content-type"), Some(&"application/json".to_string()));
    }

    #[test]
    fn test_recorded_response_headers_map_invalid_json() {
        let mut response = create_test_response();
        response.headers = "invalid".to_string();
        let headers = response.headers_map();
        assert!(headers.is_empty());
    }

    #[test]
    fn test_recorded_response_decoded_body_utf8() {
        let response = create_test_response();
        let body = response.decoded_body();
        assert!(body.is_some());
        assert_eq!(body.unwrap(), br#"{"status":"ok"}"#.to_vec());
    }

    #[test]
    fn test_recorded_response_decoded_body_base64() {
        let mut response = create_test_response();
        response.body = Some("dGVzdCBib2R5".to_string()); // "test body" in base64
        response.body_encoding = "base64".to_string();
        let body = response.decoded_body();
        assert!(body.is_some());
        assert_eq!(body.unwrap(), b"test body".to_vec());
    }

    #[test]
    fn test_recorded_response_decoded_body_none() {
        let mut response = create_test_response();
        response.body = None;
        let body = response.decoded_body();
        assert!(body.is_none());
    }

    #[test]
    fn test_recorded_response_clone() {
        let response = create_test_response();
        let cloned = response.clone();
        assert_eq!(response.request_id, cloned.request_id);
        assert_eq!(response.status_code, cloned.status_code);
    }

    #[test]
    fn test_recorded_response_serialize() {
        let response = create_test_response();
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test-123"));
        assert!(json.contains("200"));
    }

    // ==================== RecordedExchange Tests ====================

    #[test]
    fn test_recorded_exchange_with_response() {
        let exchange = RecordedExchange {
            request: create_test_request(),
            response: Some(create_test_response()),
        };
        assert!(exchange.response.is_some());
        assert_eq!(exchange.request.id, "test-123");
    }

    #[test]
    fn test_recorded_exchange_without_response() {
        let exchange = RecordedExchange {
            request: create_test_request(),
            response: None,
        };
        assert!(exchange.response.is_none());
    }

    #[test]
    fn test_recorded_exchange_serialize() {
        let exchange = RecordedExchange {
            request: create_test_request(),
            response: Some(create_test_response()),
        };
        let json = serde_json::to_string(&exchange).unwrap();
        assert!(json.contains("request"));
        assert!(json.contains("response"));
    }

    #[test]
    fn test_recorded_exchange_clone() {
        let exchange = RecordedExchange {
            request: create_test_request(),
            response: Some(create_test_response()),
        };
        let cloned = exchange.clone();
        assert_eq!(exchange.request.id, cloned.request.id);
        assert!(cloned.response.is_some());
    }
}
