//! Tunnel manager for creating and managing tunnels

use crate::{config::TunnelConfig, provider::*, Result, TunnelError, TunnelStatus};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tunnel manager for creating and managing tunnels
pub struct TunnelManager {
    provider: Arc<dyn TunnelProvider>,
    active_tunnel: Arc<RwLock<Option<TunnelStatus>>>,
}

impl TunnelManager {
    /// Create a new tunnel manager
    pub fn new(config: &TunnelConfig) -> Result<Self> {
        let provider: Arc<dyn TunnelProvider> = match config.provider {
            crate::config::TunnelProvider::SelfHosted => {
                let server_url = config.server_url.clone().ok_or_else(|| {
                    TunnelError::ConfigError(
                        "server_url required for self-hosted provider".to_string(),
                    )
                })?;
                Arc::new(SelfHostedProvider::new(server_url, config.auth_token.clone()))
            }
            crate::config::TunnelProvider::Cloud => {
                // For now, fallback to self-hosted behavior
                // In the future, this could connect to MockForge Cloud
                let server_url = config
                    .server_url
                    .clone()
                    .unwrap_or_else(|| "https://tunnel.mockforge.dev".to_string());
                Arc::new(SelfHostedProvider::new(server_url, config.auth_token.clone()))
            }
            crate::config::TunnelProvider::Cloudflare => {
                return Err(TunnelError::ProviderError(
                    "Cloudflare tunnel support coming soon".to_string(),
                ));
            }
            crate::config::TunnelProvider::Ngrok => {
                return Err(TunnelError::ProviderError(
                    "ngrok tunnel support coming soon".to_string(),
                ));
            }
            crate::config::TunnelProvider::Localtunnel => {
                return Err(TunnelError::ProviderError(
                    "localtunnel support coming soon".to_string(),
                ));
            }
        };

        Ok(Self {
            provider,
            active_tunnel: Arc::new(RwLock::new(None)),
        })
    }

    /// Create and start a tunnel
    pub async fn create_tunnel(&self, config: &TunnelConfig) -> Result<TunnelStatus> {
        // Check if tunnel already exists
        {
            let tunnel = self.active_tunnel.read().await;
            if tunnel.is_some() {
                return Err(TunnelError::AlreadyExists("Tunnel already active".to_string()));
            }
        }

        // Create tunnel via provider
        let status = self.provider.create_tunnel(config).await?;

        // Store active tunnel
        {
            let mut tunnel = self.active_tunnel.write().await;
            *tunnel = Some(status.clone());
        }

        Ok(status)
    }

    /// Get the current tunnel status
    pub async fn get_status(&self) -> Result<Option<TunnelStatus>> {
        let tunnel = self.active_tunnel.read().await;
        Ok(tunnel.clone())
    }

    /// Refresh tunnel status from provider
    pub async fn refresh_status(&self) -> Result<TunnelStatus> {
        let tunnel_id = {
            let tunnel = self.active_tunnel.read().await;
            tunnel
                .as_ref()
                .map(|t| t.tunnel_id.clone())
                .ok_or_else(|| TunnelError::NotFound("No active tunnel".to_string()))?
        };

        let status = self.provider.get_tunnel_status(&tunnel_id).await?;

        // Update stored tunnel
        {
            let mut tunnel = self.active_tunnel.write().await;
            *tunnel = Some(status.clone());
        }

        Ok(status)
    }

    /// Stop and delete the tunnel
    pub async fn stop_tunnel(&self) -> Result<()> {
        let tunnel_id = {
            let tunnel = self.active_tunnel.read().await;
            tunnel
                .as_ref()
                .map(|t| t.tunnel_id.clone())
                .ok_or_else(|| TunnelError::NotFound("No active tunnel".to_string()))?
        };

        // Delete tunnel via provider
        self.provider.delete_tunnel(&tunnel_id).await?;

        // Clear active tunnel
        {
            let mut tunnel = self.active_tunnel.write().await;
            *tunnel = None;
        }

        Ok(())
    }

    /// Stop and delete a tunnel by ID
    pub async fn stop_tunnel_by_id(&self, tunnel_id: &str) -> Result<()> {
        // Delete tunnel via provider
        self.provider.delete_tunnel(tunnel_id).await?;

        // Clear active tunnel if it matches
        {
            let mut tunnel = self.active_tunnel.write().await;
            if tunnel.as_ref().map(|t| t.tunnel_id.as_str()) == Some(tunnel_id) {
                *tunnel = None;
            }
        }

        Ok(())
    }

    /// List all tunnels
    pub async fn list_tunnels(&self) -> Result<Vec<TunnelStatus>> {
        self.provider.list_tunnels().await
    }

    /// Check if provider is available
    pub async fn is_available(&self) -> bool {
        self.provider.is_available().await
    }
}
