//! Tunnel client for forwarding requests

use crate::{manager::TunnelManager, Result};
use axum::{body::Body, extract::Request, response::Response};
use std::sync::Arc;

/// Tunnel client for forwarding HTTP requests
pub struct TunnelClient {
    #[allow(dead_code)] // Reserved for future use
    manager: Arc<TunnelManager>,
    local_url: String,
}

impl TunnelClient {
    /// Create a new tunnel client
    pub fn new(manager: Arc<TunnelManager>, local_url: impl Into<String>) -> Self {
        Self {
            manager,
            local_url: local_url.into(),
        }
    }

    /// Forward an HTTP request to the local server
    pub async fn forward_request(&self, request: Request) -> Result<Response> {
        let uri = request.uri().clone();
        let method = request.method().clone();
        let headers = request.headers().clone();

        // Build the local URL
        let local_uri = format!(
            "{}{}",
            self.local_url,
            uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("")
        );

        // Create new request to local server
        let mut local_request = reqwest::Client::new().request(method, &local_uri);

        // Copy headers (excluding hop-by-hop headers)
        for (name, value) in headers.iter() {
            let name_str = name.as_str();
            if !matches!(
                name_str,
                "connection"
                    | "keep-alive"
                    | "proxy-authenticate"
                    | "proxy-authorization"
                    | "te"
                    | "trailer"
                    | "transfer-encoding"
                    | "upgrade"
            ) {
                if let Ok(value_str) = value.to_str() {
                    local_request = local_request.header(name_str, value_str);
                }
            }
        }

        // Set Host header to local
        if let Ok(host) = url::Url::parse(&self.local_url) {
            if let Some(host_str) = host.host_str() {
                local_request = local_request.header("Host", host_str);
            }
        }

        // Forward request body if present
        let body_bytes =
            axum::body::to_bytes(request.into_body(), usize::MAX).await.map_err(|e| {
                crate::TunnelError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to read request body: {}", e),
                ))
            })?;

        if !body_bytes.is_empty() {
            local_request = local_request.body(body_bytes.to_vec());
        }

        // Send request to local server
        let response = local_request
            .send()
            .await
            .map_err(|e| crate::TunnelError::ConnectionFailed(e.to_string()))?;

        // Build response
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.bytes().await.map_err(|e| {
            crate::TunnelError::ConnectionFailed(format!("Failed to read response: {}", e))
        })?;

        let mut response_builder = Response::builder().status(status);

        // Copy response headers
        for (name, value) in headers.iter() {
            if let Ok(value_str) = value.to_str() {
                response_builder = response_builder.header(name.as_str(), value_str);
            }
        }

        response_builder.body(Body::from(body.to_vec())).map_err(|e| {
            crate::TunnelError::ConnectionFailed(format!("Failed to build response: {}", e))
        })
    }
}
