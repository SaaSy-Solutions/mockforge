//! Tunnel provider traits and implementations

use crate::{Result, TunnelConfig};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Tunnel status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelStatus {
    /// Public URL of the tunnel
    pub public_url: String,

    /// Tunnel ID
    pub tunnel_id: String,

    /// Whether the tunnel is active
    pub active: bool,

    /// Request count
    pub request_count: u64,

    /// Bytes transferred
    pub bytes_transferred: u64,

    /// Created timestamp
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Expires at (if applicable)
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Local URL (for testing/info purposes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_url: Option<String>,
}

/// Tunnel provider trait
#[async_trait]
pub trait TunnelProvider: Send + Sync {
    /// Create a new tunnel
    async fn create_tunnel(&self, config: &TunnelConfig) -> Result<TunnelStatus>;

    /// Get tunnel status
    async fn get_tunnel_status(&self, tunnel_id: &str) -> Result<TunnelStatus>;

    /// Delete/stop a tunnel
    async fn delete_tunnel(&self, tunnel_id: &str) -> Result<()>;

    /// List all active tunnels
    async fn list_tunnels(&self) -> Result<Vec<TunnelStatus>>;

    /// Check if provider is available
    async fn is_available(&self) -> bool;
}

/// Self-hosted tunnel provider
pub struct SelfHostedProvider {
    server_url: String,
    auth_token: Option<String>,
    client: reqwest::Client,
}

impl SelfHostedProvider {
    /// Create a new self-hosted provider
    pub fn new(server_url: impl Into<String>, auth_token: Option<String>) -> Self {
        Self {
            server_url: server_url.into(),
            auth_token,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl TunnelProvider for SelfHostedProvider {
    async fn create_tunnel(&self, config: &TunnelConfig) -> Result<TunnelStatus> {
        let url = format!("{}/api/tunnels", self.server_url);
        let mut request = self.client.post(&url);

        // Add auth header if token is provided
        if let Some(token) = &self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let payload = serde_json::json!({
            "local_url": config.local_url,
            "subdomain": config.subdomain,
            "custom_domain": config.custom_domain,
            "protocol": config.protocol,
            "websocket_enabled": config.websocket_enabled,
            "http2_enabled": config.http2_enabled,
        });

        let response = request
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::TunnelError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::TunnelError::ProviderError(format!(
                "Failed to create tunnel: {}",
                error_text
            )));
        }

        let status: TunnelStatus = response.json().await?;
        Ok(status)
    }

    async fn get_tunnel_status(&self, tunnel_id: &str) -> Result<TunnelStatus> {
        let url = format!("{}/api/tunnels/{}", self.server_url, tunnel_id);
        let mut request = self.client.get(&url);

        if let Some(token) = &self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| crate::TunnelError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(crate::TunnelError::NotFound(tunnel_id.to_string()));
        }

        let status: TunnelStatus = response.json().await?;
        Ok(status)
    }

    async fn delete_tunnel(&self, tunnel_id: &str) -> Result<()> {
        let url = format!("{}/api/tunnels/{}", self.server_url, tunnel_id);
        let mut request = self.client.delete(&url);

        if let Some(token) = &self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| crate::TunnelError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(crate::TunnelError::ProviderError(format!(
                "Failed to delete tunnel: {}",
                response.status()
            )));
        }

        Ok(())
    }

    async fn list_tunnels(&self) -> Result<Vec<TunnelStatus>> {
        let url = format!("{}/api/tunnels", self.server_url);
        let mut request = self.client.get(&url);

        if let Some(token) = &self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| crate::TunnelError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(crate::TunnelError::ProviderError(format!(
                "Failed to list tunnels: {}",
                response.status()
            )));
        }

        let tunnels: Vec<TunnelStatus> = response.json().await?;
        Ok(tunnels)
    }

    async fn is_available(&self) -> bool {
        let url = format!("{}/health", self.server_url);
        self.client.get(&url).timeout(Duration::from_secs(5)).send().await.is_ok()
    }
}

/// ngrok tunnel provider using the ngrok REST API
pub struct NgrokProvider {
    api_key: String,
    client: reqwest::Client,
}

impl NgrokProvider {
    /// Create a new ngrok provider
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            client: reqwest::Client::new(),
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }
}

