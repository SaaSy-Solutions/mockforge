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

    #[test]
    fn test_protocol_display() {
        assert_eq!(Protocol::Http.to_string(), "http");
        assert_eq!(Protocol::Grpc.to_string(), "grpc");
        assert_eq!(Protocol::WebSocket.to_string(), "websocket");
        assert_eq!(Protocol::GraphQL.to_string(), "graphql");
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
}
