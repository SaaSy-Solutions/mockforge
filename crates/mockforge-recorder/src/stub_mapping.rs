//! Stub mapping conversion from recorded requests/responses
//!
//! Converts recorded API interactions into MockForge fixture format for replay.

use crate::{models::RecordedExchange, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Output format for stub mappings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StubFormat {
    /// YAML format
    Yaml,
    /// JSON format
    Json,
}

/// Stub mapping fixture structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StubMapping {
    /// Unique identifier for this stub
    pub identifier: String,
    /// Human-readable name
    pub name: String,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Request matching criteria
    pub request: RequestMatcher,
    /// Response configuration
    pub response: ResponseTemplate,
    /// Metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

/// Request matching criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMatcher {
    /// HTTP method
    pub method: String,
    /// Path pattern (exact or with template variables)
    pub path: String,
    /// Query parameters (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_params: Option<HashMap<String, String>>,
    /// Headers to match (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    /// Body pattern (optional, for POST/PUT/PATCH)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_pattern: Option<String>,
}

/// Response template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseTemplate {
    /// HTTP status code
    pub status_code: i32,
    /// Response headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    /// Response body (with template variables)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// Content type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

/// Converter for generating stub mappings from recordings
pub struct StubMappingConverter {
    /// Whether to detect and replace dynamic values with templates
    detect_dynamic_values: bool,
    /// UUID pattern for detection
    uuid_pattern: Regex,
    /// Timestamp pattern for detection
    timestamp_pattern: Regex,
}

