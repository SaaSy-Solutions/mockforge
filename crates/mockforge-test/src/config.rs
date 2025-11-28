//! Configuration for MockForge test servers

use std::path::PathBuf;
use std::time::Duration;

/// Configuration for a MockForge test server
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// HTTP server port (default: auto-assigned)
    pub http_port: u16,

    /// WebSocket server port (optional)
    pub ws_port: Option<u16>,

    /// gRPC server port (optional)
    pub grpc_port: Option<u16>,

    /// Admin UI port (optional)
    pub admin_port: Option<u16>,

    /// Metrics/Prometheus port (optional)
    pub metrics_port: Option<u16>,

    /// Path to OpenAPI specification file (optional)
    pub spec_file: Option<PathBuf>,

    /// Path to workspace directory (optional)
    pub workspace_dir: Option<PathBuf>,

    /// Profile name for configuration (optional)
    pub profile: Option<String>,

    /// Enable admin UI (default: false)
    pub enable_admin: bool,

    /// Enable metrics endpoint (default: false)
    pub enable_metrics: bool,

    /// Additional CLI arguments
    pub extra_args: Vec<String>,

    /// Health check timeout (default: 30 seconds)
    pub health_timeout: Duration,

    /// Health check interval (default: 100ms)
    pub health_interval: Duration,

    /// Working directory for the server process (optional)
    pub working_dir: Option<PathBuf>,

    /// Environment variables for the server process
    pub env_vars: Vec<(String, String)>,

    /// Path to mockforge binary (optional, will search PATH if not provided)
    pub binary_path: Option<PathBuf>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            http_port: 0, // 0 means auto-assign
            ws_port: None,
            grpc_port: None,
            admin_port: None,
            metrics_port: None,
            spec_file: None,
            workspace_dir: None,
            profile: None,
            enable_admin: false,
            enable_metrics: false,
            extra_args: Vec::new(),
            health_timeout: Duration::from_secs(30),
            health_interval: Duration::from_millis(100),
            working_dir: None,
            env_vars: Vec::new(),
            binary_path: None,
        }
    }
}

impl ServerConfig {
    /// Create a new builder for ServerConfig
    pub fn builder() -> ServerConfigBuilder {
        ServerConfigBuilder::default()
    }
}

/// Builder for ServerConfig
#[derive(Debug, Default)]
pub struct ServerConfigBuilder {
    config: ServerConfig,
}

impl ServerConfigBuilder {
    /// Set HTTP port (0 for auto-assign)
    pub fn http_port(mut self, port: u16) -> Self {
        self.config.http_port = port;
        self
    }

    /// Set WebSocket port
    pub fn ws_port(mut self, port: u16) -> Self {
        self.config.ws_port = Some(port);
        self
    }

    /// Set gRPC port
    pub fn grpc_port(mut self, port: u16) -> Self {
        self.config.grpc_port = Some(port);
        self
    }

    /// Set admin UI port
    pub fn admin_port(mut self, port: u16) -> Self {
        self.config.admin_port = Some(port);
        self
    }

    /// Set metrics port
    pub fn metrics_port(mut self, port: u16) -> Self {
        self.config.metrics_port = Some(port);
        self
    }

    /// Set OpenAPI specification file
    pub fn spec_file<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.config.spec_file = Some(path.into());
        self
    }

    /// Set workspace directory
    pub fn workspace_dir<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.config.workspace_dir = Some(path.into());
        self
    }

    /// Set profile name
    pub fn profile<S: Into<String>>(mut self, profile: S) -> Self {
        self.config.profile = Some(profile.into());
        self
    }

    /// Enable admin UI
    pub fn enable_admin(mut self, enable: bool) -> Self {
        self.config.enable_admin = enable;
        self
    }

    /// Enable metrics endpoint
    pub fn enable_metrics(mut self, enable: bool) -> Self {
        self.config.enable_metrics = enable;
        self
    }

    /// Add extra CLI argument
    pub fn extra_arg<S: Into<String>>(mut self, arg: S) -> Self {
        self.config.extra_args.push(arg.into());
        self
    }

    /// Add multiple extra CLI arguments
    pub fn extra_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.config.extra_args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Set health check timeout
    pub fn health_timeout(mut self, timeout: Duration) -> Self {
        self.config.health_timeout = timeout;
        self
    }

    /// Set health check interval
    pub fn health_interval(mut self, interval: Duration) -> Self {
        self.config.health_interval = interval;
        self
    }

    /// Set working directory
    pub fn working_dir<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.config.working_dir = Some(path.into());
        self
    }

    /// Add environment variable
    pub fn env_var<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.config.env_vars.push((key.into(), value.into()));
        self
    }

    /// Set path to mockforge binary
    pub fn binary_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.config.binary_path = Some(path.into());
        self
    }

    /// Build the ServerConfig
    pub fn build(self) -> ServerConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.http_port, 0);
        assert!(config.ws_port.is_none());
        assert!(!config.enable_admin);
    }

    #[test]
    fn test_builder() {
        let config = ServerConfig::builder()
            .http_port(3000)
            .ws_port(3001)
            .grpc_port(3002)
            .admin_port(3003)
            .enable_admin(true)
            .enable_metrics(true)
            .profile("test")
            .extra_arg("--verbose")
            .build();

        assert_eq!(config.http_port, 3000);
        assert_eq!(config.ws_port, Some(3001));
        assert_eq!(config.grpc_port, Some(3002));
        assert_eq!(config.admin_port, Some(3003));
        assert!(config.enable_admin);
        assert!(config.enable_metrics);
        assert_eq!(config.profile, Some("test".to_string()));
        assert_eq!(config.extra_args, vec!["--verbose"]);
    }
}
