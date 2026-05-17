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
    /// Per-method response overrides. First matching rule wins; rules with no
    /// `match` block are catch-all rules.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overrides: Vec<GrpcOverride>,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 50051,
            host: "0.0.0.0".to_string(),
            proto_dir: None,
            tls: None,
            overrides: Vec::new(),
        }
    }
}

/// A single per-method override rule for the gRPC mock.
///
/// Use this to return specific status codes or response bodies from a method
/// without modifying the proto file. Rules are evaluated in declaration order;
/// the first one whose service+method (and optional `match`) match the
/// incoming request wins. Unmatched calls fall back to the default
/// smart-mock-generation behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GrpcOverride {
    /// Fully-qualified service name without leading dot, e.g. `myapp.OrderService`.
    /// May also be the unqualified service name; matching is exact.
    pub service: String,
    /// Method name (case-sensitive, matches proto definition).
    pub method: String,
    /// Optional request-field-equality match. Keys are top-level field names of
    /// the request message; values are stringified expected values. When
    /// omitted, the rule matches every call to the named method.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub r#match: HashMap<String, String>,
    /// Response to return when this rule fires.
    pub response: GrpcOverrideResponse,
}

/// Response shape for a `GrpcOverride`. Either `status` is set to a non-OK
/// gRPC status code (in which case the call returns an error with `message`),
/// or `body` is set to a JSON object that's serialized into the response
/// message. Setting both is allowed but `status` wins when non-OK.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct GrpcOverrideResponse {
    /// gRPC status code name (e.g. `OK`, `NOT_FOUND`, `PERMISSION_DENIED`).
    /// Case-insensitive. Defaults to `OK` when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Human-readable error message used when `status` is non-OK.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Response body as a JSON object. Field names must match the response
    /// message type from the proto. Ignored when `status` is non-OK.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
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
    /// Hostname returned in Kafka MetadataResponse so clients reach the
    /// broker after the bootstrap handshake. Defaults to `host`. On
    /// hosted-mock deployments the orchestrator sets this to the public
    /// `<app>.fly.dev` (or custom) hostname so external Kafka clients can
    /// route correctly. The mockforge-kafka broker itself must consume this
    /// value when constructing metadata responses for the wiring to be
    /// observable end-to-end (tracked in #231).
    pub advertised_host: Option<String>,
    /// Public port returned in Kafka MetadataResponse alongside
    /// `advertised_host`. Defaults to `port`. Useful when Fly maps a
    /// different public port (currently we keep them aligned at 9092).
    pub advertised_port: Option<u16>,
    /// Topic → messages map of records to inject at broker startup, before
    /// the broker accepts any client connections. A consumer reading from
    /// the beginning of a seeded topic sees these records at offset 0+.
    /// Topics referenced here are auto-created using `default_partitions`
    /// and `default_replication_factor`.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub seed_messages: HashMap<String, Vec<KafkaSeedMessage>>,
    /// Fault-injection rules applied at the protocol-handler boundary.
    /// Each rule fires either deterministically (no `probability`) or
    /// stochastically (`probability: 0.0..=1.0`). Rules with `partition`
    /// set target a single partition; without it, every partition of the
    /// named topic.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub faults: Vec<KafkaFault>,
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
            advertised_host: None,
            advertised_port: None,
            seed_messages: HashMap::new(),
            faults: Vec::new(),
        }
    }
}

/// Kafka fault-injection rule. See `KafkaConfig::faults`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct KafkaFault {
    /// Topic name to target. Required.
    pub topic: String,
    /// Specific partition to target. When omitted, the rule fires for any
    /// partition of the topic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition: Option<i32>,
    /// Which failure mode to inject.
    pub kind: KafkaFaultKind,
    /// Delay (in milliseconds) for `produce_throttle`. Ignored by other
    /// kinds. Defaults to 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<u64>,
    /// Stochastic firing probability in `0.0..=1.0`. `None` or `1.0` =
    /// every request matching this rule fires the fault. `0.0` = never.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub probability: Option<f64>,
}

/// Supported Kafka fault-injection kinds.
///
/// Each value matches a specific Kafka protocol error code the mock
/// returns on the matching request type. Tests are deterministic when
/// `probability` is None or 1.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum KafkaFaultKind {
    /// Delay produce response by `delay_ms` before allowing it through.
    /// Exercises client retry-and-backoff paths.
    ProduceThrottle,
    /// Return NOT_LEADER_OR_FOLLOWER (error code 6) for matching produce
    /// requests. Real clients re-fetch metadata and retry.
    ProduceNotLeader,
    /// Return OFFSET_OUT_OF_RANGE (error code 1) for matching fetch
    /// requests. Real consumers handle this by resetting to the
    /// configured `auto.offset.reset` policy.
    OffsetOutOfRange,
}

/// A single message to inject into a topic's log at broker startup. See
/// `KafkaConfig::seed_messages` for how these get wired in.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct KafkaSeedMessage {
    /// Optional record key. When present, the broker uses Kafka's
    /// hash-on-key strategy to assign a partition; same key always lands
    /// on the same partition. When absent, the round-robin counter picks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Record value. Stored verbatim — typically a JSON or text payload,
    /// but raw UTF-8 strings are fine for tests.
    pub value: String,
    /// Optional record headers. Same shape as on-the-wire Kafka headers.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
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
        // Bind dual-stack (`::`) when we detect we're in a container so the
        // admin port is reachable on both IPv4 and IPv6. The IPv6 side is what
        // Fly.io 6PN uses (`fdaa::/16` private network) to reach the admin
        // endpoint from sibling apps — bare `0.0.0.0` is IPv4-only on Linux
        // and breaks the cloud Resilience proxy (#468).
        //
        // On Linux the kernel's default `net.ipv6.bindv6only=0` means a `::`
        // listener also accepts IPv4 connections via IPv4-mapped IPv6
        // addresses (`::ffff:x.y.z.w`), so this single bind covers both.
        // Outside the container the default stays loopback for safety.
        let default_host = if std::env::var("DOCKER_CONTAINER").is_ok()
            || std::env::var("container").is_ok()
            || Path::new("/.dockerenv").exists()
        {
            "::".to_string()
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
