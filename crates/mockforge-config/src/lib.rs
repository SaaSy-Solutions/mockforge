//! Configuration types for MockForge
//!
//! This crate contains pure configuration data types used across the MockForge workspace.
//! It is a leaf crate with no internal MockForge dependencies, containing only structs
//! and enums that are serializable with serde.
//!
//! Types that require I/O, validation logic, or depend on core-specific types remain
//! in `mockforge-core`.

// These are data-only config structs; allow patterns standard for configuration types.
#![allow(
    clippy::doc_markdown,
    clippy::struct_excessive_bools,
    clippy::must_use_candidate,
    clippy::missing_const_for_fn,
    clippy::unreadable_literal,
    clippy::unnecessary_self_imports,
    clippy::return_self_not_must_use,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::use_self,
    clippy::derive_partial_eq_without_eq
)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Protocol Configs ────────────────────────────────────────────────────────

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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
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
            || std::path::Path::new("/.dockerenv").exists()
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

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
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

// ─── Auth Configs ────────────────────────────────────────────────────────────

/// Authentication configuration for HTTP requests
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct AuthConfig {
    /// JWT configuration
    pub jwt: Option<JwtConfig>,
    /// OAuth2 configuration
    pub oauth2: Option<OAuth2Config>,
    /// Basic auth configuration
    pub basic_auth: Option<BasicAuthConfig>,
    /// API key configuration
    pub api_key: Option<ApiKeyConfig>,
    /// Whether to require authentication for all requests
    pub require_auth: bool,
}

/// JWT authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct JwtConfig {
    /// JWT secret key for HMAC algorithms
    pub secret: Option<String>,
    /// RSA public key PEM for RSA algorithms
    pub rsa_public_key: Option<String>,
    /// ECDSA public key PEM for ECDSA algorithms
    pub ecdsa_public_key: Option<String>,
    /// Expected issuer
    pub issuer: Option<String>,
    /// Expected audience
    pub audience: Option<String>,
    /// Supported algorithms (defaults to HS256, RS256, ES256)
    pub algorithms: Vec<String>,
}

/// OAuth2 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct OAuth2Config {
    /// OAuth2 client ID
    pub client_id: String,
    /// OAuth2 client secret
    pub client_secret: String,
    /// Token introspection URL
    pub introspection_url: String,
    /// Authorization server URL
    pub auth_url: Option<String>,
    /// Token URL
    pub token_url: Option<String>,
    /// Expected token type
    pub token_type_hint: Option<String>,
}

/// Basic authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BasicAuthConfig {
    /// Username/password pairs
    pub credentials: HashMap<String, String>,
}

/// API key configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ApiKeyConfig {
    /// Expected header name (default: X-API-Key)
    pub header_name: String,
    /// Expected query parameter name
    pub query_name: Option<String>,
    /// Valid API keys
    pub keys: Vec<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt: None,
            oauth2: None,
            basic_auth: None,
            api_key: Some(ApiKeyConfig {
                header_name: "X-API-Key".to_string(),
                query_name: None,
                keys: vec![],
            }),
            require_auth: false,
        }
    }
}

// ─── Route Configs ───────────────────────────────────────────────────────────

/// Route configuration for custom HTTP routes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RouteConfig {
    /// Route path (supports path parameters like /users/{id})
    pub path: String,
    /// HTTP method
    pub method: String,
    /// Request configuration
    pub request: Option<RouteRequestConfig>,
    /// Response configuration
    pub response: RouteResponseConfig,
    /// Per-route fault injection configuration
    #[serde(default)]
    pub fault_injection: Option<RouteFaultInjectionConfig>,
    /// Per-route latency configuration
    #[serde(default)]
    pub latency: Option<RouteLatencyConfig>,
}

/// Request configuration for routes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RouteRequestConfig {
    /// Request validation configuration
    pub validation: Option<RouteValidationConfig>,
}

/// Response configuration for routes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RouteResponseConfig {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Option<serde_json::Value>,
}

/// Validation configuration for routes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RouteValidationConfig {
    /// JSON schema for request validation
    pub schema: serde_json::Value,
}

/// Per-route fault injection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RouteFaultInjectionConfig {
    /// Enable fault injection for this route
    pub enabled: bool,
    /// Probability of injecting a fault (0.0-1.0)
    pub probability: f64,
    /// Fault types to inject
    pub fault_types: Vec<RouteFaultType>,
}

/// Fault types that can be injected per route
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RouteFaultType {
    /// HTTP error with status code
    HttpError {
        /// HTTP status code to return
        status_code: u16,
        /// Optional error message
        message: Option<String>,
    },
    /// Connection error
    ConnectionError {
        /// Optional error message
        message: Option<String>,
    },
    /// Timeout error
    Timeout {
        /// Timeout duration in milliseconds
        duration_ms: u64,
        /// Optional error message
        message: Option<String>,
    },
    /// Partial response (truncate at percentage)
    PartialResponse {
        /// Percentage of response to truncate (0.0-100.0)
        truncate_percent: f64,
    },
    /// Payload corruption
    PayloadCorruption {
        /// Type of corruption to apply
        corruption_type: String,
    },
}

