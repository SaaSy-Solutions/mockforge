//! Proxy functionality for forwarding requests to upstream services

use crate::{Error, Result};
use axum::http::{HeaderMap, Method, Uri};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Per-route proxy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyRule {
    /// Route pattern (supports wildcards)
    pub pattern: String,
    /// Upstream base URL for this route
    pub upstream_url: String,
    /// Whether this rule is enabled
    pub enabled: bool,
}

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Default upstream base URL
    pub upstream_url: String,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Headers to forward from original request
    pub forward_headers: Vec<String>,
    /// Headers to add to proxied requests
    pub additional_headers: HashMap<String, String>,
    /// Whether to enable proxy mode
    pub enabled: bool,
    /// Proxy prefix (e.g., "/proxy")
    pub prefix: Option<String>,
    /// Per-route proxy rules
    #[serde(default)]
    pub rules: Vec<ProxyRule>,
    /// Passthrough by default unless an override applies
    #[serde(default = "default_passthrough")]
    pub passthrough_by_default: bool,
}

fn default_passthrough() -> bool {
    true
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            upstream_url: "http://localhost:8080".to_string(),
            timeout_seconds: 30,
            forward_headers: vec![
                "authorization".to_string(),
                "content-type".to_string(),
                "user-agent".to_string(),
                "x-request-id".to_string(),
            ],
            additional_headers: HashMap::new(),
            enabled: false,
            prefix: Some("/proxy".to_string()),
            rules: Vec::new(),
            passthrough_by_default: true,
        }
    }
}

impl ProxyConfig {
    /// Create a new proxy configuration
    pub fn new(upstream_url: String) -> Self {
        Self {
            upstream_url,
            ..Default::default()
        }
    }

    /// Check if a request should be proxied
    pub fn should_proxy(&self, _method: &Method, path: &str) -> bool {
        if !self.enabled {
            return false;
        }

        // Check per-route rules first
        for rule in &self.rules {
            if rule.enabled && self.matches_path(&rule.pattern, path) {
                return true;
            }
        }

        // If no specific rule matches, use passthrough behavior
        if self.passthrough_by_default {
            // If we have a prefix, only proxy paths that start with it
            if let Some(ref prefix) = self.prefix {
                return path.starts_with(prefix);
            }
            // Otherwise, proxy all requests
            return true;
        }

        // If passthrough is disabled, only proxy paths with the prefix
        if let Some(ref prefix) = self.prefix {
            path.starts_with(prefix)
        } else {
            false
        }
    }

    /// Get the upstream URL for a specific path
    pub fn get_upstream_url(&self, path: &str) -> String {
        // Check per-route rules first
        for rule in &self.rules {
            if rule.enabled && self.matches_path(&rule.pattern, path) {
                return rule.upstream_url.clone();
            }
        }

        // Return default upstream URL
        self.upstream_url.clone()
    }

    /// Strip the proxy prefix from a path
    pub fn strip_prefix(&self, path: &str) -> String {
        if let Some(ref prefix) = self.prefix {
            if path.starts_with(prefix) {
                path.strip_prefix(prefix).unwrap_or(path).to_string()
            } else {
                path.to_string()
            }
        } else {
            path.to_string()
        }
    }

    /// Check if a path matches a route pattern
    fn matches_path(&self, pattern: &str, path: &str) -> bool {
        if pattern == path {
            return true;
        }

        // Simple wildcard matching (* matches any segment)
        if pattern.contains('*') {
            let pattern_parts: Vec<&str> = pattern.split('/').collect();
            let path_parts: Vec<&str> = path.split('/').collect();

            if pattern_parts.len() != path_parts.len() {
                return false;
            }

            for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
                if *pattern_part != "*" && *pattern_part != *path_part {
                    return false;
                }
            }
            return true;
        }

        false
    }
}

/// Proxy handler for forwarding requests to upstream
pub struct ProxyHandler {
    pub config: ProxyConfig,
    client: Client,
}

