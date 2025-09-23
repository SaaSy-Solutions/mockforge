//! Proxy request handler

use crate::{Result, Error};
use std::collections::HashMap;
use axum::http::{Method, Uri, HeaderMap};
use super::client::ProxyClient;

/// HTTP proxy request handler
pub struct ProxyHandler {
    /// Handler configuration
    pub config: super::config::ProxyConfig,
}

impl ProxyHandler {
    /// Create a new proxy handler
    pub fn new(config: super::config::ProxyConfig) -> Self {
        Self { config }
    }

    /// Handle an HTTP request by proxying it
    pub async fn handle_request(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&[u8]>,
    ) -> Result<ProxyResponse> {
        if !self.config.enabled {
            return Err(Error::generic("Proxy is not enabled"));
        }

        // Parse method
        let reqwest_method = match method.to_uppercase().as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            "HEAD" => reqwest::Method::HEAD,
            "OPTIONS" => reqwest::Method::OPTIONS,
            "PATCH" => reqwest::Method::PATCH,
            _ => return Err(Error::generic(format!("Unsupported HTTP method: {}", method))),
        };

        // Prepare headers (merge with config headers)
        let mut request_headers = headers.clone();
        for (key, value) in &self.config.headers {
            request_headers.insert(key.clone(), value.clone());
        }

        // Create proxy client and send request
        let client = ProxyClient::new();
        let response = client.send_request(reqwest_method, url, &request_headers, body).await?;

        // Convert response back to ProxyResponse
        let mut response_headers = HeaderMap::new();
        for (key, value) in response.headers() {
            if let Ok(header_name) = axum::http::HeaderName::try_from(key.as_str()) {
                response_headers.insert(header_name, value.clone());
            }
        }

        let status_code = response.status().as_u16();
        let body_bytes = response.bytes().await
            .map_err(|e| Error::generic(format!("Failed to read response body: {}", e)))?;

        Ok(ProxyResponse {
            status_code,
            headers: response_headers,
            body: Some(body_bytes.to_vec()),
        })
    }

    /// Proxy a request with full HTTP types
    pub async fn proxy_request(
        &self,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: Option<&[u8]>,
    ) -> Result<ProxyResponse> {
        if !self.config.enabled {
            return Err(Error::generic("Proxy is not enabled"));
        }

        // Check if this request should be proxied
        if !self.config.should_proxy(method, uri.path()) {
            return Err(Error::generic("Request should not be proxied"));
        }

        // Determine the upstream URL
        let upstream_url = self.config.get_upstream_url(uri.path());

        // Convert headers from HeaderMap to HashMap
        let mut header_map = HashMap::new();
        for (key, value) in headers {
            if let Ok(value_str) = value.to_str() {
                header_map.insert(key.to_string(), value_str.to_string());
            }
        }

        // Add any configured headers
        for (key, value) in &self.config.headers {
            header_map.insert(key.clone(), value.clone());
        }

        // Convert method to reqwest method
        let reqwest_method = match *method {
            Method::GET => reqwest::Method::GET,
            Method::POST => reqwest::Method::POST,
            Method::PUT => reqwest::Method::PUT,
            Method::DELETE => reqwest::Method::DELETE,
            Method::HEAD => reqwest::Method::HEAD,
            Method::OPTIONS => reqwest::Method::OPTIONS,
            Method::PATCH => reqwest::Method::PATCH,
            _ => return Err(Error::generic(format!("Unsupported HTTP method: {}", method))),
        };

        // Create proxy client and send request
        let client = ProxyClient::new();
        let response = client.send_request(reqwest_method, &upstream_url, &header_map, body).await?;

        // Convert response back to ProxyResponse
        let mut response_headers = HeaderMap::new();
        for (key, value) in response.headers() {
            if let Ok(header_name) = axum::http::HeaderName::try_from(key.as_str()) {
                response_headers.insert(header_name, value.clone());
            }
        }

        let status_code = response.status().as_u16();
        let body_bytes = response.bytes().await
            .map_err(|e| Error::generic(format!("Failed to read response body: {}", e)))?;

        Ok(ProxyResponse {
            status_code,
            headers: response_headers,
            body: Some(body_bytes.to_vec()),
        })
    }
}

/// Response from a proxy request
pub struct ProxyResponse {
    /// HTTP status code
    pub status_code: u16,
    /// Response headers
    pub headers: HeaderMap,
    /// Response body
    pub body: Option<Vec<u8>>,
}