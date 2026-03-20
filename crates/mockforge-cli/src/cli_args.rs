//! CLI argument types and subcommand enums.
//!
//! Extracted from main.rs to reduce file size and improve organization.

use clap::{Args, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;

// Module references needed by Commands enum variants
use crate::backend_generator;
use crate::blueprint_commands;
use crate::client_generator;
use crate::cloud_commands;
use crate::deploy_commands;
use crate::dev_setup_commands;
use crate::flow_commands;
use crate::import_commands;
use crate::mockai_commands;
use crate::mod_commands;
use crate::plugin_commands;
use crate::recorder_commands;
use crate::scenario_commands;
use crate::snapshot_commands;
use crate::template_commands;
use crate::time_commands;
use crate::tunnel_commands;
use crate::vbr_commands;
use crate::voice_commands;
use crate::workspace_commands;

#[cfg(feature = "amqp")]
use crate::amqp_commands;
#[cfg(feature = "ftp")]
use crate::ftp_commands;
#[cfg(feature = "kafka")]
use crate::kafka_commands;

/// CLI arguments for the serve command (extracted to reduce enum size and prevent stack overflow)
#[derive(Args)]
pub(crate) struct ServeCliArgs {
    /// Configuration file path
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Configuration profile to use (dev, ci, demo, etc.)
    #[arg(short, long)]
    pub profile: Option<String>,

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

    /// Enable admin UI
    #[arg(long, help_heading = "Admin & UI")]
    pub admin: bool,

    /// Admin UI port (defaults to config or 9080)
    #[arg(long, help_heading = "Admin & UI")]
    pub admin_port: Option<u16>,

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

    /// GraphQL server port (defaults to config or 4000)
    #[arg(long, help_heading = "Server Ports")]
    pub graphql_port: Option<u16>,

    /// GraphQL upstream server URL for passthrough
    #[arg(long, help_heading = "Server Configuration")]
    pub graphql_upstream: Option<String>,

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

    /// Enable AI-powered features
    #[arg(long, help_heading = "AI Features")]
    pub ai_enabled: bool,

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

    /// AI/RAG provider (openai, anthropic, ollama, openai_compatible)
    #[arg(long, help_heading = "AI Features")]
    pub rag_provider: Option<String>,

    /// AI/RAG model name
    #[arg(long, help_heading = "AI Features")]
    pub rag_model: Option<String>,

    /// AI/RAG API key (or set MOCKFORGE_RAG_API_KEY)
    #[arg(long, help_heading = "AI Features")]
    pub rag_api_key: Option<String>,

    /// Validate configuration and check port availability without starting servers
    #[arg(long, help_heading = "Validation")]
    pub dry_run: bool,

    /// Show progress indicators during server startup
    #[arg(long, help_heading = "Validation")]
    pub progress: bool,

    /// Enable verbose logging output
    #[arg(long, help_heading = "Validation")]
    pub verbose: bool,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum Commands {
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
        smtp_command: SmtpCommands,
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
        mqtt_command: MqttCommands,
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
        data_command: DataCommands,
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
        schema_command: Option<SchemaCommands>,
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
        config_command: ConfigCommands,
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
        diff_command: ContractDiffCommands,
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
        gov_command: GovernanceCommands,
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
        ai_command: AiTestCommands,
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
        chaos_command: ChaosCommands,
    },

    /// Chaos experiment orchestration
    Orchestrate {
        #[command(subcommand)]
        orchestrate_command: OrchestrateCommands,
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
    },
}

#[derive(Subcommand)]
pub(crate) enum OrchestrateCommands {
    /// Start a chaos orchestration from file
    ///
    /// Example:
    ///   mockforge orchestrate start --file orchestration.yaml --base-url http://localhost:3000
    #[command(verbatim_doc_comment)]
    Start {
        /// Orchestration file (JSON or YAML)
        #[arg(short, long)]
        file: PathBuf,

        /// Base URL for API requests
        #[arg(long, default_value = "http://localhost:3000")]
        base_url: String,
    },

    /// Get orchestration status
    Status {
        /// Base URL for API requests
        #[arg(long, default_value = "http://localhost:3000")]
        base_url: String,
    },

    /// Stop running orchestration
    Stop {
        /// Base URL for API requests
        #[arg(long, default_value = "http://localhost:3000")]
        base_url: String,
    },

    /// Validate an orchestration file
    Validate {
        /// Orchestration file (JSON or YAML)
        #[arg(short, long)]
        file: PathBuf,
    },

    /// Export an orchestration template
    ///
    /// Example:
    ///   mockforge orchestrate template --output my_orchestration.yaml --format yaml
    #[command(verbatim_doc_comment)]
    Template {
        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Format (json or yaml)
        #[arg(long, default_value = "yaml")]
        format: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum AiTestCommands {
    /// Test intelligent mock generation
    ///
    /// Example:
    ///   mockforge test-ai intelligent-mock --prompt "Generate a REST API for a blog" --output mock.json
    #[command(verbatim_doc_comment)]
    IntelligentMock {
        /// Natural language prompt for generation
        #[arg(short, long)]
        prompt: String,

        /// AI/RAG provider (openai, anthropic, ollama, openai_compatible)
        #[arg(long)]
        rag_provider: Option<String>,

        /// AI/RAG model name
        #[arg(long)]
        rag_model: Option<String>,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Test data drift simulation
    Drift {
        /// Initial data file (JSON)
        #[arg(short, long)]
        initial_data: PathBuf,

        /// Number of drift iterations to simulate
        #[arg(short = 'n', long, default_value = "5")]
        iterations: usize,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Test AI event stream generation
    ///
    /// Example:
    ///   mockforge test-ai event-stream --narrative "User login flow" --event-count 10 --output events.json
    #[command(verbatim_doc_comment)]
    EventStream {
        /// Narrative description for event generation
        #[arg(short, long)]
        narrative: String,

        /// Number of events to generate
        #[arg(short = 'c', long, default_value = "10")]
        event_count: usize,

        /// AI/RAG provider (openai, anthropic, ollama, openai_compatible)
        #[arg(long)]
        rag_provider: Option<String>,

        /// AI/RAG model name
        #[arg(long)]
        rag_model: Option<String>,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub(crate) enum ConfigCommands {
    /// Validate configuration file
    Validate {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Show warnings in addition to errors
        #[arg(long)]
        warnings: bool,
    },

    /// Generate a configuration template file
    ///
    /// Creates a well-documented YAML configuration template with all
    /// available options and their default values.
    ///
    /// Examples:
    ///   mockforge config generate-template
    ///   mockforge config generate-template --output mockforge.yaml
    ///   mockforge config generate-template --format json --full
    #[command(verbatim_doc_comment)]
    GenerateTemplate {
        /// Output file path (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format (yaml or json)
        #[arg(short, long, default_value = "yaml")]
        format: String,

        /// Include all optional fields with their defaults
        #[arg(long)]
        full: bool,
    },

    /// List all supported environment variables
    ///
    /// Displays a comprehensive list of all environment variables that
    /// MockForge recognizes, along with their descriptions and defaults.
    ///
    /// Examples:
    ///   mockforge config list-env-vars
    ///   mockforge config list-env-vars --category server
    ///   mockforge config list-env-vars --format markdown
    #[command(verbatim_doc_comment)]
    ListEnvVars {
        /// Filter by category (server, protocols, database, security, ai, traffic, files, logging)
        #[arg(short, long)]
        category: Option<String>,

        /// Output format (table, markdown, json)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Show current effective configuration
    ///
    /// Displays the merged configuration from all sources (defaults,
    /// config file, environment variables) as it would be used at runtime.
    ///
    /// Examples:
    ///   mockforge config show
    ///   mockforge config show --config mockforge.yaml
    ///   mockforge config show --format json
    #[command(verbatim_doc_comment)]
    Show {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Output format (yaml or json)
        #[arg(short, long, default_value = "yaml")]
        format: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum ContractDiffCommands {
    /// Analyze a request against a contract specification
    ///
    /// Examples:
    ///   mockforge contract-diff analyze --spec api.yaml --request-path request.json
    ///   mockforge contract-diff analyze --spec api.yaml --capture-id abc123 --output results.json
    #[command(verbatim_doc_comment)]
    Analyze {
        /// Path to contract specification file (OpenAPI YAML/JSON)
        #[arg(short, long)]
        spec: PathBuf,

        /// Path to request JSON file
        #[arg(long, conflicts_with = "capture_id")]
        request_path: Option<PathBuf>,

        /// Capture ID from request capture system
        #[arg(long, conflicts_with = "request_path")]
        capture_id: Option<String>,

        /// Output file path for results (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// LLM provider (openai, anthropic, ollama, openai-compatible)
        #[arg(long)]
        llm_provider: Option<String>,

        /// LLM model name
        #[arg(long)]
        llm_model: Option<String>,

        /// LLM API key
        #[arg(long)]
        llm_api_key: Option<String>,

        /// Confidence threshold (0.0-1.0)
        #[arg(long)]
        confidence_threshold: Option<f64>,
    },

    /// Compare two contract specifications
    ///
    /// Examples:
    ///   mockforge contract-diff compare --old-spec old.yaml --new-spec new.yaml
    ///   mockforge contract-diff compare --old-spec old.yaml --new-spec new.yaml --output diff.md
    #[command(verbatim_doc_comment)]
    Compare {
        /// Path to old contract specification
        #[arg(long)]
        old_spec: PathBuf,

        /// Path to new contract specification
        #[arg(long)]
        new_spec: PathBuf,

        /// Output file path for comparison report (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate correction patch file
    ///
    /// Examples:
    ///   mockforge contract-diff generate-patch --spec api.yaml --request-path request.json --output patch.json
    ///   mockforge contract-diff generate-patch --spec api.yaml --capture-id abc123 --output patch.json
    #[command(verbatim_doc_comment)]
    GeneratePatch {
        /// Path to contract specification file
        #[arg(short, long)]
        spec: PathBuf,

        /// Path to request JSON file
        #[arg(long, conflicts_with = "capture_id")]
        request_path: Option<PathBuf>,

        /// Capture ID from request capture system
        #[arg(long, conflicts_with = "request_path")]
        capture_id: Option<String>,

        /// Output file path for patch file
        #[arg(short, long)]
        output: PathBuf,

        /// LLM provider (openai, anthropic, ollama, openai-compatible)
        #[arg(long)]
        llm_provider: Option<String>,

        /// LLM model name
        #[arg(long)]
        llm_model: Option<String>,

        /// LLM API key
        #[arg(long)]
        llm_api_key: Option<String>,
    },

    /// Apply correction patch to contract specification
    ///
    /// Examples:
    ///   mockforge contract-diff apply-patch --spec api.yaml --patch patch.json
    ///   mockforge contract-diff apply-patch --spec api.yaml --patch patch.json --output updated-api.yaml
    #[command(verbatim_doc_comment)]
    ApplyPatch {
        /// Path to contract specification file
        #[arg(short, long)]
        spec: PathBuf,

        /// Path to patch file (JSON Patch format)
        #[arg(short, long)]
        patch: PathBuf,

        /// Output file path (default: overwrites input spec)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub(crate) enum SmtpCommands {
    /// Mailbox management commands
    Mailbox {
        #[command(subcommand)]
        mailbox_command: MailboxCommands,
    },

    /// Fixture management commands
    Fixtures {
        #[command(subcommand)]
        fixtures_command: FixturesCommands,
    },

    /// Send test email
    Send {
        /// Recipient email address
        #[arg(short, long)]
        to: String,

        /// Email subject
        #[arg(short, long)]
        subject: String,

        /// Email body
        #[arg(short, long, default_value = "Test email from MockForge CLI")]
        body: String,

        /// SMTP server host
        #[arg(long, default_value = "localhost")]
        host: String,

        /// SMTP server port
        #[arg(long, default_value = "1025")]
        port: u16,

        /// Sender email address
        #[arg(long, default_value = "test@mockforge.cli")]
        from: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum MailboxCommands {
    /// List all emails in mailbox
    List,

    /// Show details of specific email
    Show {
        /// Email ID
        email_id: String,
    },

    /// Clear all emails from mailbox
    Clear,

    /// Export mailbox to file
    Export {
        /// Output format (mbox, json, csv)
        #[arg(short, long, default_value = "mbox")]
        format: String,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Search emails in mailbox
    Search {
        /// Filter by sender email
        #[arg(long)]
        sender: Option<String>,

        /// Filter by recipient email
        #[arg(long)]
        recipient: Option<String>,

        /// Filter by subject
        #[arg(long)]
        subject: Option<String>,

        /// Filter by body content
        #[arg(long)]
        body: Option<String>,

        /// Filter emails since date (RFC3339 format)
        #[arg(long)]
        since: Option<String>,

        /// Filter emails until date (RFC3339 format)
        #[arg(long)]
        until: Option<String>,

        /// Use regex matching instead of substring
        #[arg(long)]
        regex: bool,

        /// Case sensitive matching
        #[arg(long)]
        case_sensitive: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum FixturesCommands {
    /// List loaded fixtures
    List,

    /// Reload fixtures from disk
    Reload,

    /// Validate fixture file
    Validate {
        /// Fixture file path
        file: PathBuf,
    },
}

#[derive(Subcommand)]
pub(crate) enum MqttCommands {
    /// Publish message to MQTT topic
    Publish {
        /// MQTT broker host
        #[arg(long, default_value = "localhost")]
        host: String,

        /// MQTT broker port
        #[arg(long, default_value = "1883")]
        port: u16,

        /// Topic to publish to
        #[arg(short, long)]
        topic: String,

        /// Message payload (JSON string)
        #[arg(short, long)]
        payload: String,

        /// QoS level (0, 1, 2)
        #[arg(short, long, default_value = "0")]
        qos: u8,

        /// Retain message
        #[arg(long)]
        retain: bool,
    },

    /// Subscribe to MQTT topic
    Subscribe {
        /// MQTT broker host
        #[arg(long, default_value = "localhost")]
        host: String,

        /// MQTT broker port
        #[arg(long, default_value = "1883")]
        port: u16,

        /// Topic filter to subscribe to
        #[arg(short, long)]
        topic: String,

        /// QoS level (0, 1, 2)
        #[arg(short, long, default_value = "0")]
        qos: u8,
    },

    /// Topic management commands
    Topics {
        #[command(subcommand)]
        topics_command: MqttTopicsCommands,
    },

    /// Fixture management commands
    Fixtures {
        #[command(subcommand)]
        fixtures_command: MqttFixturesCommands,
    },

    /// Client management commands
    Clients {
        #[command(subcommand)]
        clients_command: MqttClientsCommands,
    },
}

#[derive(Subcommand)]
pub(crate) enum MqttTopicsCommands {
    /// List active topics
    List,

    /// Clear retained messages
    ClearRetained,
}

#[derive(Subcommand)]
pub(crate) enum MqttFixturesCommands {
    /// Load fixtures from directory
    Load {
        /// Path to fixtures directory
        path: PathBuf,
    },

    /// Start auto-publish for all fixtures
    StartAutoPublish,

    /// Stop auto-publish for all fixtures
    StopAutoPublish,
}

#[derive(Subcommand)]
pub(crate) enum MqttClientsCommands {
    /// List connected clients
    List,

    /// Disconnect client
    Disconnect {
        /// Client ID to disconnect
        client_id: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum DataCommands {
    /// Generate data from built-in templates
    ///
    /// Examples:
    ///   mockforge data template user --rows 100 --format json
    ///   mockforge data template product --rows 50 --output products.csv --format csv
    ///   mockforge data template order --rows 20 --rag --rag-provider openai --output orders.json
    #[command(verbatim_doc_comment)]
    Template {
        /// Template name (user, product, order)
        template: String,

        /// Number of rows to generate
        #[arg(short, long, default_value = "10")]
        rows: usize,

        /// Output format (json, csv, jsonl)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Enable RAG mode for enhanced generation
        #[arg(long)]
        rag: bool,

        /// RAG LLM provider (openai, anthropic, ollama, openai_compatible)
        #[arg(long)]
        rag_provider: Option<String>,

        /// RAG model name
        #[arg(long)]
        rag_model: Option<String>,

        /// RAG API endpoint
        #[arg(long)]
        rag_endpoint: Option<String>,

        /// RAG request timeout in seconds
        #[arg(long)]
        rag_timeout: Option<u64>,

        /// Maximum number of RAG API retries
        #[arg(long)]
        rag_max_retries: Option<usize>,
    },

    /// Generate data from JSON schema
    ///
    /// Example:
    ///   mockforge data schema my_schema.json --rows 100 --format jsonl --output data.jsonl
    #[command(verbatim_doc_comment)]
    Schema {
        /// JSON schema file path
        schema: PathBuf,

        /// Number of rows to generate
        #[arg(short, long, default_value = "10")]
        rows: usize,

        /// Output format (json, csv, jsonl)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate mock data from OpenAPI specification
    ///
    /// Examples:
    ///   mockforge data mock-openapi api-spec.json --rows 50 --format json
    ///   mockforge data mock-openapi api-spec.yaml --realistic --output mock-data.json
    ///   mockforge data mock-openapi api-spec.json --validate --include-optional
    #[command(verbatim_doc_comment)]
    MockOpenapi {
        /// OpenAPI specification file path (JSON or YAML)
        spec: PathBuf,

        /// Number of rows to generate per schema
        #[arg(short, long, default_value = "5")]
        rows: usize,

        /// Output format (json, csv, jsonl)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Enable realistic data generation
        #[arg(long)]
        realistic: bool,

        /// Include optional fields in generated data
        #[arg(long)]
        include_optional: bool,

        /// Validate generated data against schemas
        #[arg(long)]
        validate: bool,

        /// Default array size for generated arrays
        #[arg(long, default_value = "3")]
        array_size: usize,

        /// Maximum array size for generated arrays
        #[arg(long, default_value = "10")]
        max_array_size: usize,
    },

    /// Start a mock server based on OpenAPI specification
    ///
    /// Examples:
    ///   mockforge data mock-server api-spec.json --port 8080
    ///   mockforge data mock-server api-spec.yaml --host 0.0.0.0 --port 3000 --cors
    ///   mockforge data mock-server api-spec.json --delay /api/users 100 --log-requests
    #[command(verbatim_doc_comment)]
    MockServer {
        /// OpenAPI specification file path (JSON or YAML)
        spec: PathBuf,

        /// Port to run the mock server on
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Enable CORS headers
        #[arg(long)]
        cors: bool,

        /// Log all incoming requests
        #[arg(long)]
        log_requests: bool,

        /// Response delay for specific endpoints (format: endpoint:delay_ms)
        #[arg(long)]
        delay: Vec<String>,

        /// Enable realistic data generation
        #[arg(long)]
        realistic: bool,

        /// Include optional fields in generated data
        #[arg(long)]
        include_optional: bool,

        /// Validate generated data against schemas
        #[arg(long)]
        validate: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum ChaosCommands {
    /// Profile management operations
    Profile {
        #[command(subcommand)]
        profile_command: ProfileCommands,
    },
}

#[derive(Subcommand)]
pub(crate) enum ProfileCommands {
    /// Apply a network profile by name
    Apply {
        /// Profile name (e.g., slow_3g, flaky_wifi)
        name: String,
        /// Base URL of the MockForge server (default: http://localhost:3000)
        #[arg(long, default_value = "http://localhost:3000")]
        base_url: String,
    },
    /// Export a profile to JSON or YAML
    Export {
        /// Profile name to export
        name: String,
        /// Output format (json or yaml)
        #[arg(long, default_value = "json")]
        format: String,
        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Base URL of the MockForge server (default: http://localhost:3000)
        #[arg(long, default_value = "http://localhost:3000")]
        base_url: String,
    },
    /// Import a profile from JSON or YAML file
    Import {
        /// Input file path
        #[arg(short, long)]
        file: PathBuf,
        /// Base URL of the MockForge server (default: http://localhost:3000)
        #[arg(long, default_value = "http://localhost:3000")]
        base_url: String,
    },
    /// List all available profiles
    List {
        /// Base URL of the MockForge server (default: http://localhost:3000)
        #[arg(long, default_value = "http://localhost:3000")]
        base_url: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum GovernanceCommands {
    /// API change forecasting
    Forecast {
        #[command(subcommand)]
        forecast_command: ForecastCommands,
    },
    /// Semantic drift analysis
    Semantic {
        #[command(subcommand)]
        semantic_command: SemanticCommands,
    },
    /// Threat assessment
    Threat {
        #[command(subcommand)]
        threat_command: ThreatCommands,
    },
    /// Governance status
    Status {
        /// Workspace ID
        #[arg(long)]
        workspace_id: Option<String>,
        /// Service ID
        #[arg(long)]
        service_id: Option<String>,
    },
}

/// Forecast commands
#[derive(Subcommand)]
pub(crate) enum ForecastCommands {
    /// Generate API change forecast
    Generate {
        /// Workspace ID
        #[arg(long)]
        workspace_id: Option<String>,
        /// Service ID
        #[arg(long)]
        service_id: Option<String>,
        /// Endpoint path
        #[arg(long)]
        endpoint: Option<String>,
        /// HTTP method
        #[arg(long)]
        method: Option<String>,
        /// Forecast window in days (30, 90, or 180)
        #[arg(long, default_value = "90")]
        window_days: u32,
    },
}

/// Semantic commands
#[derive(Subcommand)]
pub(crate) enum SemanticCommands {
    /// Analyze semantic drift between contract versions
    Analyze {
        /// Path to before contract specification
        #[arg(long)]
        before: PathBuf,
        /// Path to after contract specification
        #[arg(long)]
        after: PathBuf,
        /// Endpoint path
        #[arg(long)]
        endpoint: String,
        /// HTTP method
        #[arg(long)]
        method: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

/// Threat commands
#[derive(Subcommand)]
pub(crate) enum ThreatCommands {
    /// Assess contract security threats
    Assess {
        /// Path to contract specification
        #[arg(short, long)]
        spec: PathBuf,
        /// Workspace ID
        #[arg(long)]
        workspace_id: Option<String>,
        /// Service ID
        #[arg(long)]
        service_id: Option<String>,
        /// Endpoint path
        #[arg(long)]
        endpoint: Option<String>,
        /// HTTP method
        #[arg(long)]
        method: Option<String>,
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum SchemaCommands {
    /// Generate all JSON Schemas for MockForge configuration files
    ///
    /// Generates schemas for:
    /// - Main config (mockforge.yaml)
    /// - Reality configuration
    /// - Persona configuration
    /// - Blueprint metadata
    ///
    /// Examples:
    ///   mockforge schema generate
    ///   mockforge schema generate --output schemas/
    ///   mockforge schema generate --type config
    Generate {
        /// Output directory or file path
        /// If directory, generates all schemas with standard names
        /// If file, generates only the specified schema type
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Schema type to generate (config, reality, persona, blueprint, all)
        /// If not specified and output is a file, defaults to 'config'
        /// If not specified and output is a directory, generates all schemas
        #[arg(short, long, default_value = "all")]
        r#type: String,
    },

    /// Validate configuration files against JSON Schemas
    ///
    /// Validates MockForge configuration files against their corresponding
    /// JSON Schemas to catch errors early and ensure config correctness.
    ///
    /// Examples:
    ///   mockforge schema validate mockforge.yaml
    ///   mockforge schema validate --file mockforge.yaml --schema-type config
    ///   mockforge schema validate --directory . --schema-dir schemas/
    Validate {
        /// Config file to validate (mutually exclusive with --directory)
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// Directory containing config files to validate (mutually exclusive with --file)
        #[arg(short, long)]
        directory: Option<PathBuf>,

        /// Schema type to use for validation (config, reality, persona, blueprint)
        /// If not specified, will attempt to auto-detect from file path
        #[arg(long)]
        schema_type: Option<String>,

        /// Directory containing schema files (default: looks for schemas/ in current directory)
        #[arg(long)]
        schema_dir: Option<PathBuf>,

        /// Exit with error code if validation fails (useful for CI)
        #[arg(long)]
        strict: bool,
    },
}
