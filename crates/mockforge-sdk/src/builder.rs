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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_builder_new() {
        let builder = MockServerBuilder::new();
        assert!(builder.port.is_none());
        assert!(builder.host.is_none());
        assert!(!builder.enable_admin);
        assert!(!builder.auto_port);
    }

    #[test]
    fn test_builder_default() {
        let builder = MockServerBuilder::default();
        assert!(builder.port.is_none());
        assert!(builder.host.is_none());
    }

    #[test]
    fn test_builder_port() {
        let builder = MockServerBuilder::new().port(8080);
        assert_eq!(builder.port, Some(8080));
        assert!(!builder.auto_port);
    }

    #[test]
    fn test_builder_auto_port() {
        let builder = MockServerBuilder::new().auto_port();
        assert!(builder.auto_port);
        assert!(builder.port.is_none());
    }

    #[test]
    fn test_builder_auto_port_overrides_manual_port() {
        let builder = MockServerBuilder::new().port(8080).auto_port();
        assert!(builder.auto_port);
        assert!(builder.port.is_none());
    }

    #[test]
    fn test_builder_manual_port_overrides_auto_port() {
        let builder = MockServerBuilder::new().auto_port().port(8080);
        assert!(!builder.auto_port);
        assert_eq!(builder.port, Some(8080));
    }

    #[test]
    fn test_builder_port_range() {
        let builder = MockServerBuilder::new().port_range(30000, 31000);
        assert_eq!(builder.port_range, Some((30000, 31000)));
    }

    #[test]
    fn test_builder_host() {
        let builder = MockServerBuilder::new().host("0.0.0.0");
        assert_eq!(builder.host, Some("0.0.0.0".to_string()));
    }

    #[test]
    fn test_builder_config_file() {
        let builder = MockServerBuilder::new().config_file("/path/to/config.yaml");
        assert_eq!(builder.config_file, Some(PathBuf::from("/path/to/config.yaml")));
    }

    #[test]
    fn test_builder_openapi_spec() {
        let builder = MockServerBuilder::new().openapi_spec("/path/to/spec.yaml");
        assert_eq!(builder.openapi_spec, Some(PathBuf::from("/path/to/spec.yaml")));
    }

    #[test]
    fn test_builder_latency() {
        let latency = LatencyProfile::new(100, 0);
        let builder = MockServerBuilder::new().latency(latency);
        assert!(builder.latency_profile.is_some());
    }

    #[test]
    fn test_builder_failures() {
        let failures = FailureConfig {
            global_error_rate: 0.1,
            default_status_codes: vec![500, 503],
            ..Default::default()
        };
        let builder = MockServerBuilder::new().failures(failures);
        assert!(builder.failure_config.is_some());
    }

    #[test]
    fn test_builder_proxy() {
        let proxy = ProxyConfig {
            enabled: true,
            target_url: Some("http://example.com".to_string()),
            ..Default::default()
        };
        let builder = MockServerBuilder::new().proxy(proxy);
        assert!(builder.proxy_config.is_some());
    }

    #[test]
    fn test_builder_admin() {
        let builder = MockServerBuilder::new().admin(true);
        assert!(builder.enable_admin);
    }

    #[test]
    fn test_builder_admin_port() {
        let builder = MockServerBuilder::new().admin_port(9090);
        assert_eq!(builder.admin_port, Some(9090));
    }

    #[test]
    fn test_builder_fluent_chaining() {
        let latency = LatencyProfile::new(50, 0);
        let failures = FailureConfig {
            global_error_rate: 0.05,
            default_status_codes: vec![500],
            ..Default::default()
        };

        let builder = MockServerBuilder::new()
            .port(8080)
            .host("localhost")
            .latency(latency)
            .failures(failures)
            .admin(true)
            .admin_port(9090);

        assert_eq!(builder.port, Some(8080));
        assert_eq!(builder.host, Some("localhost".to_string()));
        assert!(builder.latency_profile.is_some());
        assert!(builder.failure_config.is_some());
        assert!(builder.enable_admin);
        assert_eq!(builder.admin_port, Some(9090));
    }

    #[test]
    fn test_is_port_available_unbound_port() {
        // Port 0 should allow binding (OS will assign a port)
        assert!(is_port_available(0));
    }

    #[test]
    fn test_is_port_available_bound_port() {
        // Bind to a port first
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let port = addr.port();

        // Now that port should not be available
        assert!(!is_port_available(port));
    }

    #[test]
    fn test_find_available_port_success() {
        // Should find an available port in a large range
        let result = find_available_port(30000, 35000);
        assert!(result.is_ok());
        let port = result.unwrap();
        assert!(port >= 30000 && port <= 35000);
    }

    #[test]
    fn test_find_available_port_invalid_range_equal() {
        let result = find_available_port(8080, 8080);
        assert!(result.is_err());
        match result {
            Err(Error::InvalidConfig(msg)) => {
                assert!(msg.contains("Invalid port range"));
                assert!(msg.contains("8080"));
            }
            _ => panic!("Expected InvalidConfig error"),
        }
    }

    #[test]
    fn test_find_available_port_invalid_range_reversed() {
        let result = find_available_port(9000, 8000);
        assert!(result.is_err());
        match result {
            Err(Error::InvalidConfig(msg)) => {
                assert!(msg.contains("Invalid port range"));
            }
            _ => panic!("Expected InvalidConfig error"),
        }
    }

    #[test]
    fn test_find_available_port_no_ports_available() {
        // Bind to all ports in a small range
        let port1 = 40000;
        let port2 = 40001;
        let _listener1 = TcpListener::bind(("127.0.0.1", port1)).ok();
        let _listener2 = TcpListener::bind(("127.0.0.1", port2)).ok();

        // If both binds succeeded, the search should fail
        if _listener1.is_some() && _listener2.is_some() {
            let result = find_available_port(port1, port2);
            assert!(result.is_err());
            match result {
                Err(Error::PortDiscoveryFailed(msg)) => {
                    assert!(msg.contains("No available ports"));
                    assert!(msg.contains("40000"));
                    assert!(msg.contains("40001"));
                }
                _ => panic!("Expected PortDiscoveryFailed error"),
            }
        }
    }

    #[test]
    fn test_find_available_port_single_port_range() {
        // Even though start < end, this is a valid single-port range (inclusive)
        let result = find_available_port(45000, 45001);
        assert!(result.is_ok());
        let port = result.unwrap();
        assert!(port == 45000 || port == 45001);
    }

    #[test]
    fn test_builder_multiple_config_sources() {
        let builder = MockServerBuilder::new()
            .config_file("/path/to/config.yaml")
            .openapi_spec("/path/to/spec.yaml")
            .port(8080)
            .host("localhost");

        assert!(builder.config_file.is_some());
        assert!(builder.openapi_spec.is_some());
        assert_eq!(builder.port, Some(8080));
        assert_eq!(builder.host, Some("localhost".to_string()));
    }

    #[test]
    fn test_builder_with_all_features() {
        let latency = LatencyProfile::new(100, 0);
        let failures = FailureConfig {
            global_error_rate: 0.1,
            default_status_codes: vec![500, 503],
            ..Default::default()
        };
        let proxy = ProxyConfig {
            enabled: true,
            target_url: Some("http://backend.com".to_string()),
            ..Default::default()
        };

        let builder = MockServerBuilder::new()
            .port(8080)
            .host("0.0.0.0")
            .config_file("/config.yaml")
            .openapi_spec("/spec.yaml")
            .latency(latency)
            .failures(failures)
            .proxy(proxy)
            .admin(true)
            .admin_port(9090);

        assert!(builder.port.is_some());
        assert!(builder.host.is_some());
        assert!(builder.config_file.is_some());
        assert!(builder.openapi_spec.is_some());
        assert!(builder.latency_profile.is_some());
        assert!(builder.failure_config.is_some());
        assert!(builder.proxy_config.is_some());
        assert!(builder.enable_admin);
        assert!(builder.admin_port.is_some());
    }

    #[test]
    fn test_builder_port_range_default() {
        let builder = MockServerBuilder::new().auto_port();
        // Default range should be used if not specified
        assert!(builder.port_range.is_none());
    }

    #[test]
    fn test_builder_port_range_custom() {
        let builder = MockServerBuilder::new().auto_port().port_range(40000, 50000);
        assert_eq!(builder.port_range, Some((40000, 50000)));
    }
}
