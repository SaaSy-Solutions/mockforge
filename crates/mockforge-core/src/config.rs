//! Configuration management for MockForge

use crate::{Config as CoreConfig, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

/// Authentication configuration for HTTP requests
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct BasicAuthConfig {
    /// Username/password pairs
    pub credentials: HashMap<String, String>,
}

/// API key configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Route configuration for custom HTTP routes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    /// Route path
    pub path: String,
    /// HTTP method
    pub method: String,
    /// Request configuration
    pub request: Option<RouteRequestConfig>,
    /// Response configuration
    pub response: RouteResponseConfig,
}

/// Request configuration for routes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRequestConfig {
    /// Request validation configuration
    pub validation: Option<RouteValidationConfig>,
}

/// Response configuration for routes
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct RouteValidationConfig {
    /// JSON schema for request validation
    pub schema: serde_json::Value,
}

/// Protocol enable/disable configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolConfig {
    /// Enable this protocol
    pub enabled: bool,
}

/// Protocols configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ServerConfig {
    /// HTTP server configuration
    pub http: HttpConfig,
    /// WebSocket server configuration
    pub websocket: WebSocketConfig,
    /// GraphQL server configuration
    pub graphql: GraphQLConfig,
    /// gRPC server configuration
    pub grpc: GrpcConfig,
    /// MQTT server configuration
    pub mqtt: MqttConfig,
    /// SMTP server configuration
    pub smtp: SmtpConfig,
    /// FTP server configuration
    pub ftp: FtpConfig,
    /// Kafka server configuration
    pub kafka: KafkaConfig,
    /// AMQP server configuration
    pub amqp: AmqpConfig,
    /// Admin UI configuration
    pub admin: AdminConfig,
    /// Request chaining configuration
    pub chaining: ChainingConfig,
    /// Core MockForge configuration
    pub core: CoreConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Data generation configuration
    pub data: DataConfig,
    /// Observability configuration (metrics, tracing)
    pub observability: ObservabilityConfig,
    /// Multi-tenant workspace configuration
    pub multi_tenant: crate::multi_tenant::MultiTenantConfig,
    /// Custom routes configuration
    #[serde(default)]
    pub routes: Vec<RouteConfig>,
    /// Protocol enable/disable configuration
    #[serde(default)]
    pub protocols: ProtocolsConfig,
    /// Named configuration profiles (dev, ci, demo, etc.)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub profiles: HashMap<String, ProfileConfig>,
}

/// Profile configuration - a partial ServerConfig that overrides base settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProfileConfig {
    /// HTTP server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http: Option<HttpConfig>,
    /// WebSocket server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub websocket: Option<WebSocketConfig>,
    /// GraphQL server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graphql: Option<GraphQLConfig>,
    /// gRPC server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grpc: Option<GrpcConfig>,
    /// MQTT server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mqtt: Option<MqttConfig>,
    /// SMTP server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp: Option<SmtpConfig>,
    /// FTP server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ftp: Option<FtpConfig>,
    /// Kafka server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kafka: Option<KafkaConfig>,
    /// AMQP server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amqp: Option<AmqpConfig>,
    /// Admin UI configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin: Option<AdminConfig>,
    /// Request chaining configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chaining: Option<ChainingConfig>,
    /// Core MockForge configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core: Option<CoreConfig>,
    /// Logging configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingConfig>,
    /// Data generation configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<DataConfig>,
    /// Observability configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observability: Option<ObservabilityConfig>,
    /// Multi-tenant workspace configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multi_tenant: Option<crate::multi_tenant::MultiTenantConfig>,
    /// Custom routes configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routes: Option<Vec<RouteConfig>>,
    /// Protocol enable/disable configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocols: Option<ProtocolsConfig>,
}

// Default is derived for ServerConfig

/// HTTP validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpValidationConfig {
    /// Request validation mode: off, warn, enforce
    pub mode: String,
}

/// HTTP CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

/// HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub validation_overrides: std::collections::HashMap<String, String>,
    /// When embedding Admin UI under HTTP, skip validation for the mounted prefix
    pub skip_admin_validation: bool,
    /// Authentication configuration
    pub auth: Option<AuthConfig>,
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
            }),
            request_timeout_secs: 30,
            validation: Some(HttpValidationConfig {
                mode: "enforce".to_string(),
            }),
            aggregate_validation_errors: true,
            validate_responses: false,
            response_template_expand: false,
            validation_status: None,
            validation_overrides: std::collections::HashMap::new(),
            skip_admin_validation: true,
            auth: None,
        }
    }
}

