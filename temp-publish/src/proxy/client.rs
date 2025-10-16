//! HTTP client for proxy operations

use crate::{Error, Result};
use std::collections::HashMap;

/// HTTP client for making proxy requests
pub struct ProxyClient {
    /// HTTP client instance
    client: reqwest::Client,
}

impl ProxyClient {
    /// Create a new proxy client
    pub fn new() -> Self {
        let client = reqwest::Client::new();
        Self { client }
    }

    /// Send an HTTP request
    pub async fn send_request(
        &self,
        method: reqwest::Method,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&[u8]>,
    ) -> Result<reqwest::Response> {
        let mut request = self.client.request(method, url);

        // Add headers
        for (key, value) in headers {
            request = request.header(key, value);
        }

        // Add body if present
        if let Some(body_data) = body {
            request = request.body(body_data.to_vec());
        }

        request
            .send()
            .await
            .map_err(|e| Error::generic(format!("Proxy request failed: {}", e)))
    }
}

/// Response from a proxy request
pub struct ProxyResponse {
    /// HTTP status code
    pub status_code: u16,
    /// Response headers
    pub headers: std::collections::HashMap<String, String>,
    /// Response body
    pub body: Option<Vec<u8>>,
}

impl Default for ProxyClient {
    fn default() -> Self {
        Self::new()
    }
}
