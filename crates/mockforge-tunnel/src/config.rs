//! Tunnel configuration

use serde::{Deserialize, Serialize};

/// Tunnel provider type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TunnelProvider {
    /// Self-hosted tunneling service
    #[serde(rename = "self")]
    SelfHosted,
    /// MockForge Cloud tunneling service (if available)
    Cloud,
    /// Cloudflare Tunnel (cloudflared)
    Cloudflare,
    /// ngrok-style service
    Ngrok,
    /// Localtunnel-style service
    Localtunnel,
}

impl Default for TunnelProvider {
    fn default() -> Self {
        Self::SelfHosted
    }
}

/// Tunnel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    /// Provider to use for tunneling
    #[serde(default)]
    pub provider: TunnelProvider,

    /// Tunnel server URL (for self-hosted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,

    /// Authentication token (if required)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,

    /// Subdomain to request (optional, may not be available on all providers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdomain: Option<String>,

    /// Local server URL to tunnel
    pub local_url: String,

    /// Protocol to tunnel (http, https, ws, wss)
    #[serde(default = "default_protocol")]
    pub protocol: String,

    /// Region for tunnel (if provider supports it)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,

    /// Custom domain (if provider supports it)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_domain: Option<String>,

    /// Enable WebSocket support
    #[serde(default = "default_true")]
    pub websocket_enabled: bool,

    /// Enable HTTP/2 support
    #[serde(default = "default_true")]
    pub http2_enabled: bool,
}

fn default_protocol() -> String {
    "http".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            provider: TunnelProvider::default(),
            server_url: None,
            auth_token: None,
            subdomain: None,
            local_url: "http://localhost:3000".to_string(),
            protocol: default_protocol(),
            region: None,
            custom_domain: None,
            websocket_enabled: true,
            http2_enabled: true,
        }
    }
}

impl TunnelConfig {
    /// Create a new tunnel config for local HTTP server
    pub fn new(local_url: impl Into<String>) -> Self {
        Self {
            local_url: local_url.into(),
            ..Default::default()
        }
    }

    /// Set the provider
    pub fn with_provider(mut self, provider: TunnelProvider) -> Self {
        self.provider = provider;
        self
    }

    /// Set authentication token
    pub fn with_auth_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    /// Set subdomain
    pub fn with_subdomain(mut self, subdomain: impl Into<String>) -> Self {
        self.subdomain = Some(subdomain.into());
        self
    }