/// Per-route latency configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RouteLatencyConfig {
    /// Enable latency injection for this route
    pub enabled: bool,
    /// Probability of applying latency (0.0-1.0)
    pub probability: f64,
    /// Fixed delay in milliseconds
    pub fixed_delay_ms: Option<u64>,
    /// Random delay range (min_ms, max_ms)
    pub random_delay_range_ms: Option<(u64, u64)>,
    /// Jitter percentage (0.0-100.0)
    pub jitter_percent: f64,
    /// Latency distribution type
    #[serde(default)]
    pub distribution: LatencyDistribution,
}

/// Latency distribution type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum LatencyDistribution {
    /// Fixed delay
    #[default]
    Fixed,
    /// Normal distribution (requires mean and std_dev)
    Normal {
        /// Mean delay in milliseconds
        mean_ms: f64,
        /// Standard deviation in milliseconds
        std_dev_ms: f64,
    },
    /// Exponential distribution (requires lambda)
    Exponential {
        /// Lambda parameter for exponential distribution
        lambda: f64,
    },
    /// Uniform distribution (uses random_delay_range_ms)
    Uniform,
}

impl Default for RouteFaultInjectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            probability: 0.0,
            fault_types: Vec::new(),
        }
    }
}

impl Default for RouteLatencyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            probability: 1.0,
            fixed_delay_ms: None,
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Fixed,
        }
    }
}

// ─── Performance Configs ─────────────────────────────────────────────────────

/// Performance and resource configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[derive(Default)]
pub struct PerformanceConfig {
    /// Response compression configuration
    pub compression: CompressionConfig,
    /// Connection pooling configuration
    pub connection_pool: ConnectionPoolConfig,
    /// Request limits configuration
    pub request_limits: RequestLimitsConfig,
    /// Worker thread configuration
    pub workers: WorkerConfig,
    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfig,
}

/// Response compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct CompressionConfig {
    /// Enable response compression
    pub enabled: bool,
    /// Compression algorithm: gzip, deflate, br (brotli), zstd
    pub algorithm: String,
    /// Minimum response size to compress (bytes)
    pub min_size: usize,
    /// Compression level (1-9 for gzip/deflate, 0-11 for brotli, 1-22 for zstd)
    pub level: u32,
    /// Content types to compress (e.g., `["application/json", "text/html"]`)
    pub content_types: Vec<String>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithm: "gzip".to_string(),
            min_size: 1024, // 1KB
            level: 6,
            content_types: vec![
                "application/json".to_string(),
                "application/xml".to_string(),
                "text/plain".to_string(),
                "text/html".to_string(),
                "text/css".to_string(),
                "application/javascript".to_string(),
            ],
        }
    }
}

/// Connection pooling configuration for downstream services
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ConnectionPoolConfig {
    /// Maximum idle connections per host
    pub max_idle_per_host: usize,
    /// Maximum total connections
    pub max_connections: usize,
    /// Idle connection timeout in seconds
    pub idle_timeout_secs: u64,
    /// Connection acquire timeout in milliseconds
    pub acquire_timeout_ms: u64,
    /// Enable connection pooling
    pub enabled: bool,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_idle_per_host: 10,
            max_connections: 100,
            idle_timeout_secs: 90,
            acquire_timeout_ms: 5000,
            enabled: true,
        }
    }
}

/// Request limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct RequestLimitsConfig {
    /// Maximum request body size in bytes (default: 10MB)
    pub max_body_size: usize,
    /// Maximum header size in bytes
    pub max_header_size: usize,
    /// Maximum number of headers
    pub max_headers: usize,
    /// Maximum URI length
    pub max_uri_length: usize,
    /// Per-route body size limits (path pattern -> max bytes)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub per_route_limits: HashMap<String, usize>,
}

impl Default for RequestLimitsConfig {
    fn default() -> Self {
        Self {
            max_body_size: 10 * 1024 * 1024, // 10MB
            max_header_size: 16 * 1024,      // 16KB
            max_headers: 100,
            max_uri_length: 8192,
            per_route_limits: HashMap::new(),
        }
    }
}

/// Worker thread configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct WorkerConfig {
    /// Number of worker threads (0 = auto-detect based on CPU cores)
    pub threads: usize,
    /// Blocking thread pool size for CPU-intensive work
    pub blocking_threads: usize,
    /// Thread stack size in bytes
    pub stack_size: usize,
    /// Thread name prefix
    pub name_prefix: String,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            threads: 0, // auto-detect
            blocking_threads: 512,
            stack_size: 2 * 1024 * 1024, // 2MB
            name_prefix: "mockforge-worker".to_string(),
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct CircuitBreakerConfig {
    /// Enable circuit breaker
    pub enabled: bool,
    /// Failure threshold before opening circuit
    pub failure_threshold: u32,
    /// Success threshold before closing circuit
    pub success_threshold: u32,
    /// Half-open timeout in seconds (time before trying again after opening)
    pub half_open_timeout_secs: u64,
    /// Sliding window size for tracking failures
    pub window_size: u32,
    /// Per-endpoint circuit breaker configuration
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub per_endpoint: HashMap<String, EndpointCircuitBreakerConfig>,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            failure_threshold: 5,
            success_threshold: 2,
            half_open_timeout_secs: 30,
            window_size: 10,
            per_endpoint: HashMap::new(),
        }
    }
}

