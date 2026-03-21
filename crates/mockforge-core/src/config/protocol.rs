//! Protocol-specific configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::auth::AuthConfig;

/// HTTP validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HttpValidationConfig {
    /// Request validation mode: off, warn, enforce
    pub mode: String,
}

/// HTTP CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HttpCorsConfig {
    /// Enable CORS
    pub enabled: bool,
    /// Allowed origins
    #[serde(default)]
    pub allowed_origins: Vec<String>,
    /// Allowed methods
    #[serde(default)]
    pub allowed_methods: Vec<String>,
    /// Allowed headers
    #[serde(default)]
    pub allowed_headers: Vec<String>,
    /// Allow credentials (cookies, authorization headers)
    /// Note: Cannot be true when using wildcard origin (*)
    #[serde(default = "default_cors_allow_credentials")]
    pub allow_credentials: bool,
}

fn default_cors_allow_credentials() -> bool {
    false
}

/// HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct HttpConfig {
    /// Enable HTTP server
    pub enabled: bool,
    /// Server port
    pub port: u16,
    /// Host address
    pub host: String,
    /// Path to OpenAPI spec file for HTTP server
    pub openapi_spec: Option<String>,
    /// CORS configuration
    pub cors: Option<HttpCorsConfig>,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Request validation configuration
    pub validation: Option<HttpValidationConfig>,
    /// Aggregate validation errors into JSON array
    pub aggregate_validation_errors: bool,
    /// Validate responses (warn-only logging)
    pub validate_responses: bool,
    /// Expand templating tokens in responses/examples
    pub response_template_expand: bool,
    /// Validation error HTTP status (e.g., 400 or 422)
    pub validation_status: Option<u16>,
    /// Per-route overrides: key "METHOD path" => mode (off/warn/enforce)
    pub validation_overrides: HashMap<String, String>,
    /// When embedding Admin UI under HTTP, skip validation for the mounted prefix
    pub skip_admin_validation: bool,
    /// Authentication configuration
    pub auth: Option<AuthConfig>,
    /// TLS/HTTPS configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<HttpTlsConfig>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 3000,
            host: "0.0.0.0".to_string(),
            openapi_spec: None,
            cors: Some(HttpCorsConfig {
                enabled: true,
                allowed_origins: vec!["*".to_string()],
                allowed_methods: vec![
                    "GET".to_string(),
                    "POST".to_string(),
                    "PUT".to_string(),
                    "DELETE".to_string(),
                    "PATCH".to_string(),
                    "OPTIONS".to_string(),
                ],
                allowed_headers: vec!["content-type".to_string(), "authorization".to_string()],
                allow_credentials: false, // Must be false when using wildcard origin
            }),
            request_timeout_secs: 30,
            validation: Some(HttpValidationConfig {
                mode: "enforce".to_string(),
            }),
            aggregate_validation_errors: true,
            validate_responses: false,
            response_template_expand: false,
            validation_status: None,
            validation_overrides: HashMap::new(),
            skip_admin_validation: true,
            auth: None,
            tls: None,
        }
    }
}

/// HTTP TLS/HTTPS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HttpTlsConfig {
    /// Enable TLS/HTTPS
    pub enabled: bool,
    /// Path to TLS certificate file (PEM format)
    pub cert_file: String,
    /// Path to TLS private key file (PEM format)
    pub key_file: String,
    /// Path to CA certificate file for mutual TLS (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_file: Option<String>,
    /// Minimum TLS version (default: "1.2")
    #[serde(default = "default_tls_min_version")]
    pub min_version: String,
    /// Cipher suites to use (default: safe defaults)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cipher_suites: Vec<String>,
    /// Require client certificate (mutual TLS)
    #[serde(default)]
    pub require_client_cert: bool,
    /// Mutual TLS mode: "off" (default), "optional", "required"
    #[serde(default = "default_mtls_mode")]
    pub mtls_mode: String,
}

fn default_mtls_mode() -> String {
    "off".to_string()
}

fn default_tls_min_version() -> String {
    "1.2".to_string()
}

impl Default for HttpTlsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cert_file: String::new(),
            key_file: String::new(),
            ca_file: None,
            min_version: "1.2".to_string(),
            cipher_suites: Vec::new(),
            require_client_cert: false,
            mtls_mode: "off".to_string(),
        }
    }
}

/// WebSocket server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct WebSocketConfig {
    /// Enable WebSocket server
    pub enabled: bool,
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
            enabled: true,
            port: 3001,
            host: "0.0.0.0".to_string(),
            replay_file: None,
            connection_timeout_secs: 300,
        }
    }
}

/// gRPC server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct GrpcConfig {
    /// Enable gRPC server
    pub enabled: bool,
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
            enabled: true,
            port: 50051,
            host: "0.0.0.0".to_string(),
            proto_dir: None,
            tls: None,
        }
    }
}

