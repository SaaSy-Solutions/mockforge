//! Builder for configuring mock servers

use crate::server::MockServer;
use crate::{Error, Result};
use mockforge_core::{Config, FailureConfig, LatencyProfile, ProxyConfig, ServerConfig};
use std::path::PathBuf;

/// Builder for creating and configuring mock servers
pub struct MockServerBuilder {
    port: Option<u16>,
    host: Option<String>,
    config_file: Option<PathBuf>,
    openapi_spec: Option<PathBuf>,
    latency_profile: Option<LatencyProfile>,
    failure_config: Option<FailureConfig>,
    proxy_config: Option<ProxyConfig>,
    enable_admin: bool,
    admin_port: Option<u16>,
}

impl Default for MockServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MockServerBuilder {
    /// Create a new mock server builder
    pub fn new() -> Self {
        Self {
            port: None,
            host: None,
            config_file: None,
            openapi_spec: None,
            latency_profile: None,
            failure_config: None,
            proxy_config: None,
            enable_admin: false,
            admin_port: None,
        }
    }

    /// Set the HTTP port
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Set the host address
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    /// Load configuration from a YAML file
    pub fn config_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.config_file = Some(path.into());
        self
    }

    /// Load routes from an OpenAPI specification
    pub fn openapi_spec(mut self, path: impl Into<PathBuf>) -> Self {
        self.openapi_spec = Some(path.into());
        self
    }

    /// Set the latency profile for simulating network delays
    pub fn latency(mut self, profile: LatencyProfile) -> Self {
        self.latency_profile = Some(profile);
        self
    }

    /// Enable failure injection with configuration
    pub fn failures(mut self, config: FailureConfig) -> Self {
        self.failure_config = Some(config);
        self
    }

    /// Enable proxy mode with configuration
    pub fn proxy(mut self, config: ProxyConfig) -> Self {
        self.proxy_config = Some(config);
        self
    }

    /// Enable admin API
    pub fn admin(mut self, enabled: bool) -> Self {
        self.enable_admin = enabled;
        self
    }

    /// Set admin API port
    pub fn admin_port(mut self, port: u16) -> Self {
        self.admin_port = Some(port);
        self
    }

    /// Start the mock server
    pub async fn start(self) -> Result<MockServer> {
        // Build the configuration
        let mut config = if let Some(config_file) = self.config_file {
            mockforge_core::load_config(&config_file)
                .await
                .map_err(|e| Error::InvalidConfig(e.to_string()))?
        } else {
            ServerConfig::default()
        };

        // Apply builder settings
        if let Some(port) = self.port {
            config.http.port = port;
        }
        if let Some(host) = self.host {
            config.http.host = host;
        }
        if let Some(spec_path) = self.openapi_spec {
            config.http.openapi_spec = Some(spec_path.to_string_lossy().to_string());
        }

        // Create core config
        let mut core_config = Config::default();

        if let Some(latency) = self.latency_profile {
            core_config.latency_enabled = true;
            core_config.default_latency = latency;
        }

        if let Some(failures) = self.failure_config {
            core_config.failures_enabled = true;
            core_config.failure_config = Some(failures);
        }

        if let Some(proxy) = self.proxy_config {
            core_config.proxy = Some(proxy);
        }

        // Create and start the server
        MockServer::from_config(config, core_config).await
    }
}