/// Per-endpoint circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EndpointCircuitBreakerConfig {
    /// Failure threshold for this endpoint
    pub failure_threshold: u32,
    /// Success threshold for this endpoint
    pub success_threshold: u32,
    /// Half-open timeout in seconds
    pub half_open_timeout_secs: u64,
}

// ─── Security Configs ────────────────────────────────────────────────────────

/// Secret backend provider type
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SecretBackendType {
    /// No secret backend (use environment variables directly)
    #[default]
    None,
    /// HashCorp Vault
    Vault,
    /// AWS Secrets Manager
    AwsSecretsManager,
    /// Azure Key Vault
    AzureKeyVault,
    /// Google Cloud Secret Manager
    GcpSecretManager,
    /// Kubernetes Secrets
    Kubernetes,
    /// Local encrypted file
    EncryptedFile,
}

/// Secret backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct SecretBackendConfig {
    /// Secret backend provider
    pub provider: SecretBackendType,
    /// Vault-specific configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault: Option<VaultConfig>,
    /// AWS Secrets Manager configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws: Option<AwsSecretsConfig>,
    /// Azure Key Vault configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub azure: Option<AzureKeyVaultConfig>,
    /// GCP Secret Manager configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcp: Option<GcpSecretManagerConfig>,
    /// Kubernetes secrets configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kubernetes: Option<KubernetesSecretsConfig>,
    /// Encrypted file configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_file: Option<EncryptedFileConfig>,
    /// Secret key mappings (config key -> secret path)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub mappings: HashMap<String, String>,
    /// Cache secrets in memory (seconds, 0 = no caching)
    pub cache_ttl_secs: u64,
    /// Retry configuration for secret retrieval
    pub retry_attempts: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
}

impl Default for SecretBackendConfig {
    fn default() -> Self {
        Self {
            provider: SecretBackendType::None,
            vault: None,
            aws: None,
            azure: None,
            gcp: None,
            kubernetes: None,
            encrypted_file: None,
            mappings: HashMap::new(),
            cache_ttl_secs: 300, // 5 minutes
            retry_attempts: 3,
            retry_delay_ms: 1000,
        }
    }
}

/// HashCorp Vault configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct VaultConfig {
    /// Vault server address
    pub address: String,
    /// Vault namespace (for enterprise)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Authentication method
    pub auth_method: VaultAuthMethod,
    /// Vault token (for token auth)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Role ID (for AppRole auth)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_id: Option<String>,
    /// Secret ID (for AppRole auth)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_id: Option<String>,
    /// Kubernetes role (for Kubernetes auth)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kubernetes_role: Option<String>,
    /// Secret engine mount path
    pub mount_path: String,
    /// Secret path prefix
    pub path_prefix: String,
    /// TLS CA certificate path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_cert_path: Option<String>,
    /// Skip TLS verification (not recommended for production)
    pub skip_verify: bool,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            address: "http://127.0.0.1:8200".to_string(),
            namespace: None,
            auth_method: VaultAuthMethod::Token,
            token: None,
            role_id: None,
            secret_id: None,
            kubernetes_role: None,
            mount_path: "secret".to_string(),
            path_prefix: "mockforge".to_string(),
            ca_cert_path: None,
            skip_verify: false,
            timeout_secs: 30,
        }
    }
}

/// Vault authentication methods
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum VaultAuthMethod {
    /// Token authentication
    #[default]
    Token,
    /// AppRole authentication
    AppRole,
    /// Kubernetes authentication
    Kubernetes,
    /// AWS IAM authentication
    AwsIam,
    /// GitHub authentication
    GitHub,
    /// LDAP authentication
    Ldap,
    /// Userpass authentication
    Userpass,
}

/// AWS Secrets Manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct AwsSecretsConfig {
    /// AWS region
    pub region: String,
    /// Secret name prefix
    pub prefix: String,
    /// Use IAM role (if false, uses access keys)
    pub use_iam_role: bool,
    /// AWS access key ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,
    /// AWS secret access key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_access_key: Option<String>,
    /// Endpoint URL (for LocalStack testing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint_url: Option<String>,
}

impl Default for AwsSecretsConfig {
    fn default() -> Self {
        Self {
            region: "us-east-1".to_string(),
            prefix: "mockforge".to_string(),
            use_iam_role: true,
            access_key_id: None,
            secret_access_key: None,
            endpoint_url: None,
        }
    }
}