impl StubMappingConverter {
    /// Create a new converter
    pub fn new(detect_dynamic_values: bool) -> Self {
        Self {
            detect_dynamic_values,
            // UUID v4 pattern
            uuid_pattern: Regex::new(
                r"[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}",
            )
            .unwrap(),
            // RFC3339 timestamp pattern
            timestamp_pattern: Regex::new(
                r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})",
            )
            .unwrap(),
        }
    }

    /// Convert a recorded exchange to a stub mapping
    pub fn convert(&self, exchange: &RecordedExchange) -> Result<StubMapping> {
        let request = &exchange.request;
        let response = exchange.response.as_ref().ok_or_else(|| {
            crate::RecorderError::InvalidFilter("No response found for request".to_string())
        })?;

        // Extract request matcher
        let request_matcher = self.extract_request_matcher(request)?;

        // Extract response template
        let response_template = self.extract_response_template(response)?;

        // Generate identifier from request
        let identifier = self.generate_identifier(request);

        // Generate name
        let name = format!("{} {}", request.method, request.path);

        // Extract description from tags if available
        let description = request.tags_vec().first().map(|s| format!("Recorded from {}", s));

        // Build metadata
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), "recorder".to_string());
        metadata.insert("recorded_at".to_string(), request.timestamp.to_rfc3339());
        if let Some(ref trace_id) = request.trace_id {
            metadata.insert("trace_id".to_string(), trace_id.clone());
        }

        Ok(StubMapping {
            identifier,
            name,
            description,
            request: request_matcher,
            response: response_template,
            metadata: Some(metadata),
        })
    }

    /// Extract request matcher from recorded request
    fn extract_request_matcher(
        &self,
        request: &crate::models::RecordedRequest,
    ) -> Result<RequestMatcher> {
        let mut query_params = None;
        if let Some(ref query_str) = request.query_params {
            if let Ok(params) = serde_json::from_str::<HashMap<String, String>>(query_str) {
                if !params.is_empty() {
                    query_params = Some(params);
                }
            }
        }

        let mut headers = None;
        let headers_map = request.headers_map();
        if !headers_map.is_empty() {
            // Filter out common headers that shouldn't be used for matching
            let filtered: HashMap<String, String> = headers_map
                .into_iter()
                .filter(|(k, _)| {
                    !matches!(
                        k.to_lowercase().as_str(),
                        "host" | "user-agent" | "accept-encoding" | "connection" | "content-length"
                    )
                })
                .collect();
            if !filtered.is_empty() {
                headers = Some(filtered);
            }
        }

        // Extract body pattern if present
        let body_pattern = request
            .decoded_body()
            .and_then(|body| String::from_utf8(body).ok())
            .map(|body_str| {
                if self.detect_dynamic_values {
                    self.replace_dynamic_values(&body_str)
                } else {
                    body_str
                }
            });

        Ok(RequestMatcher {
            method: request.method.clone(),
            path: self.process_path(&request.path),
            query_params,
            headers,
            body_pattern,
        })
    }

    /// Extract response template from recorded response
    fn extract_response_template(
        &self,
        response: &crate::models::RecordedResponse,
    ) -> Result<ResponseTemplate> {
        let headers_map = response.headers_map();
        let content_type = headers_map
            .get("content-type")
            .or_else(|| headers_map.get("Content-Type"))
            .cloned();

        // Filter response headers (exclude common ones)
        let mut response_headers = HashMap::new();
        for (key, value) in &headers_map {
            if !matches!(
                key.to_lowercase().as_str(),
                "content-length" | "date" | "server" | "connection"
            ) {
                response_headers.insert(key.clone(), value.clone());
            }
        }

        let headers = if response_headers.is_empty() {
            None
        } else {
            Some(response_headers)
        };

        // Extract body and process dynamic values
        let body = response.decoded_body().and_then(|body_bytes| {
            String::from_utf8(body_bytes).ok().map(|body_str| {
                if self.detect_dynamic_values {
                    self.replace_dynamic_values(&body_str)
                } else {
                    body_str
                }
            })
        });

        Ok(ResponseTemplate {
            status_code: response.status_code,
            headers,
            body,
            content_type,
        })
    }

    /// Process path to extract dynamic segments
    fn process_path(&self, path: &str) -> String {
        if self.detect_dynamic_values {
            // Replace UUIDs in path with template variable
            let path = self.uuid_pattern.replace_all(path, "{{uuid}}");
            // Replace numeric IDs with template variable (common pattern)
            let numeric_id_pattern = Regex::new(r"/\d+").unwrap();
            let path = numeric_id_pattern.replace_all(&path, "/{{id}}");
            path.to_string()
        } else {
            path.to_string()
        }
    }

    /// Replace dynamic values in text with template variables
    fn replace_dynamic_values(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Replace UUIDs
        result = self.uuid_pattern.replace_all(&result, "{{uuid}}").to_string();

        // Replace timestamps
        result = self.timestamp_pattern.replace_all(&result, "{{now}}").to_string();

        // Try to parse as JSON and replace common dynamic fields
        if let Ok(json_value) = serde_json::from_str::<Value>(&result) {
            if let Some(processed) = self.process_json_value(&json_value) {
                if let Ok(json_str) = serde_json::to_string_pretty(&processed) {
                    return json_str;
                }
            }
        }

        result
    }

    /// Process JSON value to replace dynamic fields
    fn process_json_value(&self, value: &Value) -> Option<Value> {
        match value {
            Value::Object(map) => {
                let mut processed = serde_json::Map::new();
                for (key, val) in map {
                    let processed_val = self.process_json_value(val)?;
                    processed.insert(key.clone(), processed_val);
                }
                Some(Value::Object(processed))
            }
            Value::Array(arr) => {
                let processed: Vec<Value> =
                    arr.iter().filter_map(|v| self.process_json_value(v)).collect();
                Some(Value::Array(processed))
            }
            Value::String(s) => {
                // Check if it's a UUID
                if self.uuid_pattern.is_match(s) {
                    Some(Value::String("{{uuid}}".to_string()))
                }
                // Check if it's a timestamp
                else if self.timestamp_pattern.is_match(s) {
                    Some(Value::String("{{now}}".to_string()))
                }
                // Check if it looks like an ID (numeric string)
                else if s.chars().all(|c| c.is_ascii_digit()) && s.len() > 3 {
                    Some(Value::String("{{id}}".to_string()))
                } else {
                    Some(Value::String(s.clone()))
                }
            }
            _ => Some(value.clone()),
        }
    }

    /// Generate identifier from request
    fn generate_identifier(&self, request: &crate::models::RecordedRequest) -> String {
        // Create a simple identifier from method and path
        let base = format!("{}-{}", request.method.to_lowercase(), request.path);
        base.replace('/', "-")
            .replace(':', "")
            .replace('{', "")
            .replace('}', "")
            .chars()
            .take(50)
            .collect()
    }

    /// Convert stub mapping to YAML string
    pub fn to_yaml(&self, stub: &StubMapping) -> Result<String> {
        serde_yaml::to_string(stub).map_err(|e| {
            crate::RecorderError::InvalidFilter(format!("YAML serialization error: {}", e))
        })
    }

    /// Convert stub mapping to JSON string
    pub fn to_json(&self, stub: &StubMapping) -> Result<String> {
        serde_json::to_string_pretty(stub).map_err(|e| crate::RecorderError::Serialization(e))
    }

    /// Convert stub mapping to string in specified format
    pub fn to_string(&self, stub: &StubMapping, format: StubFormat) -> Result<String> {
        match format {
            StubFormat::Yaml => self.to_yaml(stub),
            StubFormat::Json => self.to_json(stub),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Protocol, RecordedRequest, RecordedResponse};
    use chrono::Utc;

    fn create_test_exchange() -> RecordedExchange {
        let request = RecordedRequest {
            id: "test-123".to_string(),
            protocol: Protocol::Http,
            timestamp: Utc::now(),
            method: "GET".to_string(),
            path: "/api/users/123".to_string(),
            query_params: Some(r#"{"page": "1"}"#.to_string()),
            headers: r#"{"content-type": "application/json", "authorization": "Bearer token123"}"#
                .to_string(),
            body: None,
            body_encoding: "utf8".to_string(),
            client_ip: None,
            trace_id: None,
            span_id: None,
            duration_ms: Some(50),
            status_code: Some(200),
            tags: Some(r#"["api", "users"]"#.to_string()),
        };

        let response = RecordedResponse {
            request_id: "test-123".to_string(),
            status_code: 200,
            headers: r#"{"content-type": "application/json"}"#.to_string(),
            body: Some(
                r#"{"id": "123", "name": "John", "created_at": "2024-01-01T00:00:00Z"}"#
                    .to_string(),
            ),
            body_encoding: "utf8".to_string(),
            size_bytes: 100,
            timestamp: Utc::now(),
        };

        RecordedExchange {
            request,
            response: Some(response),
        }
    }

    #[test]
    fn test_convert_basic() {
        let converter = StubMappingConverter::new(true);
        let exchange = create_test_exchange();
        let stub = converter.convert(&exchange).unwrap();

        assert_eq!(stub.request.method, "GET");
        assert_eq!(stub.request.path, "/api/users/{{id}}");
        assert_eq!(stub.response.status_code, 200);
    }

    #[test]
    fn test_uuid_detection() {
        let converter = StubMappingConverter::new(true);
        let text = "User ID: 550e8400-e29b-41d4-a716-446655440000";
        let result = converter.replace_dynamic_values(text);
        assert!(result.contains("{{uuid}}"));
    }

    #[test]
    fn test_timestamp_detection() {
        let converter = StubMappingConverter::new(true);
        let text = "Created at: 2024-01-01T00:00:00Z";
        let result = converter.replace_dynamic_values(text);
        assert!(result.contains("{{now}}"));
    }
}
