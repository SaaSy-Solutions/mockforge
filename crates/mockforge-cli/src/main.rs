use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;

mod ai_commands;
#[cfg(feature = "amqp")]
mod amqp_commands;
mod backend_generator;
mod blueprint_commands;
mod chaos_commands;
mod client_generator;
#[allow(dead_code)]
mod cloud_commands;
mod config_commands;
#[allow(dead_code)]
mod contract_diff_commands;
#[allow(dead_code)]
mod contract_sync_commands;
mod data_commands;
mod deploy_commands;
#[allow(dead_code, unexpected_cfgs)]
mod dev_setup_commands;
mod fixture_validation;
mod flow_commands;
#[cfg(feature = "ftp")]
mod ftp_commands;
mod generate_commands;
mod git_watch_commands;
#[allow(dead_code)]
mod governance_commands;
mod import_commands;
mod insomnia_import;
#[cfg(feature = "kafka")]
mod kafka_commands;
#[allow(dead_code)]
mod logs_commands;
mod mockai_commands;
mod mod_commands;
#[cfg(feature = "mqtt")]
mod mqtt_commands;
mod orchestrate_commands;
mod plugin_commands;
#[allow(dead_code)]
mod progress;
#[cfg(feature = "recorder")]
#[allow(dead_code)]
mod recorder_commands;
#[cfg(feature = "scenarios")]
mod scenario_commands;
mod schema;
mod serve;
#[cfg(feature = "smtp")]
#[allow(dead_code)]
mod smtp_commands;
mod snapshot_commands;
#[allow(dead_code)]
mod template_commands;
#[allow(dead_code)]
mod time_commands;
#[cfg(feature = "tunnel")]
mod tunnel_commands;
#[cfg(feature = "vbr")]
mod vbr_commands;
#[allow(dead_code)]
mod voice_commands;
#[allow(dead_code)]
mod wizard;
mod workspace_commands;

#[cfg(test)]
mod tests;

#[derive(Parser)]
#[command(name = "mockforge")]
#[command(about = "MockForge - Comprehensive API Mocking Framework")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Set log level (error, warn, info, debug, trace)
    #[arg(short = 'v', long, global = true, default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

/// Network port configuration for all supported protocols
#[derive(Args)]
struct PortArgs {
    /// HTTP server port (defaults to config or 3000)
    #[arg(long, help_heading = "Server Ports")]
    pub http_port: Option<u16>,

    /// WebSocket server port (defaults to config or 3001)
    #[arg(long, help_heading = "Server Ports")]
    pub ws_port: Option<u16>,

    /// gRPC server port (defaults to config or 50051)
    #[arg(long, help_heading = "Server Ports")]
    pub grpc_port: Option<u16>,

    /// SMTP server port (defaults to config or 1025)
    #[arg(long, help_heading = "Server Ports")]
    pub smtp_port: Option<u16>,

    /// MQTT server port (defaults to config or 1883)
    #[arg(long, help_heading = "Server Ports")]
    pub mqtt_port: Option<u16>,

    /// Kafka broker port (defaults to config or 9092)
    #[arg(long, help_heading = "Server Ports")]
    pub kafka_port: Option<u16>,

    /// AMQP broker port (defaults to config or 5672)
    #[arg(long, help_heading = "Server Ports")]
    pub amqp_port: Option<u16>,

    /// TCP server port (defaults to config or 9999)
    #[arg(long, help_heading = "Server Ports")]
    pub tcp_port: Option<u16>,

    /// GraphQL server port (defaults to config or 4000)
    #[arg(long, help_heading = "Server Ports")]
    pub graphql_port: Option<u16>,
}

/// TLS/HTTPS configuration
#[derive(Args)]
struct TlsArgs {
    /// Enable TLS/HTTPS
    #[arg(long, help_heading = "TLS/HTTPS")]
    pub tls_enabled: bool,

    /// Path to TLS certificate file (PEM format)
    #[arg(long, help_heading = "TLS/HTTPS")]
    pub tls_cert: Option<PathBuf>,

    /// Path to TLS private key file (PEM format)
    #[arg(long, help_heading = "TLS/HTTPS")]
    pub tls_key: Option<PathBuf>,

    /// Path to CA certificate file for mTLS (optional)
    #[arg(long, help_heading = "TLS/HTTPS")]
    pub tls_ca: Option<PathBuf>,

    /// Minimum TLS version (1.2 or 1.3, default: 1.2)
    #[arg(long, default_value = "1.2", help_heading = "TLS/HTTPS")]
    pub tls_min_version: String,

    /// Mutual TLS mode: off (default), optional, required
    #[arg(long, default_value = "off", help_heading = "TLS/HTTPS")]
    pub mtls: String,
}

/// Observability configuration (metrics + distributed tracing)
#[derive(Args)]
struct ObservabilityArgs {
    /// Enable Prometheus metrics endpoint
    #[arg(long, help_heading = "Observability & Metrics")]
    pub metrics: bool,

    /// Metrics server port (defaults to config or 9090)
    #[arg(long, help_heading = "Observability & Metrics")]
    pub metrics_port: Option<u16>,

    /// Enable OpenTelemetry distributed tracing
    #[arg(long, help_heading = "Tracing")]
    pub tracing: bool,

    /// Service name for traces
    #[arg(long, default_value = "mockforge", help_heading = "Tracing")]
    pub tracing_service_name: String,

    /// Tracing environment (development, staging, production)
    #[arg(long, default_value = "development", help_heading = "Tracing")]
    pub tracing_environment: String,

    /// Jaeger endpoint for trace export
    #[arg(
        long,
        default_value = "http://localhost:14268/api/traces",
        help_heading = "Tracing"
    )]
    pub jaeger_endpoint: String,

    /// Tracing sampling rate (0.0 to 1.0)
    #[arg(long, default_value = "1.0", help_heading = "Tracing")]
    pub tracing_sampling_rate: f64,
}

/// API Flight Recorder configuration
#[derive(Args)]
struct RecorderArgs {
    /// Enable API Flight Recorder
    #[arg(long, help_heading = "API Flight Recorder")]
    pub recorder: bool,

    /// Recorder database file path
    #[arg(
        long,
        default_value = "./mockforge-recordings.db",
        help_heading = "API Flight Recorder"
    )]
    pub recorder_db: String,

    /// Disable recorder management API
    #[arg(long, help_heading = "API Flight Recorder")]
    pub recorder_no_api: bool,

    /// Recorder management API port (defaults to main port)
    #[arg(long, help_heading = "API Flight Recorder")]
    pub recorder_api_port: Option<u16>,

    /// Maximum number of recorded requests (0 for unlimited)
    #[arg(long, default_value = "10000", help_heading = "API Flight Recorder")]
    pub recorder_max_requests: i64,

    /// Auto-delete recordings older than N days (0 to disable)
    #[arg(long, default_value = "7", help_heading = "API Flight Recorder")]
    pub recorder_retention_days: i64,
}

/// Chaos engineering, fault injection, and resilience pattern configuration
#[derive(Args)]
struct ChaosArgs {
    /// Enable chaos engineering (fault injection and reliability testing)
    #[arg(long, help_heading = "Chaos Engineering")]
    pub chaos: bool,

    /// Predefined chaos scenario: network_degradation, service_instability, cascading_failure, peak_traffic, slow_backend
    #[arg(long, help_heading = "Chaos Engineering")]
    pub chaos_scenario: Option<String>,

    /// Chaos latency: fixed delay in milliseconds
    #[arg(long, help_heading = "Chaos Engineering")]
    pub chaos_latency_ms: Option<u64>,

    /// Chaos latency: random delay range (min-max) in milliseconds
    #[arg(long, help_heading = "Chaos Engineering")]
    pub chaos_latency_range: Option<String>,

    /// Chaos latency probability (0.0-1.0)
    #[arg(long, default_value = "1.0", help_heading = "Chaos Engineering")]
    pub chaos_latency_probability: f64,

    /// Chaos fault injection: HTTP error codes (comma-separated)
    #[arg(long, help_heading = "Chaos Engineering")]
    pub chaos_http_errors: Option<String>,

    /// Chaos fault injection: HTTP error probability (0.0-1.0)
    #[arg(long, default_value = "0.1", help_heading = "Chaos Engineering")]
    pub chaos_http_error_probability: f64,

    /// Chaos rate limit: requests per second
    #[arg(long, help_heading = "Chaos Engineering")]
    pub chaos_rate_limit: Option<u32>,

    /// Chaos: bandwidth limit in bytes/sec (e.g., 10000 = 10KB/s for slow network simulation)
    #[arg(long, help_heading = "Chaos Engineering")]
    pub chaos_bandwidth_limit: Option<u64>,

    /// Chaos: packet loss percentage 0-100 (e.g., 5.0 = 5% packet loss)
    #[arg(long, help_heading = "Chaos Engineering")]
    pub chaos_packet_loss: Option<f64>,

    /// Enable gRPC-specific chaos engineering
    #[arg(long, help_heading = "Chaos Engineering - gRPC")]
    pub chaos_grpc: bool,

    /// gRPC chaos: status codes to inject (comma-separated)
    #[arg(long, help_heading = "Chaos Engineering - gRPC")]
    pub chaos_grpc_status_codes: Option<String>,

    /// gRPC chaos: stream interruption probability (0.0-1.0)
    #[arg(long, default_value = "0.1", help_heading = "Chaos Engineering - gRPC")]
    pub chaos_grpc_stream_interruption_probability: f64,

    /// Enable WebSocket-specific chaos engineering
    #[arg(long, help_heading = "Chaos Engineering - WebSocket")]
    pub chaos_websocket: bool,

    /// WebSocket chaos: close codes to inject (comma-separated)
    #[arg(long, help_heading = "Chaos Engineering - WebSocket")]
    pub chaos_websocket_close_codes: Option<String>,

    /// WebSocket chaos: message drop probability (0.0-1.0)
    #[arg(
        long,
        default_value = "0.05",
        help_heading = "Chaos Engineering - WebSocket"
    )]
    pub chaos_websocket_message_drop_probability: f64,

    /// WebSocket chaos: message corruption probability (0.0-1.0)
    #[arg(
        long,
        default_value = "0.05",
        help_heading = "Chaos Engineering - WebSocket"
    )]
    pub chaos_websocket_message_corruption_probability: f64,

    /// Enable GraphQL-specific chaos engineering
    #[arg(long, help_heading = "Chaos Engineering - GraphQL")]
    pub chaos_graphql: bool,

    /// GraphQL chaos: error codes to inject (comma-separated)
    #[arg(long, help_heading = "Chaos Engineering - GraphQL")]
    pub chaos_graphql_error_codes: Option<String>,

    /// GraphQL chaos: partial data probability (0.0-1.0)
    #[arg(
        long,
        default_value = "0.1",
        help_heading = "Chaos Engineering - GraphQL"
    )]
    pub chaos_graphql_partial_data_probability: f64,

    /// GraphQL chaos: enable resolver-level latency injection
    #[arg(long, help_heading = "Chaos Engineering - GraphQL")]
    pub chaos_graphql_resolver_latency: bool,

    /// Enable random chaos mode (randomly injects errors and delays)
    #[arg(long, help_heading = "Chaos Engineering - Random")]
    pub chaos_random: bool,

    /// Random chaos: error injection rate (0.0-1.0)
    #[arg(
        long,
        default_value = "0.1",
        help_heading = "Chaos Engineering - Random"
    )]
    pub chaos_random_error_rate: f64,

    /// Random chaos: delay injection rate (0.0-1.0)
    #[arg(
        long,
        default_value = "0.3",
        help_heading = "Chaos Engineering - Random"
    )]
    pub chaos_random_delay_rate: f64,

    /// Random chaos: minimum delay in milliseconds
    #[arg(
        long,
        default_value = "100",
        help_heading = "Chaos Engineering - Random"
    )]
    pub chaos_random_min_delay: u64,

    /// Random chaos: maximum delay in milliseconds
    #[arg(
        long,
        default_value = "2000",
        help_heading = "Chaos Engineering - Random"
    )]
    pub chaos_random_max_delay: u64,

    /// Apply a chaos network profile by name (e.g., slow_3g, flaky_wifi)
    #[arg(long, help_heading = "Chaos Engineering - Profiles")]
    pub chaos_profile: Option<String>,

    /// Enable circuit breaker pattern
    #[arg(long, help_heading = "Resilience Patterns")]
    pub circuit_breaker: bool,

    /// Circuit breaker: failure threshold
    #[arg(long, default_value = "5", help_heading = "Resilience Patterns")]
    pub circuit_breaker_failure_threshold: u64,

    /// Circuit breaker: success threshold
    #[arg(long, default_value = "2", help_heading = "Resilience Patterns")]
    pub circuit_breaker_success_threshold: u64,

    /// Circuit breaker: timeout in milliseconds
    #[arg(long, default_value = "60000", help_heading = "Resilience Patterns")]
    pub circuit_breaker_timeout_ms: u64,

    /// Circuit breaker: failure rate threshold percentage (0-100)
    #[arg(long, default_value = "50.0", help_heading = "Resilience Patterns")]
    pub circuit_breaker_failure_rate: f64,

    /// Enable bulkhead pattern
    #[arg(long, help_heading = "Resilience Patterns")]
    pub bulkhead: bool,

    /// Bulkhead: maximum concurrent requests
    #[arg(long, default_value = "100", help_heading = "Resilience Patterns")]
    pub bulkhead_max_concurrent: u32,

    /// Bulkhead: maximum queue size
    #[arg(long, default_value = "10", help_heading = "Resilience Patterns")]
    pub bulkhead_max_queue: u32,

    /// Bulkhead: queue timeout in milliseconds
    #[arg(long, default_value = "5000", help_heading = "Resilience Patterns")]
    pub bulkhead_queue_timeout_ms: u64,
}