/// Azure Key Vault configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct AzureKeyVaultConfig {
    /// Key Vault URL
    pub vault_url: String,
    /// Tenant ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Client ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// Client secret
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    /// Use managed identity
    pub use_managed_identity: bool,
    /// Secret name prefix
    pub prefix: String,
}

impl Default for AzureKeyVaultConfig {
    fn default() -> Self {
        Self {
            vault_url: String::new(),
            tenant_id: None,
            client_id: None,
            client_secret: None,
            use_managed_identity: true,
            prefix: "mockforge".to_string(),
        }
    }
}

/// GCP Secret Manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct GcpSecretManagerConfig {
    /// GCP project ID
    pub project_id: String,
    /// Secret name prefix
    pub prefix: String,
    /// Service account key file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials_file: Option<String>,
    /// Use default credentials (ADC)
    pub use_default_credentials: bool,
}

impl Default for GcpSecretManagerConfig {
    fn default() -> Self {
        Self {
            project_id: String::new(),
            prefix: "mockforge".to_string(),
            credentials_file: None,
            use_default_credentials: true,
        }
    }
}

/// Kubernetes Secrets configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct KubernetesSecretsConfig {
    /// Namespace to read secrets from
    pub namespace: String,
    /// Secret name prefix
    pub prefix: String,
    /// Label selector
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_selector: Option<String>,
    /// Use in-cluster config
    pub in_cluster: bool,
    /// Kubeconfig path (if not in-cluster)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kubeconfig_path: Option<String>,
}

impl Default for KubernetesSecretsConfig {
    fn default() -> Self {
        Self {
            namespace: "default".to_string(),
            prefix: "mockforge".to_string(),
            label_selector: None,
            in_cluster: true,
            kubeconfig_path: None,
        }
    }
}

/// Encrypted file configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct EncryptedFileConfig {
    /// Path to encrypted secrets file
    pub file_path: String,
    /// Encryption algorithm
    pub algorithm: String,
    /// Key derivation function
    pub kdf: String,
    /// Master key (from env var)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub master_key_env: Option<String>,
    /// Key file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_file: Option<String>,
}

impl Default for EncryptedFileConfig {
    fn default() -> Self {
        Self {
            file_path: "secrets.enc".to_string(),
            algorithm: "aes-256-gcm".to_string(),
            kdf: "argon2id".to_string(),
            master_key_env: Some("MOCKFORGE_MASTER_KEY".to_string()),
            key_file: None,
        }
    }
}

// ─── Behavioral Configs ─────────────────────────────────────────────────────

/// Behavioral cloning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct BehavioralCloningConfig {
    /// Whether behavioral cloning is enabled
    pub enabled: bool,
    /// Path to recorder database (defaults to ./recordings.db)
    pub database_path: Option<String>,
    /// Enable middleware to apply learned behavior
    pub enable_middleware: bool,
    /// Minimum frequency threshold for sequence learning (0.0 to 1.0)
    pub min_sequence_frequency: f64,
    /// Minimum requests per trace for sequence discovery
    pub min_requests_per_trace: Option<i32>,
    /// Flow recording configuration
    #[serde(default)]
    pub flow_recording: FlowRecordingConfig,
    /// Scenario replay configuration
    #[serde(default)]
    pub scenario_replay: ScenarioReplayConfig,
}

/// Flow recording configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct FlowRecordingConfig {
    /// Whether flow recording is enabled
    pub enabled: bool,
    /// How to group requests into flows (trace_id, session_id, ip_time_window)
    pub group_by: String,
    /// Time window in seconds for IP-based grouping
    pub time_window_seconds: u64,
}

impl Default for FlowRecordingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            group_by: "trace_id".to_string(),
            time_window_seconds: 300, // 5 minutes
        }
    }
}

/// Scenario replay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ScenarioReplayConfig {
    /// Whether scenario replay is enabled
    pub enabled: bool,
    /// Default replay mode (strict or flex)
    pub default_mode: String,
    /// List of scenario IDs to activate on startup
    pub active_scenarios: Vec<String>,
}

impl Default for ScenarioReplayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_mode: "strict".to_string(),
            active_scenarios: Vec::new(),
        }
    }
}

impl Default for BehavioralCloningConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            database_path: None,
            enable_middleware: false,
            min_sequence_frequency: 0.1, // 10% default
            min_requests_per_trace: None,
            flow_recording: FlowRecordingConfig::default(),
            scenario_replay: ScenarioReplayConfig::default(),
        }
    }
}

// ─── Contracts Configs ───────────────────────────────────────────────────────

/// Consumer contracts configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ConsumerContractsConfig {
    /// Whether consumer contracts are enabled
    pub enabled: bool,
    /// Auto-register consumers from requests
    pub auto_register: bool,
    /// Track field usage
    pub track_usage: bool,
}

/// Contracts configuration for fitness rules and contract management
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ContractsConfig {
    /// Fitness rules for contract validation
    pub fitness_rules: Vec<FitnessRuleConfig>,
}