impl ProxyHandler {
    /// Create a new proxy handler
    pub fn new(config: ProxyConfig) -> Result<Self> {
        let timeout = Duration::from_secs(config.timeout_seconds);
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| Error::generic(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    /// Proxy a request to the upstream service
    pub async fn proxy_request(
        &self,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: Option<&[u8]>,
    ) -> Result<ProxyResponse> {
        if !self.config.should_proxy(method, uri.path()) {
            return Err(Error::generic("Request should not be proxied".to_string()));
        }

        // Get the upstream URL for this path
        let upstream_base = self.config.get_upstream_url(uri.path());
        
        // Build upstream URL
        let upstream_path = self.config.strip_prefix(uri.path());
        let mut upstream_url = format!("{}{}", upstream_base, upstream_path);

        if let Some(query) = uri.query() {
            upstream_url = format!("{}?{}", upstream_url, query);
        }

        // Build request
        let mut request_builder = self.client
            .request(method.clone(), &upstream_url);

        // Forward headers
        for header_name in &self.config.forward_headers {
            if let Some(header_value) = headers.get(header_name) {
                request_builder = request_builder.header(header_name, header_value);
            }
        }

        // Add additional headers
        for (key, value) in &self.config.additional_headers {
            request_builder = request_builder.header(key, value);
        }

        // Add body if present
        if let Some(body_data) = body {
            request_builder = request_builder.body(body_data.to_vec());
        }

        // Send request
        let response = request_builder
            .send()
            .await
            .map_err(|e| Error::generic(format!("Failed to send proxy request: {}", e)))?;

        let status = response.status();
        let response_headers = response.headers().clone();
        let response_body = response
            .bytes()
            .await
            .map_err(|e| Error::generic(format!("Failed to read response body: {}", e)))?;

        Ok(ProxyResponse {
            status_code: status.as_u16(),
            headers: response_headers,
            body: response_body.to_vec(),
        })
    }
}

/// Proxy response
#[derive(Debug, Clone)]
pub struct ProxyResponse {
    /// Response status code
    pub status_code: u16,
    /// Response headers
    pub headers: HeaderMap,
    /// Response body
    pub body: Vec<u8>,
}


#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Method;

    #[test]
    fn test_proxy_config() {
        let mut config = ProxyConfig::new("http://api.example.com".to_string());
        config.enabled = true;
        assert!(config.should_proxy(&Method::GET, "/proxy/users"));
        assert!(!config.should_proxy(&Method::GET, "/api/users"));

        let stripped = config.strip_prefix("/proxy/users");
        assert_eq!(stripped, "/users");
    }

    #[test]
    fn test_proxy_config_no_prefix() {
        let mut config = ProxyConfig::new("http://api.example.com".to_string());
        config.prefix = None;
        config.enabled = true;

        assert!(config.should_proxy(&Method::GET, "/api/users"));
        assert!(config.should_proxy(&Method::GET, "/any/path"));

        let stripped = config.strip_prefix("/api/users");
        assert_eq!(stripped, "/api/users");
    }

    #[test]
    fn test_proxy_config_with_rules() {
        let mut config = ProxyConfig::new("http://default.example.com".to_string());
        config.enabled = true;
        config.rules.push(ProxyRule {
            pattern: "/api/users/*".to_string(),
            upstream_url: "http://users.example.com".to_string(),
            enabled: true,
        });
        config.rules.push(ProxyRule {
            pattern: "/api/orders/*".to_string(),
            upstream_url: "http://orders.example.com".to_string(),
            enabled: true,
        });

        assert!(config.should_proxy(&Method::GET, "/api/users/123"));
        assert!(config.should_proxy(&Method::GET, "/api/orders/456"));
        
        assert_eq!(config.get_upstream_url("/api/users/123"), "http://users.example.com");
        assert_eq!(config.get_upstream_url("/api/orders/456"), "http://orders.example.com");
        assert_eq!(config.get_upstream_url("/api/products"), "http://default.example.com");
    }

    #[test]
    fn test_proxy_config_passthrough() {
        let mut config = ProxyConfig::new("http://api.example.com".to_string());
        config.passthrough_by_default = true;
        config.prefix = None;
        config.enabled = true;

        // With passthrough enabled, all requests should be proxied
        assert!(config.should_proxy(&Method::GET, "/api/users"));
        assert!(config.should_proxy(&Method::POST, "/api/orders"));

        // Disable passthrough
        config.passthrough_by_default = false;
        config.prefix = Some("/proxy".to_string());

        // Now only requests with the prefix should be proxied
        assert!(config.should_proxy(&Method::GET, "/proxy/users"));
        assert!(!config.should_proxy(&Method::GET, "/api/users"));
    }
}