/// Traffic shaping and network simulation configuration
#[derive(Args)]
struct TrafficArgs {
    /// Enable traffic shaping (bandwidth throttling and packet loss simulation)
    #[arg(long, help_heading = "Traffic Shaping")]
    pub traffic_shaping: bool,

    /// Maximum bandwidth in bytes per second (e.g., 1000000 = 1MB/s)
    #[arg(long, default_value = "1000000", help_heading = "Traffic Shaping")]
    pub bandwidth_limit: u64,

    /// Maximum burst size in bytes (allows temporary bursts above bandwidth limit)
    #[arg(long, default_value = "10000", help_heading = "Traffic Shaping")]
    pub burst_size: u64,

    /// Network condition profile (3g, 4g, 5g, satellite_leo, satellite_geo, congested, lossy, high_latency, intermittent, extremely_poor, perfect)
    #[arg(long, help_heading = "Network Profiles")]
    pub network_profile: Option<String>,

    /// List all available network profiles with descriptions
    #[arg(long, help_heading = "Network Profiles")]
    pub list_network_profiles: bool,
}

/// AI-powered features and reality simulation configuration
#[derive(Args)]
struct AiArgs {
    /// Enable AI-powered features
    #[arg(long, help_heading = "AI Features")]
    pub ai_enabled: bool,

    /// AI/RAG provider (openai, anthropic, ollama, openai_compatible)
    #[arg(long, help_heading = "AI Features")]
    pub rag_provider: Option<String>,

    /// AI/RAG model name
    #[arg(long, help_heading = "AI Features")]
    pub rag_model: Option<String>,

    /// AI/RAG API key (or set MOCKFORGE_RAG_API_KEY)
    #[arg(long, help_heading = "AI Features")]
    pub rag_api_key: Option<String>,

    /// Reality level (1-5) for unified realism control
    ///
    /// Controls chaos, latency, and MockAI behavior:
    ///   1 = Static Stubs (no chaos, instant, no AI)
    ///   2 = Light Simulation (minimal latency, basic AI)
    ///   3 = Moderate Realism (some chaos, moderate latency, full AI)
    ///   4 = High Realism (increased chaos, realistic latency, session state)
    ///   5 = Production Chaos (maximum chaos, production-like latency, full features)
    ///
    /// Can also be set via MOCKFORGE_REALITY_LEVEL environment variable.
    #[arg(long, help_heading = "Reality Slider")]
    pub reality_level: Option<u8>,
}

/// CLI arguments for the serve command (extracted to reduce enum size and prevent stack overflow)
///
/// Grouped into logical sub-structs for navigability. CLI flags are unchanged —
/// `#[command(flatten)]` inlines all sub-struct args at the top level.
#[derive(Args)]
struct ServeCliArgs {
    /// Configuration file path
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Configuration profile to use (dev, ci, demo, etc.)
    #[arg(short, long)]
    pub profile: Option<String>,

    /// OpenAPI spec file(s) for HTTP server (can be repeated multiple times)
    #[arg(short, long, help_heading = "Server Configuration", action = clap::ArgAction::Append)]
    pub spec: Vec<PathBuf>,

    /// Directory containing OpenAPI spec files (discovers .json, .yaml, .yml files)
    #[arg(long, help_heading = "Server Configuration")]
    pub spec_dir: Option<PathBuf>,

    /// Conflict resolution strategy when merging multiple specs: error (default), first, last
    #[arg(long, default_value = "error", help_heading = "Server Configuration")]
    pub merge_conflicts: String,

    /// API versioning mode: none (default), info, path-prefix
    #[arg(long, default_value = "none", help_heading = "Server Configuration")]
    pub api_versioning: String,

    /// API base path prefix (e.g., "/api" or "/v2/api")
    ///
    /// Prepends this path to all API endpoint paths from the OpenAPI spec.
    /// If not specified, the base path is extracted from the OpenAPI spec's
    /// servers URL (e.g., "https://example.com/api" → "/api").
    ///
    /// The CLI option takes priority over the spec's base path.
    /// Use empty string "" to override and disable any base path.
    ///
    /// Example:
    ///   --base-path /api           (all routes served at /api/...)
    ///   --base-path /v2            (all routes served at /v2/...)
    ///   --base-path ""             (disable base path, use paths as-is)
    #[arg(long, value_name = "PATH", help_heading = "Server Configuration")]
    pub base_path: Option<String>,

    /// WebSocket replay file
    #[arg(long, help_heading = "Server Configuration")]
    pub ws_replay_file: Option<PathBuf>,

    /// GraphQL schema file (.graphql or .gql)
    #[arg(long, help_heading = "Server Configuration")]
    pub graphql: Option<PathBuf>,

    /// GraphQL upstream server URL for passthrough
    #[arg(long, help_heading = "Server Configuration")]
    pub graphql_upstream: Option<String>,

    /// Enable admin UI
    #[arg(long, help_heading = "Admin & UI")]
    pub admin: bool,

    /// Admin UI port (defaults to config or 9080)
    #[arg(long, help_heading = "Admin & UI")]
    pub admin_port: Option<u16>,

    /// Validate configuration and check port availability without starting servers
    #[arg(long, help_heading = "Validation")]
    pub dry_run: bool,

    /// Show progress indicators during server startup
    #[arg(long, help_heading = "Validation")]
    pub progress: bool,

    /// Enable verbose logging output
    #[arg(long, help_heading = "Validation")]
    pub verbose: bool,

    #[command(flatten)]
    pub ports: PortArgs,

    #[command(flatten)]
    pub tls: TlsArgs,

    #[command(flatten)]
    pub observability: ObservabilityArgs,

    #[command(flatten)]
    pub recorder_opts: RecorderArgs,

    #[command(flatten)]
    pub chaos_opts: ChaosArgs,

    #[command(flatten)]
    pub traffic: TrafficArgs,

    #[command(flatten)]
    pub ai: AiArgs,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
enum Commands {
    /// Start mock servers (HTTP, WebSocket, gRPC)
    ///
    /// Examples:
    ///   mockforge serve --config mockforge.yaml
    ///   mockforge serve --http-port 8080 --admin --metrics
    ///   mockforge serve --chaos --chaos-scenario network_degradation --chaos-latency-ms 200
    ///   mockforge serve --traffic-shaping --bandwidth-limit 500000 --burst-size 50000
    #[command(verbatim_doc_comment)]
    Serve(Box<ServeCliArgs>),

    /// SMTP server management and mailbox operations
    ///
    /// Examples:
    ///   mockforge smtp mailbox list
    ///   mockforge smtp mailbox show email-123
    ///   mockforge smtp mailbox clear
    ///   mockforge smtp fixtures list
    ///   mockforge smtp send --to user@example.com --subject "Test"
    #[cfg(feature = "smtp")]
    #[command(verbatim_doc_comment)]
    Smtp {
        #[command(subcommand)]
        smtp_command: smtp_commands::SmtpCommands,
    },

    #[cfg(feature = "mqtt")]
    /// MQTT broker management and topic operations
    ///
    /// Examples:
    ///   mockforge mqtt publish --topic "sensors/temp" --payload '{"temp": 22.5}'
    ///   mockforge mqtt subscribe --topic "sensors/#"
    ///   mockforge mqtt topics list
    ///   mockforge mqtt fixtures load ./fixtures/mqtt/
    #[command(verbatim_doc_comment)]
    Mqtt {
        #[command(subcommand)]
        mqtt_command: mqtt_commands::MqttCommands,
    },

    #[cfg(feature = "ftp")]
    /// FTP server management
    ///
    /// Examples:
    ///   mockforge ftp serve --port 2121
    ///   mockforge ftp fixtures load ./fixtures/ftp/
    ///   mockforge ftp vfs add /test.txt --content "Hello World"
    #[command(verbatim_doc_comment)]
    Ftp {
        #[command(subcommand)]
        ftp_command: ftp_commands::FtpCommands,
    },

    /// Kafka broker management and topic operations
    ///
    /// Examples:
    ///   mockforge kafka serve --port 9092
    ///   mockforge kafka produce --topic orders --value '{"id": "123"}'
    ///   mockforge kafka consume --topic orders --group test-group
    ///   mockforge kafka topic create orders --partitions 3
    #[cfg(feature = "kafka")]
    #[command(verbatim_doc_comment)]
    Kafka {
        #[command(subcommand)]
        kafka_command: kafka_commands::KafkaCommands,
    },

    #[cfg(feature = "amqp")]
    /// AMQP broker management and message operations
    ///
    /// Examples:
    ///   mockforge amqp serve --port 5672
    ///   mockforge amqp publish --exchange orders --routing-key "order.created" --body '{"id": "123"}'
    ///   mockforge amqp consume --queue orders.new
    ///   mockforge amqp exchange declare orders --type topic --durable
    #[command(verbatim_doc_comment)]
    Amqp {
        #[command(subcommand)]
        amqp_command: amqp_commands::AmqpCommands,
    },

    /// Generate synthetic data
    Data {
        #[command(subcommand)]
        data_command: data_commands::DataCommands,
    },

