//! Admin API client for runtime mock management
//!
//! Provides programmatic access to MockForge's management API for
//! creating, updating, and managing mocks at runtime.

use crate::{Error, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Admin API client for managing mocks
pub struct AdminClient {
    base_url: String,
    client: Client,
}

/// Mock configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockConfig {
    /// Unique identifier for the mock (auto-generated if empty)
    #[serde(skip_serializing_if = "String::is_empty")]
    pub id: String,
    /// Human-readable name for the mock
    pub name: String,
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: String,
    /// URL path pattern (supports path parameters)
    pub path: String,
    /// Response configuration
    pub response: MockResponse,
    /// Whether this mock is currently active
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Optional latency to simulate in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// HTTP status code to return (default: 200)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
}

fn default_true() -> bool {
    true
}

/// Mock response configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockResponse {
    /// Response body (supports JSON values and templates)
    pub body: serde_json::Value,
    /// Optional HTTP headers to include in the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

/// Server statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStats {
    /// Server uptime in seconds
    pub uptime_seconds: u64,
    /// Total number of requests served
    pub total_requests: u64,
    /// Number of registered mocks (active and inactive)
    pub active_mocks: usize,
    /// Number of currently enabled mocks
    pub enabled_mocks: usize,
    /// Total number of registered routes
    pub registered_routes: usize,
}

/// Server configuration info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// MockForge version
    pub version: String,
    /// HTTP port the server is running on
    pub port: u16,
    /// Whether an OpenAPI spec is loaded
    pub has_openapi_spec: bool,
    /// Path to the OpenAPI spec file (if loaded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_path: Option<String>,
}

/// List of mocks with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockList {
    /// List of mock configurations
    pub mocks: Vec<MockConfig>,
    /// Total number of mocks
    pub total: usize,
    /// Number of enabled mocks
    pub enabled: usize,
}

impl AdminClient {
    /// Create a new admin client
    ///
    /// The base URL should be the root URL of the MockForge server
    /// (e.g., "http://localhost:3000"). Trailing slashes are automatically removed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mockforge_sdk::AdminClient;
    ///
    /// let client = AdminClient::new("http://localhost:3000");
    /// // Also works with trailing slash:
    /// let client = AdminClient::new("http://localhost:3000/");
    /// ```
    pub fn new(base_url: impl Into<String>) -> Self {
        let mut url = base_url.into();

        // Normalize URL: remove trailing slashes
        while url.ends_with('/') {
            url.pop();
        }

        Self {
            base_url: url,
            client: Client::new(),
        }
    }