/// WebSocket server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct TlsConfig {
    /// Certificate file path
    pub cert_path: String,
    /// Private key file path
    pub key_path: String,
}

/// MQTT server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        }
    }
}

/// Admin UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        Self {
            enabled: false,
            port: 9080,
            host: "127.0.0.1".to_string(),
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

/// Request chaining configuration for multi-step request workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Data generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Observability configuration for metrics and distributed tracing
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

/// Prometheus metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        }
    }
}

/// Chaos engineering configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

    // SMTP server overrides
    if let Ok(port) = std::env::var("MOCKFORGE_SMTP_PORT") {
        if let Ok(port_num) = port.parse() {
            config.smtp.port = port_num;
        }
    }

    if let Ok(host) = std::env::var("MOCKFORGE_SMTP_HOST") {
        config.smtp.host = host;
    }

    if let Ok(enabled) = std::env::var("MOCKFORGE_SMTP_ENABLED") {
        config.smtp.enabled = enabled == "1" || enabled.eq_ignore_ascii_case("true");
    }

    if let Ok(hostname) = std::env::var("MOCKFORGE_SMTP_HOSTNAME") {
        config.smtp.hostname = hostname;
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

    if let Ok(prometheus_url) = std::env::var("PROMETHEUS_URL") {
        config.admin.prometheus_url = prometheus_url;
    }

    // Core configuration overrides
    if let Ok(latency_enabled) = std::env::var("MOCKFORGE_LATENCY_ENABLED") {
        let enabled = latency_enabled == "1" || latency_enabled.eq_ignore_ascii_case("true");
        config.core.latency_enabled = enabled;
    }

    if let Ok(failures_enabled) = std::env::var("MOCKFORGE_FAILURES_ENABLED") {
        let enabled = failures_enabled == "1" || failures_enabled.eq_ignore_ascii_case("true");
        config.core.failures_enabled = enabled;
    }

    if let Ok(overrides_enabled) = std::env::var("MOCKFORGE_OVERRIDES_ENABLED") {
        let enabled = overrides_enabled == "1" || overrides_enabled.eq_ignore_ascii_case("true");
        config.core.overrides_enabled = enabled;
    }

    if let Ok(traffic_shaping_enabled) = std::env::var("MOCKFORGE_TRAFFIC_SHAPING_ENABLED") {
        let enabled =
            traffic_shaping_enabled == "1" || traffic_shaping_enabled.eq_ignore_ascii_case("true");
        config.core.traffic_shaping_enabled = enabled;
    }

    // Traffic shaping overrides
    if let Ok(bandwidth_enabled) = std::env::var("MOCKFORGE_BANDWIDTH_ENABLED") {
        let enabled = bandwidth_enabled == "1" || bandwidth_enabled.eq_ignore_ascii_case("true");
        config.core.traffic_shaping.bandwidth.enabled = enabled;
    }

    if let Ok(max_bytes_per_sec) = std::env::var("MOCKFORGE_BANDWIDTH_MAX_BYTES_PER_SEC") {
        if let Ok(bytes) = max_bytes_per_sec.parse() {
            config.core.traffic_shaping.bandwidth.max_bytes_per_sec = bytes;
            config.core.traffic_shaping.bandwidth.enabled = true;
        }
    }

    if let Ok(burst_capacity) = std::env::var("MOCKFORGE_BANDWIDTH_BURST_CAPACITY_BYTES") {
        if let Ok(bytes) = burst_capacity.parse() {
            config.core.traffic_shaping.bandwidth.burst_capacity_bytes = bytes;
        }
    }

    if let Ok(burst_loss_enabled) = std::env::var("MOCKFORGE_BURST_LOSS_ENABLED") {
        let enabled = burst_loss_enabled == "1" || burst_loss_enabled.eq_ignore_ascii_case("true");
        config.core.traffic_shaping.burst_loss.enabled = enabled;
    }

    if let Ok(burst_probability) = std::env::var("MOCKFORGE_BURST_LOSS_PROBABILITY") {
        if let Ok(prob) = burst_probability.parse::<f64>() {
            config.core.traffic_shaping.burst_loss.burst_probability = prob.clamp(0.0, 1.0);
            config.core.traffic_shaping.burst_loss.enabled = true;
        }
    }

    if let Ok(burst_duration) = std::env::var("MOCKFORGE_BURST_LOSS_DURATION_MS") {
        if let Ok(ms) = burst_duration.parse() {
            config.core.traffic_shaping.burst_loss.burst_duration_ms = ms;
        }
    }

    if let Ok(loss_rate) = std::env::var("MOCKFORGE_BURST_LOSS_RATE") {
        if let Ok(rate) = loss_rate.parse::<f64>() {
            config.core.traffic_shaping.burst_loss.loss_rate_during_burst = rate.clamp(0.0, 1.0);
        }
    }

    if let Ok(recovery_time) = std::env::var("MOCKFORGE_BURST_LOSS_RECOVERY_MS") {
        if let Ok(ms) = recovery_time.parse() {
            config.core.traffic_shaping.burst_loss.recovery_time_ms = ms;
        }
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

/// Apply a profile to a base configuration
pub fn apply_profile(mut base: ServerConfig, profile: ProfileConfig) -> ServerConfig {
    // Macro to merge optional fields
    macro_rules! merge_field {
        ($field:ident) => {
            if let Some(override_val) = profile.$field {
                base.$field = override_val;
            }
        };
    }

    merge_field!(http);
    merge_field!(websocket);
    merge_field!(graphql);
    merge_field!(grpc);
    merge_field!(mqtt);
    merge_field!(smtp);
    merge_field!(ftp);
    merge_field!(kafka);
    merge_field!(amqp);
    merge_field!(admin);
    merge_field!(chaining);
    merge_field!(core);
    merge_field!(logging);
    merge_field!(data);
    merge_field!(observability);
    merge_field!(multi_tenant);
    merge_field!(routes);
    merge_field!(protocols);

    base
}

/// Load configuration with profile support
pub async fn load_config_with_profile<P: AsRef<Path>>(
    path: P,
    profile_name: Option<&str>,
) -> Result<ServerConfig> {
    // Use load_config_auto to support all formats
    let mut config = load_config_auto(&path).await?;

    // Apply profile if specified
    if let Some(profile) = profile_name {
        if let Some(profile_config) = config.profiles.remove(profile) {
            tracing::info!("Applying profile: {}", profile);
            config = apply_profile(config, profile_config);
        } else {
            return Err(Error::generic(format!(
                "Profile '{}' not found in configuration. Available profiles: {}",
                profile,
                config.profiles.keys().map(|k| k.as_str()).collect::<Vec<_>>().join(", ")
            )));
        }
    }

    // Clear profiles from final config to save memory
    config.profiles.clear();

    Ok(config)
}

/// Load configuration from TypeScript/JavaScript file
pub async fn load_config_from_js<P: AsRef<Path>>(path: P) -> Result<ServerConfig> {
    use rquickjs::{Context, Runtime};

    let content = fs::read_to_string(&path)
        .await
        .map_err(|e| Error::generic(format!("Failed to read JS/TS config file: {}", e)))?;

    // Create a JavaScript runtime
    let runtime = Runtime::new()
        .map_err(|e| Error::generic(format!("Failed to create JS runtime: {}", e)))?;
    let context = Context::full(&runtime)
        .map_err(|e| Error::generic(format!("Failed to create JS context: {}", e)))?;

    context.with(|ctx| {
        // For TypeScript files, we need to strip type annotations
        // This is a simple approach - for production, consider using a proper TS compiler
        let js_content = if path
            .as_ref()
            .extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext == "ts")
            .unwrap_or(false)
        {
            strip_typescript_types(&content)
        } else {
            content
        };

        // Evaluate the config file
        let result: rquickjs::Value = ctx
            .eval(js_content.as_bytes())
            .map_err(|e| Error::generic(format!("Failed to evaluate JS config: {}", e)))?;

        // Convert to JSON string
        let json_str: String = ctx
            .json_stringify(result)
            .map_err(|e| Error::generic(format!("Failed to stringify JS config: {}", e)))?
            .ok_or_else(|| Error::generic("JS config returned undefined"))?
            .get()
            .map_err(|e| Error::generic(format!("Failed to get JSON string: {}", e)))?;

        // Parse JSON into ServerConfig
        serde_json::from_str(&json_str).map_err(|e| {
            Error::generic(format!("Failed to parse JS config as ServerConfig: {}", e))
        })
    })
}

/// Simple TypeScript type stripper (removes type annotations)
/// Note: This is a basic implementation. For production use, consider using swc or esbuild
fn strip_typescript_types(content: &str) -> String {
    use regex::Regex;

    let mut result = content.to_string();

    // Remove interface declarations (handles multi-line)
    let interface_re = Regex::new(r"(?ms)interface\s+\w+\s*\{[^}]*\}\s*").unwrap();
    result = interface_re.replace_all(&result, "").to_string();

    // Remove type aliases
    let type_alias_re = Regex::new(r"(?m)^type\s+\w+\s*=\s*[^;]+;\s*").unwrap();
    result = type_alias_re.replace_all(&result, "").to_string();

    // Remove type annotations (: Type)
    let type_annotation_re = Regex::new(r":\s*[A-Z]\w*(<[^>]+>)?(\[\])?").unwrap();
    result = type_annotation_re.replace_all(&result, "").to_string();

    // Remove type imports and exports
    let type_import_re = Regex::new(r"(?m)^(import|export)\s+type\s+.*$").unwrap();
    result = type_import_re.replace_all(&result, "").to_string();

    // Remove as Type
    let as_type_re = Regex::new(r"\s+as\s+\w+").unwrap();
    result = as_type_re.replace_all(&result, "").to_string();

    result
}

/// Enhanced load_config that supports multiple formats including JS/TS
pub async fn load_config_auto<P: AsRef<Path>>(path: P) -> Result<ServerConfig> {
    let ext = path.as_ref().extension().and_then(|s| s.to_str()).unwrap_or("");

    match ext {
        "ts" | "js" => load_config_from_js(&path).await,
        "yaml" | "yml" | "json" => load_config(&path).await,
        _ => Err(Error::generic(format!(
            "Unsupported config file format: {}. Supported: .ts, .js, .yaml, .yml, .json",
            ext
        ))),
    }
}

/// Discover configuration file with support for all formats
pub async fn discover_config_file_all_formats() -> Result<std::path::PathBuf> {
    let current_dir = std::env::current_dir()
        .map_err(|e| Error::generic(format!("Failed to get current directory: {}", e)))?;

    let config_names = vec![
        "mockforge.config.ts",
        "mockforge.config.js",
        "mockforge.yaml",
        "mockforge.yml",
        ".mockforge.yaml",
        ".mockforge.yml",
    ];

    // Check current directory
    for name in &config_names {
        let path = current_dir.join(name);
        if tokio::fs::metadata(&path).await.is_ok() {
            return Ok(path);
        }
    }

    // Check parent directories (up to 5 levels)
    let mut dir = current_dir.clone();
    for _ in 0..5 {
        if let Some(parent) = dir.parent() {
            for name in &config_names {
                let path = parent.join(name);
                if tokio::fs::metadata(&path).await.is_ok() {
                    return Ok(path);
                }
            }
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }

    Err(Error::generic(
        "No configuration file found. Expected one of: mockforge.config.ts, mockforge.config.js, mockforge.yaml, mockforge.yml",
    ))
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
        assert_eq!(config.admin.port, 9080);
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

    #[test]
    fn test_apply_profile() {
        let mut base = ServerConfig::default();
        assert_eq!(base.http.port, 3000);

        let mut profile = ProfileConfig::default();
        profile.http = Some(HttpConfig {
            port: 8080,
            ..Default::default()
        });
        profile.logging = Some(LoggingConfig {
            level: "debug".to_string(),
            ..Default::default()
        });

        let merged = apply_profile(base, profile);
        assert_eq!(merged.http.port, 8080);
        assert_eq!(merged.logging.level, "debug");
        assert_eq!(merged.websocket.port, 3001); // Unchanged
    }

    #[test]
    fn test_strip_typescript_types() {
        let ts_code = r#"
interface Config {
    port: number;
    host: string;
}

const config: Config = {
    port: 3000,
    host: "localhost"
} as Config;
"#;

        let stripped = strip_typescript_types(ts_code);
        assert!(!stripped.contains("interface"));
        assert!(!stripped.contains(": Config"));
        assert!(!stripped.contains("as Config"));
    }
}
