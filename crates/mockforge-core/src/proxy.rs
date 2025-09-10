//! Proxy functionality for forwarding requests to upstream services

use crate::{Error, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

    /// Enable the proxy
    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }

    /// Disable the proxy
    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Add a header to forward
    pub fn forward_header(mut self, header: String) -> Self {
        if !self.forward_headers.contains(&header) {
            self.forward_headers.push(header);
        }
        self
    }

    /// Add an additional header
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.additional_headers.insert(key, value);
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }
}

/// HTTP proxy for forwarding requests
#[derive(Debug)]
pub struct HttpProxy {
    /// HTTP client for making requests
    client: Client,
    /// Proxy configuration
    config: ProxyConfig,
}

impl HttpProxy {
    /// Create a new HTTP proxy
    pub fn new(config: ProxyConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    /// Check if proxy is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Proxy an HTTP request
    pub async fn proxy_request(
        &self,
        method: &str,
        path: &str,
        query: Option<&str>,
        headers: &reqwest::header::HeaderMap,
        body: Option<&[u8]>,
    ) -> Result<reqwest::Response> {
        if !self.is_enabled() {
            return Err(Error::proxy("Proxy is not enabled"));
        }

        // Build the upstream URL
        let mut upstream_url =
            format!("{}{}", self.config.upstream_url.trim_end_matches('/'), path);
        if let Some(query) = query {
            upstream_url.push('?');
            upstream_url.push_str(query);
        }

        // Build the request
        let mut request_builder = self.client.request(
            method.parse().map_err(|_| Error::proxy("Invalid HTTP method"))?,
            &upstream_url,
        );

        // Forward selected headers
        for header_name in &self.config.forward_headers {
            if let Some(value) = headers.get(header_name) {
                request_builder = request_builder.header(header_name, value);
            }
        }

        // Add additional headers
        for (key, value) in &self.config.additional_headers {
            request_builder = request_builder.header(key, value);
        }

        // Add body if present
        if let Some(body) = body {
            request_builder = request_builder.body(body.to_vec());
        }

        // Execute the request
        let response = request_builder
            .send()
            .await
            .map_err(|e| Error::proxy(format!("Failed to proxy request: {}", e)))?;

        Ok(response)
    }

    /// Proxy a simple GET request
    pub async fn proxy_get(&self, path: &str, query: Option<&str>) -> Result<reqwest::Response> {
        self.proxy_request("GET", path, query, &reqwest::header::HeaderMap::new(), None)
            .await
    }

    /// Proxy a POST request with JSON body
    pub async fn proxy_post_json(
        &self,
        path: &str,
        json_body: &serde_json::Value,
    ) -> Result<reqwest::Response> {
        let body = serde_json::to_vec(json_body)
            .map_err(|e| Error::proxy(format!("Failed to serialize JSON: {}", e)))?;

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::CONTENT_TYPE, "application/json".parse().unwrap());

        self.proxy_request("POST", path, None, &headers, Some(&body)).await
    }

    /// Update proxy configuration
    pub fn update_config(&mut self, config: ProxyConfig) -> Result<()> {
        self.config = config;
        // Recreate client with new timeout
        self.client = Client::builder()
            .timeout(std::time::Duration::from_secs(self.config.timeout_seconds))
            .build()
            .map_err(|e| Error::proxy(format!("Failed to update client: {}", e)))?;
        Ok(())
    }
}

impl Default for HttpProxy {
    fn default() -> Self {
        Self::new(ProxyConfig::default())
    }
}

/// Proxy manager for handling different types of proxy operations
#[derive(Debug)]
pub struct ProxyManager {
    /// HTTP proxy
    http_proxy: HttpProxy,
    /// gRPC proxy (placeholder)
    grpc_enabled: bool,
}

impl ProxyManager {
    /// Create a new proxy manager
    pub fn new(http_config: ProxyConfig) -> Self {
        Self {
            http_proxy: HttpProxy::new(http_config),
            grpc_enabled: false,
        }
    }

    /// Get the HTTP proxy
    pub fn http_proxy(&self) -> &HttpProxy {
        &self.http_proxy
    }

    /// Get mutable HTTP proxy
    pub fn http_proxy_mut(&mut self) -> &mut HttpProxy {
        &mut self.http_proxy
    }

    /// Enable gRPC proxying (placeholder)
    pub fn enable_grpc_proxy(&mut self) {
        self.grpc_enabled = true;
        tracing::info!("gRPC proxy enabled (placeholder implementation)");
    }

    /// Check if any proxy is enabled
    pub fn has_active_proxy(&self) -> bool {
        self.http_proxy.is_enabled() || self.grpc_enabled
    }

    /// Update HTTP proxy configuration
    pub fn update_http_config(&mut self, config: ProxyConfig) -> Result<()> {
        self.http_proxy.update_config(config)
    }
}

impl Default for ProxyManager {
    fn default() -> Self {
        Self::new(ProxyConfig::default())
    }
}