#[async_trait]
impl TunnelProvider for NgrokProvider {
    async fn create_tunnel(&self, config: &TunnelConfig) -> Result<TunnelStatus> {
        let payload = serde_json::json!({
            "proto": "http",
            "addr": config.local_url,
            "subdomain": config.subdomain,
        });

        let response = self
            .client
            .post("https://api.ngrok.com/tunnels")
            .header("Authorization", self.auth_header())
            .header("Ngrok-Version", "2")
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::TunnelError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::TunnelError::ProviderError(format!(
                "Failed to create ngrok tunnel: {}",
                error_text
            )));
        }

        let body: serde_json::Value = response.json().await?;
        Ok(TunnelStatus {
            tunnel_id: body["id"].as_str().unwrap_or_default().to_string(),
            public_url: body["public_url"].as_str().unwrap_or_default().to_string(),
            active: true,
            request_count: 0,
            bytes_transferred: 0,
            created_at: Some(chrono::Utc::now()),
            expires_at: None,
            local_url: Some(config.local_url.clone()),
        })
    }

    async fn get_tunnel_status(&self, tunnel_id: &str) -> Result<TunnelStatus> {
        let url = format!("https://api.ngrok.com/tunnels/{}", tunnel_id);
        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .header("Ngrok-Version", "2")
            .send()
            .await
            .map_err(|e| crate::TunnelError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(crate::TunnelError::NotFound(tunnel_id.to_string()));
        }

        let body: serde_json::Value = response.json().await?;
        Ok(TunnelStatus {
            tunnel_id: body["id"].as_str().unwrap_or_default().to_string(),
            public_url: body["public_url"].as_str().unwrap_or_default().to_string(),
            active: true,
            request_count: 0,
            bytes_transferred: 0,
            created_at: None,
            expires_at: None,
            local_url: None,
        })
    }

    async fn delete_tunnel(&self, tunnel_id: &str) -> Result<()> {
        let url = format!("https://api.ngrok.com/tunnels/{}", tunnel_id);
        let response = self
            .client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .header("Ngrok-Version", "2")
            .send()
            .await
            .map_err(|e| crate::TunnelError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(crate::TunnelError::ProviderError(format!(
                "Failed to delete ngrok tunnel: {}",
                response.status()
            )));
        }

        Ok(())
    }

    async fn list_tunnels(&self) -> Result<Vec<TunnelStatus>> {
        let response = self
            .client
            .get("https://api.ngrok.com/tunnels")
            .header("Authorization", self.auth_header())
            .header("Ngrok-Version", "2")
            .send()
            .await
            .map_err(|e| crate::TunnelError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(crate::TunnelError::ProviderError(format!(
                "Failed to list ngrok tunnels: {}",
                response.status()
            )));
        }

        let body: serde_json::Value = response.json().await?;
        let tunnels = body["tunnels"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|t| TunnelStatus {
                tunnel_id: t["id"].as_str().unwrap_or_default().to_string(),
                public_url: t["public_url"].as_str().unwrap_or_default().to_string(),
                active: true,
                request_count: 0,
                bytes_transferred: 0,
                created_at: None,
                expires_at: None,
                local_url: None,
            })
            .collect();

        Ok(tunnels)
    }

    async fn is_available(&self) -> bool {
        self.client
            .get("https://api.ngrok.com/tunnels")
            .header("Authorization", self.auth_header())
            .header("Ngrok-Version", "2")
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map(|r| r.status().is_success() || r.status().as_u16() == 401)
            .unwrap_or(false)
    }
}

/// Cloudflare Tunnel provider using the Cloudflare API
pub struct CloudflareProvider {
    api_token: String,
    account_id: String,
    client: reqwest::Client,
}

impl CloudflareProvider {
    /// Create a new Cloudflare tunnel provider
    pub fn new(api_token: impl Into<String>, account_id: impl Into<String>) -> Self {
        Self {
            api_token: api_token.into(),
            account_id: account_id.into(),
            client: reqwest::Client::new(),
        }
    }

    fn base_url(&self) -> String {
        format!("https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel", self.account_id)
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_token)
    }
}