/// GraphQL server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct GraphQLConfig {
    /// Enable GraphQL server
    pub enabled: bool,
    /// Server port
    pub port: u16,
    /// Host address
    pub host: String,
    /// GraphQL schema file path (.graphql or .gql)
    pub schema_path: Option<String>,
    /// Handlers directory for custom resolvers
    pub handlers_dir: Option<String>,
    /// Enable GraphQL Playground UI
    pub playground_enabled: bool,
    /// Upstream GraphQL server URL for passthrough
    pub upstream_url: Option<String>,
    /// Enable introspection queries
    pub introspection_enabled: bool,
}

impl Default for GraphQLConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 4000,
            host: "0.0.0.0".to_string(),
            schema_path: None,
            handlers_dir: None,
            playground_enabled: true,
            upstream_url: None,
            introspection_enabled: true,
        }
    }
}

/// TLS configuration for gRPC
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TlsConfig {
    /// Certificate file path
    pub cert_path: String,
    /// Private key file path
    pub key_path: String,
}

/// MQTT server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MqttConfig {
    /// Enable MQTT server
    pub enabled: bool,
    /// Server port
    pub port: u16,
    /// Host address
    pub host: String,
    /// Maximum connections
    pub max_connections: usize,
    /// Maximum packet size
    pub max_packet_size: usize,
    /// Keep-alive timeout in seconds
    pub keep_alive_secs: u16,
    /// Directory containing fixture files
    pub fixtures_dir: Option<std::path::PathBuf>,
    /// Enable retained messages
    pub enable_retained_messages: bool,
    /// Maximum retained messages
    pub max_retained_messages: usize,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 1883,
            host: "0.0.0.0".to_string(),
            max_connections: 1000,
            max_packet_size: 268435456, // 256 MB
            keep_alive_secs: 60,
            fixtures_dir: None,
            enable_retained_messages: true,
            max_retained_messages: 10000,
        }
    }
}

/// SMTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SmtpConfig {
    /// Enable SMTP server
    pub enabled: bool,
    /// Server port
    pub port: u16,
    /// Host address
    pub host: String,
    /// Server hostname for SMTP greeting
    pub hostname: String,
    /// Directory containing fixture files
    pub fixtures_dir: Option<std::path::PathBuf>,
    /// Connection timeout in seconds
    pub timeout_secs: u64,
    /// Maximum connections
    pub max_connections: usize,
    /// Enable mailbox storage
    pub enable_mailbox: bool,
    /// Maximum mailbox size
    pub max_mailbox_messages: usize,
    /// Enable STARTTLS support
    pub enable_starttls: bool,
    /// Path to TLS certificate file
    pub tls_cert_path: Option<std::path::PathBuf>,
    /// Path to TLS private key file
    pub tls_key_path: Option<std::path::PathBuf>,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 1025,
            host: "0.0.0.0".to_string(),
            hostname: "mockforge-smtp".to_string(),
            fixtures_dir: Some(std::path::PathBuf::from("./fixtures/smtp")),
            timeout_secs: 300,
            max_connections: 10,
            enable_mailbox: true,
            max_mailbox_messages: 1000,
            enable_starttls: false,
            tls_cert_path: None,
            tls_key_path: None,
        }
    }
}

/// FTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FtpConfig {
    /// Enable FTP server
    pub enabled: bool,
    /// Server port
    pub port: u16,
    /// Host address
    pub host: String,
    /// Passive mode port range
    pub passive_ports: (u16, u16),
    /// Maximum connections
    pub max_connections: usize,
    /// Connection timeout in seconds
    pub timeout_secs: u64,
    /// Allow anonymous access
    pub allow_anonymous: bool,
    /// Fixtures directory
    pub fixtures_dir: Option<std::path::PathBuf>,
    /// Virtual root directory
    pub virtual_root: std::path::PathBuf,
}

impl Default for FtpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 2121,
            host: "0.0.0.0".to_string(),
            passive_ports: (50000, 51000),
            max_connections: 100,
            timeout_secs: 300,
            allow_anonymous: true,
            fixtures_dir: None,
            virtual_root: std::path::PathBuf::from("/mockforge"),
        }
    }
}

/// Kafka server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct KafkaConfig {
    /// Enable Kafka server
    pub enabled: bool,
    /// Server port
    pub port: u16,
    /// Host address
    pub host: String,
    /// Broker ID
    pub broker_id: i32,
    /// Maximum connections
    pub max_connections: usize,
    /// Log retention time in milliseconds
    pub log_retention_ms: i64,
    /// Log segment size in bytes
    pub log_segment_bytes: i64,
    /// Fixtures directory
    pub fixtures_dir: Option<std::path::PathBuf>,
    /// Auto-create topics
    pub auto_create_topics: bool,
    /// Default number of partitions for new topics
    pub default_partitions: i32,
    /// Default replication factor for new topics
    pub default_replication_factor: i16,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 9092, // Standard Kafka port
            host: "0.0.0.0".to_string(),
            broker_id: 1,
            max_connections: 1000,
            log_retention_ms: 604800000,   // 7 days
            log_segment_bytes: 1073741824, // 1 GB
            fixtures_dir: None,
            auto_create_topics: true,
            default_partitions: 3,
            default_replication_factor: 1,
        }
    }
}

