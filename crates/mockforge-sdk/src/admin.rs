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
    /// Request matching criteria (headers, query params, body patterns)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_match: Option<RequestMatchCriteria>,
    /// Priority for mock ordering (higher priority mocks are matched first)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    /// Scenario name for stateful mocking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
    /// Required scenario state for this mock to be active
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_scenario_state: Option<String>,
    /// New scenario state after this mock is matched
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_scenario_state: Option<String>,
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

/// Request matching criteria for advanced request matching
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestMatchCriteria {
    /// Headers that must be present and match (case-insensitive header names)
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
    /// Query parameters that must be present and match
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub query_params: HashMap<String, String>,
    /// Request body pattern (supports exact match or regex)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_pattern: Option<String>,
    /// JSONPath expression for JSON body matching
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_path: Option<String>,
    /// XPath expression for XML body matching
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xpath: Option<String>,
    /// Custom matcher expression (e.g., "headers.content-type == \"application/json\"")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_matcher: Option<String>,
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
            return Err(Error::General(format!("Failed to get mock: HTTP {}", response.status())));
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
            return Err(Error::General(format!("Failed to get stats: HTTP {}", response.status())));
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

/// Builder for creating mock configurations with fluent API
///
/// This builder provides a WireMock-like fluent API for creating mock configurations
/// with comprehensive request matching and response configuration.
///
/// # Examples
///
/// ```rust
/// use mockforge_sdk::admin::MockConfigBuilder;
/// use serde_json::json;
///
/// // Basic mock
/// let mock = MockConfigBuilder::new("GET", "/api/users")
///     .name("Get Users")
///     .status(200)
///     .body(json!([{"id": 1, "name": "Alice"}]))
///     .build();
///
/// // Advanced matching with headers and query params
/// let mock = MockConfigBuilder::new("POST", "/api/users")
///     .name("Create User")
///     .with_header("Authorization", "Bearer.*")
///     .with_query_param("role", "admin")
///     .with_body_pattern(r#"{"name":".*"}"#)
///     .status(201)
///     .body(json!({"id": 123, "created": true}))
///     .priority(10)
///     .build();
/// ```
pub struct MockConfigBuilder {
    config: MockConfig,
}

impl MockConfigBuilder {
    /// Create a new mock configuration builder
    ///
    /// # Arguments
    /// * `method` - HTTP method (GET, POST, PUT, DELETE, etc.)
    /// * `path` - URL path pattern (supports path parameters like `/users/{id}`)
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
                request_match: None,
                priority: None,
                scenario: None,
                required_scenario_state: None,
                new_scenario_state: None,
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

    /// Set the response body (supports templating with {{variables}})
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

    // ========== Request Matching Methods ==========

    /// Require a specific header to be present and match (supports regex patterns)
    ///
    /// # Examples
    /// ```rust
    /// MockConfigBuilder::new("GET", "/api/users")
    ///     .with_header("Authorization", "Bearer.*")
    ///     .with_header("Content-Type", "application/json")
    /// ```
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let match_criteria =
            self.config.request_match.get_or_insert_with(RequestMatchCriteria::default);
        match_criteria.headers.insert(name.into(), value.into());
        self
    }

    /// Require multiple headers to be present and match
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        let match_criteria =
            self.config.request_match.get_or_insert_with(RequestMatchCriteria::default);
        match_criteria.headers.extend(headers);
        self
    }

    /// Require a specific query parameter to be present and match
    ///
    /// # Examples
    /// ```rust
    /// MockConfigBuilder::new("GET", "/api/users")
    ///     .with_query_param("role", "admin")
    ///     .with_query_param("limit", "10")
    /// ```
    pub fn with_query_param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let match_criteria =
            self.config.request_match.get_or_insert_with(RequestMatchCriteria::default);
        match_criteria.query_params.insert(name.into(), value.into());
        self
    }

    /// Require multiple query parameters to be present and match
    pub fn with_query_params(mut self, params: HashMap<String, String>) -> Self {
        let match_criteria =
            self.config.request_match.get_or_insert_with(RequestMatchCriteria::default);
        match_criteria.query_params.extend(params);
        self
    }

    /// Require the request body to match a pattern (supports exact match or regex)
    ///
    /// # Examples
    /// ```rust
    /// MockConfigBuilder::new("POST", "/api/users")
    ///     .with_body_pattern(r#"{"name":".*"}"#)  // Regex pattern
    ///     .with_body_pattern("exact string match")  // Exact match
    /// ```
    pub fn with_body_pattern(mut self, pattern: impl Into<String>) -> Self {
        let match_criteria =
            self.config.request_match.get_or_insert_with(RequestMatchCriteria::default);
        match_criteria.body_pattern = Some(pattern.into());
        self
    }

    /// Require the request body to match a JSONPath expression
    ///
    /// # Examples
    /// ```rust
    /// MockConfigBuilder::new("POST", "/api/users")
    ///     .with_json_path("$.name")  // Body must have a 'name' field
    ///     .with_json_path("$.age > 18")  // Body must have age > 18
    /// ```
    pub fn with_json_path(mut self, json_path: impl Into<String>) -> Self {
        let match_criteria =
            self.config.request_match.get_or_insert_with(RequestMatchCriteria::default);
        match_criteria.json_path = Some(json_path.into());
        self
    }

    /// Require the request body to match an XPath expression (for XML)
    ///
    /// # Examples
    /// ```rust
    /// MockConfigBuilder::new("POST", "/api/users")
    ///     .with_xpath("/users/user[@id='123']")
    /// ```
    pub fn with_xpath(mut self, xpath: impl Into<String>) -> Self {
        let match_criteria =
            self.config.request_match.get_or_insert_with(RequestMatchCriteria::default);
        match_criteria.xpath = Some(xpath.into());
        self
    }

    /// Set a custom matcher expression for advanced matching logic
    ///
    /// # Examples
    /// ```rust
    /// MockConfigBuilder::new("GET", "/api/users")
    ///     .with_custom_matcher("headers.content-type == \"application/json\"")
    ///     .with_custom_matcher("path =~ \"/api/.*\"")
    /// ```
    pub fn with_custom_matcher(mut self, expression: impl Into<String>) -> Self {
        let match_criteria =
            self.config.request_match.get_or_insert_with(RequestMatchCriteria::default);
        match_criteria.custom_matcher = Some(expression.into());
        self
    }

    // ========== Priority and Scenario Methods ==========

    /// Set the priority for this mock (higher priority mocks are matched first)
    ///
    /// Default priority is 0. Higher numbers = higher priority.
    pub fn priority(mut self, priority: i32) -> Self {
        self.config.priority = Some(priority);
        self
    }

    /// Set the scenario name for stateful mocking
    ///
    /// Scenarios allow you to create stateful mock sequences where the response
    /// depends on previous requests.
    pub fn scenario(mut self, scenario: impl Into<String>) -> Self {
        self.config.scenario = Some(scenario.into());
        self
    }

    /// Require a specific scenario state for this mock to be active
    ///
    /// This mock will only match if the scenario is in the specified state.
    pub fn when_scenario_state(mut self, state: impl Into<String>) -> Self {
        self.config.required_scenario_state = Some(state.into());
        self
    }

    /// Set the new scenario state after this mock is matched
    ///
    /// After this mock responds, the scenario will transition to this state.
    pub fn will_set_scenario_state(mut self, state: impl Into<String>) -> Self {
        self.config.new_scenario_state = Some(state.into());
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
    fn test_mock_config_builder_basic() {
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

    #[test]
    fn test_mock_config_builder_with_matching() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer.*".to_string());

        let mut query_params = HashMap::new();
        query_params.insert("role".to_string(), "admin".to_string());

        let mock = MockConfigBuilder::new("POST", "/api/users")
            .name("Create User")
            .with_headers(headers.clone())
            .with_query_params(query_params.clone())
            .with_body_pattern(r#"{"name":".*"}"#)
            .status(201)
            .body(serde_json::json!({"id": 123, "created": true}))
            .priority(10)
            .build();

        assert_eq!(mock.method, "POST");
        assert!(mock.request_match.is_some());
        let match_criteria = mock.request_match.unwrap();
        assert_eq!(match_criteria.headers.get("Authorization"), Some(&"Bearer.*".to_string()));
        assert_eq!(match_criteria.query_params.get("role"), Some(&"admin".to_string()));
        assert_eq!(match_criteria.body_pattern, Some(r#"{"name":".*"}"#.to_string()));
        assert_eq!(mock.priority, Some(10));
    }

    #[test]
    fn test_mock_config_builder_with_scenario() {
        let mock = MockConfigBuilder::new("GET", "/api/checkout")
            .name("Checkout Step 1")
            .scenario("checkout-flow")
            .when_scenario_state("started")
            .will_set_scenario_state("payment")
            .status(200)
            .body(serde_json::json!({"step": 1}))
            .build();

        assert_eq!(mock.scenario, Some("checkout-flow".to_string()));
        assert_eq!(mock.required_scenario_state, Some("started".to_string()));
        assert_eq!(mock.new_scenario_state, Some("payment".to_string()));
    }

    #[test]
    fn test_mock_config_builder_fluent_chaining() {
        let mock = MockConfigBuilder::new("GET", "/api/users/{id}")
            .id("user-get-123")
            .name("Get User by ID")
            .with_header("Accept", "application/json")
            .with_query_param("include", "profile")
            .with_json_path("$.id")
            .status(200)
            .body(serde_json::json!({"id": "{{request.path.id}}", "name": "Alice"}))
            .header("X-Request-ID", "{{uuid}}")
            .latency_ms(50)
            .priority(5)
            .enabled(true)
            .build();

        assert_eq!(mock.id, "user-get-123");
        assert_eq!(mock.name, "Get User by ID");
        assert!(mock.request_match.is_some());
        let match_criteria = mock.request_match.unwrap();
        assert!(match_criteria.headers.contains_key("Accept"));
        assert!(match_criteria.query_params.contains_key("include"));
        assert_eq!(match_criteria.json_path, Some("$.id".to_string()));
        assert_eq!(mock.priority, Some(5));
    }
}