/// Configuration for a fitness rule (YAML config format)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FitnessRuleConfig {
    /// Human-readable name for the fitness rule
    pub name: String,
    /// Scope where this rule applies (endpoint pattern, service name, or "global")
    pub scope: String,
    /// Type of fitness rule
    #[serde(rename = "type")]
    pub rule_type: FitnessRuleType,
    /// Maximum percent increase for response size (for response_size_delta type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_percent_increase: Option<f64>,
    /// Maximum number of fields (for field_count type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fields: Option<u32>,
    /// Maximum schema depth (for schema_complexity type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
}

/// Type of fitness rule (YAML config format)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum FitnessRuleType {
    /// Response size must not increase by more than max_percent_increase
    ResponseSizeDelta,
    /// No new required fields allowed
    NoNewRequiredFields,
    /// Field count must not exceed max_fields
    FieldCount,
    /// Schema complexity (depth) must not exceed max_depth
    SchemaComplexity,
}

// ─── Drift Learning Configs ──────────────────────────────────────────────────

/// Drift Learning configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct DriftLearningConfig {
    /// Enable or disable drift learning
    pub enabled: bool,
    /// Learning mode (behavioral, statistical, hybrid)
    #[serde(default)]
    pub mode: DriftLearningMode,
    /// How quickly mocks adapt to new patterns (0.0 - 1.0)
    #[serde(default = "default_learning_sensitivity")]
    pub sensitivity: f64,
    /// How quickly old patterns are forgotten (0.0 - 1.0)
    #[serde(default = "default_learning_decay")]
    pub decay: f64,
    /// Minimum number of samples required to learn a pattern
    #[serde(default = "default_min_samples")]
    pub min_samples: u64,
    /// Enable persona-specific behavior adaptation
    #[serde(default)]
    pub persona_adaptation: bool,
    /// Opt-in configuration for specific personas to learn
    #[serde(default)]
    pub persona_learning: HashMap<String, bool>, // persona_id -> enabled
    /// Opt-in configuration for specific endpoints to learn
    #[serde(default)]
    pub endpoint_learning: HashMap<String, bool>, // endpoint_pattern -> enabled
}

/// Drift learning mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum DriftLearningMode {
    /// Behavioral learning - adapts to behavior patterns
    #[default]
    Behavioral,
    /// Statistical learning - adapts to statistical patterns
    Statistical,
    /// Hybrid - combines behavioral and statistical
    Hybrid,
}

fn default_learning_sensitivity() -> f64 {
    0.2
}

fn default_learning_decay() -> f64 {
    0.05
}

fn default_min_samples() -> u64 {
    10
}

// ─── Deployment Configs ──────────────────────────────────────────────────────

/// Production-like CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProductionCorsConfig {
    /// Allowed origins (use "*" for all origins)
    #[serde(default)]
    pub allowed_origins: Vec<String>,
    /// Allowed HTTP methods
    #[serde(default)]
    pub allowed_methods: Vec<String>,
    /// Allowed headers (use "*" for all headers)
    #[serde(default)]
    pub allowed_headers: Vec<String>,
    /// Allow credentials (cookies, authorization headers)
    pub allow_credentials: bool,
}

/// Production-like rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProductionRateLimitConfig {
    /// Requests per minute allowed
    pub requests_per_minute: u32,
    /// Burst capacity (maximum requests in a short burst)
    pub burst: u32,
    /// Enable per-IP rate limiting
    pub per_ip: bool,
}

/// Production-like OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProductionOAuthConfig {
    /// OAuth2 client ID
    pub client_id: String,
    /// OAuth2 client secret
    pub client_secret: String,
    /// Token introspection URL
    pub introspection_url: String,
    /// Authorization server URL
    pub auth_url: Option<String>,
    /// Token URL
    pub token_url: Option<String>,
    /// Expected token type hint
    pub token_type_hint: Option<String>,
}

impl From<ProductionOAuthConfig> for OAuth2Config {
    /// Convert ProductionOAuthConfig to OAuth2Config for use in auth middleware
    fn from(prod: ProductionOAuthConfig) -> Self {
        OAuth2Config {
            client_id: prod.client_id,
            client_secret: prod.client_secret,
            introspection_url: prod.introspection_url,
            auth_url: prod.auth_url,
            token_url: prod.token_url,
            token_type_hint: prod.token_type_hint,
        }
    }
}

// ─── Plugin Configs ──────────────────────────────────────────────────────────

/// Plugin runtime resource configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct PluginResourceConfig {
    /// Enable plugin system
    pub enabled: bool,
    /// Maximum memory per plugin in bytes (default: 10MB)
    pub max_memory_per_plugin: usize,
    /// Maximum CPU usage per plugin (0.0-1.0, default: 0.5 = 50%)
    pub max_cpu_per_plugin: f64,
    /// Maximum execution time per plugin in milliseconds (default: 5000ms)
    pub max_execution_time_ms: u64,
    /// Allow plugins network access
    pub allow_network_access: bool,
    /// Filesystem paths plugins can access (empty = no fs access)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_fs_paths: Vec<String>,
    /// Maximum concurrent plugin executions
    pub max_concurrent_executions: usize,
    /// Plugin cache directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<String>,
    /// Enable debug logging for plugins
    pub debug_logging: bool,
    /// Maximum WASM module size in bytes (default: 5MB)
    pub max_module_size: usize,
    /// Maximum table elements per plugin
    pub max_table_elements: usize,
    /// Maximum WASM stack size in bytes (default: 2MB)
    pub max_stack_size: usize,
}