/// AMQP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AmqpConfig {
    /// Enable AMQP server
    pub enabled: bool,
    /// Server port
    pub port: u16,
    /// Host address
    pub host: String,
    /// Maximum connections
    pub max_connections: usize,
    /// Maximum channels per connection
    pub max_channels_per_connection: u16,
    /// Frame max size
    pub frame_max: u32,
    /// Heartbeat interval in seconds
    pub heartbeat_interval: u16,
    /// Fixtures directory
    pub fixtures_dir: Option<std::path::PathBuf>,
    /// Virtual hosts
    pub virtual_hosts: Vec<String>,
    /// Enable TLS
    pub tls_enabled: bool,
    /// TLS port (5671 is standard AMQPS port)
    pub tls_port: u16,
    /// Path to TLS certificate file (PEM format)
    pub tls_cert_path: Option<std::path::PathBuf>,
    /// Path to TLS private key file (PEM format)
    pub tls_key_path: Option<std::path::PathBuf>,
    /// Path to CA certificate for client verification (optional)
    pub tls_ca_path: Option<std::path::PathBuf>,
    /// Require client certificate authentication
    pub tls_client_auth: bool,
}

impl Default for AmqpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 5672, // Standard AMQP port
            host: "0.0.0.0".to_string(),
            max_connections: 1000,
            max_channels_per_connection: 100,
            frame_max: 131072, // 128 KB
            heartbeat_interval: 60,
            fixtures_dir: None,
            virtual_hosts: vec!["/".to_string()],
            tls_enabled: false,
            tls_port: 5671, // Standard AMQPS port
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
            tls_client_auth: false,
        }
    }
}

/// TCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TcpConfig {
    /// Enable TCP server
    pub enabled: bool,
    /// Server port
    pub port: u16,
    /// Host address
    pub host: String,
    /// Maximum connections
    pub max_connections: usize,
    /// Connection timeout in seconds
    pub timeout_secs: u64,
    /// Directory containing fixture files
    pub fixtures_dir: Option<std::path::PathBuf>,
    /// Enable echo mode (echo received data back)
    pub echo_mode: bool,
    /// Enable TLS support
    pub enable_tls: bool,
    /// Path to TLS certificate file
    pub tls_cert_path: Option<std::path::PathBuf>,
    /// Path to TLS private key file
    pub tls_key_path: Option<std::path::PathBuf>,
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 9999,
            host: "0.0.0.0".to_string(),
            max_connections: 1000,
            timeout_secs: 300,
            fixtures_dir: Some(std::path::PathBuf::from("./fixtures/tcp")),
            echo_mode: true,
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
        }
    }
}

/// Admin UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
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
    /// Prometheus server URL for analytics queries
    pub prometheus_url: String,
}

impl Default for AdminConfig {
    fn default() -> Self {
        // Default to 0.0.0.0 if running in Docker (detected via common Docker env vars)
        // This makes Admin UI accessible from outside the container by default
        let default_host = if std::env::var("DOCKER_CONTAINER").is_ok()
            || std::env::var("container").is_ok()
            || Path::new("/.dockerenv").exists()
        {
            "0.0.0.0".to_string()
        } else {
            "127.0.0.1".to_string()
        };

        Self {
            enabled: false,
            port: 9080,
            host: default_host,
            auth_required: false,
            username: None,
            password: None,
            mount_path: None,
            api_enabled: true,
            prometheus_url: "http://localhost:9090".to_string(),
        }
    }
}

/// Protocol enable/disable configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProtocolConfig {
    /// Enable this protocol
    pub enabled: bool,
}

/// Protocols configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProtocolsConfig {
    /// HTTP protocol configuration
    pub http: ProtocolConfig,
    /// GraphQL protocol configuration
    pub graphql: ProtocolConfig,
    /// gRPC protocol configuration
    pub grpc: ProtocolConfig,
    /// WebSocket protocol configuration
    pub websocket: ProtocolConfig,
    /// SMTP protocol configuration
    pub smtp: ProtocolConfig,
    /// MQTT protocol configuration
    pub mqtt: ProtocolConfig,
    /// FTP protocol configuration
    pub ftp: ProtocolConfig,
    /// Kafka protocol configuration
    pub kafka: ProtocolConfig,
    /// RabbitMQ protocol configuration
    pub rabbitmq: ProtocolConfig,
    /// AMQP protocol configuration
    pub amqp: ProtocolConfig,
    /// TCP protocol configuration
    pub tcp: ProtocolConfig,
}

impl Default for ProtocolsConfig {
    fn default() -> Self {
        Self {
            http: ProtocolConfig { enabled: true },
            graphql: ProtocolConfig { enabled: true },
            grpc: ProtocolConfig { enabled: true },
            websocket: ProtocolConfig { enabled: true },
            smtp: ProtocolConfig { enabled: false },
            mqtt: ProtocolConfig { enabled: true },
            ftp: ProtocolConfig { enabled: false },
            kafka: ProtocolConfig { enabled: false },
            rabbitmq: ProtocolConfig { enabled: false },
            amqp: ProtocolConfig { enabled: false },
            tcp: ProtocolConfig { enabled: false },
        }
    }
}
