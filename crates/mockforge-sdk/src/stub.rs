//! Response stub configuration

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A response stub for mocking API endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseStub {
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: String,
    /// Path pattern (supports {{path_params}})
    pub path: String,
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body (supports templates like {{uuid}}, {{faker.name}})
    pub body: Value,
    /// Optional latency in milliseconds
    pub latency_ms: Option<u64>,
}

impl ResponseStub {
    /// Create a new response stub
    pub fn new(method: impl Into<String>, path: impl Into<String>, body: Value) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            status: 200,
            headers: HashMap::new(),
            body,
            latency_ms: None,
        }
    }

    /// Set the HTTP status code
    pub fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    /// Add a response header
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set response latency in milliseconds
    pub fn latency(mut self, ms: u64) -> Self {
        self.latency_ms = Some(ms);
        self
    }
}

/// Builder for creating response stubs
pub struct StubBuilder {
    method: String,
    path: String,
    status: u16,
    headers: HashMap<String, String>,
    body: Value,
    latency_ms: Option<u64>,
}

impl StubBuilder {
    /// Create a new stub builder
    pub fn new(method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            status: 200,
            headers: HashMap::new(),
            body: Value::Null,
            latency_ms: None,
        }
    }

    /// Set the HTTP status code
    pub fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    /// Add a response header
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set the response body
    pub fn body(mut self, body: Value) -> Self {
        self.body = body;
        self
    }

    /// Set response latency in milliseconds
    pub fn latency(mut self, ms: u64) -> Self {
        self.latency_ms = Some(ms);
        self
    }

    /// Build the response stub
    pub fn build(self) -> ResponseStub {
        ResponseStub {
            method: self.method,
            path: self.path,
            status: self.status,
            headers: self.headers,
            body: self.body,
            latency_ms: self.latency_ms,
        }
    }
}
