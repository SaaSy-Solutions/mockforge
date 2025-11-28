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
