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