    /// Set custom domain
    pub fn with_custom_domain(mut self, domain: impl Into<String>) -> Self {
        self.custom_domain = Some(domain.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tunnel_provider_default() {
        let provider = TunnelProvider::default();
        assert_eq!(provider, TunnelProvider::SelfHosted);
    }

    #[test]
    fn test_tunnel_provider_eq() {
        assert_eq!(TunnelProvider::Cloud, TunnelProvider::Cloud);
        assert_ne!(TunnelProvider::Cloud, TunnelProvider::Ngrok);
    }

    #[test]
    fn test_tunnel_provider_clone() {
        let provider = TunnelProvider::Cloudflare;
        let cloned = provider.clone();
        assert_eq!(provider, cloned);
    }

    #[test]
    fn test_tunnel_provider_debug() {
        let provider = TunnelProvider::Localtunnel;
        let debug = format!("{:?}", provider);
        assert!(debug.contains("Localtunnel"));
    }

    #[test]
    fn test_tunnel_provider_serialize() {
        let provider = TunnelProvider::Cloud;
        let json = serde_json::to_string(&provider).unwrap();
        assert_eq!(json, "\"cloud\"");
    }

    #[test]
    fn test_tunnel_provider_serialize_self() {
        let provider = TunnelProvider::SelfHosted;
        let json = serde_json::to_string(&provider).unwrap();
        assert_eq!(json, "\"self\"");
    }

    #[test]
    fn test_tunnel_provider_deserialize() {
        let provider: TunnelProvider = serde_json::from_str("\"ngrok\"").unwrap();
        assert_eq!(provider, TunnelProvider::Ngrok);
    }

    #[test]
    fn test_tunnel_config_default() {
        let config = TunnelConfig::default();
        assert_eq!(config.provider, TunnelProvider::SelfHosted);
        assert!(config.server_url.is_none());
        assert!(config.auth_token.is_none());
        assert!(config.subdomain.is_none());
        assert_eq!(config.local_url, "http://localhost:3000");
        assert_eq!(config.protocol, "http");
        assert!(config.region.is_none());
        assert!(config.custom_domain.is_none());
        assert!(config.websocket_enabled);
        assert!(config.http2_enabled);
    }

    #[test]
    fn test_tunnel_config_new() {
        let config = TunnelConfig::new("http://localhost:8080");
        assert_eq!(config.local_url, "http://localhost:8080");
        assert_eq!(config.provider, TunnelProvider::SelfHosted);
    }

    #[test]
    fn test_tunnel_config_with_provider() {
        let config =
            TunnelConfig::new("http://localhost:8080").with_provider(TunnelProvider::Cloudflare);
        assert_eq!(config.provider, TunnelProvider::Cloudflare);
    }

    #[test]
    fn test_tunnel_config_with_auth_token() {
        let config = TunnelConfig::new("http://localhost:8080").with_auth_token("my-secret-token");
        assert_eq!(config.auth_token, Some("my-secret-token".to_string()));
    }

    #[test]
    fn test_tunnel_config_with_subdomain() {
        let config = TunnelConfig::new("http://localhost:8080").with_subdomain("myapp");
        assert_eq!(config.subdomain, Some("myapp".to_string()));
    }

    #[test]
    fn test_tunnel_config_with_custom_domain() {
        let config =
            TunnelConfig::new("http://localhost:8080").with_custom_domain("api.example.com");
        assert_eq!(config.custom_domain, Some("api.example.com".to_string()));
    }

    #[test]
    fn test_tunnel_config_builder_chain() {
        let config = TunnelConfig::new("http://localhost:3000")
            .with_provider(TunnelProvider::SelfHosted)
            .with_auth_token("token123")
            .with_subdomain("test")
            .with_custom_domain("test.example.com");

        assert_eq!(config.local_url, "http://localhost:3000");
        assert_eq!(config.provider, TunnelProvider::SelfHosted);
        assert_eq!(config.auth_token, Some("token123".to_string()));
        assert_eq!(config.subdomain, Some("test".to_string()));
        assert_eq!(config.custom_domain, Some("test.example.com".to_string()));
    }

    #[test]
    fn test_tunnel_config_clone() {
        let config = TunnelConfig::new("http://localhost:8080")
            .with_provider(TunnelProvider::Ngrok)
            .with_auth_token("secret");

        let cloned = config.clone();
        assert_eq!(config.local_url, cloned.local_url);
        assert_eq!(config.provider, cloned.provider);
        assert_eq!(config.auth_token, cloned.auth_token);
    }

    #[test]
    fn test_tunnel_config_debug() {
        let config = TunnelConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("TunnelConfig"));
        assert!(debug.contains("localhost"));
    }

    #[test]
    fn test_tunnel_config_serialize() {
        let config = TunnelConfig::new("http://localhost:8080");
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"local_url\":\"http://localhost:8080\""));
    }

    #[test]
    fn test_tunnel_config_deserialize() {
        let json = r#"{
            "provider": "cloud",
            "local_url": "http://localhost:5000",
            "protocol": "https",
            "websocket_enabled": false,
            "http2_enabled": true
        }"#;

        let config: TunnelConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.provider, TunnelProvider::Cloud);
        assert_eq!(config.local_url, "http://localhost:5000");
        assert_eq!(config.protocol, "https");
        assert!(!config.websocket_enabled);
        assert!(config.http2_enabled);
    }

    #[test]
    fn test_tunnel_config_serialize_skip_none() {
        let config = TunnelConfig::new("http://localhost:8080");
        let json = serde_json::to_string(&config).unwrap();
        // Optional None fields should be skipped
        assert!(!json.contains("server_url"));
        assert!(!json.contains("auth_token"));
    }
}
