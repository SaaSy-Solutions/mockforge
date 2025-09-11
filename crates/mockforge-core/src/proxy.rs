//! Proxy functionality for forwarding requests to upstream services

use crate::{Error, Result};
use axum::http::{HeaderMap, Method, Uri};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Upstream base URL
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
    pub fn should_proxy(&self, path: &str) -> bool {
        if !self.enabled {
            return false;
        }

        if let Some(ref prefix) = self.prefix {
            path.starts_with(prefix)
        } else {
            true
        }
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
        if !self.config.should_proxy(uri.path()) {
            return Err(Error::generic("Request should not be proxied".to_string()));
        }

        // Build upstream URL
        let upstream_path = self.config.strip_prefix(uri.path());
        let mut upstream_url = format!("{}{}", self.config.upstream_url, upstream_path);

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
    use axum::http::HeaderValue;

    #[test]
    fn test_proxy_config() {
        let config = ProxyConfig::new("http://api.example.com".to_string());
        assert!(config.should_proxy("/proxy/users"));
        assert!(!config.should_proxy("/api/users"));

        let stripped = config.strip_prefix("/proxy/users");
        assert_eq!(stripped, "/users");
    }

    #[test]
    fn test_proxy_config_no_prefix() {
        let mut config = ProxyConfig::new("http://api.example.com".to_string());
        config.prefix = None;

        assert!(config.should_proxy("/api/users"));
        assert!(config.should_proxy("/any/path"));

        let stripped = config.strip_prefix("/api/users");
        assert_eq!(stripped, "/api/users");
    }
}