impl Default for PluginResourceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_memory_per_plugin: 10 * 1024 * 1024, // 10MB
            max_cpu_per_plugin: 0.5,                 // 50% of one core
            max_execution_time_ms: 5000,             // 5 seconds
            allow_network_access: false,
            allowed_fs_paths: Vec::new(),
            max_concurrent_executions: 10,
            cache_dir: None,
            debug_logging: false,
            max_module_size: 5 * 1024 * 1024, // 5MB
            max_table_elements: 1000,
            max_stack_size: 2 * 1024 * 1024, // 2MB
        }
    }
}

// ─── Hot Reload Configs ──────────────────────────────────────────────────────

/// Configuration hot-reload settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ConfigHotReloadConfig {
    /// Enable configuration hot-reload
    pub enabled: bool,
    /// Check interval in seconds
    pub check_interval_secs: u64,
    /// Debounce delay in milliseconds (prevent rapid reloads)
    pub debounce_delay_ms: u64,
    /// Paths to watch for changes (config files, fixture directories)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub watch_paths: Vec<String>,
    /// Reload on OpenAPI spec changes
    pub reload_on_spec_change: bool,
    /// Reload on fixture file changes
    pub reload_on_fixture_change: bool,
    /// Reload on plugin changes
    pub reload_on_plugin_change: bool,
    /// Graceful reload (wait for in-flight requests)
    pub graceful_reload: bool,
    /// Graceful reload timeout in seconds
    pub graceful_timeout_secs: u64,
    /// Validate config before applying reload
    pub validate_before_reload: bool,
    /// Rollback to previous config on reload failure
    pub rollback_on_failure: bool,
}

impl Default for ConfigHotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            check_interval_secs: 5,
            debounce_delay_ms: 1000,
            watch_paths: Vec::new(),
            reload_on_spec_change: true,
            reload_on_fixture_change: true,
            reload_on_plugin_change: true,
            graceful_reload: true,
            graceful_timeout_secs: 30,
            validate_before_reload: true,
            rollback_on_failure: true,
        }
    }
}

// ─── Chaining Config ─────────────────────────────────────────────────────────

/// Request chaining configuration for multi-step request workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default, rename_all = "camelCase")]
pub struct ChainingConfig {
    /// Enable request chaining
    pub enabled: bool,
    /// Maximum chain length to prevent infinite loops
    pub max_chain_length: usize,
    /// Global timeout for chain execution in seconds
    pub global_timeout_secs: u64,
    /// Enable parallel execution when dependencies allow
    pub enable_parallel_execution: bool,
}

impl Default for ChainingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_chain_length: 20,
            global_timeout_secs: 300,
            enable_parallel_execution: false,
        }
    }
}

// ─── Data Configs ────────────────────────────────────────────────────────────

/// Data generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
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
    /// Active persona profile domain (e.g., "finance", "ecommerce", "healthcare")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_domain: Option<String>,
    /// Enable persona-based consistency
    #[serde(default = "default_false")]
    pub persona_consistency_enabled: bool,
    /// Persona registry configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_registry: Option<PersonaRegistryConfig>,
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            default_rows: 100,
            default_format: "json".to_string(),
            locale: "en".to_string(),
            templates: HashMap::new(),
            rag: RagConfig::default(),
            persona_domain: None,
            persona_consistency_enabled: false,
            persona_registry: None,
        }
    }
}

/// RAG configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct RagConfig {
    /// Enable RAG by default
    pub enabled: bool,
    /// LLM provider (openai, anthropic, ollama, openai_compatible)
    #[serde(default)]
    pub provider: String,
    /// API endpoint for LLM
    pub api_endpoint: Option<String>,
    /// API key for LLM
    pub api_key: Option<String>,
    /// Model name
    pub model: Option<String>,
    /// Maximum tokens for generation
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    /// Temperature for generation (0.0 to 2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    /// Context window size
    pub context_window: usize,
    /// Enable caching
    #[serde(default = "default_true")]
    pub caching: bool,
    /// Cache TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_secs: u64,
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Maximum retries for failed requests
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
}

fn default_max_tokens() -> usize {
    1024
}

fn default_temperature() -> f64 {
    0.7
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_cache_ttl() -> u64 {
    3600
}

fn default_timeout() -> u64 {
    30
}

fn default_max_retries() -> usize {
    3
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "openai".to_string(),
            api_endpoint: None,
            api_key: None,
            model: Some("gpt-3.5-turbo".to_string()),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            context_window: 4000,
            caching: default_true(),
            cache_ttl_secs: default_cache_ttl(),
            timeout_secs: default_timeout(),
            max_retries: default_max_retries(),
        }
    }
}

