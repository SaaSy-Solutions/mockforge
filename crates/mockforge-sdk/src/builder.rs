//! Builder for configuring mock servers

use crate::server::MockServer;
use crate::{Error, Result};
use mockforge_core::{Config, FailureConfig, LatencyProfile, ProxyConfig, ServerConfig};
use std::net::TcpListener;
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
    auto_port: bool,
    port_range: Option<(u16, u16)>,
}

impl Default for MockServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MockServerBuilder {
    /// Create a new mock server builder
    #[must_use]
    pub const fn new() -> Self {
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
            auto_port: false,
            port_range: None,
        }
    }

    /// Set the HTTP port
    #[must_use]
    pub const fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self.auto_port = false;
        self
    }

    /// Automatically discover an available port
    #[must_use]
    pub const fn auto_port(mut self) -> Self {
        self.auto_port = true;
        self.port = None;
        self
    }

    /// Set the port range for automatic port discovery
    /// Default range is 30000-30100
    #[must_use]
    pub const fn port_range(mut self, start: u16, end: u16) -> Self {
        self.port_range = Some((start, end));
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

    /// Load routes from an `OpenAPI` specification
    pub fn openapi_spec(mut self, path: impl Into<PathBuf>) -> Self {
        self.openapi_spec = Some(path.into());
        self
    }

    /// Set the latency profile for simulating network delays
    #[must_use]
    pub fn latency(mut self, profile: LatencyProfile) -> Self {
        self.latency_profile = Some(profile);
        self
    }

    /// Enable failure injection with configuration
    #[must_use]
    pub fn failures(mut self, config: FailureConfig) -> Self {
        self.failure_config = Some(config);
        self
    }

    /// Enable proxy mode with configuration
    #[must_use]
    pub fn proxy(mut self, config: ProxyConfig) -> Self {
        self.proxy_config = Some(config);
        self
    }

    /// Enable admin API
    #[must_use]
    pub const fn admin(mut self, enabled: bool) -> Self {
        self.enable_admin = enabled;
        self
    }

    /// Set admin API port
    #[must_use]
    pub const fn admin_port(mut self, port: u16) -> Self {
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

        // Apply port settings
        if self.auto_port {
            // Discover an available port
            let (start, end) = self.port_range.unwrap_or((30000, 30100));
            let port = find_available_port(start, end)?;
            config.http.port = port;
        } else if let Some(port) = self.port {
            config.http.port = port;
        }

        // Apply other builder settings
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

/// Check if a port is available by attempting to bind to it
///
/// Note: There is a small race condition (TOCTOU - Time Of Check, Time Of Use)
/// between checking availability and the actual server binding. In practice,
/// this is rarely an issue for test environments. For guaranteed port assignment,
/// consider using `port(0)` to let the OS assign any available port.
fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// Find an available port in the specified range
///
/// Scans the port range and returns the first available port.
/// Returns an error if no ports are available in the range.
///
/// # Arguments
/// * `start` - Starting port number (inclusive)
/// * `end` - Ending port number (inclusive)
///
/// # Errors
/// Returns `Error::InvalidConfig` if start >= end
/// Returns `Error::PortDiscoveryFailed` if no ports are available
fn find_available_port(start: u16, end: u16) -> Result<u16> {
    // Validate port range
    if start >= end {
        return Err(Error::InvalidConfig(format!(
            "Invalid port range: start ({start}) must be less than end ({end})"
        )));
    }

    // Try to find an available port
    for port in start..=end {
        if is_port_available(port) {
            return Ok(port);
        }
    }

    Err(Error::PortDiscoveryFailed(format!(
        "No available ports found in range {start}-{end}"
    )))
}
