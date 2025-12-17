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
                crate::TunnelError::Io(std::io::Error::other(format!(
                    "Failed to read request body: {}",
                    e
                )))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TunnelConfig;
    use std::sync::Arc;

    fn create_test_manager() -> Arc<TunnelManager> {
        let config = TunnelConfig {
            provider: crate::config::TunnelProvider::SelfHosted,
            server_url: Some("https://tunnel.example.com".to_string()),
            auth_token: Some("test-token".to_string()),
            subdomain: Some("test".to_string()),
            local_url: "http://localhost:3000".to_string(),
            protocol: "http".to_string(),
            region: None,
            custom_domain: None,
            websocket_enabled: true,
            http2_enabled: true,
        };

        Arc::new(TunnelManager::new(&config).unwrap())
    }

    #[test]
    fn test_tunnel_client_new() {
        let manager = create_test_manager();
        let client = TunnelClient::new(manager.clone(), "http://localhost:3000");

        assert_eq!(client.local_url, "http://localhost:3000");
    }

    #[test]
    fn test_tunnel_client_new_with_string() {
        let manager = create_test_manager();
        let url = String::from("http://127.0.0.1:8080");
        let client = TunnelClient::new(manager.clone(), url.clone());

        assert_eq!(client.local_url, url);
    }

    #[test]
    fn test_tunnel_client_new_with_different_urls() {
        let manager = create_test_manager();

        let urls = vec![
            "http://localhost:3000",
            "http://127.0.0.1:8080",
            "http://0.0.0.0:5000",
            "https://internal-api:443",
            "http://[::1]:9000",
        ];

        for url in urls {
            let client = TunnelClient::new(manager.clone(), url);
            assert_eq!(client.local_url, url, "URL mismatch for {}", url);
        }
    }

    #[test]
    fn test_tunnel_client_new_with_https() {
        let manager = create_test_manager();
        let client = TunnelClient::new(manager.clone(), "https://localhost:8443");

        assert_eq!(client.local_url, "https://localhost:8443");
    }

    #[test]
    fn test_tunnel_client_new_with_custom_port() {
        let manager = create_test_manager();
        let client = TunnelClient::new(manager.clone(), "http://localhost:4040");

        assert_eq!(client.local_url, "http://localhost:4040");
    }

    #[test]
    fn test_tunnel_client_local_url_formatting() {
        let manager = create_test_manager();

        // Test various URL formats
        let test_cases = vec![
            ("http://localhost:3000", "http://localhost:3000"),
            ("http://localhost:3000/", "http://localhost:3000/"),
            ("http://api.local:8080", "http://api.local:8080"),
        ];

        for (input, expected) in test_cases {
            let client = TunnelClient::new(manager.clone(), input);
            assert_eq!(client.local_url, expected);
        }
    }

    #[test]
    fn test_tunnel_client_manager_reference() {
        let manager = create_test_manager();
        let manager_clone = manager.clone();

        let _client = TunnelClient::new(manager, "http://localhost:3000");

        // Original manager should still be accessible via clone
        assert!(Arc::strong_count(&manager_clone) >= 1);
    }

    #[test]
    fn test_tunnel_client_creation_multiple() {
        let manager = create_test_manager();

        let client1 = TunnelClient::new(manager.clone(), "http://localhost:3000");
        let client2 = TunnelClient::new(manager.clone(), "http://localhost:4000");

        assert_eq!(client1.local_url, "http://localhost:3000");
        assert_eq!(client2.local_url, "http://localhost:4000");
    }

    #[test]
    fn test_tunnel_client_into_conversion() {
        let manager = create_test_manager();

        // Test that Into<String> trait works for various types
        let url_str = "http://localhost:3000";
        let url_string = String::from("http://localhost:3000");

        let client1 = TunnelClient::new(manager.clone(), url_str);
        let client2 = TunnelClient::new(manager.clone(), url_string);

        assert_eq!(client1.local_url, client2.local_url);
    }

    #[test]
    fn test_tunnel_client_with_ipv6() {
        let manager = create_test_manager();
        let client = TunnelClient::new(manager, "http://[::1]:3000");

        assert_eq!(client.local_url, "http://[::1]:3000");
    }

    #[test]
    fn test_tunnel_client_with_hostname() {
        let manager = create_test_manager();
        let client = TunnelClient::new(manager, "http://backend-service:8080");

        assert_eq!(client.local_url, "http://backend-service:8080");
    }

    #[test]
    fn test_tunnel_client_url_without_port() {
        let manager = create_test_manager();

        // HTTP default port
        let client_http = TunnelClient::new(manager.clone(), "http://localhost");
        assert_eq!(client_http.local_url, "http://localhost");

        // HTTPS default port
        let client_https = TunnelClient::new(manager.clone(), "https://localhost");
        assert_eq!(client_https.local_url, "https://localhost");
    }

    #[test]
    fn test_tunnel_client_empty_url() {
        let manager = create_test_manager();
        let client = TunnelClient::new(manager, "");

        assert_eq!(client.local_url, "");
    }

    #[test]
    fn test_tunnel_client_with_path() {
        let manager = create_test_manager();
        let client = TunnelClient::new(manager, "http://localhost:3000/api");

        // Path should be preserved in local_url
        assert_eq!(client.local_url, "http://localhost:3000/api");
    }

    #[test]
    fn test_tunnel_client_with_query_params() {
        let manager = create_test_manager();
        let client = TunnelClient::new(manager, "http://localhost:3000?debug=true");

        assert_eq!(client.local_url, "http://localhost:3000?debug=true");
    }
}