    /// List all mocks
    pub async fn list_mocks(&self) -> Result<MockList> {
        let url = format!("{}/api/mocks", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::General(format!("Failed to list mocks: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::General(format!(
                "Failed to list mocks: HTTP {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::General(format!("Failed to parse response: {}", e)))
    }

    /// Get a specific mock by ID
    pub async fn get_mock(&self, id: &str) -> Result<MockConfig> {
        let url = format!("{}/api/mocks/{}", self.base_url, id);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::General(format!("Failed to get mock: {}", e)))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::General(format!("Mock not found: {}", id)));
        }

        if !response.status().is_success() {
            return Err(Error::General(format!(
                "Failed to get mock: HTTP {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::General(format!("Failed to parse response: {}", e)))
    }

    /// Create a new mock
    pub async fn create_mock(&self, mock: MockConfig) -> Result<MockConfig> {
        let url = format!("{}/api/mocks", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&mock)
            .send()
            .await
            .map_err(|e| Error::General(format!("Failed to create mock: {}", e)))?;

        if response.status() == reqwest::StatusCode::CONFLICT {
            return Err(Error::General(format!("Mock with ID {} already exists", mock.id)));
        }

        if !response.status().is_success() {
            return Err(Error::General(format!(
                "Failed to create mock: HTTP {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::General(format!("Failed to parse response: {}", e)))
    }

    /// Update an existing mock
    pub async fn update_mock(&self, id: &str, mock: MockConfig) -> Result<MockConfig> {
        let url = format!("{}/api/mocks/{}", self.base_url, id);
        let response = self
            .client
            .put(&url)
            .json(&mock)
            .send()
            .await
            .map_err(|e| Error::General(format!("Failed to update mock: {}", e)))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::General(format!("Mock not found: {}", id)));
        }

        if !response.status().is_success() {
            return Err(Error::General(format!(
                "Failed to update mock: HTTP {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::General(format!("Failed to parse response: {}", e)))
    }

    /// Delete a mock
    pub async fn delete_mock(&self, id: &str) -> Result<()> {
        let url = format!("{}/api/mocks/{}", self.base_url, id);
        let response = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| Error::General(format!("Failed to delete mock: {}", e)))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::General(format!("Mock not found: {}", id)));
        }

        if !response.status().is_success() {
            return Err(Error::General(format!(
                "Failed to delete mock: HTTP {}",
                response.status()
            )));
        }

        Ok(())
    }

    /// Get server statistics
    pub async fn get_stats(&self) -> Result<ServerStats> {
        let url = format!("{}/api/stats", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::General(format!("Failed to get stats: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::General(format!(
                "Failed to get stats: HTTP {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::General(format!("Failed to parse response: {}", e)))
    }

    /// Get server configuration
    pub async fn get_config(&self) -> Result<ServerConfig> {
        let url = format!("{}/api/config", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::General(format!("Failed to get config: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::General(format!(
                "Failed to get config: HTTP {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::General(format!("Failed to parse response: {}", e)))
    }

    /// Reset all mocks to initial state
    pub async fn reset(&self) -> Result<()> {
        let url = format!("{}/api/reset", self.base_url);
        let response = self
            .client
            .post(&url)
            .send()
            .await
            .map_err(|e| Error::General(format!("Failed to reset mocks: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::General(format!(
                "Failed to reset mocks: HTTP {}",
                response.status()
            )));
        }

        Ok(())
    }
}

/// Builder for creating mock configurations
pub struct MockConfigBuilder {
    config: MockConfig,
}

impl MockConfigBuilder {
    /// Create a new mock configuration builder
    pub fn new(method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            config: MockConfig {
                id: String::new(),
                name: String::new(),
                method: method.into().to_uppercase(),
                path: path.into(),
                response: MockResponse {
                    body: serde_json::json!({}),
                    headers: None,
                },
                enabled: true,
                latency_ms: None,
                status_code: None,
            },
        }
    }

    /// Set the mock ID
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.config.id = id.into();
        self
    }

    /// Set the mock name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.config.name = name.into();
        self
    }

    /// Set the response body
    pub fn body(mut self, body: serde_json::Value) -> Self {
        self.config.response.body = body;
        self
    }

    /// Set the response status code
    pub fn status(mut self, status: u16) -> Self {
        self.config.status_code = Some(status);
        self
    }

    /// Set response headers
    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.config.response.headers = Some(headers);
        self
    }

    /// Add a single response header
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let headers = self.config.response.headers.get_or_insert_with(HashMap::new);
        headers.insert(key.into(), value.into());
        self
    }

    /// Set the latency in milliseconds
    pub fn latency_ms(mut self, ms: u64) -> Self {
        self.config.latency_ms = Some(ms);
        self
    }

    /// Enable or disable the mock
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }

    /// Build the mock configuration
    pub fn build(self) -> MockConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_config_builder() {
        let mock = MockConfigBuilder::new("GET", "/api/users")
            .name("Get Users")
            .status(200)
            .body(serde_json::json!([{"id": 1, "name": "Alice"}]))
            .latency_ms(100)
            .header("Content-Type", "application/json")
            .build();

        assert_eq!(mock.method, "GET");
        assert_eq!(mock.path, "/api/users");
        assert_eq!(mock.name, "Get Users");
        assert_eq!(mock.status_code, Some(200));
        assert_eq!(mock.latency_ms, Some(100));
        assert!(mock.enabled);
    }
}