/// Persona registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[derive(Default)]
pub struct PersonaRegistryConfig {
    /// Enable persistence (save personas to disk)
    #[serde(default = "default_false")]
    pub persistent: bool,
    /// Storage path for persistent personas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_path: Option<String>,
    /// Default traits for new personas
    #[serde(default)]
    pub default_traits: HashMap<String, String>,
}

// ─── Observability Configs ───────────────────────────────────────────────────

/// Prometheus metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct PrometheusConfig {
    /// Enable Prometheus metrics endpoint
    pub enabled: bool,
    /// Port for metrics endpoint
    pub port: u16,
    /// Host for metrics endpoint
    pub host: String,
    /// Path for metrics endpoint
    pub path: String,
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 9090,
            host: "0.0.0.0".to_string(),
            path: "/metrics".to_string(),
        }
    }
}

/// OpenTelemetry distributed tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct OpenTelemetryConfig {
    /// Enable OpenTelemetry tracing
    pub enabled: bool,
    /// Service name for traces
    pub service_name: String,
    /// Deployment environment (development, staging, production)
    pub environment: String,
    /// Jaeger endpoint for trace export
    pub jaeger_endpoint: String,
    /// OTLP endpoint (alternative to Jaeger)
    pub otlp_endpoint: Option<String>,
    /// Protocol: grpc or http
    pub protocol: String,
    /// Sampling rate (0.0 to 1.0)
    pub sampling_rate: f64,
}

impl Default for OpenTelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            service_name: "mockforge".to_string(),
            environment: "development".to_string(),
            jaeger_endpoint: "http://localhost:14268/api/traces".to_string(),
            otlp_endpoint: Some("http://localhost:4317".to_string()),
            protocol: "grpc".to_string(),
            sampling_rate: 1.0,
        }
    }
}

/// API Flight Recorder configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct RecorderConfig {
    /// Enable recording
    pub enabled: bool,
    /// Database file path
    pub database_path: String,
    /// Enable management API
    pub api_enabled: bool,
    /// Management API port (if different from main port)
    pub api_port: Option<u16>,
    /// Maximum number of requests to store (0 for unlimited)
    pub max_requests: i64,
    /// Auto-delete requests older than N days (0 to disable)
    pub retention_days: i64,
    /// Record HTTP requests
    pub record_http: bool,
    /// Record gRPC requests
    pub record_grpc: bool,
    /// Record WebSocket messages
    pub record_websocket: bool,
    /// Record GraphQL requests
    pub record_graphql: bool,
    /// Record proxied requests (requests that are forwarded to real backends)
    /// When enabled, proxied requests/responses will be recorded with metadata indicating proxy source
    #[serde(default = "default_true")]
    pub record_proxy: bool,
}

impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            database_path: "./mockforge-recordings.db".to_string(),
            api_enabled: true,
            api_port: None,
            max_requests: 10000,
            retention_days: 7,
            record_http: true,
            record_grpc: true,
            record_websocket: true,
            record_graphql: true,
            record_proxy: true,
        }
    }
}

/// Observability configuration for metrics and distributed tracing
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ObservabilityConfig {
    /// Prometheus metrics configuration
    pub prometheus: PrometheusConfig,
    /// OpenTelemetry distributed tracing configuration
    pub opentelemetry: Option<OpenTelemetryConfig>,
    /// API Flight Recorder configuration
    pub recorder: Option<RecorderConfig>,
    /// Chaos engineering configuration
    pub chaos: Option<ChaosEngConfig>,
}

// ─── Chaos Engineering Configs ───────────────────────────────────────────────

/// Chaos engineering configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ChaosEngConfig {
    /// Enable chaos engineering
    pub enabled: bool,
    /// Latency injection configuration
    pub latency: Option<LatencyInjectionConfig>,
    /// Fault injection configuration
    pub fault_injection: Option<FaultConfig>,
    /// Rate limiting configuration
    pub rate_limit: Option<RateLimitingConfig>,
    /// Traffic shaping configuration
    pub traffic_shaping: Option<NetworkShapingConfig>,
    /// Predefined scenario to use
    pub scenario: Option<String>,
}

/// Latency injection configuration for chaos engineering
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LatencyInjectionConfig {
    /// Enable latency injection
    pub enabled: bool,
    /// Fixed delay to inject (in milliseconds)
    pub fixed_delay_ms: Option<u64>,
    /// Random delay range (min_ms, max_ms) in milliseconds
    pub random_delay_range_ms: Option<(u64, u64)>,
    /// Jitter percentage to add variance to delays (0.0 to 1.0)
    pub jitter_percent: f64,
    /// Probability of injecting latency (0.0 to 1.0)
    pub probability: f64,
}