    /// Start admin UI only (standalone server)
    Admin {
        /// Admin UI port
        #[arg(short, long, default_value = "9080")]
        port: u16,

        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Start sync daemon for bidirectional workspace synchronization
    ///
    /// The sync daemon monitors a directory for .yaml/.yml file changes and automatically
    /// imports them into MockForge workspaces. Perfect for version control integration,
    /// team collaboration via Git, and file-based development workflows.
    ///
    /// Examples:
    ///   mockforge sync --workspace-dir ./workspaces
    ///   mockforge sync -w /path/to/git/repo/workspaces
    ///
    /// What you'll see:
    ///   • Real-time notifications when files are created, modified, or deleted
    ///   • Import success/failure status for each file
    ///   • Clear error messages if files can't be imported
    ///   • Informative startup message explaining what's monitored
    ///
    /// The daemon will continue running until you press Ctrl+C.
    #[command(verbatim_doc_comment)]
    Sync {
        /// Workspace directory to monitor for file changes
        #[arg(short, long)]
        workspace_dir: PathBuf,

        /// Configuration file path (optional)
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Quick REST mock mode - spin up instant mock API from JSON
    ///
    /// Perfect for rapid prototyping with zero configuration. Auto-detects routes
    /// from JSON keys and creates full CRUD endpoints instantly.
    ///
    /// Examples:
    ///   mockforge quick data.json
    ///   mockforge quick sample.json --port 4000
    ///   mockforge quick mock.json --admin --metrics
    ///
    /// JSON file structure:
    /// {
    ///   "users": [{"id": 1, "name": "Alice"}],
    ///   "posts": [{"id": 1, "title": "First Post"}]
    /// }
    ///
    /// Auto-generated routes:
    ///   GET    /users      - List all users
    ///   GET    /users/:id  - Get single user
    ///   POST   /users      - Create user
    ///   PUT    /users/:id  - Update user
    ///   DELETE /users/:id  - Delete user
    ///   (same for all root-level JSON keys)
    ///
    /// Supports dynamic data generation:
    ///   "$random.uuid", "$random.int", "$faker.name", "$faker.email", "$ai(prompt)"
    #[command(verbatim_doc_comment)]
    Quick {
        /// JSON file path containing mock data
        file: PathBuf,

        /// HTTP server port (defaults to 3000)
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Enable admin UI
        #[arg(long)]
        admin: bool,

        /// Admin UI port (defaults to 9080)
        #[arg(long, default_value = "9080")]
        admin_port: u16,

        /// Enable Prometheus metrics endpoint
        #[arg(long)]
        metrics: bool,

        /// Metrics server port (defaults to 9090)
        #[arg(long, default_value = "9090")]
        metrics_port: u16,

        /// Enable request logging
        #[arg(long)]
        logging: bool,

        /// Host to bind to (defaults to 127.0.0.1)
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Initialize a new MockForge project
    ///
    /// Examples:
    ///   mockforge init my-project
    ///   mockforge init . --blueprint ecommerce
    ///   mockforge init . --blueprint b2c-saas
    Init {
        /// Project name (defaults to current directory name)
        #[arg(default_value = ".")]
        name: String,

        /// Skip creating example files
        #[arg(long)]
        no_examples: bool,

        /// Create project from a blueprint (e.g., ecommerce, b2c-saas, banking-lite)
        #[arg(long)]
        blueprint: Option<String>,
    },

    /// Interactive getting started wizard
    ///
    /// Guides you through setting up your first MockForge project with
    /// auto-detection and sample mock generation.
    ///
    /// Examples:
    ///   mockforge wizard
    Wizard,

    /// Validate HTTP fixtures
    ///
    /// Validates fixture files in a directory or a single file.
    /// Supports both flat and nested fixture formats.
    ///
    /// Examples:
    ///   mockforge validate-fixtures --dir ./fixtures
    ///   mockforge validate-fixtures --file ./fixtures/auth-login.json
    ///   mockforge validate-fixtures --dir ./fixtures --verbose
    #[command(verbatim_doc_comment)]
    ValidateFixtures {
        /// Directory containing fixture files to validate
        #[arg(short, long, conflicts_with = "file")]
        dir: Option<PathBuf>,

        /// Single fixture file to validate
        #[arg(short, long, conflicts_with = "dir")]
        file: Option<PathBuf>,

        /// Show detailed output for all fixtures
        #[arg(long)]
        verbose: bool,
    },

    /// Generate mock servers from OpenAPI specifications
    ///
    /// Examples:
    ///   mockforge generate --spec openapi.yaml
    ///   mockforge generate --spec api.json --output ./generated
    ///   mockforge generate  # Uses mockforge.toml config
    #[command(verbatim_doc_comment)]
    Generate {
        /// Path to mockforge.toml configuration file
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// OpenAPI specification file (JSON or YAML)
        #[arg(short, long)]
        spec: Option<PathBuf>,

        /// Output directory path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Generate verbose output
        #[arg(long)]
        verbose: bool,

        /// Dry run (validate config without generating)
        #[arg(long)]
        dry_run: bool,

        /// Watch mode - regenerate when files change
        #[arg(long)]
        watch: bool,

        /// Watch debounce time in milliseconds
        #[arg(long, default_value = "500")]
        watch_debounce: u64,

        /// Show progress bar during generation
        #[arg(long)]
        progress: bool,
    },

    /// Generate JSON Schema for MockForge configuration files
    ///
    /// Generates JSON Schemas that can be used by IDEs and editors
    /// to provide autocomplete, validation, and documentation for
    /// mockforge.yaml, persona files, reality config, and blueprint files.
    ///
    /// Examples:
    ///   mockforge schema generate
    ///   mockforge schema generate --output schemas/
    ///   mockforge schema generate --type config --output mockforge-config.schema.json
    #[command(verbatim_doc_comment)]
    Schema {
        #[command(subcommand)]
        schema_command: Option<generate_commands::SchemaCommands>,
    },

    /// One-command frontend integration setup
    ///
    /// Sets up MockForge integration for frontend frameworks with:
    /// - Typed client generation from OpenAPI spec
    /// - Example hooks/composables/services
    /// - Environment configuration
    /// - SDK dependencies
    ///
    /// Examples:
    ///   mockforge dev-setup react
    ///   mockforge dev-setup vue --spec api.yaml
    ///   mockforge dev-setup next --base-url http://localhost:3000
    #[command(verbatim_doc_comment)]
    DevSetup {
        #[command(flatten)]
        args: dev_setup_commands::DevSetupArgs,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        config_command: config_commands::ConfigCommands,
    },

    /// Watch a Git repository for OpenAPI spec changes and auto-sync
    ///
    /// Monitors a Git repository for changes to OpenAPI specification files
    /// and automatically reloads mocks when changes are detected.
    ///
    /// Examples:
    ///   mockforge git-watch https://github.com/user/api-specs --spec-paths "specs/*.yaml"
    ///   mockforge git-watch https://github.com/user/api-specs --branch develop --poll-interval 30
    ///   mockforge git-watch https://github.com/user/api-specs --reload-command "mockforge serve --spec"
    #[command(verbatim_doc_comment)]
    GitWatch {
        /// Git repository URL (HTTPS or SSH)
        #[arg(value_name = "REPOSITORY_URL")]
        repository_url: String,

        /// Branch to watch (default: "main")
        #[arg(short, long, default_value = "main")]
        branch: Option<String>,

        /// Path(s) to OpenAPI spec files in the repository (supports glob patterns)
        /// Default: ["**/*.yaml", "**/*.json", "**/openapi*.yaml", "**/openapi*.json"]
        #[arg(short, long, value_name = "PATH")]
        spec_paths: Vec<String>,

        /// Polling interval in seconds (default: 60)
        #[arg(long, default_value = "60")]
        poll_interval: Option<u64>,

        /// Authentication token for private repositories
        #[arg(long, value_name = "TOKEN")]
        auth_token: Option<String>,

        /// Local cache directory for cloned repository (default: "./.mockforge-git-cache")
        #[arg(long, value_name = "DIR")]
        cache_dir: Option<PathBuf>,

        /// Command to execute when spec files change
        /// Spec file paths will be appended as arguments
        #[arg(long, value_name = "COMMAND")]
        reload_command: Option<String>,
    },

    /// Sync and validate mocks against Git-hosted OpenAPI specs
    ///
    /// Fetches OpenAPI specifications from a Git repository and validates
    /// that mocks conform to the contract. Can optionally update mocks to match specs.
    ///
    /// Examples:
    ///   mockforge contract-sync https://github.com/user/api-specs --mock-config mocks.yaml
    ///   mockforge contract-sync https://github.com/user/api-specs --branch develop --strict
    ///   mockforge contract-sync https://github.com/user/api-specs --update --output report.md
    #[command(verbatim_doc_comment)]
    ContractSync {
        /// Git repository URL (HTTPS or SSH)
        #[arg(value_name = "REPOSITORY_URL")]
        repository_url: String,

        /// Branch to sync from (default: "main")
        #[arg(short, long, default_value = "main")]
        branch: Option<String>,

        /// Path(s) to OpenAPI spec files in the repository (supports glob patterns)
        /// Default: ["**/*.yaml", "**/*.json", "**/openapi*.yaml", "**/openapi*.json"]
        #[arg(short, long, value_name = "PATH")]
        spec_paths: Vec<String>,

        /// Path to mock configuration file to validate/update
        #[arg(long, value_name = "FILE")]
        mock_config: Option<PathBuf>,

        /// Authentication token for private repositories
        #[arg(long, value_name = "TOKEN")]
        auth_token: Option<String>,

        /// Local cache directory for cloned repository (default: "./.mockforge-git-cache")
        #[arg(long, value_name = "DIR")]
        cache_dir: Option<PathBuf>,

        /// Use strict validation mode (fails on warnings)
        #[arg(long)]
        strict: bool,

        /// Output file path for validation report (optional)
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Update mock configuration to match Git specs
        #[arg(long)]
        update: bool,
    },

    /// AI-powered contract diff analysis
    ///
    /// Analyze front-end requests against backend contract specifications,
    /// detect mismatches, and generate correction proposals.
    ///
    /// Examples:
    ///   mockforge contract-diff analyze --spec api.yaml --request-path request.json
    ///   mockforge contract-diff analyze --spec api.yaml --capture-id abc123 --output results.json
    ///   mockforge contract-diff compare --old-spec old.yaml --new-spec new.yaml
    ///   mockforge contract-diff generate-patch --spec api.yaml --request-path request.json --output patch.json
    ///   mockforge contract-diff apply-patch --spec api.yaml --patch patch.json
    #[command(verbatim_doc_comment)]
    ContractDiff {
        #[command(subcommand)]
        diff_command: contract_diff_commands::ContractDiffCommands,
    },

    /// API governance and safety features
    ///
    /// Manage API change forecasting, semantic drift detection, and threat modeling.
    ///
    /// Examples:
    ///   mockforge governance forecast generate --window-days 90
    ///   mockforge governance semantic analyze --before old.yaml --after new.yaml --endpoint /api/users --method GET
    ///   mockforge governance threat assess --spec api.yaml
    ///   mockforge governance status
    #[command(verbatim_doc_comment)]
    Governance {
        #[command(subcommand)]
        gov_command: governance_commands::GovernanceCommands,
    },

    /// Import API specifications and generate mocks (OpenAPI, AsyncAPI)
    ///
    /// Examples:
    ///   mockforge import openapi ./specs/api.yaml
    ///   mockforge import openapi ./specs/api.json --output mocks.json --verbose
    ///   mockforge import asyncapi ./specs/events.yaml --protocol mqtt
    ///   mockforge import coverage ./specs/api.yaml
    #[command(verbatim_doc_comment)]
    Import {
        #[command(subcommand)]
        import_command: import_commands::ImportCommands,
    },

    /// Test AI-powered features
    TestAi {
        #[command(subcommand)]
        ai_command: ai_commands::AiTestCommands,
    },

    /// Plugin management
    Plugin {
        #[command(subcommand)]
        plugin_command: plugin_commands::PluginCommands,
    },

    /// Recorder management (stub mapping conversion)
    ///
    /// Convert recorded API interactions into replayable stub mappings (fixtures).
    ///
    /// Examples:
    ///   mockforge recorder convert --recording-id abc123 --output fixtures/user-api.yaml
    ///   mockforge recorder convert --input recordings.db --output fixtures/ --format yaml
    #[cfg(feature = "recorder")]
    Recorder {
        #[command(subcommand)]
        recorder_command: recorder_commands::RecorderCommands,
    },

    /// Flow recording and behavioral cloning
    ///
    /// Record multi-step flows, view timelines, and compile behavioral scenarios.
    ///
    /// Examples:
    ///   mockforge flow list
    ///   mockforge flow view <flow-id>
    ///   mockforge flow tag <flow-id> --name "checkout_success"
    ///   mockforge flow compile <flow-id> --scenario-name "checkout"
    Flow {
        #[command(subcommand)]
        flow_command: flow_commands::FlowCommands,
    },

    /// Scenario marketplace management
    ///
    /// Examples:
    ///   mockforge scenario install ./scenarios/ecommerce-store
    ///   mockforge scenario list
    ///   mockforge scenario search ecommerce
    ///   mockforge scenario use ecommerce-store
    #[cfg(feature = "scenarios")]
    #[command(verbatim_doc_comment)]
    Scenario {
        #[command(subcommand)]
        scenario_command: scenario_commands::ScenarioCommands,
    },

    /// Reality Profile Pack management
    ///
    /// Manage and apply reality profile packs that configure hyper-realistic mock behaviors.
    ///
    /// Examples:
    ///   mockforge reality-profile install ecommerce-peak-season
    ///   mockforge reality-profile list
    ///   mockforge reality-profile apply ecommerce-peak-season --workspace default
    #[cfg(feature = "scenarios")]
    #[command(verbatim_doc_comment)]
    RealityProfile {
        #[command(subcommand)]
        reality_profile_command: scenario_commands::RealityProfileCommands,
    },

    /// Behavioral Economics Engine management
    ///
    /// Configure behavior rules that make mocks react to pressure, load, pricing, and fraud.
    ///
    /// Examples:
    ///   mockforge behavior-rule add --name "latency-conversion" --condition latency --threshold 400 --action modify-conversion-rate --multiplier 0.8
    ///   mockforge behavior-rule list
    ///   mockforge behavior-rule enable
    #[cfg(feature = "scenarios")]
    #[command(verbatim_doc_comment)]
    BehaviorRule {
        #[command(subcommand)]
        behavior_rule_command: scenario_commands::BehaviorRuleCommands,
    },

    /// Drift Learning configuration
    ///
    /// Configure drift learning that allows mocks to learn from recorded traffic patterns.
    ///
    /// Examples:
    ///   mockforge drift-learning enable --sensitivity 0.2 --min-samples 10
    ///   mockforge drift-learning status
    ///   mockforge drift-learning disable
    #[cfg(feature = "scenarios")]
    #[command(verbatim_doc_comment)]
    DriftLearning {
        #[command(subcommand)]
        drift_learning_command: scenario_commands::DriftLearningCommands,
    },

    /// Template library management
    ///
    /// Manage shared templates with versioning and marketplace support.
    ///
    /// Examples:
    ///   mockforge template register --id user-profile --name "User Profile" --content "{{faker.name}}"
    ///   mockforge template list
    ///   mockforge template search user
    ///   mockforge template install user-profile --registry https://registry.mockforge.dev
    ///   mockforge template marketplace search payment --registry https://registry.mockforge.dev
    #[command(verbatim_doc_comment)]
    Template {
        #[command(subcommand)]
        template_command: template_commands::TemplateCommands,
    },

    /// Blueprint management - predefined app archetypes
    ///
    /// Blueprints provide opinionated "Golden Path" workflows with:
    /// - Pre-configured personas for different user types
    /// - Reality defaults optimized for the use case
    /// - Sample flows demonstrating common workflows
    /// - Playground collections for testing
    ///
    /// Examples:
    ///   mockforge blueprint list
    ///   mockforge blueprint create my-app --blueprint b2c-saas
    ///   mockforge blueprint info ecommerce
    #[command(verbatim_doc_comment)]
    Blueprint {
        #[command(subcommand)]
        blueprint_command: blueprint_commands::BlueprintCommands,
    },

    /// Client code generation for frontend frameworks
    ///
    /// Examples:
    ///   mockforge client generate --spec api.json --framework react --output ./generated
    ///   mockforge client generate --spec api.yaml --framework vue --base-url https://api.example.com
    ///   mockforge client list
    #[command(verbatim_doc_comment)]
    Client {
        #[command(subcommand)]
        client_command: client_generator::ClientCommand,
    },

    /// Backend server code generation from OpenAPI specifications
    ///
    /// Examples:
    ///   mockforge backend generate --spec api.json --backend rust --output ./my-backend
    ///   mockforge backend generate --spec api.yaml --backend rust --port 8080 --database postgres
    ///   mockforge backend list
    #[command(verbatim_doc_comment)]
    Backend {
        #[command(subcommand)]
        backend_command: backend_generator::BackendCommand,
    },

    /// Multi-tenant workspace management
    ///
    /// Examples:
    ///   mockforge workspace list
    ///   mockforge workspace create my-workspace --name "My Workspace"
    ///   mockforge workspace info my-workspace
    ///   mockforge workspace delete my-workspace
    #[command(verbatim_doc_comment)]
    Workspace {
        #[command(subcommand)]
        workspace_command: workspace_commands::WorkspaceCommands,
    },

    /// Cloud sync and collaboration commands
    ///
    /// Examples:
    ///   mockforge cloud login
    ///   mockforge cloud sync --workspace my-workspace
    ///   mockforge cloud workspace list
    ///   mockforge cloud team members --workspace my-workspace
    #[command(verbatim_doc_comment)]
    Cloud {
        #[command(subcommand)]
        cloud_command: cloud_commands::CloudCommands,
    },

    /// Authenticate with MockForge Cloud (alias for 'cloud login')
    ///
    /// Examples:
    ///   mockforge login
    ///   mockforge login --token <api-token>
    ///   mockforge login --provider github
    #[command(verbatim_doc_comment)]
    Login {
        /// API token for authentication
        #[arg(long)]
        token: Option<String>,

        /// OAuth provider (github, google)
        #[arg(long)]
        provider: Option<String>,

        /// Cloud service URL
        #[arg(long, default_value = "https://api.mockforge.dev")]
        service_url: String,
    },

    /// Expose local MockForge server via public URL (tunneling)
    ///
    /// Examples:
    ///   mockforge tunnel start --local-url http://localhost:3000
    ///   mockforge tunnel start --local-url http://localhost:3000 --subdomain my-api
    ///   mockforge tunnel status
    ///   mockforge tunnel stop
    ///   mockforge tunnel list
    #[cfg(feature = "tunnel")]
    #[command(verbatim_doc_comment)]
    Tunnel {
        #[command(subcommand)]
        tunnel_command: tunnel_commands::TunnelSubcommand,
    },

    /// Deploy mock APIs with production-like configuration (deceptive deploy)
    ///
    /// Examples:
    ///   mockforge deploy --config config.yaml --spec api.yaml
    ///   mockforge deploy --config config.yaml --auto-tunnel
    ///   mockforge deploy --production-preset
    ///   mockforge deploy status
    ///   mockforge deploy stop
    #[command(verbatim_doc_comment)]
    Deploy {
        #[command(subcommand)]
        deploy_command: deploy_commands::DeploySubcommand,
    },

    /// Virtual Backend Reality (VBR) engine management
    ///
    /// Create stateful mock servers with persistent databases, auto-generated
    /// CRUD APIs, and relationship endpoints.
    ///
    /// Examples:
    ///   mockforge vbr create entity User --fields id:string,name:string,email:string
    ///   mockforge vbr serve --port 3000 --storage sqlite
    ///   mockforge vbr manage entities list
    #[cfg(feature = "vbr")]
    #[command(verbatim_doc_comment)]
    Vbr {
        #[command(subcommand)]
        vbr_command: vbr_commands::VbrCommands,
    },

    /// MockAI (Behavioral Mock Intelligence) management
    ///
    /// AI-powered mock generation and response realism. Auto-generate rules
    /// from examples or OpenAPI specs, enable intelligent behavior for endpoints.
    ///
    /// Examples:
    ///   mockforge mockai learn --from-examples examples.json
    ///   mockforge mockai generate --from-openapi api.yaml
    ///   mockforge mockai enable --endpoint "/api/users"
    ///   mockforge mockai status
    #[command(verbatim_doc_comment)]
    Mockai {
        #[command(subcommand)]
        mockai_command: mockai_commands::MockAICommands,
    },

    /// Time travel and snapshot management
    ///
    /// Save and restore entire system states (across protocols, personas, and reality level).
    /// Enables point-in-time recovery and state management.
    ///
    /// Examples:
    ///   mockforge snapshot save "post-checkout-failure" --description "State after checkout failure"
    ///   mockforge snapshot load "post-checkout-failure"
    ///   mockforge snapshot list
    ///   mockforge snapshot info "post-checkout-failure"
    ///   mockforge snapshot delete "old-snapshot"
    #[command(verbatim_doc_comment)]
    Snapshot {
        #[command(subcommand)]
        snapshot_command: snapshot_commands::SnapshotCommands,
    },

    /// Voice + LLM Interface for conversational mock creation
    ///
    /// Build mocks conversationally using natural language commands powered by LLM.
    /// Supports both single-shot and interactive conversational modes.
    ///
    /// Examples:
    ///   mockforge voice create --command "Create a fake e-commerce API with 20 products" --output api.yaml
    ///   mockforge voice create --serve --port 3000
    ///   mockforge voice interactive
    #[command(verbatim_doc_comment)]
    Voice {
        #[command(subcommand)]
        voice_command: voice_commands::VoiceCommands,
    },

    /// Time travel / temporal simulation control
    ///
    /// Control virtual clock for testing time-dependent behavior. Requires
    /// MockForge server to be running with admin UI enabled.
    ///
    /// Examples:
    ///   mockforge time status
    ///   mockforge time enable --time "2025-01-01T00:00:00Z"
    ///   mockforge time advance 1month
    ///   mockforge time advance 2h
    ///   mockforge time set "2025-06-01T12:00:00Z"
    ///   mockforge time scale 2.0
    ///   mockforge time reset
    ///   mockforge time save "1-month-later" --description "Scenario after 1 month"
    ///   mockforge time load "1-month-later"
    ///   mockforge time list
    #[command(verbatim_doc_comment)]
    Time {
        #[command(subcommand)]
        time_command: time_commands::TimeCommands,
        /// Admin UI URL (default: http://localhost:9080)
        #[arg(long)]
        admin_url: Option<String>,
    },

    /// View logs from MockForge server or log files
    ///
    /// View request logs from a running MockForge server (via Admin API) or from log files.
    /// Supports filtering, following (like tail -f), and JSON output.
    ///
    /// Examples:
    ///   mockforge logs
    ///   mockforge logs -f
    ///   mockforge logs --method GET --path /api/users
    ///   mockforge logs --status 500 --limit 20
    ///   mockforge logs --file logs/mockforge.log -f
    ///   mockforge logs --json
    #[command(verbatim_doc_comment)]
    Logs {
        /// Admin UI URL (default: http://localhost:9080)
        #[arg(long)]
        admin_url: Option<String>,

        /// Read from log file instead of Admin API
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// Follow logs in real-time (like tail -f)
        #[arg(short = 'F', long)]
        follow: bool,

        /// Filter by HTTP method (GET, POST, etc.)
        #[arg(long)]
        method: Option<String>,

        /// Filter by path pattern
        #[arg(long)]
        path: Option<String>,

        /// Filter by status code
        #[arg(long)]
        status: Option<u16>,

        /// Limit number of log entries
        #[arg(short, long)]
        limit: Option<usize>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Configuration file path (used to find log file path)
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// MOD (Mock-Oriented Development) commands
    ///
    /// Mock-Oriented Development (MOD) is a methodology that places mocks at the center
    /// of the development workflow. Use MOD to design APIs, coordinate teams, and build
    /// with confidence.
    ///
    /// Examples:
    ///   mockforge mod init --template small-team
    ///   mockforge mod validate --contract contracts/api.yaml --target http://localhost:8080
    ///   mockforge mod review --contract contracts/api.yaml --mock http://localhost:3000 --implementation http://localhost:8080
    ///   mockforge mod generate --from-openapi contracts/api.yaml --output mocks/
    ///   mockforge mod templates
    #[command(verbatim_doc_comment)]
    Mod {
        #[command(subcommand)]
        mod_command: mod_commands::ModCommands,
    },

    /// Chaos engineering profile management
    ///
    /// Examples:
    ///   mockforge chaos profile apply slow_3g
    ///   mockforge chaos profile export slow_3g --format json
    ///   mockforge chaos profile import --file profile.json
    #[command(verbatim_doc_comment)]
    Chaos {
        #[command(subcommand)]
        chaos_command: chaos_commands::ChaosCommands,
    },

    /// Chaos experiment orchestration
    Orchestrate {
        #[command(subcommand)]
        orchestrate_command: orchestrate_commands::OrchestrateCommands,
    },

    /// Generate tests from recorded API interactions
    ///
    /// Examples:
    ///   mockforge generate-tests --format rust_reqwest --output tests.rs
    ///   mockforge generate-tests --format k6 --protocol http --method GET --limit 20
    ///   mockforge generate-tests --format python_pytest --ai-descriptions --llm-provider openai
    ///   mockforge generate-tests --format postman --path "/api/users/*" --status-code 200
    #[command(verbatim_doc_comment)]
    GenerateTests {
        /// Recorder database file path
        #[arg(short, long, default_value = "./mockforge-recordings.db")]
        database: PathBuf,

        /// Test format (rust_reqwest, http_file, curl, postman, k6, python_pytest, javascript_jest, go_test)
        #[arg(short, long, default_value = "rust_reqwest")]
        format: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Filter by protocol (http, grpc, websocket, graphql)
        #[arg(long)]
        protocol: Option<String>,

        /// Filter by HTTP method (GET, POST, etc.)
        #[arg(long)]
        method: Option<String>,

        /// Filter by path pattern (supports wildcards)
        #[arg(long)]
        path: Option<String>,

        /// Filter by status code
        #[arg(long)]
        status_code: Option<u16>,

        /// Limit number of tests to generate
        #[arg(short, long, default_value = "50")]
        limit: usize,

        /// Test suite name
        #[arg(long, default_value = "generated_tests")]
        suite_name: String,

        /// Base URL for generated tests
        #[arg(long, default_value = "http://localhost:3000")]
        base_url: String,

        /// Use AI to generate intelligent test descriptions
        #[arg(long)]
        ai_descriptions: bool,

        /// LLM provider for AI descriptions (openai, ollama)
        #[arg(long, default_value = "ollama")]
        llm_provider: String,

        /// LLM model for AI descriptions
        #[arg(long, default_value = "llama2")]
        llm_model: String,

        /// LLM API endpoint
        #[arg(long)]
        llm_endpoint: Option<String>,

        /// LLM API key (for OpenAI, Anthropic)
        #[arg(long)]
        llm_api_key: Option<String>,

        /// Include body validation assertions
        #[arg(long, default_value = "true")]
        validate_body: bool,

        /// Include status code validation assertions
        #[arg(long, default_value = "true")]
        validate_status: bool,

        /// Include header validation assertions
        #[arg(long)]
        validate_headers: bool,

        /// Include timing validation assertions
        #[arg(long)]
        validate_timing: bool,

        /// Maximum duration threshold in ms for timing validation
        #[arg(long)]
        max_duration_ms: Option<u64>,
    },

    /// AI-powered API specification suggestion
    ///
    /// Generate complete OpenAPI specs or MockForge configs from minimal input.
    /// Provide a single endpoint example, API description, or partial spec, and
    /// MockForge will use AI to suggest additional endpoints and generate a
    /// complete specification.
    ///
    /// Examples:
    ///   mockforge suggest --from example.json --output openapi.yaml
    ///   mockforge suggest --from-description "A blog API with posts and comments" --format both
    ///   mockforge suggest --from example.json --num-suggestions 10 --domain e-commerce
    #[command(verbatim_doc_comment)]
    Suggest {
        /// Input file (JSON containing endpoint example, description, or partial spec)
        #[arg(short, long, conflicts_with = "from_description")]
        from: Option<PathBuf>,

        /// Generate from text description instead of file
        #[arg(long, conflicts_with = "from")]
        from_description: Option<String>,

        /// Output format (openapi, mockforge, both)
        #[arg(long, default_value = "openapi")]
        format: String,

        /// Output file path (without extension for 'both' format)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Number of additional endpoints to suggest
        #[arg(long, default_value = "5")]
        num_suggestions: usize,

        /// Include examples in generated specs
        #[arg(long, default_value = "true")]
        include_examples: bool,

        /// API domain hint (e-commerce, social-media, fintech, etc.)
        #[arg(long)]
        domain: Option<String>,

        /// LLM provider (openai, anthropic, ollama, openai-compatible)
        #[arg(long, default_value = "openai")]
        llm_provider: String,

        /// LLM model name
        #[arg(long)]
        llm_model: Option<String>,

        /// LLM API endpoint (for custom providers)
        #[arg(long)]
        llm_endpoint: Option<String>,

        /// LLM API key (or set OPENAI_API_KEY, ANTHROPIC_API_KEY, etc.)
        #[arg(long)]
        llm_api_key: Option<String>,

        /// Temperature for LLM generation (0.0-1.0)
        #[arg(long, default_value = "0.7")]
        temperature: f64,

        /// Print suggestions as JSON to stdout instead of saving
        #[arg(long)]
        print_json: bool,
    },

    /// Load test a real service using an API specification
    ///
    /// Examples:
    ///   mockforge bench --spec api.yaml --target https://api.example.com
    ///   mockforge bench --spec api.yaml --target https://staging.api.com --duration 5m --vus 100
    ///   mockforge bench --spec api.yaml --target https://api.com --scenario spike --output results/
    ///   mockforge bench --spec api.yaml --target https://api.com --operations "GET /users,POST /users"
    ///   mockforge bench --spec api.yaml --targets-file /path/to/targets.txt --max-concurrency 20
    ///   mockforge bench --spec api.yaml --target https://api.com --params-file params.json
    ///
    /// Multi-spec mode:
    ///   mockforge bench --spec pools.yaml --spec vs.yaml --target https://api.com
    ///   mockforge bench --spec-dir ./specs/ --target https://api.com
    ///   mockforge bench --spec a.yaml --spec b.yaml --merge-conflicts first --target https://api.com
    #[cfg(feature = "bench")]
    #[command(verbatim_doc_comment)]
    Bench {
        /// API specification file(s) (OpenAPI/Swagger). Can specify multiple.
        #[arg(short, long, action = clap::ArgAction::Append)]
        spec: Vec<PathBuf>,

        /// Directory containing OpenAPI spec files (discovers .json, .yaml, .yml files)
        #[arg(long)]
        spec_dir: Option<PathBuf>,

        /// Conflict resolution strategy when merging multiple specs: "error" (default), "first", "last"
        #[arg(long, default_value = "error")]
        merge_conflicts: String,

        /// Spec mode: "merge" (default) combines all specs, "sequential" runs them in order
        #[arg(long, default_value = "merge")]
        spec_mode: String,

        /// Dependency configuration file (YAML/JSON) for cross-spec value passing
        /// Only used when --spec-mode is "sequential"
        #[arg(long)]
        dependency_config: Option<PathBuf>,

        /// Target service URL (mutually exclusive with --targets-file)
        #[arg(short, long)]
        target: Option<String>,

        /// File containing multiple targets (one per line or JSON array)
        /// Mutually exclusive with --target. Supports absolute paths.
        #[arg(long)]
        targets_file: Option<PathBuf>,

        /// API base path prefix (e.g., "/api" or "/v2/api")
        ///
        /// Prepends this path to all API endpoint paths in the generated test.
        /// If not specified, the base path is extracted from the OpenAPI spec's
        /// servers URL (e.g., "https://example.com/api" → "/api").
        ///
        /// The CLI option takes priority over the spec's base path.
        /// Use empty string "" to override and disable any base path.
        ///
        /// Example:
        ///   --base-path /api           (all requests go to /api/...)
        ///   --base-path /v2            (all requests go to /v2/...)
        ///   --base-path ""             (disable base path, use paths as-is)
        #[arg(long, value_name = "PATH")]
        base_path: Option<String>,

        /// Test duration (e.g., 30s, 5m, 1h)
        #[arg(short, long, default_value = "1m")]
        duration: String,

        /// Number of virtual users (concurrent connections)
        #[arg(long, default_value = "10")]
        vus: u32,

        /// Load test scenario (constant, ramp-up, spike, stress, soak)
        #[arg(long, default_value = "ramp-up")]
        scenario: String,

        /// Filter operations to test (comma-separated, e.g., "GET /users,POST /users")
        #[arg(long)]
        operations: Option<String>,

        /// Exclude operations from testing (comma-separated)
        ///
        /// Supports "METHOD /path" or just "METHOD" to exclude all operations of that type.
        /// Examples:
        ///   --exclude-operations "DELETE"              (exclude all DELETE operations)
        ///   --exclude-operations "DELETE,POST"         (exclude all DELETE and POST)
        ///   --exclude-operations "DELETE /users/{id}"  (exclude specific operation)
        #[arg(long)]
        exclude_operations: Option<String>,

        /// Authentication header value (e.g., "Bearer token123")
        #[arg(long)]
        auth: Option<String>,

        /// Additional headers (format: "Key:Value,Key2:Value2")
        #[arg(long)]
        headers: Option<String>,

        /// Output directory for results
        #[arg(short, long, default_value = "bench-results")]
        output: PathBuf,

        /// Generate k6 script without running
        #[arg(long)]
        generate_only: bool,

        /// k6 script output path (when using --generate-only)
        #[arg(long)]
        script_output: Option<PathBuf>,

        /// Response time threshold percentile (p(50), p(75), p(90), p(95), p(99))
        #[arg(long, default_value = "p(95)")]
        threshold_percentile: String,

        /// Response time threshold in milliseconds
        #[arg(long, default_value = "500")]
        threshold_ms: u64,

        /// Maximum acceptable error rate (0.0-1.0)
        #[arg(long, default_value = "0.05")]
        max_error_rate: f64,

        /// Enable verbose output
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Skip TLS certificate validation (insecure, for test environments only)
        #[arg(long)]
        insecure: bool,

        /// Maximum number of parallel test executions (for multi-target mode)
        /// Only used when --targets-file is specified
        #[arg(long, default_value = "10")]
        max_concurrency: u32,

        /// Results format: "per-target", "aggregated", or "both" (default: "both")
        /// Only used when --targets-file is specified
        #[arg(long, default_value = "both")]
        results_format: String,

        /// Parameter values override file (JSON or YAML)
        ///
        /// Allows providing custom values for path parameters, query parameters,
        /// headers, and request bodies instead of auto-generated placeholder values.
        ///
        /// Example file format:
        /// {
        ///   "defaults": { "path_params": { "id": "123" } },
        ///   "operations": { "createUser": { "body": { "name": "Test" } } }
        /// }
        #[arg(long, value_name = "FILE")]
        params_file: Option<PathBuf>,

        // === CRUD Flow Options ===
        /// Enable CRUD flow mode (auto-detect from spec or use --flow-config)
        ///
        /// Automatically detects Create/Read/Update/Delete patterns from the OpenAPI spec
        /// and executes them as a sequential flow with response chaining.
        #[arg(long)]
        crud_flow: bool,

        /// Custom CRUD flow configuration file (YAML)
        ///
        /// Overrides auto-detection with explicit flow definition.
        /// See documentation for flow config format.
        #[arg(long, value_name = "FILE")]
        flow_config: Option<PathBuf>,

        /// Fields to extract from responses (comma-separated)
        ///
        /// Used in CRUD flow mode to chain values between requests.
        /// Example: --extract-fields "id,uuid,name"
        #[arg(long)]
        extract_fields: Option<String>,

        // === Parallel Execution Options ===
        /// Create N resources in parallel using http.batch()
        ///
        /// Executes POST requests in parallel batches for high-throughput testing.
        /// Example: --parallel-create 300
        #[arg(long, value_name = "N")]
        parallel_create: Option<u32>,

        // === Data-Driven Testing Options ===
        /// Test data file (CSV or JSON)
        ///
        /// Loads test data for data-driven testing using k6 SharedArray.
        /// CSV files should have headers matching field names.
        #[arg(long, value_name = "FILE")]
        data_file: Option<PathBuf>,

        /// Data distribution strategy
        ///
        /// How to distribute data across VUs and iterations:
        /// - unique-per-vu: Each VU gets unique row (default)
        /// - unique-per-iteration: Each iteration gets unique row
        /// - random: Random row selection
        /// - sequential: Sequential iteration through all rows
        #[arg(long, default_value = "unique-per-vu")]
        data_distribution: String,

        /// Data column to request field mappings
        ///
        /// Format: "column:target,column2:target2"
        /// Targets: body.field, path.param, query.param, header.name
        /// Example: --data-mappings "email:body.email,userId:path.id"
        #[arg(long)]
        data_mappings: Option<String>,

        // === Invalid Data Testing Options ===
        /// Percentage of requests to send with invalid data (0.0-1.0)
        ///
        /// Enables error testing by mixing valid and invalid requests.
        /// Example: --error-rate 0.2 for 20% invalid requests
        #[arg(long)]
        error_rate: Option<f64>,

        /// Types of invalid data to generate (comma-separated)
        ///
        /// Options: missing-field, wrong-type, empty, null, out-of-range, malformed
        /// Example: --error-types "missing-field,wrong-type,null"
        #[arg(long)]
        error_types: Option<String>,

        // === Security Testing Options ===
        /// Enable security payload injection testing
        ///
        /// Injects common attack payloads (SQLi, XSS, etc.) to test error handling.
        /// ONLY use against test/staging environments!
        #[arg(long)]
        security_test: bool,

        /// Custom security payloads file (JSON)
        ///
        /// Extends built-in payloads with custom attack patterns.
        /// See documentation for payload file format.
        #[arg(long, value_name = "FILE")]
        security_payloads: Option<PathBuf>,

        /// Security test categories (comma-separated)
        ///
        /// Options: sqli, xss, command-injection, path-traversal, ssti, ldap
        /// Example: --security-categories "sqli,xss"
        #[arg(long)]
        security_categories: Option<String>,

        /// Fields to target for security payload injection
        ///
        /// If not specified, injects into first string field.
        /// Example: --security-target-fields "name,email,query"
        #[arg(long)]
        security_target_fields: Option<String>,

        // === WAFBench Integration ===
        /// WAFBench test directory or glob pattern
        ///
        /// Load attack patterns from WAFBench YAML files (CRS rule sets).
        /// Supports glob patterns for selecting specific rule categories.
        ///
        /// Examples:
        ///   --wafbench-dir ./wafbench/REQUEST-941-*        (all XSS rules)
        ///   --wafbench-dir ./wafbench/REQUEST-942-*        (all SQLi rules)
        ///   --wafbench-dir ./wafbench/**/*.yaml            (all rules)
        ///
        /// See: https://github.com/microsoft/WAFBench
        #[arg(long, value_name = "PATH")]
        wafbench_dir: Option<String>,

        /// Cycle through ALL WAFBench payloads instead of random sampling
        ///
        /// By default, k6 randomly selects payloads for each request.
        /// With this flag, payloads are cycled through sequentially,
        /// ensuring all attack patterns are tested.
        #[arg(long)]
        wafbench_cycle_all: bool,

        // === OWASP API Security Top 10 Testing ===
        /// Enable OWASP API Security Top 10 (2023) testing mode
        ///
        /// Runs automated security tests for all 10 OWASP API security categories:
        /// API1: BOLA, API2: Auth, API3: Mass Assignment, API4: Rate Limiting,
        /// API5: Function Auth, API6: Business Logic, API7: SSRF,
        /// API8: Misconfiguration, API9: Inventory, API10: Unsafe Consumption
        ///
        /// ONLY use against test/staging environments!
        #[arg(long)]
        owasp_api_top10: bool,

        /// OWASP API categories to test (comma-separated)
        ///
        /// Options: api1, api2, api3, api4, api5, api6, api7, api8, api9, api10
        /// Also accepts aliases: bola, auth, ssrf, misconfig, etc.
        /// Default: all categories
        ///
        /// Example: --owasp-categories "api1,api2,api7"
        #[arg(long)]
        owasp_categories: Option<String>,

        /// Authorization header name for OWASP auth tests
        ///
        /// Default: "Authorization"
        #[arg(long, default_value = "Authorization")]
        owasp_auth_header: String,

        /// Valid authorization token for OWASP baseline requests
        ///
        /// Required for accurate auth bypass and BOLA testing.
        /// Example: --owasp-auth-token "Bearer your-token-here"
        #[arg(long)]
        owasp_auth_token: Option<String>,

        /// File containing admin/privileged paths to test
        ///
        /// One path per line. Used for API5 (Broken Function Authorization).
        /// Default: built-in list (/admin, /internal, etc.)
        #[arg(long, value_name = "FILE")]
        owasp_admin_paths: Option<PathBuf>,

        /// Fields containing resource IDs for BOLA testing
        ///
        /// Comma-separated list of field names that contain resource IDs.
        /// Default: id, uuid, user_id, userId, account_id, accountId
        ///
        /// Example: --owasp-id-fields "id,resourceId,orderId"
        #[arg(long)]
        owasp_id_fields: Option<String>,

        /// OWASP report output file
        ///
        /// Default: owasp-report.json
        #[arg(long, value_name = "FILE")]
        owasp_report: Option<PathBuf>,

        /// OWASP report format
        ///
        /// Options: json, sarif
        /// SARIF format integrates with IDEs and CI/CD tools.
        #[arg(long, default_value = "json")]
        owasp_report_format: String,

        /// Number of iterations per VU for OWASP tests
        ///
        /// Controls how many times each virtual user runs through
        /// the security tests. Default: 1
        ///
        /// Example: --owasp-iterations 5
        #[arg(long, default_value = "1")]
        owasp_iterations: u32,

        /// Run OpenAPI 3.0.0 conformance testing
        ///
        /// Generates and runs a comprehensive k6 script that exercises
        /// all OpenAPI 3.0.0 features (parameters, request bodies, schema types,
        /// composition, string formats, constraints, response codes, HTTP methods,
        /// content negotiation, and security schemes).
        ///
        /// Reports per-feature pass/fail results.
        ///
        /// Example: mockforge bench --conformance --target http://localhost:3000
        #[arg(long)]
        conformance: bool,

        /// API key for conformance security scheme tests
        ///
        /// Used to test API key authentication in conformance mode.
        ///
        /// Example: --conformance-api-key "my-api-key"
        #[arg(long)]
        conformance_api_key: Option<String>,

        /// Basic auth credentials for conformance security scheme tests
        ///
        /// Format: username:password
        ///
        /// Example: --conformance-basic-auth "admin:secret"
        #[arg(long)]
        conformance_basic_auth: Option<String>,

        /// Conformance report output file
        ///
        /// Default: conformance-report.json
        #[arg(long, value_name = "FILE", default_value = "conformance-report.json")]
        conformance_report: PathBuf,

        /// Conformance categories to test (comma-separated)
        ///
        /// Only run conformance tests for specific categories.
        /// Valid categories: parameters, request-bodies, schema-types, composition,
        /// string-formats, constraints, response-codes, http-methods, content-types, security,
        /// response-validation
        ///
        /// Example: --conformance-categories "parameters,security"
        #[arg(long)]
        conformance_categories: Option<String>,

        /// Conformance report format
        ///
        /// Output format for the conformance report: "json" (default) or "sarif" (SARIF 2.1.0).
        /// SARIF format is compatible with GitHub Code Scanning and VS Code SARIF Viewer.
        ///
        /// Example: --conformance-report-format sarif
        #[arg(long, default_value = "json")]
        conformance_report_format: String,

        /// Custom headers to inject into every conformance request (repeatable)
        ///
        /// Use this to provide authentication headers when testing against
        /// real APIs that require credentials. Each header is in "Name: Value" format.
        /// Custom headers override spec-derived placeholder values for matching names.
        ///
        /// Example: --conformance-header "X-CSRFToken: real-token" --conformance-header "Cookie: sessionid=abc"
        #[arg(long = "conformance-header", value_name = "HEADER")]
        conformance_headers: Vec<String>,

        /// Test ALL API operations in conformance mode (not just representative samples)
        ///
        /// By default, spec-driven conformance picks one representative operation per
        /// feature check (e.g., one GET, one POST). This flag tests every operation
        /// for method, response code, and body categories, using path-qualified check
        /// names like "method:GET:/api/users".
        ///
        /// Example: --conformance-all-operations
        #[arg(long)]
        conformance_all_operations: bool,

        /// Custom conformance checks YAML file
        ///
        /// Define additional conformance checks beyond the built-in OpenAPI 3.0.0 feature set.
        /// Custom checks appear under a "Custom" category in the report.
        ///
        /// Example: --conformance-custom custom-checks.yaml
        #[arg(long, value_name = "FILE")]
        conformance_custom: Option<PathBuf>,

        /// Delay in milliseconds between consecutive conformance requests.
        ///
        /// Useful when testing against rate-limited APIs to avoid 429 responses.
        /// Default: 0 (no delay). Example: --conformance-delay 100 for 100ms between requests.
        #[arg(long, value_name = "MS", default_value = "0")]
        conformance_delay: u64,

        /// Use k6 for conformance test execution instead of the native Rust executor
        ///
        /// By default, conformance tests run using a native Rust executor (no k6 required).
        /// Use this flag to fall back to the k6-based execution path.
        #[arg(long)]
        use_k6: bool,

        /// Regex filter for custom conformance checks.
        ///
        /// Only custom checks whose name or path matches the regex pattern
        /// are included. All spec-driven checks still run.
        ///
        /// Examples:
        ///   --conformance-custom-filter "wafcrs|ssl"
        ///   --conformance-custom-filter "GET"
        ///   --conformance-custom-filter "/api/users"
        #[arg(long, value_name = "REGEX")]
        conformance_custom_filter: Option<String>,

        /// Export all request/response pairs to conformance-requests.json.
        ///
        /// Creates a JSON file in the output directory containing every HTTP
        /// request sent during conformance testing along with the full response
        /// (status, headers, body). Useful for comparing against your product's
        /// expected behavior.
        #[arg(long)]
        export_requests: bool,

        /// Validate each request against the OpenAPI spec.
        ///
        /// Checks that request bodies match the spec's requestBody schema,
        /// required parameters are present, and content types are correct.
        /// Violations are written to conformance-request-violations.json.
        #[arg(long)]
        validate_requests: bool,
    },

    /// Convert a HAR file to conformance custom-checks YAML
    ///
    /// Reads a recorded HTTP Archive (.har) file and generates a YAML config
    /// that can be used with `mockforge bench --conformance-custom`.
    ///
    /// Examples:
    ///   mockforge har-to-conformance --har recording.har
    ///   mockforge har-to-conformance --har recording.har --output checks.yaml
    ///   mockforge har-to-conformance --har recording.har --include-headers content-type,x-api-version
    #[cfg(feature = "bench")]
    #[command(verbatim_doc_comment)]
    HarToConformance {
        /// Path to the HAR file
        #[arg(long)]
        har: PathBuf,

        /// Output file path (default: stdout)
        #[arg(long)]
        output: Option<PathBuf>,

        /// Base URL to strip from entry URLs (auto-detected if omitted).
        /// Example: --base-url https://192.168.2.86/api
        #[arg(long)]
        base_url: Option<String>,

        /// Base path to strip from generated paths (e.g., /api).
        /// Use this when your HAR URLs include a path prefix that you also
        /// pass via --base-path to the bench command, to avoid path doubling.
        #[arg(long)]
        strip_base_path: Option<String>,

        /// Skip static asset entries (.js, .css, .png, etc.)
        #[arg(long, default_value = "true")]
        skip_static: bool,

        /// Response headers to include in checks (comma-separated)
        #[arg(long, value_delimiter = ',')]
        include_headers: Vec<String>,

        /// Maximum number of HAR entries to process (0 = unlimited)
        #[arg(long, default_value = "0")]
        max_entries: usize,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    // Initialize logging with the provided log level
    // Note: Full logging configuration (JSON format, file output) will be applied
    // after loading the config file in the serve command
    let initial_logging_config = mockforge_observability::LoggingConfig {
        level: cli.log_level.clone(),
        json_format: false, // Will be overridden by config file if present
        file_path: None,
        max_file_size_mb: 10,
        max_files: 5,
    };

    if let Err(e) = mockforge_observability::init_logging(initial_logging_config) {
        eprintln!("Failed to initialize logging: {}", e);
        std::process::exit(1);
    }

    match cli.command {
        Commands::Serve(args) => {
            // Handle --list-network-profiles flag
            if args.traffic.list_network_profiles {
                let catalog = mockforge_core::NetworkProfileCatalog::new();
                println!("\n📡 Available Network Profiles:\n");
                for (name, description) in catalog.list_profiles_with_description() {
                    println!("  • {:<20} {}", name, description);
                }
                println!();
                return Ok(());
            }

            // Validate TLS flags
            if args.tls.tls_enabled {
                if args.tls.tls_cert.is_none() || args.tls.tls_key.is_none() {
                    eprintln!("Error: --tls-enabled requires --tls-cert and --tls-key");
                    std::process::exit(1);
                }
                if (args.tls.mtls == "optional" || args.tls.mtls == "required")
                    && args.tls.tls_ca.is_none()
                {
                    eprintln!("Error: --mtls {} requires --tls-ca", args.tls.mtls);
                    std::process::exit(1);
                }
            }
            if (args.tls.mtls == "optional" || args.tls.mtls == "required") && !args.tls.tls_enabled
            {
                eprintln!("Error: --mtls {} requires --tls-enabled", args.tls.mtls);
                std::process::exit(1);
            }

            // Validate spec flags (mutually exclusive)
            if !args.spec.is_empty() && args.spec_dir.is_some() {
                eprintln!("Error: --spec and --spec-dir cannot be used together");
                std::process::exit(1);
            }

            // Validate merge_conflicts and api_versioning values
            if !matches!(args.merge_conflicts.as_str(), "error" | "first" | "last") {
                eprintln!("Error: --merge-conflicts must be one of: error, first, last");
                std::process::exit(1);
            }
            if !matches!(args.api_versioning.as_str(), "none" | "info" | "path-prefix") {
                eprintln!("Error: --api-versioning must be one of: none, info, path-prefix");
                std::process::exit(1);
            }

            serve::handle_serve(serve::ServeArgs {
                config_path: args.config,
                profile: args.profile,
                http_port: args.ports.http_port,
                ws_port: args.ports.ws_port,
                grpc_port: args.ports.grpc_port,
                tcp_port: args.ports.tcp_port,
                admin: args.admin,
                admin_port: args.admin_port,
                metrics: args.observability.metrics,
                metrics_port: args.observability.metrics_port,
                tracing: args.observability.tracing,
                tracing_service_name: args.observability.tracing_service_name,
                tracing_environment: args.observability.tracing_environment,
                jaeger_endpoint: args.observability.jaeger_endpoint,
                tracing_sampling_rate: args.observability.tracing_sampling_rate,
                recorder: args.recorder_opts.recorder,
                recorder_db: args.recorder_opts.recorder_db,
                recorder_no_api: args.recorder_opts.recorder_no_api,
                recorder_api_port: args.recorder_opts.recorder_api_port,
                recorder_max_requests: args.recorder_opts.recorder_max_requests,
                recorder_retention_days: args.recorder_opts.recorder_retention_days,
                chaos: args.chaos_opts.chaos,
                chaos_scenario: args.chaos_opts.chaos_scenario,
                chaos_latency_ms: args.chaos_opts.chaos_latency_ms,
                chaos_latency_range: args.chaos_opts.chaos_latency_range,
                chaos_latency_probability: args.chaos_opts.chaos_latency_probability,
                chaos_http_errors: args.chaos_opts.chaos_http_errors,
                chaos_http_error_probability: args.chaos_opts.chaos_http_error_probability,
                chaos_rate_limit: args.chaos_opts.chaos_rate_limit,
                chaos_bandwidth_limit: args.chaos_opts.chaos_bandwidth_limit,
                chaos_packet_loss: args.chaos_opts.chaos_packet_loss,
                spec: args.spec,
                spec_dir: args.spec_dir,
                merge_conflicts: args.merge_conflicts,
                api_versioning: args.api_versioning,
                base_path: args.base_path,
                tls_enabled: args.tls.tls_enabled,
                tls_cert: args.tls.tls_cert,
                tls_key: args.tls.tls_key,
                tls_ca: args.tls.tls_ca,
                tls_min_version: args.tls.tls_min_version,
                mtls: args.tls.mtls,
                ws_replay_file: args.ws_replay_file,
                graphql: args.graphql,
                graphql_port: args.ports.graphql_port,
                graphql_upstream: args.graphql_upstream,
                traffic_shaping: args.traffic.traffic_shaping,
                bandwidth_limit: args.traffic.bandwidth_limit,
                burst_size: args.traffic.burst_size,
                ai_enabled: args.ai.ai_enabled,
                rag_provider: args.ai.rag_provider,
                rag_model: args.ai.rag_model,
                rag_api_key: args.ai.rag_api_key,
                network_profile: args.traffic.network_profile,
                chaos_random: args.chaos_opts.chaos_random,
                chaos_random_error_rate: args.chaos_opts.chaos_random_error_rate,
                chaos_random_delay_rate: args.chaos_opts.chaos_random_delay_rate,
                chaos_random_min_delay: args.chaos_opts.chaos_random_min_delay,
                chaos_random_max_delay: args.chaos_opts.chaos_random_max_delay,
                reality_level: args.ai.reality_level,
                dry_run: args.dry_run,
                progress: args.progress,
                verbose: args.verbose,
            })
            .await?;
        }
        #[cfg(feature = "smtp")]
        Commands::Smtp { smtp_command } => {
            smtp_commands::handle_smtp_command(smtp_command).await?;
        }
        #[cfg(feature = "mqtt")]
        Commands::Mqtt { mqtt_command } => {
            mqtt_commands::handle_mqtt_command(mqtt_command).await?;
        }
        #[cfg(feature = "ftp")]
        Commands::Ftp { ftp_command } => {
            ftp_commands::handle_ftp_command(ftp_command).await?;
        }
        #[cfg(feature = "kafka")]
        Commands::Kafka { kafka_command } => {
            kafka_commands::handle_kafka_command(kafka_command).await?;
        }
        #[cfg(feature = "amqp")]
        Commands::Amqp { amqp_command } => {
            amqp_commands::execute_amqp_command(amqp_command).await?;
        }
        Commands::Data { data_command } => {
            data_commands::handle_data(data_command).await?;
        }
        Commands::Admin { port, config } => {
            handle_admin(port, config).await?;
        }
        Commands::Sync {
            workspace_dir,
            config,
        } => {
            handle_sync(workspace_dir, config).await?;
        }
        Commands::Quick {
            file,
            port,
            admin,
            admin_port,
            metrics,
            metrics_port,
            logging,
            host,
        } => {
            handle_quick(file, port, host, admin, admin_port, metrics, metrics_port, logging)
                .await?;
        }
        Commands::Completions { shell } => {
            handle_completions(shell);
        }
        Commands::Init {
            name,
            no_examples,
            blueprint,
        } => {
            generate_commands::handle_init(name, no_examples, blueprint).await?;
        }
        Commands::Wizard => {
            let config = wizard::run_wizard().await?;
            wizard::generate_project(&config).await?;
        }
        Commands::ValidateFixtures { dir, file, verbose } => {
            use fixture_validation::{print_results, validate_directory, validate_file};

            if let Some(dir_path) = dir {
                let results = validate_directory(&dir_path).await?;
                print_results(&results, verbose);

                // Exit with error code if any fixtures are invalid
                let invalid_count = results.iter().filter(|r| !r.valid).count();
                if invalid_count > 0 {
                    std::process::exit(1);
                }
            } else if let Some(file_path) = file {
                let result = validate_file(&file_path).await?;
                let is_valid = result.valid;
                print_results(&[result], verbose);

                if !is_valid {
                    std::process::exit(1);
                }
            } else {
                eprintln!("Error: Either --dir or --file must be specified");
                std::process::exit(1);
            }
        }
        Commands::Generate {
            config,
            spec,
            output,
            verbose,
            dry_run,
            watch,
            watch_debounce,
            progress,
        } => {
            generate_commands::handle_generate(
                config,
                spec,
                output,
                verbose,
                dry_run,
                watch,
                watch_debounce,
                progress,
            )
            .await?;
        }
        Commands::Schema { schema_command } => {
            generate_commands::handle_schema(schema_command).await?;
        }
        Commands::DevSetup { args } => {
            dev_setup_commands::execute_dev_setup(args).await?;
        }
        Commands::Config { config_command } => {
            config_commands::handle_config(config_command).await?;
        }
        Commands::GitWatch {
            repository_url,
            branch,
            spec_paths,
            poll_interval,
            auth_token,
            cache_dir,
            reload_command,
        } => {
            git_watch_commands::handle_git_watch(
                repository_url,
                branch,
                spec_paths,
                poll_interval,
                auth_token,
                cache_dir,
                reload_command,
            )
            .await?;
        }
        Commands::ContractSync {
            repository_url,
            branch,
            spec_paths,
            mock_config,
            auth_token,
            cache_dir,
            strict,
            output,
            update,
        } => {
            contract_sync_commands::handle_contract_sync(
                repository_url,
                branch,
                spec_paths,
                mock_config,
                auth_token,
                cache_dir,
                strict,
                output,
                update,
            )
            .await?;
        }
        Commands::ContractDiff { diff_command } => {
            contract_diff_commands::handle_contract_diff(diff_command).await?;
        }
        Commands::Governance { gov_command } => {
            governance_commands::handle_governance(gov_command).await?;
        }
        Commands::Import { import_command } => {
            import_commands::handle_import_command(import_command).await?;
        }
        Commands::TestAi { ai_command } => {
            ai_commands::handle_test_ai(ai_command).await?;
        }

        Commands::Plugin { plugin_command } => {
            plugin_commands::handle_plugin_command(plugin_command).await?;
        }
        #[cfg(feature = "recorder")]
        Commands::Recorder { recorder_command } => {
            recorder_commands::handle_recorder_command(recorder_command).await?;
        }
        Commands::Flow { flow_command } => {
            flow_commands::handle_flow_command(flow_command).await?;
        }
        #[cfg(feature = "scenarios")]
        Commands::Scenario { scenario_command } => {
            scenario_commands::handle_scenario_command(scenario_command).await?;
        }
        #[cfg(feature = "scenarios")]
        Commands::RealityProfile {
            reality_profile_command,
        } => {
            scenario_commands::handle_reality_profile_command(reality_profile_command).await?;
        }
        #[cfg(feature = "scenarios")]
        Commands::BehaviorRule {
            behavior_rule_command,
        } => {
            scenario_commands::handle_behavior_rule_command(behavior_rule_command).await?;
        }
        #[cfg(feature = "scenarios")]
        Commands::DriftLearning {
            drift_learning_command,
        } => {
            scenario_commands::handle_drift_learning_command(drift_learning_command).await?;
        }
        Commands::Template { template_command } => {
            template_commands::handle_template_command(template_command).await?;
        }
        Commands::Blueprint { blueprint_command } => match blueprint_command {
            blueprint_commands::BlueprintCommands::List { detailed, category } => {
                blueprint_commands::list_blueprints(detailed, category)?;
            }
            blueprint_commands::BlueprintCommands::Create {
                name,
                blueprint,
                output,
                force,
            } => {
                blueprint_commands::create_from_blueprint(name, blueprint, output, force)?;
            }
            blueprint_commands::BlueprintCommands::Info { blueprint_id } => {
                blueprint_commands::show_blueprint_info(blueprint_id)?;
            }
        },
        Commands::Client { client_command } => {
            client_generator::execute_client_command(client_command).await?;
        }
        Commands::Backend { backend_command } => {
            backend_generator::handle_backend_command(backend_command).await?;
        }
        Commands::Workspace { workspace_command } => {
            workspace_commands::handle_workspace_command(workspace_command).await?;
        }

        Commands::Cloud { cloud_command } => {
            cloud_commands::handle_cloud_command(cloud_command)
                .await
                .map_err(|e| anyhow::anyhow!("Cloud command failed: {}", e))?;
        }

        Commands::Login {
            token,
            provider,
            service_url,
        } => {
            cloud_commands::handle_cloud_command(cloud_commands::CloudCommands::Login {
                token,
                provider,
                service_url,
            })
            .await
            .map_err(|e| anyhow::anyhow!("Login failed: {}", e))?;
        }

        #[cfg(feature = "tunnel")]
        Commands::Tunnel { tunnel_command } => {
            tunnel_commands::handle_tunnel_command(tunnel_command)
                .await
                .map_err(|e| anyhow::anyhow!("Tunnel command failed: {}", e))?;
        }

        Commands::Deploy { deploy_command } => {
            deploy_commands::handle_deploy_command(deploy_command)
                .await
                .map_err(|e| anyhow::anyhow!("Deploy command failed: {}", e))?;
        }

        #[cfg(feature = "vbr")]
        Commands::Vbr { vbr_command } => {
            vbr_commands::execute_vbr_command(vbr_command)
                .await
                .map_err(|e| anyhow::anyhow!("VBR command failed: {}", e))?;
        }

        Commands::Snapshot { snapshot_command } => {
            snapshot_commands::handle_snapshot_command(snapshot_command)
                .await
                .map_err(|e| anyhow::anyhow!("Snapshot command failed: {}", e))?;
        }

        Commands::Mockai { mockai_command } => {
            mockai_commands::handle_mockai_command(mockai_command)
                .await
                .map_err(|e| anyhow::anyhow!("MockAI command failed: {}", e))?;
        }
        Commands::Voice { voice_command } => {
            voice_commands::handle_voice_command(voice_command)
                .await
                .map_err(|e| anyhow::anyhow!("Voice command failed: {}", e))?;
        }

        Commands::Mod { mod_command } => {
            mod_commands::handle_mod_command(mod_command).await?;
        }
        Commands::Chaos { chaos_command } => {
            chaos_commands::handle_chaos_command(chaos_command).await?;
        }
        Commands::Time {
            time_command,
            admin_url,
        } => {
            time_commands::execute_time_command(time_command, admin_url)
                .await
                .map_err(|e| anyhow::anyhow!("Time command failed: {}", e))?;
        }

        Commands::Logs {
            admin_url,
            file,
            follow,
            method,
            path,
            status,
            limit,
            json,
            config,
        } => {
            logs_commands::execute_logs_command(
                admin_url, file, follow, method, path, status, limit, json, config,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Logs command failed: {}", e))?;
        }

        Commands::Orchestrate {
            orchestrate_command,
        } => {
            orchestrate_commands::handle_orchestrate(orchestrate_command).await?;
        }

        Commands::GenerateTests {
            database,
            format,
            output,
            protocol,
            method,
            path,
            status_code,
            limit,
            suite_name,
            base_url,
            ai_descriptions,
            llm_provider,
            llm_model,
            llm_endpoint,
            llm_api_key,
            validate_body,
            validate_status,
            validate_headers,
            validate_timing,
            max_duration_ms,
        } => {
            ai_commands::handle_generate_tests(
                database,
                format,
                output,
                protocol,
                method,
                path,
                status_code,
                limit,
                suite_name,
                base_url,
                ai_descriptions,
                llm_provider,
                llm_model,
                llm_endpoint,
                llm_api_key,
                validate_body,
                validate_status,
                validate_headers,
                validate_timing,
                max_duration_ms,
            )
            .await?;
        }

        Commands::Suggest {
            from,
            from_description,
            format,
            output,
            num_suggestions,
            include_examples,
            domain,
            llm_provider,
            llm_model,
            llm_endpoint,
            llm_api_key,
            temperature,
            print_json,
        } => {
            orchestrate_commands::handle_suggest(
                from,
                from_description,
                format,
                output,
                num_suggestions,
                include_examples,
                domain,
                llm_provider,
                llm_model,
                llm_endpoint,
                llm_api_key,
                temperature,
                print_json,
            )
            .await?;
        }

        #[cfg(feature = "bench")]
        Commands::Bench {
            spec,
            spec_dir,
            merge_conflicts,
            spec_mode,
            dependency_config,
            target,
            targets_file,
            base_path,
            duration,
            vus,
            scenario,
            operations,
            exclude_operations,
            auth,
            headers,
            output,
            generate_only,
            script_output,
            threshold_percentile,
            threshold_ms,
            max_error_rate,
            verbose,
            insecure,
            max_concurrency,
            results_format,
            params_file,
            crud_flow,
            flow_config,
            extract_fields,
            parallel_create,
            data_file,
            data_distribution,
            data_mappings,
            error_rate,
            error_types,
            security_test,
            security_payloads,
            security_categories,
            security_target_fields,
            wafbench_dir,
            wafbench_cycle_all,
            owasp_api_top10,
            owasp_categories,
            owasp_auth_header,
            owasp_auth_token,
            owasp_admin_paths,
            owasp_id_fields,
            owasp_report,
            owasp_report_format,
            owasp_iterations,
            conformance,
            conformance_api_key,
            conformance_basic_auth,
            conformance_report,
            conformance_categories,
            conformance_report_format,
            conformance_headers,
            conformance_all_operations,
            conformance_custom,
            conformance_delay,
            use_k6,
            conformance_custom_filter,
            export_requests,
            validate_requests,
        } => {
            // Validate that either --target or --targets-file is provided, but not both
            match (&target, &targets_file) {
                (None, None) => {
                    eprintln!("Error: Either --target or --targets-file must be specified");
                    std::process::exit(1);
                }
                (Some(_), Some(_)) => {
                    eprintln!("Error: --target and --targets-file are mutually exclusive");
                    std::process::exit(1);
                }
                _ => {}
            }

            // Validate results_format
            if !matches!(results_format.as_str(), "per-target" | "aggregated" | "both") {
                eprintln!("Error: --results-format must be one of: per-target, aggregated, both");
                std::process::exit(1);
            }

            // Use empty string for target if targets_file is provided (not used in multi-target mode)
            let target_str = target.unwrap_or_default();

            let bench_cmd = mockforge_bench::BenchCommand {
                spec,
                spec_dir,
                merge_conflicts,
                spec_mode,
                dependency_config,
                target: target_str,
                base_path,
                duration,
                vus,
                scenario,
                operations,
                exclude_operations,
                auth,
                headers,
                output,
                generate_only,
                script_output,
                threshold_percentile,
                threshold_ms,
                max_error_rate,
                verbose,
                skip_tls_verify: insecure,
                targets_file,
                max_concurrency: Some(max_concurrency),
                results_format,
                params_file,
                crud_flow,
                flow_config,
                extract_fields,
                parallel_create,
                data_file,
                data_distribution,
                data_mappings,
                per_uri_control: false,
                error_rate,
                error_types,
                security_test,
                security_payloads,
                security_categories,
                security_target_fields,
                wafbench_dir,
                wafbench_cycle_all,
                owasp_api_top10,
                owasp_categories,
                owasp_auth_header,
                owasp_auth_token,
                owasp_admin_paths,
                owasp_id_fields,
                owasp_report,
                owasp_report_format,
                owasp_iterations,
                conformance,
                conformance_api_key,
                conformance_basic_auth,
                conformance_report,
                conformance_categories,
                conformance_report_format,
                conformance_headers,
                conformance_all_operations,
                conformance_custom,
                conformance_delay_ms: conformance_delay,
                use_k6,
                conformance_custom_filter,
                export_requests,
                validate_requests,
            };

            if let Err(e) = bench_cmd.execute().await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }

        #[cfg(feature = "bench")]
        Commands::HarToConformance {
            har,
            output,
            base_url,
            strip_base_path,
            skip_static,
            include_headers,
            max_entries,
        } => {
            use mockforge_bench::conformance::har_to_custom::{
                generate_custom_yaml_from_har, HarToCustomOptions,
            };

            // If --strip-base-path is given, auto-detect the host from the HAR
            // and append the base path to form the full base URL for stripping.
            let effective_base_url = if let Some(ref bu) = base_url {
                Some(bu.clone())
            } else if let Some(ref sbp) = strip_base_path {
                // Read the HAR to detect the host, then append the base path
                let raw = std::fs::read_to_string(&har)?;
                let archive: serde_json::Value = serde_json::from_str(&raw)?;
                if let Some(url_str) =
                    archive.pointer("/log/entries/0/request/url").and_then(|v| v.as_str())
                {
                    if let Ok(parsed) = url::Url::parse(url_str) {
                        let mut host_base = format!(
                            "{}://{}",
                            parsed.scheme(),
                            parsed.host_str().unwrap_or("localhost")
                        );
                        if let Some(port) = parsed.port() {
                            host_base.push_str(&format!(":{}", port));
                        }
                        let bp = sbp.trim_end_matches('/');
                        let bp = if bp.starts_with('/') {
                            bp.to_string()
                        } else {
                            format!("/{}", bp)
                        };
                        Some(format!("{}{}", host_base, bp))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let options = HarToCustomOptions {
                base_url: effective_base_url,
                skip_static,
                include_headers: if include_headers.is_empty() {
                    vec!["content-type".to_string()]
                } else {
                    include_headers
                },
                max_entries,
            };

            match generate_custom_yaml_from_har(&har, options) {
                Ok(yaml) => {
                    if let Some(output_path) = output {
                        std::fs::write(&output_path, &yaml)?;
                        println!("Custom conformance YAML written to: {}", output_path.display());
                    } else {
                        println!("{}", yaml);
                    }
                }
                Err(e) => {
                    eprintln!("Error generating conformance YAML from HAR: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn parses_admin_port_override() {
        // Run on a thread with a larger stack to avoid stack overflow
        // from clap parsing the large Commands enum (~50 variants)
        let result = std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn(|| {
                let cli = Cli::parse_from([
                    "mockforge",
                    "serve",
                    "--admin",
                    "--admin-port",
                    "3100",
                    "--http-port",
                    "3200",
                    "--ws-port",
                    "3201",
                    "--grpc-port",
                    "5200",
                ]);

                match cli.command {
                    Commands::Serve(args) => assert_eq!(args.admin_port, Some(3100)),
                    _ => panic!("expected serve command"),
                }
            })
            .expect("failed to spawn thread")
            .join();

        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }
}

async fn handle_admin(
    port: u16,
    _config: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("\u{1f39b}\u{fe0f} Starting MockForge Admin UI...");

    // Start the admin UI server
    let addr = format!("127.0.0.1:{}", port).parse()?;
    let prometheus_url =
        std::env::var("PROMETHEUS_URL").unwrap_or_else(|_| "http://localhost:9090".to_string());
    mockforge_ui::start_admin_server(
        addr,
        None, // http_server_addr
        None, // ws_server_addr
        None, // grpc_server_addr
        None, // graphql_server_addr
        true, // api_enabled
        prometheus_url,
        None, // chaos_api_state
        None, // latency_injector
        None, // mockai
        None, // continuum_config
        None, // virtual_clock
        None, // recorder
        None, // federation
        None, // vbr_engine
    )
    .await?;

    println!("\u{2705} Admin UI started successfully!");
    println!("\u{1f310} Access at: http://localhost:{}/", port);

    // Keep running until shutdown signal
    tokio::signal::ctrl_c().await?;
    println!("\u{1f44b} Shutting down admin UI...");

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_quick(
    file: PathBuf,
    port: u16,
    host: String,
    admin: bool,
    admin_port: u16,
    metrics: bool,
    metrics_port: u16,
    logging: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use mockforge_http::quick_mock::{build_quick_router, QuickMockState};
    use std::fs;

    println!("\n\u{26a1} MockForge Quick Mock Mode");
    println!("\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}");
    println!("\u{1f4c1} Loading data from: {}", file.display());

    // Load JSON file
    let json_str = fs::read_to_string(&file)
        .map_err(|e| format!("Failed to read file '{}': {}", file.display(), e))?;

    let json_data: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse JSON from '{}': {}", file.display(), e))?;

    println!("\u{2713} JSON loaded successfully");

    // Create quick mock state
    println!("\u{1f50d} Auto-detecting routes from JSON keys...");
    let state = QuickMockState::from_json(json_data)
        .await
        .map_err(|e| format!("Failed to create quick mock state: {}", e))?;

    let resource_names = state.resource_names().await;
    println!("\u{2713} Detected {} resource(s):", resource_names.len());
    for resource in &resource_names {
        println!("  \u{2022} /{}", resource);
    }

    // Build router
    let app = build_quick_router(state).await;

    println!();
    println!("\u{1f680} Quick Mock Server Configuration:");
    println!("\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}");
    println!("   HTTP Server:  http://{}:{}", host, port);

    if admin {
        println!("   Admin UI:     http://{}:{}", host, admin_port);
    }
    if metrics {
        println!("   Metrics:      http://{}:{}/__metrics", host, metrics_port);
    }
    if logging {
        println!("   Logging:      Enabled");
    }

    println!();
    println!("\u{1f4da} Available Endpoints:");
    println!("\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}");
    for resource in &resource_names {
        println!("   GET    /{}          - List all", resource);
        println!("   GET    /{}/:id      - Get by ID", resource);
        println!("   POST   /{}          - Create new", resource);
        println!("   PUT    /{}/:id      - Update by ID", resource);
        println!("   DELETE /{}/:id      - Delete by ID", resource);
        println!();
    }
    println!("   GET    /__quick/info       - API information");
    println!("\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}");

    // Start server
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    let listener = TcpListener::bind(addr).await?;

    println!();
    println!("\u{2705} Server started successfully!");
    println!("\u{1f4a1} Press Ctrl+C to stop");
    println!("\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\n");

    // Serve with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.unwrap_or_else(|e| {
                eprintln!(
                    "\u{26a0}\u{fe0f}  Warning: Failed to install CTRL+C signal handler: {}",
                    e
                );
                eprintln!("\u{1f4a1} Server may not shut down gracefully on SIGINT");
            });
        })
        .await?;

    println!("\n\u{1f44b} Server stopped\n");

    Ok(())
}

async fn handle_sync(
    workspace_dir: PathBuf,
    _config: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("\n\u{1f504} Starting MockForge Sync Daemon...");
    println!("\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}");
    println!("\u{1f4c1} Workspace directory: {}", workspace_dir.display());
    println!();
    println!("\u{2139}\u{fe0f}  What the sync daemon does:");
    println!("   \u{2022} Monitors the workspace directory for .yaml/.yml file changes");
    println!("   \u{2022} Automatically imports new or modified request files");
    println!("   \u{2022} Syncs changes bidirectionally between files and workspace");
    println!("   \u{2022} Skips hidden files (starting with .)");
    println!();
    println!("\u{1f50d} Monitoring for file changes...");
    println!("\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}");
    println!();

    // Create sync service
    let sync_service = mockforge_core::SyncService::new(&workspace_dir);

    // Start the sync service
    sync_service.start().await?;

    println!("\u{2705} Sync daemon started successfully!");
    println!("\u{1f4a1} Press Ctrl+C to stop\n");

    // Keep running until shutdown signal
    tokio::signal::ctrl_c().await?;
    println!("\n\u{1f6d1} Received shutdown signal");

    // Stop the sync service
    sync_service.stop().await?;
    println!("\u{1f44b} Sync daemon stopped\n");

    Ok(())
}

/// Handle shell completions generation
fn handle_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();
    generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
}
