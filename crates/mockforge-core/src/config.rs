//! Configuration management for MockForge

use crate::{Config as CoreConfig, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerConfig {
    /// HTTP server configuration
    pub http: HttpConfig,
    /// WebSocket server configuration
    pub websocket: WebSocketConfig,
    /// gRPC server configuration
    pub grpc: GrpcConfig,
    /// Admin UI configuration
    pub admin: AdminConfig,
    /// Core MockForge configuration
    pub core: CoreConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Data generation configuration
    pub data: DataConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            http: HttpConfig::default(),
            websocket: WebSocketConfig::default(),
            grpc: GrpcConfig::default(),
            admin: AdminConfig::default(),
            core: CoreConfig::default(),
            logging: LoggingConfig::default(),
            data: DataConfig::default(),
        }
    }
}

/// HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    /// Server port
    pub port: u16,
    /// Host address
    pub host: String,
    /// OpenAPI spec path
    pub openapi_spec: Option<String>,
    /// Enable CORS
    pub cors_enabled: bool,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            host: "0.0.0.0".to_string(),
            openapi_spec: None,
            cors_enabled: true,
            request_timeout_secs: 30,
        }
    }
}

/// WebSocket server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// Server port
    pub port: u16,
    /// Host address
    pub host: String,
    /// Replay file path
    pub replay_file: Option<String>,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            port: 3001,
            host: "0.0.0.0".to_string(),
            replay_file: None,
            connection_timeout_secs: 300,
        }
    }
}

/// gRPC server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
    /// Server port
    pub port: u16,
    /// Host address
    pub host: String,
    /// Proto files directory
    pub proto_dir: Option<String>,
    /// TLS configuration
    pub tls: Option<TlsConfig>,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            port: 50051,
            host: "0.0.0.0".to_string(),
            proto_dir: None,
            tls: None,
        }
    }
}

/// TLS configuration for gRPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Certificate file path
    pub cert_path: String,
    /// Private key file path
    pub key_path: String,
}

/// Admin UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    /// Enable admin UI
    pub enabled: bool,
    /// Admin UI port
    pub port: u16,
    /// Host address
    pub host: String,
    /// Authentication required
    pub auth_required: bool,
    /// Admin username (if auth required)
    pub username: Option<String>,
    /// Admin password (if auth required)
    pub password: Option<String>,
    /// Optional mount path to embed Admin UI under HTTP server (e.g., "/admin")
    pub mount_path: Option<String>,
    /// Enable Admin API endpoints (under `__mockforge`)
    pub api_enabled: bool,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 8080,
            host: "127.0.0.1".to_string(),
            auth_required: false,
            username: None,
            password: None,
            mount_path: None,
            api_enabled: true,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Enable JSON logging
    pub json_format: bool,
    /// Log file path (optional)
    pub file_path: Option<String>,
    /// Maximum log file size in MB
    pub max_file_size_mb: u64,
    /// Maximum number of log files to keep
    pub max_files: u32,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json_format: false,
            file_path: None,
            max_file_size_mb: 10,
            max_files: 5,
        }
    }
}

/// Data generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConfig {
    /// Default number of rows to generate
    pub default_rows: usize,
    /// Default output format
    pub default_format: String,
    /// Faker locale
    pub locale: String,
    /// Custom faker templates
    pub templates: HashMap<String, String>,
    /// RAG configuration
    pub rag: RagConfig,
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            default_rows: 100,
            default_format: "json".to_string(),
            locale: "en".to_string(),
            templates: HashMap::new(),
            rag: RagConfig::default(),
        }
    }
}

/// RAG configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    /// Enable RAG by default
    pub enabled: bool,
    /// API endpoint for LLM
    pub api_endpoint: Option<String>,
    /// API key for LLM
    pub api_key: Option<String>,
    /// Model name
    pub model: Option<String>,
    /// Context window size
    pub context_window: usize,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_endpoint: None,
            api_key: None,
            model: None,
            context_window: 4000,
        }
    }
}

/// Load configuration from file
pub async fn load_config<P: AsRef<Path>>(path: P) -> Result<ServerConfig> {
    let content = fs::read_to_string(&path)
        .await
        .map_err(|e| Error::generic(format!("Failed to read config file: {}", e)))?;

    let config: ServerConfig = if path.as_ref().extension().and_then(|s| s.to_str()) == Some("yaml")
        || path.as_ref().extension().and_then(|s| s.to_str()) == Some("yml")
    {
        serde_yaml::from_str(&content)
            .map_err(|e| Error::generic(format!("Failed to parse YAML config: {}", e)))?
    } else {
        serde_json::from_str(&content)
            .map_err(|e| Error::generic(format!("Failed to parse JSON config: {}", e)))?
    };

    Ok(config)
}