/// Fault injection configuration for chaos engineering
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FaultConfig {
    /// Enable fault injection
    pub enabled: bool,
    /// HTTP status codes to randomly return (e.g., [500, 502, 503])
    pub http_errors: Vec<u16>,
    /// Probability of returning HTTP errors (0.0 to 1.0)
    pub http_error_probability: f64,
    /// Enable connection errors (connection refused, reset, etc.)
    pub connection_errors: bool,
    /// Probability of connection errors (0.0 to 1.0)
    pub connection_error_probability: f64,
    /// Enable timeout errors
    pub timeout_errors: bool,
    /// Timeout duration in milliseconds
    pub timeout_ms: u64,
    /// Probability of timeout errors (0.0 to 1.0)
    pub timeout_probability: f64,
}

/// Rate limiting configuration for traffic control
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RateLimitingConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Maximum requests per second allowed
    pub requests_per_second: u32,
    /// Maximum burst size before rate limiting kicks in
    pub burst_size: u32,
    /// Apply rate limiting per IP address
    pub per_ip: bool,
    /// Apply rate limiting per endpoint/path
    pub per_endpoint: bool,
}

/// Network shaping configuration for simulating network conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct NetworkShapingConfig {
    /// Enable network shaping
    pub enabled: bool,
    /// Bandwidth limit in bits per second
    pub bandwidth_limit_bps: u64,
    /// Packet loss percentage (0.0 to 1.0)
    pub packet_loss_percent: f64,
    /// Maximum concurrent connections allowed
    pub max_connections: u32,
}

// ─── Incident Storage Config ─────────────────────────────────────────────────

/// Incident storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct IncidentStorageConfig {
    /// Use in-memory cache (default: true)
    pub use_cache: bool,
    /// Use database persistence (default: true)
    pub use_database: bool,
    /// Retention period for resolved incidents (days)
    pub retention_days: u32,
}

impl Default for IncidentStorageConfig {
    fn default() -> Self {
        Self {
            use_cache: true,
            use_database: true,
            retention_days: 90,
        }
    }
}

// ─── Reality Level ───────────────────────────────────────────────────────────

/// Reality level for mock environments (1-5)
///
/// Each level represents a different degree of realism, from simple static mocks
/// to full production-like chaos behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum RealityLevel {
    /// Level 1: Static Stubs - Simple, instant responses with no chaos
    StaticStubs = 1,
    /// Level 2: Light Simulation - Minimal latency, basic intelligence
    LightSimulation = 2,
    /// Level 3: Moderate Realism - Some chaos, moderate latency, full intelligence
    #[default]
    ModerateRealism = 3,
    /// Level 4: High Realism - Increased chaos, realistic latency, session state
    HighRealism = 4,
    /// Level 5: Production Chaos - Maximum chaos, production-like latency, full features
    ProductionChaos = 5,
}

impl RealityLevel {
    /// Get the numeric value (1-5)
    pub fn value(&self) -> u8 {
        *self as u8
    }

    /// Get a human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            RealityLevel::StaticStubs => "Static Stubs",
            RealityLevel::LightSimulation => "Light Simulation",
            RealityLevel::ModerateRealism => "Moderate Realism",
            RealityLevel::HighRealism => "High Realism",
            RealityLevel::ProductionChaos => "Production Chaos",
        }
    }

    /// Get a short description
    pub fn description(&self) -> &'static str {
        match self {
            RealityLevel::StaticStubs => "Simple, instant responses with no chaos",
            RealityLevel::LightSimulation => "Minimal latency, basic intelligence",
            RealityLevel::ModerateRealism => "Some chaos, moderate latency, full intelligence",
            RealityLevel::HighRealism => "Increased chaos, realistic latency, session state",
            RealityLevel::ProductionChaos => {
                "Maximum chaos, production-like latency, full features"
            }
        }
    }

    /// Create from numeric value (1-5)
    pub fn from_value(value: u8) -> Option<Self> {
        match value {
            1 => Some(RealityLevel::StaticStubs),
            2 => Some(RealityLevel::LightSimulation),
            3 => Some(RealityLevel::ModerateRealism),
            4 => Some(RealityLevel::HighRealism),
            5 => Some(RealityLevel::ProductionChaos),
            _ => None,
        }
    }

    /// Get all available levels
    pub fn all() -> Vec<Self> {
        vec![
            RealityLevel::StaticStubs,
            RealityLevel::LightSimulation,
            RealityLevel::ModerateRealism,
            RealityLevel::HighRealism,
            RealityLevel::ProductionChaos,
        ]
    }
}

/// Reality slider configuration for YAML config files
///
/// This is a simplified configuration that stores just the level.
/// The full RealityConfig with all subsystem settings is generated
/// automatically from the level via the RealityEngine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct RealitySliderConfig {
    /// Reality level (1-5)
    pub level: RealityLevel,
    /// Whether to enable reality slider (if false, uses individual subsystem configs)
    pub enabled: bool,
}

impl Default for RealitySliderConfig {
    fn default() -> Self {
        Self {
            level: RealityLevel::ModerateRealism,
            enabled: true,
        }
    }
}