#[async_trait]
impl TunnelProvider for CloudflareProvider {
    async fn create_tunnel(&self, config: &TunnelConfig) -> Result<TunnelStatus> {
        let tunnel_name = config
            .subdomain
            .clone()
            .unwrap_or_else(|| format!("mockforge-{}", uuid::Uuid::new_v4()));

        let payload = serde_json::json!({
            "name": tunnel_name,
            "tunnel_secret": base64_secret(),
        });

        let response = self
            .client
            .post(self.base_url())
            .header("Authorization", self.auth_header())
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::TunnelError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::TunnelError::ProviderError(format!(
                "Failed to create Cloudflare tunnel: {}",
                error_text
            )));
        }

        let body: serde_json::Value = response.json().await?;
        let result = &body["result"];
        let tunnel_id = result["id"].as_str().unwrap_or_default().to_string();

        Ok(TunnelStatus {
            tunnel_id: tunnel_id.clone(),
            public_url: format!("https://{}.cfargotunnel.com", tunnel_id),
            active: true,
            request_count: 0,
            bytes_transferred: 0,
            created_at: Some(chrono::Utc::now()),
            expires_at: None,
            local_url: Some(config.local_url.clone()),
        })
    }

    async fn get_tunnel_status(&self, tunnel_id: &str) -> Result<TunnelStatus> {
        let url = format!("{}/{}", self.base_url(), tunnel_id);
        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| crate::TunnelError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(crate::TunnelError::NotFound(tunnel_id.to_string()));
        }

        let body: serde_json::Value = response.json().await?;
        let result = &body["result"];
        let status_str = result["status"].as_str().unwrap_or("unknown");

        Ok(TunnelStatus {
            tunnel_id: result["id"].as_str().unwrap_or_default().to_string(),
            public_url: format!(
                "https://{}.cfargotunnel.com",
                result["id"].as_str().unwrap_or_default()
            ),
            active: status_str == "active" || status_str == "healthy",
            request_count: 0,
            bytes_transferred: 0,
            created_at: None,
            expires_at: None,
            local_url: None,
        })
    }

    async fn delete_tunnel(&self, tunnel_id: &str) -> Result<()> {
        let url = format!("{}/{}", self.base_url(), tunnel_id);
        let response = self
            .client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| crate::TunnelError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(crate::TunnelError::ProviderError(format!(
                "Failed to delete Cloudflare tunnel: {}",
                response.status()
            )));
        }

        Ok(())
    }

    async fn list_tunnels(&self) -> Result<Vec<TunnelStatus>> {
        let response = self
            .client
            .get(self.base_url())
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| crate::TunnelError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(crate::TunnelError::ProviderError(format!(
                "Failed to list Cloudflare tunnels: {}",
                response.status()
            )));
        }

        let body: serde_json::Value = response.json().await?;
        let tunnels = body["result"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|t| {
                let tid = t["id"].as_str().unwrap_or_default().to_string();
                let status_str = t["status"].as_str().unwrap_or("unknown");
                TunnelStatus {
                    tunnel_id: tid.clone(),
                    public_url: format!("https://{}.cfargotunnel.com", tid),
                    active: status_str == "active" || status_str == "healthy",
                    request_count: 0,
                    bytes_transferred: 0,
                    created_at: None,
                    expires_at: None,
                    local_url: None,
                }
            })
            .collect();

        Ok(tunnels)
    }

    async fn is_available(&self) -> bool {
        self.client
            .get(self.base_url())
            .header("Authorization", self.auth_header())
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map(|r| r.status().is_success() || r.status().as_u16() == 401)
            .unwrap_or(false)
    }
}