/// Save configuration to file
pub async fn save_config<P: AsRef<Path>>(path: P, config: &ServerConfig) -> Result<()> {
    let content = if path.as_ref().extension().and_then(|s| s.to_str()) == Some("yaml")
        || path.as_ref().extension().and_then(|s| s.to_str()) == Some("yml")
    {
        serde_yaml::to_string(config)
            .map_err(|e| Error::generic(format!("Failed to serialize config to YAML: {}", e)))?
    } else {
        serde_json::to_string_pretty(config)
            .map_err(|e| Error::generic(format!("Failed to serialize config to JSON: {}", e)))?
    };

    fs::write(path, content)
        .await
        .map_err(|e| Error::generic(format!("Failed to write config file: {}", e)))?;

    Ok(())
}

/// Load configuration with fallback to default
pub async fn load_config_with_fallback<P: AsRef<Path>>(path: P) -> ServerConfig {
    match load_config(&path).await {
        Ok(config) => {
            tracing::info!("Loaded configuration from {:?}", path.as_ref());
            config
        }
        Err(e) => {
            tracing::warn!(
                "Failed to load config from {:?}: {}. Using defaults.",
                path.as_ref(),
                e
            );
            ServerConfig::default()
        }
    }
}

/// Create default configuration file
pub async fn create_default_config<P: AsRef<Path>>(path: P) -> Result<()> {
    let config = ServerConfig::default();
    save_config(path, &config).await?;
    Ok(())
}

/// Environment variable overrides for configuration
pub fn apply_env_overrides(mut config: ServerConfig) -> ServerConfig {
    // HTTP server overrides
    if let Ok(port) = std::env::var("MOCKFORGE_HTTP_PORT") {
        if let Ok(port_num) = port.parse() {
            config.http.port = port_num;
        }
    }

    if let Ok(host) = std::env::var("MOCKFORGE_HTTP_HOST") {
        config.http.host = host;
    }

    // WebSocket server overrides
    if let Ok(port) = std::env::var("MOCKFORGE_WS_PORT") {
        if let Ok(port_num) = port.parse() {
            config.websocket.port = port_num;
        }
    }

    // gRPC server overrides
    if let Ok(port) = std::env::var("MOCKFORGE_GRPC_PORT") {
        if let Ok(port_num) = port.parse() {
            config.grpc.port = port_num;
        }
    }

    // Admin UI overrides
    if let Ok(port) = std::env::var("MOCKFORGE_ADMIN_PORT") {
        if let Ok(port_num) = port.parse() {
            config.admin.port = port_num;
        }
    }

    if std::env::var("MOCKFORGE_ADMIN_ENABLED").unwrap_or_default() == "true" {
        config.admin.enabled = true;
    }

    if let Ok(mount_path) = std::env::var("MOCKFORGE_ADMIN_MOUNT_PATH") {
        if !mount_path.trim().is_empty() {
            config.admin.mount_path = Some(mount_path);
        }
    }

    if let Ok(api_enabled) = std::env::var("MOCKFORGE_ADMIN_API_ENABLED") {
        let on = api_enabled == "1" || api_enabled.eq_ignore_ascii_case("true");
        config.admin.api_enabled = on;
    }

    // Logging overrides
    if let Ok(level) = std::env::var("MOCKFORGE_LOG_LEVEL") {
        config.logging.level = level;
    }

    config
}

/// Validate configuration
pub fn validate_config(config: &ServerConfig) -> Result<()> {
    // Validate port ranges
    if config.http.port == 0 {
        return Err(Error::generic("HTTP port cannot be 0"));
    }
    if config.websocket.port == 0 {
        return Err(Error::generic("WebSocket port cannot be 0"));
    }
    if config.grpc.port == 0 {
        return Err(Error::generic("gRPC port cannot be 0"));
    }
    if config.admin.port == 0 {
        return Err(Error::generic("Admin port cannot be 0"));
    }

    // Check for port conflicts
    let ports = [
        ("HTTP", config.http.port),
        ("WebSocket", config.websocket.port),
        ("gRPC", config.grpc.port),
        ("Admin", config.admin.port),
    ];

    for i in 0..ports.len() {
        for j in (i + 1)..ports.len() {
            if ports[i].1 == ports[j].1 {
                return Err(Error::generic(format!(
                    "Port conflict: {} and {} both use port {}",
                    ports[i].0, ports[j].0, ports[i].1
                )));
            }
        }
    }

    // Validate log level
    let valid_levels = ["trace", "debug", "info", "warn", "error"];
    if !valid_levels.contains(&config.logging.level.as_str()) {
        return Err(Error::generic(format!(
            "Invalid log level: {}. Valid levels: {}",
            config.logging.level,
            valid_levels.join(", ")
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.http.port, 3000);
        assert_eq!(config.websocket.port, 3001);
        assert_eq!(config.grpc.port, 50051);
        assert_eq!(config.admin.port, 8080);
    }

    #[test]
    fn test_config_validation() {
        let mut config = ServerConfig::default();
        assert!(validate_config(&config).is_ok());

        // Test port conflict
        config.websocket.port = config.http.port;
        assert!(validate_config(&config).is_err());

        // Test invalid log level
        config.websocket.port = 3001; // Fix port conflict
        config.logging.level = "invalid".to_string();
        assert!(validate_config(&config).is_err());
    }
}
