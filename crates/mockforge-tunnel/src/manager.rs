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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TunnelConfig;

    fn create_test_config() -> TunnelConfig {
        TunnelConfig {
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
        }
    }

    #[test]
    fn test_tunnel_manager_new() {
        let config = create_test_config();
        let manager = TunnelManager::new(&config);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_tunnel_manager_new_without_server_url() {
        let mut config = create_test_config();
        config.server_url = None;

        let result = TunnelManager::new(&config);
        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                TunnelError::ConfigError(msg) => {
                    assert!(msg.contains("server_url"));
                }
                _ => panic!("Expected ConfigError"),
            }
        }
    }

    #[test]
    fn test_tunnel_manager_new_cloud_provider() {
        let mut config = create_test_config();
        config.provider = crate::config::TunnelProvider::Cloud;
        config.server_url = None; // Cloud should use default

        let result = TunnelManager::new(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tunnel_manager_new_cloudflare_provider() {
        let mut config = create_test_config();
        config.provider = crate::config::TunnelProvider::Cloudflare;

        let result = TunnelManager::new(&config);
        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                TunnelError::ProviderError(msg) => {
                    assert!(msg.contains("Cloudflare"));
                    assert!(msg.contains("coming soon"));
                }
                _ => panic!("Expected ProviderError"),
            }
        }
    }

    #[test]
    fn test_tunnel_manager_new_ngrok_provider() {
        let mut config = create_test_config();
        config.provider = crate::config::TunnelProvider::Ngrok;

        let result = TunnelManager::new(&config);
        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                TunnelError::ProviderError(msg) => {
                    assert!(msg.contains("ngrok"));
                    assert!(msg.contains("coming soon"));
                }
                _ => panic!("Expected ProviderError"),
            }
        }
    }

    #[test]
    fn test_tunnel_manager_new_localtunnel_provider() {
        let mut config = create_test_config();
        config.provider = crate::config::TunnelProvider::Localtunnel;

        let result = TunnelManager::new(&config);
        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                TunnelError::ProviderError(msg) => {
                    assert!(msg.contains("localtunnel"));
                    assert!(msg.contains("coming soon"));
                }
                _ => panic!("Expected ProviderError"),
            }
        }
    }

    #[tokio::test]
    async fn test_get_status_no_tunnel() {
        let config = create_test_config();
        let manager = TunnelManager::new(&config).unwrap();

        let status = manager.get_status().await;
        assert!(status.is_ok());
        assert!(status.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_refresh_status_no_tunnel() {
        let config = create_test_config();
        let manager = TunnelManager::new(&config).unwrap();

        let result = manager.refresh_status().await;
        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                TunnelError::NotFound(msg) => {
                    assert!(msg.contains("No active tunnel"));
                }
                _ => panic!("Expected NotFound error"),
            }
        }
    }

    #[tokio::test]
    async fn test_stop_tunnel_no_tunnel() {
        let config = create_test_config();
        let manager = TunnelManager::new(&config).unwrap();

        let result = manager.stop_tunnel().await;
        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                TunnelError::NotFound(msg) => {
                    assert!(msg.contains("No active tunnel"));
                }
                _ => panic!("Expected NotFound error"),
            }
        }
    }

    #[test]
    fn test_tunnel_manager_with_different_protocols() {
        let protocols = vec!["http", "https", "ws", "wss"];

        for protocol in protocols {
            let mut config = create_test_config();
            config.protocol = protocol.to_string();

            let result = TunnelManager::new(&config);
            assert!(result.is_ok(), "Failed to create manager with protocol: {}", protocol);
        }
    }

    #[test]
    fn test_tunnel_manager_with_websocket_disabled() {
        let mut config = create_test_config();
        config.websocket_enabled = false;

        let result = TunnelManager::new(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tunnel_manager_with_http2_disabled() {
        let mut config = create_test_config();
        config.http2_enabled = false;

        let result = TunnelManager::new(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tunnel_manager_with_custom_domain() {
        let mut config = create_test_config();
        config.custom_domain = Some("api.example.com".to_string());

        let result = TunnelManager::new(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tunnel_manager_with_region() {
        let mut config = create_test_config();
        config.region = Some("us-west".to_string());

        let result = TunnelManager::new(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tunnel_manager_without_auth_token() {
        let mut config = create_test_config();
        config.auth_token = None;

        let result = TunnelManager::new(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tunnel_manager_with_different_local_urls() {
        let urls = vec![
            "http://localhost:3000",
            "http://127.0.0.1:8080",
            "http://0.0.0.0:5000",
            "https://internal-api:443",
        ];

        for url in urls {
            let mut config = create_test_config();
            config.local_url = url.to_string();

            let result = TunnelManager::new(&config);
            assert!(result.is_ok(), "Failed to create manager with local_url: {}", url);
        }
    }

    #[test]
    fn test_tunnel_manager_builder_pattern() {
        let config = TunnelConfig::new("http://localhost:3000")
            .with_provider(crate::config::TunnelProvider::SelfHosted)
            .with_auth_token("token123")
            .with_subdomain("myapp");

        let mut config_with_server = config;
        config_with_server.server_url = Some("https://tunnel.example.com".to_string());

        let result = TunnelManager::new(&config_with_server);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tunnel_manager_clone_config() {
        let config = create_test_config();
        let cloned = config.clone();

        let manager1 = TunnelManager::new(&config);
        let manager2 = TunnelManager::new(&cloned);

        assert!(manager1.is_ok());
        assert!(manager2.is_ok());
    }
}
