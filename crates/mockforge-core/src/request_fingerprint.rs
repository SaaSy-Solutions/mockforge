//! Request fingerprinting system for unique request identification
//! and priority-based response selection.

use axum::http::{HeaderMap, Method, Uri};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Request fingerprint for unique identification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequestFingerprint {
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Query parameters (sorted for consistency)
    pub query: String,
    /// Important headers (sorted for consistency)
    pub headers: HashMap<String, String>,
    /// Request body hash (if available)
    pub body_hash: Option<String>,
}

impl RequestFingerprint {
    /// Create a new request fingerprint
    pub fn new(method: Method, uri: &Uri, headers: &HeaderMap, body: Option<&[u8]>) -> Self {
        let mut query_parts = Vec::new();
        if let Some(query) = uri.query() {
            let mut params: Vec<&str> = query.split('&').collect();
            params.sort(); // Sort for consistency
            query_parts = params;
        }

        // Extract important headers (sorted for consistency)
        let mut important_headers = HashMap::new();
        let important_header_names = [
            "authorization",
            "content-type",
            "accept",
            "user-agent",
            "x-request-id",
            "x-api-key",
            "x-auth-token",
        ];

        for header_name in &important_header_names {
            if let Some(header_value) = headers.get(*header_name) {
                if let Ok(value_str) = header_value.to_str() {
                    important_headers.insert(header_name.to_string(), value_str.to_string());
                }
            }
        }

        // Calculate body hash if body is provided
        let body_hash = body.map(|b| {
            use std::collections::hash_map::DefaultHasher;
            let mut hasher = DefaultHasher::new();
            b.hash(&mut hasher);
            format!("{:x}", hasher.finish())
        });

        Self {
            method: method.to_string(),
            path: uri.path().to_string(),
            query: query_parts.join("&"),
            headers: important_headers,
            body_hash,
        }
    }
}

impl fmt::Display for RequestFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        parts.push(self.method.clone());
        parts.push(self.path.clone());
        parts.push(self.query.clone());

        // Add headers in sorted order
        let mut sorted_headers: Vec<_> = self.headers.iter().collect();
        sorted_headers.sort_by_key(|(k, _)| *k);
        for (key, value) in sorted_headers {
            parts.push(format!("{}:{}", key, value));
        }

        if let Some(ref hash) = self.body_hash {
            parts.push(format!("body:{}", hash));
        }

        write!(f, "{}", parts.join("|"))
    }
}

impl RequestFingerprint {
    /// Generate a short hash of the fingerprint for use as filename
    pub fn to_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        self.method.hash(&mut hasher);
        self.path.hash(&mut hasher);
        self.query.hash(&mut hasher);
        for (k, v) in &self.headers {
            k.hash(&mut hasher);
            v.hash(&mut hasher);
        }
        self.body_hash.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Get tags for the request (extracted from path for now)
    pub fn tags(&self) -> Vec<String> {
        // For now, extract tags from the path
        // In a real implementation, this would come from OpenAPI operation tags
        let mut tags = Vec::new();

        // Extract path segments as potential tags
        for segment in self.path.split('/').filter(|s| !s.is_empty()) {
            if !segment.starts_with('{') && !segment.starts_with(':') {
                tags.push(segment.to_string());
            }
        }

        // Add method as a tag
        tags.push(self.method.to_lowercase());

        tags
    }
}

/// Response priority levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResponsePriority {
    /// Replay from recorded fixtures (highest priority)
    Replay = 0,
    /// Fail injection (second priority)
    Fail = 1,
    /// Proxy to upstream (third priority)
    Proxy = 2,
    /// Mock from OpenAPI spec (fourth priority)
    Mock = 3,
    /// Record request for future replay (lowest priority)
    Record = 4,
}

/// Response source information
#[derive(Debug, Clone)]
pub struct ResponseSource {
    /// Priority level of this response
    pub priority: ResponsePriority,
    /// Source type
    pub source_type: String,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ResponseSource {
    /// Create a new response source
    pub fn new(priority: ResponsePriority, source_type: String) -> Self {
        Self {
            priority,
            source_type,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the response source
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Request handler result
#[derive(Debug, Clone)]
pub enum RequestHandlerResult {
    /// Response was handled (stop processing)
    Handled(ResponseSource),
    /// Continue to next handler
    Continue,
    /// Error occurred
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Uri;

    #[test]
    fn test_request_fingerprint_creation() {
        let method = Method::GET;
        let uri = Uri::from_static("/api/users?page=1&limit=10");
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer token123".parse().unwrap());
        headers.insert("content-type", "application/json".parse().unwrap());

        let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);

        assert_eq!(fingerprint.method, "GET");
        assert_eq!(fingerprint.path, "/api/users");
        assert_eq!(fingerprint.query, "limit=10&page=1"); // Sorted
        assert_eq!(fingerprint.headers.get("authorization"), Some(&"Bearer token123".to_string()));
        assert_eq!(fingerprint.headers.get("content-type"), Some(&"application/json".to_string()));
    }

    #[test]
    fn test_fingerprint_consistency() {
        let method = Method::POST;
        let uri = Uri::from_static("/api/users?b=2&a=1");
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "key123".parse().unwrap());
        headers.insert("authorization", "Bearer token".parse().unwrap());

        let fingerprint1 = RequestFingerprint::new(method.clone(), &uri, &headers, None);
        let fingerprint2 = RequestFingerprint::new(method, &uri, &headers, None);

        assert_eq!(fingerprint1.to_string(), fingerprint2.to_string());
        assert_eq!(fingerprint1.to_hash(), fingerprint2.to_hash());
    }

    #[test]
    fn test_response_priority_ordering() {
        assert!(ResponsePriority::Replay < ResponsePriority::Fail);
        assert!(ResponsePriority::Fail < ResponsePriority::Proxy);
        assert!(ResponsePriority::Proxy < ResponsePriority::Mock);
        assert!(ResponsePriority::Mock < ResponsePriority::Record);
    }
}