/// Generate a random secret for Cloudflare tunnel (hex-encoded)
fn base64_secret() -> String {
    // Use two UUIDs concatenated for 256 bits of randomness
    let id1 = uuid::Uuid::new_v4();
    let id2 = uuid::Uuid::new_v4();
    format!("{}{}", id1.as_simple(), id2.as_simple())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_status() -> TunnelStatus {
        TunnelStatus {
            public_url: "https://test.tunnel.dev".to_string(),
            tunnel_id: "tunnel-123".to_string(),
            active: true,
            request_count: 100,
            bytes_transferred: 5000,
            created_at: Some(Utc::now()),
            expires_at: None,
            local_url: Some("http://localhost:3000".to_string()),
        }
    }

    #[test]
    fn test_tunnel_status_clone() {
        let status = create_test_status();
        let cloned = status.clone();
        assert_eq!(status.public_url, cloned.public_url);
        assert_eq!(status.tunnel_id, cloned.tunnel_id);
        assert_eq!(status.active, cloned.active);
        assert_eq!(status.request_count, cloned.request_count);
        assert_eq!(status.bytes_transferred, cloned.bytes_transferred);
    }

    #[test]
    fn test_tunnel_status_debug() {
        let status = create_test_status();
        let debug = format!("{:?}", status);
        assert!(debug.contains("TunnelStatus"));
        assert!(debug.contains("tunnel-123"));
    }

    #[test]
    fn test_tunnel_status_serialize() {
        let status = create_test_status();
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"public_url\":\"https://test.tunnel.dev\""));
        assert!(json.contains("\"tunnel_id\":\"tunnel-123\""));
        assert!(json.contains("\"active\":true"));
        assert!(json.contains("\"request_count\":100"));
        assert!(json.contains("\"bytes_transferred\":5000"));
    }

    #[test]
    fn test_tunnel_status_deserialize() {
        let json = r#"{
            "public_url": "https://example.tunnel.dev",
            "tunnel_id": "tun-456",
            "active": false,
            "request_count": 50,
            "bytes_transferred": 2500,
            "created_at": null,
            "expires_at": null
        }"#;

        let status: TunnelStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status.public_url, "https://example.tunnel.dev");
        assert_eq!(status.tunnel_id, "tun-456");
        assert!(!status.active);
        assert_eq!(status.request_count, 50);
        assert_eq!(status.bytes_transferred, 2500);
        assert!(status.created_at.is_none());
        assert!(status.expires_at.is_none());
        assert!(status.local_url.is_none());
    }

    #[test]
    fn test_tunnel_status_serialize_skip_none_local_url() {
        let status = TunnelStatus {
            public_url: "https://test.tunnel.dev".to_string(),
            tunnel_id: "tunnel-123".to_string(),
            active: true,
            request_count: 0,
            bytes_transferred: 0,
            created_at: None,
            expires_at: None,
            local_url: None,
        };
        let json = serde_json::to_string(&status).unwrap();
        // local_url should be skipped when None
        assert!(!json.contains("local_url"));
    }

    #[test]
    fn test_tunnel_status_with_local_url() {
        let status = TunnelStatus {
            public_url: "https://test.tunnel.dev".to_string(),
            tunnel_id: "tunnel-123".to_string(),
            active: true,
            request_count: 0,
            bytes_transferred: 0,
            created_at: None,
            expires_at: None,
            local_url: Some("http://localhost:8080".to_string()),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"local_url\":\"http://localhost:8080\""));
    }

    #[test]
    fn test_tunnel_status_with_timestamps() {
        let now = Utc::now();
        let expires = now + chrono::Duration::hours(24);

        let status = TunnelStatus {
            public_url: "https://test.tunnel.dev".to_string(),
            tunnel_id: "tunnel-123".to_string(),
            active: true,
            request_count: 0,
            bytes_transferred: 0,
            created_at: Some(now),
            expires_at: Some(expires),
            local_url: None,
        };

        assert!(status.created_at.is_some());
        assert!(status.expires_at.is_some());
        assert!(status.expires_at.unwrap() > status.created_at.unwrap());
    }

    #[test]
    fn test_self_hosted_provider_new() {
        let provider = SelfHostedProvider::new("https://tunnel.example.com", None);
        assert_eq!(provider.server_url, "https://tunnel.example.com");
        assert!(provider.auth_token.is_none());
    }

    #[test]
    fn test_self_hosted_provider_new_with_token() {
        let provider = SelfHostedProvider::new(
            "https://tunnel.example.com",
            Some("my-secret-token".to_string()),
        );
        assert_eq!(provider.server_url, "https://tunnel.example.com");
        assert_eq!(provider.auth_token, Some("my-secret-token".to_string()));
    }

    #[test]
    fn test_self_hosted_provider_new_with_string_conversion() {
        let provider = SelfHostedProvider::new(String::from("https://api.tunnel.dev"), None);
        assert_eq!(provider.server_url, "https://api.tunnel.dev");
    }

    #[test]
    fn test_tunnel_status_roundtrip_serialization() {
        let status = create_test_status();
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: TunnelStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(status.public_url, deserialized.public_url);
        assert_eq!(status.tunnel_id, deserialized.tunnel_id);
        assert_eq!(status.active, deserialized.active);
        assert_eq!(status.request_count, deserialized.request_count);
        assert_eq!(status.bytes_transferred, deserialized.bytes_transferred);
        assert_eq!(status.local_url, deserialized.local_url);
    }

    #[test]
    fn test_tunnel_status_inactive() {
        let status = TunnelStatus {
            public_url: String::new(),
            tunnel_id: "inactive-tunnel".to_string(),
            active: false,
            request_count: 0,
            bytes_transferred: 0,
            created_at: None,
            expires_at: None,
            local_url: None,
        };

        assert!(!status.active);
        assert!(status.public_url.is_empty());
    }

    #[test]
    fn test_tunnel_status_high_traffic() {
        let status = TunnelStatus {
            public_url: "https://high-traffic.tunnel.dev".to_string(),
            tunnel_id: "high-traffic-1".to_string(),
            active: true,
            request_count: u64::MAX,
            bytes_transferred: u64::MAX,
            created_at: Some(Utc::now()),
            expires_at: None,
            local_url: None,
        };

        assert_eq!(status.request_count, u64::MAX);
        assert_eq!(status.bytes_transferred, u64::MAX);
    }
}
