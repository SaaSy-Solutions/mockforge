//! Testing utilities for plugin development
//!
//! This module provides test harnesses, mock contexts, and utilities
//! for testing plugins in isolation.

use mockforge_plugin_core::*;
use std::collections::HashMap;

/// Test harness for plugin development
///
/// Provides a mock environment for testing plugins without
/// loading them into the actual plugin loader.
///
/// # Example
///
/// ```rust
/// use mockforge_plugin_sdk::testing::TestHarness;
/// use mockforge_plugin_sdk::prelude::*;
///
/// #[tokio::test]
/// async fn test_my_plugin() {
///     let harness = TestHarness::new();
///     let context = harness.create_context("test-plugin", "req-123");
///
///     // Test your plugin here
/// }
/// ```
pub struct TestHarness {
    /// Mock plugin contexts
    contexts: HashMap<String, PluginContext>,
}

impl TestHarness {
    /// Create a new test harness
    pub fn new() -> Self {
        Self {
            contexts: HashMap::new(),
        }
    }

    /// Create a mock plugin context
    pub fn create_context(&mut self, plugin_id: &str, request_id: &str) -> PluginContext {
        let mut context = PluginContext::new(
            PluginId::new(plugin_id),
            PluginVersion::new(0, 1, 0),
        );

        // Override request_id if provided
        context.request_id = request_id.to_string();

        self.contexts.insert(plugin_id.to_string(), context.clone());
        context
    }

    /// Create a context with custom data
    pub fn create_context_with_custom(
        &mut self,
        plugin_id: &str,
        request_id: &str,
        custom_data: HashMap<String, serde_json::Value>,
    ) -> PluginContext {
        let mut context = self.create_context(plugin_id, request_id);
        for (key, value) in custom_data {
            context = context.with_custom(key, value);
        }
        context
    }

    /// Get a context by plugin ID
    pub fn get_context(&self, plugin_id: &str) -> Option<&PluginContext> {
        self.contexts.get(plugin_id)
    }
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock authentication request helpers for testing
pub struct MockAuthRequest;

impl MockAuthRequest {
    /// Create a mock auth request with basic auth header
    pub fn with_basic_auth(username: &str, password: &str) -> AuthRequest {
        use axum::http::{HeaderMap, HeaderValue, Method, Uri};
        use base64::Engine;

        let credentials = format!("{}:{}", username, password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        let auth_value = format!("Basic {}", encoded);

        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_str(&auth_value).unwrap());

        AuthRequest::from_axum(
            Method::GET,
            Uri::from_static("/"),
            headers,
            None,
        )
    }

    /// Create a mock auth request with bearer token
    pub fn with_bearer_token(token: &str) -> AuthRequest {
        use axum::http::{HeaderMap, HeaderValue, Method, Uri};

        let auth_value = format!("Bearer {}", token);

        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_str(&auth_value).unwrap());

        AuthRequest::from_axum(
            Method::GET,
            Uri::from_static("/"),
            headers,
            None,
        )
    }

    /// Create a mock auth request with custom headers
    pub fn with_headers(headers_map: HashMap<String, String>) -> AuthRequest {
        use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, Uri};

        let mut headers = HeaderMap::new();
        for (key, value) in headers_map {
            if let (Ok(header_name), Ok(header_value)) = (
                key.parse::<HeaderName>(),
                HeaderValue::from_str(&value)
            ) {
                headers.insert(header_name, header_value);
            }
        }

        AuthRequest::from_axum(
            Method::GET,
            Uri::from_static("/"),
            headers,
            None,
        )
    }
}

/// Assert that a plugin result is successful
#[macro_export]
macro_rules! assert_plugin_ok {
    ($result:expr) => {
        match $result {
            Ok(_) => (),
            Err(e) => panic!("Plugin returned error: {:?}", e),
        }
    };
    ($result:expr, $msg:expr) => {
        match $result {
            Ok(_) => (),
            Err(e) => panic!("{}: {:?}", $msg, e),
        }
    };
}

/// Assert that a plugin result is an error
#[macro_export]
macro_rules! assert_plugin_err {
    ($result:expr) => {
        match $result {
            Ok(_) => panic!("Expected plugin error, got success"),
            Err(_) => (),
        }
    };
    ($result:expr, $msg:expr) => {
        match $result {
            Ok(_) => panic!("{}: Expected error, got success", $msg),
            Err(_) => (),
        }
    };
}

/// Create a test plugin context
pub fn test_context() -> PluginContext {
    let mut context = PluginContext::new(
        PluginId::new("test-plugin"),
        PluginVersion::new(0, 1, 0),
    );
    context.request_id = "test-request".to_string();
    context
}

/// Create a test plugin context with custom ID
pub fn test_context_with_id(plugin_id: &str) -> PluginContext {
    let mut context = PluginContext::new(
        PluginId::new(plugin_id),
        PluginVersion::new(0, 1, 0),
    );
    context.request_id = "test-request".to_string();
    context
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_creation() {
        let harness = TestHarness::new();
        assert_eq!(harness.contexts.len(), 0);
    }

    #[test]
    fn test_context_creation() {
        let mut harness = TestHarness::new();
        let context = harness.create_context("test", "req-1");
        assert_eq!(context.plugin_id.as_str(), "test");
        assert_eq!(context.request_id, "req-1");
    }

    #[test]
    fn test_mock_auth_request() {
        let request = MockAuthRequest::with_basic_auth("user", "pass");
        let auth_header = request.authorization_header();
        assert!(auth_header.is_some());
        assert!(auth_header.unwrap().starts_with("Basic "));

        let (username, password) = request.basic_credentials().unwrap();
        assert_eq!(username, "user");
        assert_eq!(password, "pass");
    }
}
