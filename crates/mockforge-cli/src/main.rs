use axum::serve;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use mockforge_core::encryption::init_key_store;
use mockforge_core::{
    apply_env_overrides, build_file_naming_context, process_generated_file, BarrelGenerator,
    GeneratedFile, OpenApiSpec, ServerConfig,
};
use mockforge_data::rag::{EmbeddingProvider, LlmProvider, RagConfig};
use mockforge_observability::prometheus::{prometheus_router, MetricsRegistry};
use mockforge_chaos::api::create_chaos_api_router;
use mockforge_chaos::config::ChaosConfig;
use serde_json::json;
use std::any::Any;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;

#[cfg(feature = "amqp")]
mod amqp_commands;
mod backend_generator;
mod client_generator;
mod contract_diff_commands;
mod contract_sync_commands;
mod deploy_commands;
#[cfg(feature = "ftp")]
mod ftp_commands;
mod git_watch_commands;
mod import_commands;
#[cfg(feature = "kafka")]
mod kafka_commands;
mod mockai_commands;
#[cfg(feature = "mqtt")]
mod mqtt_commands;
mod plugin_commands;
mod progress;
mod recorder_commands;
mod scenario_commands;
#[cfg(feature = "smtp")]
mod smtp_commands;
mod template_commands;
mod time_commands;
mod tunnel_commands;
mod vbr_commands;
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
    Serve {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Configuration profile to use (dev, ci, demo, etc.)
        #[arg(short, long)]
        profile: Option<String>,

        /// HTTP server port (defaults to config or 3000)
        #[arg(long, help_heading = "Server Ports")]
        http_port: Option<u16>,

        /// WebSocket server port (defaults to config or 3001)
        #[arg(long, help_heading = "Server Ports")]
        ws_port: Option<u16>,

        /// gRPC server port (defaults to config or 50051)
        #[arg(long, help_heading = "Server Ports")]
        grpc_port: Option<u16>,

        /// SMTP server port (defaults to config or 1025)
        #[arg(long, help_heading = "Server Ports")]
        smtp_port: Option<u16>,

        /// MQTT server port (defaults to config or 1883)
        #[arg(long, help_heading = "Server Ports")]
        mqtt_port: Option<u16>,

        /// Kafka broker port (defaults to config or 9092)
        #[arg(long, help_heading = "Server Ports")]
        kafka_port: Option<u16>,

        /// AMQP broker port (defaults to config or 5672)
        #[arg(long, help_heading = "Server Ports")]
        amqp_port: Option<u16>,

        /// TCP server port (defaults to config or 9999)
        #[arg(long, help_heading = "Server Ports")]
        tcp_port: Option<u16>,

        /// Enable admin UI
        #[arg(long, help_heading = "Admin & UI")]
        admin: bool,

        /// Admin UI port (defaults to config or 9080)
        #[arg(long, help_heading = "Admin & UI")]
        admin_port: Option<u16>,

        /// Enable Prometheus metrics endpoint
        #[arg(long, help_heading = "Observability & Metrics")]
        metrics: bool,

        /// Metrics server port (defaults to config or 9090)
        #[arg(long, help_heading = "Observability & Metrics")]
        metrics_port: Option<u16>,

        /// Enable OpenTelemetry distributed tracing
        #[arg(long, help_heading = "Tracing")]
        tracing: bool,

        /// Service name for traces
        #[arg(long, default_value = "mockforge", help_heading = "Tracing")]
        tracing_service_name: String,

        /// Tracing environment (development, staging, production)
        #[arg(long, default_value = "development", help_heading = "Tracing")]
        tracing_environment: String,

        /// Jaeger endpoint for trace export
        #[arg(
            long,
            default_value = "http://localhost:14268/api/traces",
            help_heading = "Tracing"
        )]
        jaeger_endpoint: String,

        /// Tracing sampling rate (0.0 to 1.0)
        #[arg(long, default_value = "1.0", help_heading = "Tracing")]
        tracing_sampling_rate: f64,

        /// Enable API Flight Recorder
        #[arg(long, help_heading = "API Flight Recorder")]
        recorder: bool,

        /// Recorder database file path
        #[arg(
            long,
            default_value = "./mockforge-recordings.db",
            help_heading = "API Flight Recorder"
        )]
        recorder_db: String,

        /// Disable recorder management API
        #[arg(long, help_heading = "API Flight Recorder")]
        recorder_no_api: bool,

        /// Recorder management API port (defaults to main port)
        #[arg(long, help_heading = "API Flight Recorder")]
        recorder_api_port: Option<u16>,

        /// Maximum number of recorded requests (0 for unlimited)
        #[arg(long, default_value = "10000", help_heading = "API Flight Recorder")]
        recorder_max_requests: i64,

        /// Auto-delete recordings older than N days (0 to disable)
        #[arg(long, default_value = "7", help_heading = "API Flight Recorder")]
        recorder_retention_days: i64,

        /// Enable chaos engineering (fault injection and reliability testing)
        #[arg(long, help_heading = "Chaos Engineering")]
        chaos: bool,

        /// Predefined chaos scenario: network_degradation, service_instability, cascading_failure, peak_traffic, slow_backend
        #[arg(long, help_heading = "Chaos Engineering")]
        chaos_scenario: Option<String>,

        /// Chaos latency: fixed delay in milliseconds
        #[arg(long, help_heading = "Chaos Engineering")]
        chaos_latency_ms: Option<u64>,

        /// Chaos latency: random delay range (min-max) in milliseconds
        #[arg(long, help_heading = "Chaos Engineering")]
        chaos_latency_range: Option<String>,

        /// Chaos latency probability (0.0-1.0)
        #[arg(long, default_value = "1.0", help_heading = "Chaos Engineering")]
        chaos_latency_probability: f64,

        /// Chaos fault injection: HTTP error codes (comma-separated)
        #[arg(long, help_heading = "Chaos Engineering")]
        chaos_http_errors: Option<String>,

        /// Chaos fault injection: HTTP error probability (0.0-1.0)
        #[arg(long, default_value = "0.1", help_heading = "Chaos Engineering")]
        chaos_http_error_probability: f64,

        /// Chaos rate limit: requests per second
        #[arg(long, help_heading = "Chaos Engineering")]
        chaos_rate_limit: Option<u32>,

        /// Chaos: bandwidth limit in bytes/sec (e.g., 10000 = 10KB/s for slow network simulation)
        #[arg(long, help_heading = "Chaos Engineering")]
        chaos_bandwidth_limit: Option<u64>,

        /// Chaos: packet loss percentage 0-100 (e.g., 5.0 = 5% packet loss)
        #[arg(long, help_heading = "Chaos Engineering")]
        chaos_packet_loss: Option<f64>,

        /// Enable gRPC-specific chaos engineering
        #[arg(long, help_heading = "Chaos Engineering - gRPC")]
        chaos_grpc: bool,

        /// gRPC chaos: status codes to inject (comma-separated)
        #[arg(long, help_heading = "Chaos Engineering - gRPC")]
        chaos_grpc_status_codes: Option<String>,

        /// gRPC chaos: stream interruption probability (0.0-1.0)
        #[arg(long, default_value = "0.1", help_heading = "Chaos Engineering - gRPC")]
        chaos_grpc_stream_interruption_probability: f64,

        /// Enable WebSocket-specific chaos engineering
        #[arg(long, help_heading = "Chaos Engineering - WebSocket")]
        chaos_websocket: bool,

        /// WebSocket chaos: close codes to inject (comma-separated)
        #[arg(long, help_heading = "Chaos Engineering - WebSocket")]
        chaos_websocket_close_codes: Option<String>,

        /// WebSocket chaos: message drop probability (0.0-1.0)
        #[arg(
            long,
            default_value = "0.05",
            help_heading = "Chaos Engineering - WebSocket"
        )]
        chaos_websocket_message_drop_probability: f64,

        /// WebSocket chaos: message corruption probability (0.0-1.0)
        #[arg(
            long,
            default_value = "0.05",
            help_heading = "Chaos Engineering - WebSocket"
        )]
        chaos_websocket_message_corruption_probability: f64,

        /// Enable GraphQL-specific chaos engineering
        #[arg(long, help_heading = "Chaos Engineering - GraphQL")]
        chaos_graphql: bool,

        /// GraphQL chaos: error codes to inject (comma-separated)
        #[arg(long, help_heading = "Chaos Engineering - GraphQL")]
        chaos_graphql_error_codes: Option<String>,

        /// GraphQL chaos: partial data probability (0.0-1.0)
        #[arg(
            long,
            default_value = "0.1",
            help_heading = "Chaos Engineering - GraphQL"
        )]
        chaos_graphql_partial_data_probability: f64,

        /// GraphQL chaos: enable resolver-level latency injection
        #[arg(long, help_heading = "Chaos Engineering - GraphQL")]
        chaos_graphql_resolver_latency: bool,

        /// Enable circuit breaker pattern
        #[arg(long, help_heading = "Resilience Patterns")]
        circuit_breaker: bool,

        /// Circuit breaker: failure threshold
        #[arg(long, default_value = "5", help_heading = "Resilience Patterns")]
        circuit_breaker_failure_threshold: u64,

        /// Circuit breaker: success threshold
        #[arg(long, default_value = "2", help_heading = "Resilience Patterns")]
        circuit_breaker_success_threshold: u64,

        /// Circuit breaker: timeout in milliseconds
        #[arg(long, default_value = "60000", help_heading = "Resilience Patterns")]
        circuit_breaker_timeout_ms: u64,

        /// Circuit breaker: failure rate threshold percentage (0-100)
        #[arg(long, default_value = "50.0", help_heading = "Resilience Patterns")]
        circuit_breaker_failure_rate: f64,

        /// Enable bulkhead pattern
        #[arg(long, help_heading = "Resilience Patterns")]
        bulkhead: bool,

        /// Bulkhead: maximum concurrent requests
        #[arg(long, default_value = "100", help_heading = "Resilience Patterns")]
        bulkhead_max_concurrent: u32,

        /// Bulkhead: maximum queue size
        #[arg(long, default_value = "10", help_heading = "Resilience Patterns")]
        bulkhead_max_queue: u32,

        /// Bulkhead: queue timeout in milliseconds
        #[arg(long, default_value = "5000", help_heading = "Resilience Patterns")]
        bulkhead_queue_timeout_ms: u64,

        /// OpenAPI spec file for HTTP server
        #[arg(short, long, help_heading = "Server Configuration")]
        spec: Option<PathBuf>,

        /// WebSocket replay file
        #[arg(long, help_heading = "Server Configuration")]
        ws_replay_file: Option<PathBuf>,

        /// GraphQL schema file (.graphql or .gql)
        #[arg(long, help_heading = "Server Configuration")]
        graphql: Option<PathBuf>,

        /// GraphQL server port (defaults to config or 4000)
        #[arg(long, help_heading = "Server Ports")]
        graphql_port: Option<u16>,

        /// GraphQL upstream server URL for passthrough
        #[arg(long, help_heading = "Server Configuration")]
        graphql_upstream: Option<String>,

        /// Enable traffic shaping (bandwidth throttling and packet loss simulation)
        #[arg(long, help_heading = "Traffic Shaping")]
        traffic_shaping: bool,

        /// Maximum bandwidth in bytes per second (e.g., 1000000 = 1MB/s)
        #[arg(long, default_value = "1000000", help_heading = "Traffic Shaping")]
        bandwidth_limit: u64,

        /// Maximum burst size in bytes (allows temporary bursts above bandwidth limit)
        #[arg(long, default_value = "10000", help_heading = "Traffic Shaping")]
        burst_size: u64,

        /// Network condition profile (3g, 4g, 5g, satellite_leo, satellite_geo, congested, lossy, high_latency, intermittent, extremely_poor, perfect)
        #[arg(long, help_heading = "Network Profiles")]
        network_profile: Option<String>,

        /// List all available network profiles with descriptions
        #[arg(long, help_heading = "Network Profiles")]
        list_network_profiles: bool,

        /// Enable random chaos mode (randomly injects errors and delays)
        #[arg(long, help_heading = "Chaos Engineering - Random")]
        chaos_random: bool,

        /// Random chaos: error injection rate (0.0-1.0)
        #[arg(
            long,
            default_value = "0.1",
            help_heading = "Chaos Engineering - Random"
        )]
        chaos_random_error_rate: f64,

        /// Random chaos: delay injection rate (0.0-1.0)
        #[arg(
            long,
            default_value = "0.3",
            help_heading = "Chaos Engineering - Random"
        )]
        chaos_random_delay_rate: f64,

        /// Random chaos: minimum delay in milliseconds
        #[arg(
            long,
            default_value = "100",
            help_heading = "Chaos Engineering - Random"
        )]
        chaos_random_min_delay: u64,

        /// Random chaos: maximum delay in milliseconds
        #[arg(
            long,
            default_value = "2000",
            help_heading = "Chaos Engineering - Random"
        )]
        chaos_random_max_delay: u64,

        /// Apply a chaos network profile by name (e.g., slow_3g, flaky_wifi)
        #[arg(long, help_heading = "Chaos Engineering - Profiles")]
        chaos_profile: Option<String>,

        /// Enable AI-powered features
        #[arg(long, help_heading = "AI Features")]
        ai_enabled: bool,

        /// AI/RAG provider (openai, anthropic, ollama, openai_compatible)
        #[arg(long, help_heading = "AI Features")]
        rag_provider: Option<String>,

        /// AI/RAG model name
        #[arg(long, help_heading = "AI Features")]
        rag_model: Option<String>,

        /// AI/RAG API key (or set MOCKFORGE_RAG_API_KEY)
        #[arg(long, help_heading = "AI Features")]
        rag_api_key: Option<String>,

        /// Validate configuration and check port availability without starting servers
        #[arg(long, help_heading = "Validation")]
        dry_run: bool,

        /// Show progress indicators during server startup
        #[arg(long, help_heading = "Validation")]
        progress: bool,

        /// Enable verbose logging output
        #[arg(long, help_heading = "Validation")]
        verbose: bool,
    },

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
    Init {
        /// Project name (defaults to current directory name)
        #[arg(default_value = ".")]
        name: String,

        /// Skip creating example files
        #[arg(long)]
        no_examples: bool,
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
    #[command(verbatim_doc_comment)]
    Bench {
        /// API specification file (OpenAPI/Swagger)
        #[arg(short, long)]
        spec: PathBuf,

        /// Target service URL
        #[arg(short, long)]
        target: String,

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

        /// Response time threshold percentile (p50, p75, p90, p95, p99)
        #[arg(long, default_value = "p95")]
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
    },
}

#[derive(Subcommand)]
enum OrchestrateCommands {
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
enum AiTestCommands {
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
enum ConfigCommands {
    /// Validate configuration file
    Validate {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum ContractDiffCommands {
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
enum SmtpCommands {
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
enum MailboxCommands {
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
enum FixturesCommands {
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
enum MqttCommands {
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
enum MqttTopicsCommands {
    /// List active topics
    List,

    /// Clear retained messages
    ClearRetained,
}

#[derive(Subcommand)]
enum MqttFixturesCommands {
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
enum MqttClientsCommands {
    /// List connected clients
    List,

    /// Disconnect client
    Disconnect {
        /// Client ID to disconnect
        client_id: String,
    },
}

#[derive(Subcommand)]
enum DataCommands {
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
enum ChaosCommands {
    /// Profile management operations
    Profile {
        #[command(subcommand)]
        profile_command: ProfileCommands,
    },
}

#[derive(Subcommand)]
enum ProfileCommands {
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
        Commands::Serve {
            config,
            profile,
            http_port,
            ws_port,
            grpc_port,
            smtp_port: _smtp_port,
            mqtt_port,
            kafka_port,
            amqp_port,
            tcp_port,
            admin,
            admin_port,
            metrics,
            metrics_port,
            tracing,
            tracing_service_name,
            tracing_environment,
            jaeger_endpoint,
            tracing_sampling_rate,
            recorder,
            recorder_db,
            recorder_no_api,
            recorder_api_port,
            recorder_max_requests,
            recorder_retention_days,
            chaos,
            chaos_scenario,
            chaos_latency_ms,
            chaos_latency_range,
            chaos_latency_probability,
            chaos_http_errors,
            chaos_http_error_probability,
            chaos_rate_limit,
            chaos_bandwidth_limit,
            chaos_packet_loss,
            chaos_grpc: _,
            chaos_grpc_status_codes: _,
            chaos_grpc_stream_interruption_probability: _,
            chaos_websocket: _,
            chaos_websocket_close_codes: _,
            chaos_websocket_message_drop_probability: _,
            chaos_websocket_message_corruption_probability: _,
            chaos_graphql: _,
            chaos_graphql_error_codes: _,
            chaos_graphql_partial_data_probability: _,
            chaos_graphql_resolver_latency: _,
            circuit_breaker: _,
            circuit_breaker_failure_threshold: _,
            circuit_breaker_success_threshold: _,
            circuit_breaker_timeout_ms: _,
            circuit_breaker_failure_rate: _,
            bulkhead: _,
            bulkhead_max_concurrent: _,
            bulkhead_max_queue: _,
            bulkhead_queue_timeout_ms: _,
            spec,
            ws_replay_file,
            graphql,
            graphql_port,
            graphql_upstream,
            traffic_shaping,
            bandwidth_limit,
            burst_size,
            network_profile,
            list_network_profiles,
            chaos_random,
            chaos_random_error_rate,
            chaos_random_delay_rate,
            chaos_random_min_delay,
            chaos_random_max_delay,
            chaos_profile,
            ai_enabled,
            rag_provider,
            rag_model,
            rag_api_key,
            dry_run,
            progress,
            verbose,
        } => {
            // Handle --list-network-profiles flag
            if list_network_profiles {
                let catalog = mockforge_core::NetworkProfileCatalog::new();
                println!("\n📡 Available Network Profiles:\n");
                for (name, description) in catalog.list_profiles_with_description() {
                    println!("  • {:<20} {}", name, description);
                }
                println!();
                return Ok(());
            }

            handle_serve(
                config,
                profile,
                http_port,
                ws_port,
                grpc_port,
                _smtp_port,
                tcp_port,
                admin,
                admin_port,
                metrics,
                metrics_port,
                tracing,
                tracing_service_name,
                tracing_environment,
                jaeger_endpoint,
                tracing_sampling_rate,
                recorder,
                recorder_db,
                recorder_no_api,
                recorder_api_port,
                recorder_max_requests,
                recorder_retention_days,
                chaos,
                chaos_scenario,
                chaos_latency_ms,
                chaos_latency_range,
                chaos_latency_probability,
                chaos_http_errors,
                chaos_http_error_probability,
                chaos_rate_limit,
                chaos_bandwidth_limit,
                chaos_packet_loss,
                spec,
                ws_replay_file,
                graphql,
                graphql_port,
                graphql_upstream,
                traffic_shaping,
                bandwidth_limit,
                burst_size,
                network_profile,
                chaos_random,
                chaos_random_error_rate,
                chaos_random_delay_rate,
                chaos_random_min_delay,
                chaos_random_max_delay,
                chaos_profile,
                ai_enabled,
                rag_provider,
                rag_model,
                rag_api_key,
                dry_run,
                progress,
                verbose,
            )
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
            handle_data(data_command).await?;
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
        Commands::Init { name, no_examples } => {
            handle_init(name, no_examples).await?;
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
            handle_generate(
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
        Commands::Config { config_command } => {
            handle_config(config_command).await?;
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
            handle_contract_diff(diff_command).await?;
        }
        Commands::Import { import_command } => {
            import_commands::handle_import_command(import_command).await?;
        }
        Commands::TestAi { ai_command } => {
            handle_test_ai(ai_command).await?;
        }

        Commands::Plugin { plugin_command } => {
            plugin_commands::handle_plugin_command(plugin_command).await?;
        }
        Commands::Recorder { recorder_command } => {
            recorder_commands::handle_recorder_command(recorder_command).await?;
        }
        Commands::Scenario { scenario_command } => {
            scenario_commands::handle_scenario_command(scenario_command).await?;
        }
        Commands::Template { template_command } => {
            template_commands::handle_template_command(template_command).await?;
        }
        Commands::Client { client_command } => {
            client_generator::execute_client_command(client_command).await?;
        }
        Commands::Backend { backend_command } => {
            backend_generator::handle_backend_command(backend_command).await?;
        }
        Commands::Workspace { workspace_command } => {
            workspace_commands::handle_workspace_command(workspace_command).await?;
        }

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

        Commands::Vbr { vbr_command } => {
            vbr_commands::execute_vbr_command(vbr_command)
                .await
                .map_err(|e| anyhow::anyhow!("VBR command failed: {}", e))?;
        }

        Commands::Mockai { mockai_command } => {
            mockai_commands::handle_mockai_command(mockai_command)
                .await
                .map_err(|e| anyhow::anyhow!("MockAI command failed: {}", e))?;
        }

        Commands::Chaos { chaos_command } => {
            handle_chaos_command(chaos_command).await?;
        }
        Commands::Time {
            time_command,
            admin_url,
        } => {
            time_commands::execute_time_command(time_command, admin_url)
                .await
                .map_err(|e| anyhow::anyhow!("Time command failed: {}", e))?;
        }

        Commands::Orchestrate {
            orchestrate_command,
        } => {
            handle_orchestrate(orchestrate_command).await?;
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
            handle_generate_tests(
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
            handle_suggest(
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

        Commands::Bench {
            spec,
            target,
            duration,
            vus,
            scenario,
            operations,
            auth,
            headers,
            output,
            generate_only,
            script_output,
            threshold_percentile,
            threshold_ms,
            max_error_rate,
            verbose,
        } => {
            let bench_cmd = mockforge_bench::BenchCommand {
                spec,
                target,
                duration,
                vus,
                scenario,
                operations,
                auth,
                headers,
                output,
                generate_only,
                script_output,
                threshold_percentile,
                threshold_ms,
                max_error_rate,
                verbose,
            };

            if let Err(e) = bench_cmd.execute().await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

/// Arguments for building server configuration
#[derive(Debug)]
struct ServeArgs {
    config_path: Option<PathBuf>,
    profile: Option<String>,
    http_port: Option<u16>,
    ws_port: Option<u16>,
    grpc_port: Option<u16>,
    tcp_port: Option<u16>,
    admin: bool,
    admin_port: Option<u16>,
    metrics: bool,
    metrics_port: Option<u16>,
    tracing: bool,
    tracing_service_name: String,
    tracing_environment: String,
    jaeger_endpoint: String,
    tracing_sampling_rate: f64,
    recorder: bool,
    recorder_db: String,
    recorder_no_api: bool,
    recorder_api_port: Option<u16>,
    recorder_max_requests: i64,
    recorder_retention_days: i64,
    chaos: bool,
    chaos_scenario: Option<String>,
    chaos_latency_ms: Option<u64>,
    chaos_latency_range: Option<String>,
    chaos_latency_probability: f64,
    chaos_http_errors: Option<String>,
    chaos_http_error_probability: f64,
    chaos_rate_limit: Option<u32>,
    chaos_bandwidth_limit: Option<u64>,
    chaos_packet_loss: Option<f64>,
    spec: Option<PathBuf>,
    ws_replay_file: Option<PathBuf>,
    graphql: Option<PathBuf>,
    graphql_port: Option<u16>,
    graphql_upstream: Option<String>,
    traffic_shaping: bool,
    bandwidth_limit: u64,
    burst_size: u64,
    ai_enabled: bool,
    rag_provider: Option<String>,
    rag_model: Option<String>,
    rag_api_key: Option<String>,
    network_profile: Option<String>,
    chaos_random: bool,
    /// Random chaos: error injection rate (0.0-1.0)
    chaos_random_error_rate: f64,
    /// Random chaos: delay injection rate (0.0-1.0)
    chaos_random_delay_rate: f64,
    /// Random chaos: minimum delay in milliseconds
    chaos_random_min_delay: u64,
    /// Random chaos: maximum delay in milliseconds
    chaos_random_max_delay: u64,
    dry_run: bool,
    progress: bool,
    verbose: bool,
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn parses_admin_port_override() {
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
            Commands::Serve { admin_port, .. } => assert_eq!(admin_port, Some(3100)),
            _ => panic!("expected serve command"),
        }
    }
}

/// Build ServerConfig from CLI arguments, config file, and environment variables
/// Precedence: CLI args > Env vars > Profile > Config file > Defaults
async fn build_server_config_from_cli(serve_args: &ServeArgs) -> ServerConfig {
    use mockforge_core::config::{
        discover_config_file_all_formats, load_config_auto, load_config_with_profile,
    };

    // Step 1: Load config from file if provided, otherwise try to auto-discover, otherwise use defaults
    let mut config = if let Some(path) = &serve_args.config_path {
        println!("📄 Loading configuration from: {}", path.display());

        // Try auto-format detection (supports .ts, .js, .yaml, .yml, .json)
        match load_config_auto(path).await {
            Ok(cfg) => {
                // Apply profile if specified
                if let Some(profile_name) = &serve_args.profile {
                    match load_config_with_profile(path, Some(profile_name)).await {
                        Ok(cfg_with_profile) => {
                            println!("✅ Applied profile: {}", profile_name);
                            cfg_with_profile
                        }
                        Err(e) => {
                            eprintln!("⚠️  Failed to apply profile '{}': {}", profile_name, e);
                            eprintln!("   Using base configuration without profile");
                            cfg
                        }
                    }
                } else {
                    cfg
                }
            }
            Err(e) => {
                eprintln!("⚠️  Failed to load config file: {}", e);
                eprintln!("   Using default configuration");
                ServerConfig::default()
            }
        }
    } else {
        // Try to auto-discover config file (now supports all formats)
        match discover_config_file_all_formats().await {
            Ok(discovered_path) => {
                println!("📄 Auto-discovered configuration from: {}", discovered_path.display());

                match load_config_auto(&discovered_path).await {
                    Ok(cfg) => {
                        // Apply profile if specified
                        if let Some(profile_name) = &serve_args.profile {
                            match load_config_with_profile(&discovered_path, Some(profile_name))
                                .await
                            {
                                Ok(cfg_with_profile) => {
                                    println!("✅ Applied profile: {}", profile_name);
                                    cfg_with_profile
                                }
                                Err(e) => {
                                    eprintln!(
                                        "⚠️  Failed to apply profile '{}': {}",
                                        profile_name, e
                                    );
                                    eprintln!("   Using base configuration without profile");
                                    cfg
                                }
                            }
                        } else {
                            cfg
                        }
                    }
                    Err(e) => {
                        eprintln!("⚠️  Failed to load auto-discovered config: {}", e);
                        ServerConfig::default()
                    }
                }
            }
            Err(_) => {
                // No config file found
                if serve_args.profile.is_some() {
                    eprintln!("⚠️  Profile specified but no config file found");
                    eprintln!("   Using default configuration");
                }
                ServerConfig::default()
            }
        }
    };

    // Step 2: Apply environment variable overrides
    config = apply_env_overrides(config);

    // Step 3: Apply CLI argument overrides (CLI takes highest precedence)

    // HTTP configuration
    if let Some(http_port) = serve_args.http_port {
        config.http.port = http_port;
    }
    if let Some(spec_path) = &serve_args.spec {
        config.http.openapi_spec = Some(spec_path.to_string_lossy().to_string());
    }

    // WebSocket configuration
    if let Some(ws_port) = serve_args.ws_port {
        config.websocket.port = ws_port;
    }
    if let Some(replay_path) = &serve_args.ws_replay_file {
        config.websocket.replay_file = Some(replay_path.to_string_lossy().to_string());
    }

    // GraphQL configuration
    if let Some(graphql_port) = serve_args.graphql_port {
        config.graphql.port = graphql_port;
    }
    if let Some(schema_path) = &serve_args.graphql {
        config.graphql.schema_path = Some(schema_path.to_string_lossy().to_string());
    }
    if let Some(upstream_url) = &serve_args.graphql_upstream {
        config.graphql.upstream_url = Some(upstream_url.clone());
    }

    // gRPC configuration
    if let Some(grpc_port) = serve_args.grpc_port {
        config.grpc.port = grpc_port;
    }

    // TCP configuration
    if let Some(tcp_port) = serve_args.tcp_port {
        config.tcp.port = tcp_port;
    }

    // Protocol-specific configurations are handled by their respective modules
    // MQTT, Kafka, and AMQP ports are configured through their individual modules

    // Admin configuration
    if serve_args.admin {
        config.admin.enabled = true;
    }
    if let Some(admin_port) = serve_args.admin_port {
        config.admin.port = admin_port;
    }

    // Prometheus metrics configuration
    if serve_args.metrics {
        config.observability.prometheus.enabled = true;
    }
    if let Some(metrics_port) = serve_args.metrics_port {
        config.observability.prometheus.port = metrics_port;
    }

    // OpenTelemetry tracing configuration
    if serve_args.tracing {
        config.observability.opentelemetry = Some(mockforge_core::config::OpenTelemetryConfig {
            enabled: true,
            service_name: serve_args.tracing_service_name.clone(),
            environment: serve_args.tracing_environment.clone(),
            jaeger_endpoint: serve_args.jaeger_endpoint.clone(),
            otlp_endpoint: None,
            protocol: "grpc".to_string(),
            sampling_rate: serve_args.tracing_sampling_rate,
        });
    }

    // API Flight Recorder configuration
    if serve_args.recorder {
        config.observability.recorder = Some(mockforge_core::config::RecorderConfig {
            enabled: true,
            database_path: serve_args.recorder_db.clone(),
            api_enabled: !serve_args.recorder_no_api,
            api_port: serve_args.recorder_api_port,
            max_requests: serve_args.recorder_max_requests,
            retention_days: serve_args.recorder_retention_days,
            record_http: true,
            record_grpc: true,
            record_websocket: true,
            record_graphql: true,
            record_proxy: true,
        });
    }

    // Chaos engineering configuration
    if serve_args.chaos {
        let mut chaos_config = mockforge_core::config::ChaosEngConfig {
            enabled: true,
            scenario: serve_args.chaos_scenario.clone(),
            latency: None,
            fault_injection: None,
            rate_limit: None,
            traffic_shaping: None,
        };

        // Configure latency injection
        if serve_args.chaos_latency_ms.is_some() || serve_args.chaos_latency_range.is_some() {
            let random_delay_range_ms = serve_args.chaos_latency_range.as_ref().and_then(|range| {
                let parts: Vec<&str> = range.split('-').collect();
                if parts.len() == 2 {
                    let min = parts[0].parse::<u64>().ok()?;
                    let max = parts[1].parse::<u64>().ok()?;
                    Some((min, max))
                } else {
                    None
                }
            });

            chaos_config.latency = Some(mockforge_core::config::LatencyInjectionConfig {
                enabled: true,
                fixed_delay_ms: serve_args.chaos_latency_ms,
                random_delay_range_ms,
                jitter_percent: 0.0,
                probability: serve_args.chaos_latency_probability,
            });
        }

        // Configure fault injection
        if serve_args.chaos_http_errors.is_some() {
            let http_errors = serve_args
                .chaos_http_errors
                .as_ref()
                .map(|errors| {
                    errors.split(',').filter_map(|s| s.trim().parse::<u16>().ok()).collect()
                })
                .unwrap_or_default();

            chaos_config.fault_injection = Some(mockforge_core::config::FaultConfig {
                enabled: true,
                http_errors,
                http_error_probability: serve_args.chaos_http_error_probability,
                connection_errors: false,
                connection_error_probability: 0.0,
                timeout_errors: false,
                timeout_ms: 30000,
                timeout_probability: 0.0,
            });
        }

        // Configure rate limiting
        if let Some(rps) = serve_args.chaos_rate_limit {
            chaos_config.rate_limit = Some(mockforge_core::config::RateLimitingConfig {
                enabled: true,
                requests_per_second: rps,
                burst_size: rps * 2,
                per_ip: false,
                per_endpoint: false,
            });
        }

        // Configure traffic shaping
        if serve_args.chaos_bandwidth_limit.is_some() || serve_args.chaos_packet_loss.is_some() {
            chaos_config.traffic_shaping = Some(mockforge_core::config::NetworkShapingConfig {
                enabled: true,
                bandwidth_limit_bps: serve_args.chaos_bandwidth_limit.unwrap_or(1_000_000),
                packet_loss_percent: serve_args.chaos_packet_loss.unwrap_or(0.0),
                max_connections: 100,
            });
        }

        config.observability.chaos = Some(chaos_config);
    }

    // Traffic shaping configuration (core feature)
    if serve_args.traffic_shaping {
        config.core.traffic_shaping_enabled = true;
        config.core.traffic_shaping.bandwidth.enabled = true;
        config.core.traffic_shaping.bandwidth.max_bytes_per_sec = serve_args.bandwidth_limit;
        config.core.traffic_shaping.bandwidth.burst_capacity_bytes = serve_args.burst_size;
    }

    // AI/RAG configuration
    if serve_args.ai_enabled {
        config.data.rag.enabled = true;
        if let Some(provider) = &serve_args.rag_provider {
            config.data.rag.provider = provider.clone();
        }
        if let Some(model) = &serve_args.rag_model {
            config.data.rag.model = Some(model.clone());
        }
        if let Some(api_key) = &serve_args.rag_api_key {
            config.data.rag.api_key = Some(api_key.clone());
        }
    }

    config
}

fn ensure_ports_available(ports: &[(u16, &str)]) -> Result<(), String> {
    let mut unavailable_ports = Vec::new();

    for (port, name) in ports {
        match std::net::TcpListener::bind(("127.0.0.1", *port)) {
            Ok(_) => {}
            Err(err) => unavailable_ports.push((*port, *name, err)),
        }
    }

    if unavailable_ports.is_empty() {
        return Ok(());
    }

    let mut error_msg = String::from("One or more ports are already in use:\n\n");
    for (port, name, err) in &unavailable_ports {
        error_msg.push_str(&format!("  • {} port {}: {}\n", name, port, err));
    }
    error_msg.push_str("\nPossible solutions:\n");
    error_msg.push_str("  1. Stop the process using these ports\n");
    error_msg.push_str("  2. Use different ports with flags like --http-port, --ws-port, etc.\n");
    error_msg.push_str(
        "  3. Find the process using the port with: lsof -i :<port> or netstat -tulpn | grep <port>\n",
    );

    Err(error_msg)
}

/// Validate server configuration before starting
async fn validate_serve_config(
    config_path: &Option<PathBuf>,
    spec_path: &Option<PathBuf>,
    ports: &[(u16, &str)],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::fs;

    // Validate config file if provided
    if let Some(config) = config_path {
        if !config.exists() {
            return Err(format!(
                "Configuration file not found: {}\n\n\
                 Hint: Check that the path is correct and the file exists.",
                config.display()
            )
            .into());
        }

        // Try to read the file to ensure it's accessible
        if let Err(e) = fs::read_to_string(config) {
            return Err(format!(
                "Cannot read configuration file: {}\n\n\
                 Error: {}\n\
                 Hint: Check file permissions and ensure the file is readable.",
                config.display(),
                e
            )
            .into());
        }
    }

    // Validate spec file if provided
    if let Some(spec) = spec_path {
        if !spec.exists() {
            return Err(format!(
                "OpenAPI spec file not found: {}\n\n\
                 Hint: Check that the path is correct and the file exists.",
                spec.display()
            )
            .into());
        }

        // Try to read the file to ensure it's accessible
        if let Err(e) = fs::read_to_string(spec) {
            return Err(format!(
                "Cannot read OpenAPI spec file: {}\n\n\
                 Error: {}\n\
                 Hint: Check file permissions and ensure the file is readable.",
                spec.display(),
                e
            )
            .into());
        }
    }

    if let Err(err) = ensure_ports_available(ports) {
        return Err(err.into());
    }

    Ok(())
}

/// Initialize OpenTelemetry tracing with the given configuration
fn initialize_opentelemetry_tracing(
    otel_config: &mockforge_core::config::OpenTelemetryConfig,
    logging_config: &mockforge_observability::LoggingConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use mockforge_tracing::{init_tracer, TracingConfig};

    // Create tracing configuration from OpenTelemetry config
    let tracing_config = if let Some(ref otlp_endpoint) = otel_config.otlp_endpoint {
        TracingConfig::with_otlp(otel_config.service_name.clone(), otlp_endpoint.clone())
    } else {
        TracingConfig::with_jaeger(
            otel_config.service_name.clone(),
            otel_config.jaeger_endpoint.clone(),
        )
    }
    .with_sampling_rate(otel_config.sampling_rate)
    .with_environment(otel_config.environment.clone());

    // Initialize the tracer
    let tracer = init_tracer(tracing_config)?;

    // Create OpenTelemetry layer
    let otel_layer =
        tracing_opentelemetry::layer::<tracing_subscriber::Registry>().with_tracer(tracer);

    // Initialize logging with OpenTelemetry layer
    mockforge_observability::init_logging_with_otel(logging_config.clone(), otel_layer)?;

    tracing::info!("OpenTelemetry tracing initialized successfully");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_serve(
    config_path: Option<PathBuf>,
    profile: Option<String>,
    http_port: Option<u16>,
    ws_port: Option<u16>,
    grpc_port: Option<u16>,
    _smtp_port: Option<u16>,
    tcp_port: Option<u16>,
    admin: bool,
    admin_port: Option<u16>,
    metrics: bool,
    metrics_port: Option<u16>,
    tracing: bool,
    tracing_service_name: String,
    tracing_environment: String,
    jaeger_endpoint: String,
    tracing_sampling_rate: f64,
    recorder: bool,
    recorder_db: String,
    recorder_no_api: bool,
    recorder_api_port: Option<u16>,
    recorder_max_requests: i64,
    recorder_retention_days: i64,
    chaos: bool,
    chaos_scenario: Option<String>,
    chaos_latency_ms: Option<u64>,
    chaos_latency_range: Option<String>,
    chaos_latency_probability: f64,
    chaos_http_errors: Option<String>,
    chaos_http_error_probability: f64,
    chaos_rate_limit: Option<u32>,
    chaos_bandwidth_limit: Option<u64>,
    chaos_packet_loss: Option<f64>,
    spec: Option<PathBuf>,
    ws_replay_file: Option<PathBuf>,
    graphql: Option<PathBuf>,
    graphql_port: Option<u16>,
    graphql_upstream: Option<String>,
    traffic_shaping: bool,
    bandwidth_limit: u64,
    burst_size: u64,
    network_profile: Option<String>,
    chaos_random: bool,
    chaos_random_error_rate: f64,
    chaos_random_delay_rate: f64,
    chaos_random_min_delay: u64,
    chaos_random_max_delay: u64,
    chaos_profile: Option<String>,
    ai_enabled: bool,
    rag_provider: Option<String>,
    rag_model: Option<String>,
    rag_api_key: Option<String>,
    dry_run: bool,
    progress: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Auto-discover config file if not provided
    let effective_config_path = if config_path.is_some() {
        config_path.clone()
    } else {
        // Try to discover config file
        if let Ok(current_dir) = std::env::current_dir() {
            let config_names = vec![
                "mockforge.yaml",
                "mockforge.yml",
                ".mockforge.yaml",
                ".mockforge.yml",
            ];

            // Check current directory
            let mut discovered = None;
            for name in &config_names {
                let path = current_dir.join(name);
                if path.exists() {
                    discovered = Some(path);
                    break;
                }
            }
            discovered
        } else {
            None
        }
    };

    let serve_args = ServeArgs {
        config_path: effective_config_path.clone(),
        profile,
        http_port,
        ws_port,
        grpc_port,
        tcp_port,
        admin,
        admin_port,
        metrics,
        metrics_port,
        tracing,
        tracing_service_name,
        tracing_environment,
        jaeger_endpoint,
        tracing_sampling_rate,
        recorder,
        recorder_db,
        recorder_no_api,
        recorder_api_port,
        recorder_max_requests,
        recorder_retention_days,
        chaos,
        chaos_scenario,
        chaos_latency_ms,
        chaos_latency_range,
        chaos_latency_probability,
        chaos_http_errors,
        chaos_http_error_probability,
        chaos_rate_limit,
        chaos_bandwidth_limit,
        chaos_packet_loss,
        spec,
        ws_replay_file,
        graphql,
        graphql_port,
        graphql_upstream,
        traffic_shaping,
        bandwidth_limit,
        burst_size,
        ai_enabled,
        rag_provider,
        rag_model,
        rag_api_key,
        network_profile,
        chaos_random,
        chaos_random_error_rate,
        chaos_random_delay_rate,
        chaos_random_min_delay,
        chaos_random_max_delay,
        dry_run,
        progress,
        verbose,
    };

    // Validate config and spec paths (skip port checks for now)
    validate_serve_config(&serve_args.config_path, &serve_args.spec, &[]).await?;

    // Merge configuration sources
    let mut config = build_server_config_from_cli(&serve_args).await;

    // Determine ports to validate using final configuration
    let mut final_ports = vec![
        (config.http.port, "HTTP"),
        (config.websocket.port, "WebSocket"),
        (config.grpc.port, "gRPC"),
    ];

    if config.admin.enabled {
        final_ports.push((config.admin.port, "Admin UI"));
    }

    if config.observability.prometheus.enabled {
        final_ports.push((config.observability.prometheus.port, "Metrics"));
    }

    if let Err(port_error) = ensure_ports_available(&final_ports) {
        return Err(port_error.into());
    }

    if serve_args.dry_run {
        println!("✅ Configuration validation passed!");
        println!("✅ All required ports are available");
        if serve_args.config_path.is_some() {
            println!("✅ Configuration file is valid");
        }
        if serve_args.spec.is_some() {
            println!("✅ OpenAPI spec file is valid");
        }
        println!("\n🎉 Dry run successful - no issues found!");
        return Ok(());
    }

    if !config.routes.is_empty() {
        println!("📄 Found {} routes in config", config.routes.len());
    } else {
        println!("📄 No routes found in config");
    }

    // Apply network profile if specified
    if let Some(profile_name) = serve_args.network_profile {
        use mockforge_core::NetworkProfileCatalog;
        let catalog = NetworkProfileCatalog::new();

        if let Some(profile) = catalog.get(&profile_name) {
            println!("📡 Applying network profile: {} - {}", profile.name, profile.description);
            let (latency_profile, traffic_shaping_config) = profile.apply();

            // Apply latency profile
            config.core.default_latency = latency_profile;
            config.core.latency_enabled = true;

            // Apply traffic shaping
            config.core.traffic_shaping = traffic_shaping_config;
            config.core.traffic_shaping_enabled = true;
        } else {
            eprintln!("⚠️  Warning: Unknown network profile '{}'. Use --list-network-profiles to see available profiles.", profile_name);
        }
    }

    // Enable random chaos mode if specified
    if serve_args.chaos_random {
        use mockforge_core::ChaosConfig;

        println!("🎲 Random chaos mode enabled");
        println!("   Error rate: {:.1}%", chaos_random_error_rate * 100.0);
        println!("   Delay rate: {:.1}%", chaos_random_delay_rate * 100.0);
        println!("   Delay range: {}-{} ms", chaos_random_min_delay, chaos_random_max_delay);

        // Create and apply chaos config
        let chaos_config = ChaosConfig::new(chaos_random_error_rate, chaos_random_delay_rate)
            .with_delay_range(chaos_random_min_delay, chaos_random_max_delay);

        config.core.chaos_random = Some(chaos_config);
    }

    // Re-initialize logging with configuration from config file
    // This allows JSON logging, file output, and OpenTelemetry integration
    let logging_config = mockforge_observability::LoggingConfig {
        level: config.logging.level.clone(),
        json_format: config.logging.json_format,
        file_path: config.logging.file_path.as_ref().map(|p| p.into()),
        max_file_size_mb: config.logging.max_file_size_mb,
        max_files: config.logging.max_files,
    };

    // If OpenTelemetry tracing is enabled, initialize with tracing layer
    if let Some(ref otel_config) = config.observability.opentelemetry {
        if otel_config.enabled {
            // Initialize OpenTelemetry tracer
            if let Err(e) = initialize_opentelemetry_tracing(otel_config, &logging_config) {
                tracing::warn!("Failed to initialize OpenTelemetry tracing: {}", e);
                // Fall back to standard logging
                if let Err(e) = mockforge_observability::init_logging(logging_config) {
                    eprintln!("Failed to initialize logging: {}", e);
                }
            }
        }
    }

    println!("🚀 Starting MockForge servers...");
    println!("📡 HTTP server on port {}", config.http.port);
    println!("🔌 WebSocket server on port {}", config.websocket.port);
    println!("⚡ gRPC server on port {}", config.grpc.port);
    if config.tcp.enabled {
        println!("🔌 TCP server on port {}", config.tcp.port);
    }

    if config.admin.enabled {
        println!("🎛️ Admin UI on port {}", config.admin.port);
    }

    if config.observability.prometheus.enabled {
        println!("📊 Metrics endpoint on port {}", config.observability.prometheus.port);
    }

    if let Some(ref tracing_config) = config.observability.opentelemetry {
        if tracing_config.enabled {
            println!("🔍 OpenTelemetry tracing enabled");
            println!("   Service: {}", tracing_config.service_name);
            println!("   Environment: {}", tracing_config.environment);
            println!("   Jaeger endpoint: {}", tracing_config.jaeger_endpoint);
        }
    }

    if let Some(ref recorder_config) = config.observability.recorder {
        if recorder_config.enabled {
            println!("📹 API Flight Recorder enabled");
            println!("   Database: {}", recorder_config.database_path);
            println!("   Max requests: {}", recorder_config.max_requests);
        }
    }

    if let Some(ref chaos_config) = config.observability.chaos {
        if chaos_config.enabled {
            println!("🌀 Chaos engineering enabled");
            if let Some(ref scenario) = chaos_config.scenario {
                println!("   Scenario: {}", scenario);
            }
        }
    }

    if config.data.rag.enabled {
        println!("🧠 AI features enabled");
        println!("   Provider: {}", config.data.rag.provider);
        if let Some(ref model) = config.data.rag.model {
            println!("   Model: {}", model);
        }
    }

    if config.core.traffic_shaping_enabled {
        println!("🚦 Traffic shaping enabled");
        println!(
            "   Bandwidth limit: {} bytes/sec",
            config.core.traffic_shaping.bandwidth.max_bytes_per_sec
        );
    }

    // Set AI environment variables if configured
    if let Some(ref api_key) = config.data.rag.api_key {
        std::env::set_var("MOCKFORGE_RAG_API_KEY", api_key);
    }
    std::env::set_var("MOCKFORGE_RAG_PROVIDER", &config.data.rag.provider);
    if let Some(ref model) = config.data.rag.model {
        std::env::set_var("MOCKFORGE_RAG_MODEL", model);
    }

    // Initialize key store at startup
    init_key_store();

    // Initialize request capture manager for contract diff analysis
    use mockforge_core::request_capture::init_global_capture_manager;
    init_global_capture_manager(1000); // Keep last 1000 requests
    tracing::info!("Request capture manager initialized for contract diff analysis");

    // Build HTTP router with OpenAPI spec, chain support, multi-tenant, and traffic shaping if enabled
    let multi_tenant_config = if config.multi_tenant.enabled {
        Some(config.multi_tenant.clone())
    } else {
        None
    };

    // Create SMTP registry if enabled
    #[cfg(feature = "smtp")]
    let smtp_registry = if config.smtp.enabled {
        use mockforge_smtp::SmtpSpecRegistry;
        use std::sync::Arc;

        let mut registry = SmtpSpecRegistry::new();

        if let Some(fixtures_dir) = &config.smtp.fixtures_dir {
            if fixtures_dir.exists() {
                if let Err(e) = registry.load_fixtures(fixtures_dir) {
                    eprintln!(
                        "⚠️  Warning: Failed to load SMTP fixtures from {:?}: {}",
                        fixtures_dir, e
                    );
                } else {
                    println!("   Loaded SMTP fixtures from {:?}", fixtures_dir);
                }
            } else {
                println!("   No SMTP fixtures directory found at {:?}", fixtures_dir);
            }
        }

        Some(Arc::new(registry) as Arc<dyn Any + Send + Sync>)
    } else {
        None
    };
    #[cfg(not(feature = "smtp"))]
    let smtp_registry = None::<Arc<dyn std::any::Any + Send + Sync>>;

    #[cfg(feature = "mqtt")]
    let mqtt_registry = if config.mqtt.enabled {
        use mockforge_mqtt::MqttSpecRegistry;
        use std::sync::Arc;

        let mut registry = MqttSpecRegistry::new();

        if let Some(fixtures_dir) = &config.mqtt.fixtures_dir {
            if fixtures_dir.exists() {
                if let Err(e) = registry.load_fixtures(fixtures_dir) {
                    eprintln!(
                        "⚠️  Warning: Failed to load MQTT fixtures from {:?}: {}",
                        fixtures_dir, e
                    );
                } else {
                    println!("   Loaded MQTT fixtures from {:?}", fixtures_dir);
                }
            } else {
                println!("   No MQTT fixtures directory found at {:?}", fixtures_dir);
            }
        }

        Some(Arc::new(registry))
    } else {
        None
    };

    #[cfg(feature = "mqtt")]
    let mqtt_broker = if let Some(ref registry_ref) = mqtt_registry {
        let mqtt_config = config.mqtt.clone();

        // Convert core MqttConfig to mockforge_mqtt::MqttConfig
        let broker_config = mockforge_mqtt::broker::MqttConfig {
            port: mqtt_config.port,
            host: mqtt_config.host.clone(),
            max_connections: mqtt_config.max_connections,
            max_packet_size: mqtt_config.max_packet_size,
            keep_alive_secs: mqtt_config.keep_alive_secs,
            version: mockforge_mqtt::broker::MqttVersion::default(),
        };

        // MQTT registry is already Some, so we can safely clone it
        Some(Arc::new(mockforge_mqtt::MqttBroker::new(
            broker_config.clone(),
            registry_ref.clone(),
        )))
    } else {
        None
    };

    #[cfg(feature = "mqtt")]
    let mqtt_broker_for_http = mqtt_broker
        .as_ref()
        .map(|broker| Arc::clone(broker) as Arc<dyn Any + Send + Sync>);
    #[cfg(not(feature = "mqtt"))]
    let mqtt_broker_for_http = None::<Arc<dyn Any + Send + Sync>>;

    // Create health manager for Kubernetes-native health checks
    use mockforge_http::HealthManager;
    use std::sync::Arc;
    use std::time::Duration;

    let health_manager = Arc::new(HealthManager::with_init_timeout(Duration::from_secs(60)));
    let health_manager_for_router = health_manager.clone();

    // Initialize TimeTravelManager if configured
    use mockforge_core::TimeTravelManager;
    use mockforge_ui::time_travel_handlers;

    let time_travel_manager = {
        let time_travel_config = config.core.time_travel.clone();
        let manager = Arc::new(TimeTravelManager::new(time_travel_config));

        // Initialize the global time travel manager for UI handlers
        time_travel_handlers::init_time_travel_manager(manager.clone());

        if manager.clock().is_enabled() {
            println!("⏰ Time travel enabled");
            if let Some(virtual_time) = manager.clock().status().current_time {
                println!("   Virtual time: {}", virtual_time);
            }
            println!("   Scale factor: {}x", manager.clock().get_scale());
        }

        // Start cron scheduler background task
        let cron_scheduler = manager.cron_scheduler();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
            loop {
                interval.tick().await;
                if let Err(e) = cron_scheduler.check_and_execute().await {
                    tracing::warn!("Error checking cron jobs: {}", e);
                }
            }
        });

        manager
    };

    // Initialize MutationRuleManager for time-based data mutations
    use mockforge_vbr::MutationRuleManager;
    let mutation_rule_manager = Arc::new(MutationRuleManager::new());
    time_travel_handlers::init_mutation_rule_manager(mutation_rule_manager.clone());

    // Initialize MockAI if enabled
    let mockai = if config.mockai.enabled {
        use mockforge_core::intelligent_behavior::{IntelligentBehaviorConfig, MockAI};
        use std::sync::Arc;
        use tokio::sync::RwLock;
        use tracing::{info, warn};

        // Create MockAI from OpenAPI spec if available
        if let Some(ref spec_path) = config.http.openapi_spec {
            match mockforge_core::openapi::OpenApiSpec::from_file(spec_path).await {
                Ok(openapi_spec) => {
                    let behavior_config = config.mockai.intelligent_behavior.clone();
                    match MockAI::from_openapi(&openapi_spec, behavior_config).await {
                        Ok(mockai_instance) => {
                            info!("✅ MockAI initialized from OpenAPI spec");
                            Some(Arc::new(RwLock::new(mockai_instance)))
                        }
                        Err(e) => {
                            warn!("Failed to initialize MockAI from OpenAPI spec: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to load OpenAPI spec for MockAI: {}", e);
                    None
                }
            }
        } else {
            // Create MockAI with default config if no spec
            let behavior_config = config.mockai.intelligent_behavior.clone();
            let mockai_instance = MockAI::new(behavior_config);
            info!("✅ MockAI initialized with default configuration");
            Some(Arc::new(RwLock::new(mockai_instance)))
        }
    } else {
        None
    };

    // Use standard router (traffic shaping temporarily disabled)
    let mut http_app = mockforge_http::build_router_with_chains_and_multi_tenant(
        config.http.openapi_spec.clone(),
        None,
        None, // circling_config
        multi_tenant_config,
        Some(config.routes.clone()),
        config.http.cors.clone(),
        None, // ai_generator
        smtp_registry.as_ref().cloned(),
        mqtt_broker_for_http,
        None,                                  // traffic_shaper
        false,                                 // traffic_shaping_enabled
        Some(health_manager_for_router),       // health_manager
        mockai.clone(),                        // mockai
        Some(config.deceptive_deploy.clone()), // deceptive_deploy_config
        None,                                  // proxy_config (ProxyConfig not in ServerConfig)
    )
    .await;

    // Integrate chaos engineering API router
    // Convert from ServerConfig's ChaosEngConfig to mockforge-chaos's ChaosConfig
    let chaos_config = if let Some(ref chaos_eng_config) = config.observability.chaos {
        // Convert ChaosEngConfig to ChaosConfig
        let mut chaos_cfg = ChaosConfig {
            enabled: chaos_eng_config.enabled,
            latency: chaos_eng_config.latency.as_ref().map(|l| {
                mockforge_chaos::config::LatencyConfig {
                    enabled: l.enabled,
                    fixed_delay_ms: l.fixed_delay_ms,
                    random_delay_range_ms: l.random_delay_range_ms,
                    jitter_percent: l.jitter_percent,
                    probability: l.probability,
                }
            }),
            fault_injection: chaos_eng_config.fault_injection.as_ref().map(|f| {
                mockforge_chaos::config::FaultInjectionConfig {
                    enabled: f.enabled,
                    http_errors: f.http_errors.clone(),
                    http_error_probability: f.http_error_probability,
                    connection_errors: f.connection_errors,
                    connection_error_probability: f.connection_error_probability,
                    timeout_errors: f.timeout_errors,
                    timeout_ms: f.timeout_ms,
                    timeout_probability: f.timeout_probability,
                    partial_responses: false,
                    partial_response_probability: 0.0,
                    payload_corruption: false,
                    payload_corruption_probability: 0.0,
                    corruption_type: mockforge_chaos::config::CorruptionType::None,
                    error_pattern: None,
                    mockai_enabled: false,
                }
            }),
            rate_limit: chaos_eng_config.rate_limit.as_ref().map(|r| {
                mockforge_chaos::config::RateLimitConfig {
                    enabled: r.enabled,
                    requests_per_second: r.requests_per_second,
                    burst_size: r.burst_size,
                    per_ip: r.per_ip,
                    per_endpoint: r.per_endpoint,
                }
            }),
            traffic_shaping: chaos_eng_config.traffic_shaping.as_ref().map(|t| {
                mockforge_chaos::config::TrafficShapingConfig {
                    enabled: t.enabled,
                    bandwidth_limit_bps: t.bandwidth_limit_bps,
                    packet_loss_percent: t.packet_loss_percent,
                    max_connections: 0,
                    connection_timeout_ms: 30000,
                }
            }),
            circuit_breaker: None,
            bulkhead: None,
        };
        chaos_cfg
    } else {
        // Default chaos config if not configured
        ChaosConfig::default()
    };

    // Create and merge chaos API router
    // Pass MockAI instance if available for dynamic error message generation
    let (chaos_router, _chaos_config_arc) = create_chaos_api_router(chaos_config, mockai.clone());
    http_app = http_app.merge(chaos_router);
    println!("✅ Chaos Engineering API available at /api/chaos/*");

    println!(
        "✅ HTTP server configured with health checks at http://localhost:{}/health (live, ready, startup)",
        config.http.port
    );
    if !config.routes.is_empty() {
        println!("✅ Loaded {} custom routes", config.routes.len());
    }
    println!("✅ WebSocket server configured at ws://localhost:{}/ws", config.websocket.port);
    println!("✅ gRPC server configured at localhost:{}", config.grpc.port);
    if config.admin.enabled {
        println!("✅ Admin UI configured at http://localhost:{}", config.admin.port);
    }

    println!("💡 Press Ctrl+C to stop");

    // Create metrics registry (use global registry)
    let metrics_registry = std::sync::Arc::new(MetricsRegistry::new());

    // Start system metrics collector if Prometheus is enabled
    if config.observability.prometheus.enabled {
        use mockforge_observability::{get_global_registry, SystemMetricsConfig};
        let system_metrics_config = SystemMetricsConfig {
            enabled: true,
            interval_seconds: 15,
        };
        mockforge_observability::system_metrics::start_with_config(
            get_global_registry(),
            system_metrics_config,
        );
        println!("📈 System metrics collector started (interval: 15s)");
    }

    // Create a cancellation token for graceful shutdown
    use tokio_util::sync::CancellationToken;
    let shutdown_token = CancellationToken::new();

    // Set up graceful shutdown integration with health manager
    let health_manager_for_shutdown = health_manager.clone();
    let shutdown_token_for_health = shutdown_token.clone();
    tokio::spawn(async move {
        shutdown_token_for_health.cancelled().await;
        health_manager_for_shutdown.trigger_shutdown().await;
    });

    // Start HTTP server
    let http_port = config.http.port;
    let http_tls_config = config.http.tls.clone();
    let http_shutdown = shutdown_token.clone();
    let http_handle = tokio::spawn(async move {
        if let Some(ref tls) = http_tls_config {
            if tls.enabled {
                println!("🔒 HTTPS server listening on https://localhost:{}", http_port);
            } else {
                println!("📡 HTTP server listening on http://localhost:{}", http_port);
            }
        } else {
            println!("📡 HTTP server listening on http://localhost:{}", http_port);
        }
        tokio::select! {
            result = mockforge_http::serve_router_with_tls(http_port, http_app, http_tls_config) => {
                result.map_err(|e| format!("HTTP server error: {}", e))
            }
            _ = http_shutdown.cancelled() => {
                Ok(())
            }
        }
    });

    // Start WebSocket server
    let ws_port = config.websocket.port;
    let ws_shutdown = shutdown_token.clone();
    let ws_handle = tokio::spawn(async move {
        println!("🔌 WebSocket server listening on ws://localhost:{}", ws_port);
        tokio::select! {
            result = mockforge_ws::start_with_latency(ws_port, None) => {
                result.map_err(|e| format!("WebSocket server error: {}", e))
            }
            _ = ws_shutdown.cancelled() => {
                Ok(())
            }
        }
    });

    // Start gRPC server
    let grpc_port = config.grpc.port;
    let grpc_shutdown = shutdown_token.clone();
    let grpc_handle = tokio::spawn(async move {
        println!("⚡ gRPC server listening on localhost:{}", grpc_port);
        tokio::select! {
            result = mockforge_grpc::start(grpc_port) => {
                result.map_err(|e| format!("gRPC server error: {}", e))
            }
            _ = grpc_shutdown.cancelled() => {
                Ok(())
            }
        }
    });

    #[cfg(feature = "smtp")]
    let _smtp_handle = if let Some(ref smtp_registry) = smtp_registry {
        let smtp_config = config.smtp.clone();
        let smtp_shutdown = shutdown_token.clone();

        // Convert core SmtpConfig to mockforge_smtp::SmtpConfig
        let server_config = mockforge_smtp::SmtpConfig {
            port: smtp_config.port,
            host: smtp_config.host.clone(),
            hostname: smtp_config.hostname.clone(),
            fixtures_dir: smtp_config.fixtures_dir.clone(),
            timeout_secs: smtp_config.timeout_secs,
            max_connections: smtp_config.max_connections,
            enable_mailbox: smtp_config.enable_mailbox,
            max_mailbox_messages: smtp_config.max_mailbox_messages,
            enable_starttls: smtp_config.enable_starttls,
            tls_cert_path: smtp_config.tls_cert_path.clone(),
            tls_key_path: smtp_config.tls_key_path.clone(),
        };

        // Downcast the registry with proper error handling
        let smtp_reg = match smtp_registry.clone().downcast::<mockforge_smtp::SmtpSpecRegistry>() {
            Ok(reg) => reg,
            Err(_) => {
                use crate::progress::{CliError, ExitCode};
                CliError::new(
                    "SMTP registry type mismatch - failed to downcast registry".to_string(),
                    ExitCode::ConfigurationError,
                )
                .with_suggestion(
                    "Ensure SMTP registry is properly configured and initialized".to_string(),
                )
                .display_and_exit();
            }
        };

        Some(tokio::spawn(async move {
            println!("📧 SMTP server listening on {}:{}", smtp_config.host, smtp_config.port);

            tokio::select! {
                result = async {
                    let server = mockforge_smtp::SmtpServer::new(server_config, smtp_reg)?;
                    server.start().await
                } => {
                    result.map_err(|e| format!("SMTP server error: {}", e))
                }
                _ = smtp_shutdown.cancelled() => {
                    println!("🛑 Shutting down SMTP server...");
                    Ok(())
                }
            }
        }))
    } else {
        None
    };

    #[cfg(feature = "mqtt")]
    let _mqtt_handle = if let Some(ref _mqtt_registry) = mqtt_registry {
        let mqtt_config = config.mqtt.clone();
        let mqtt_shutdown = shutdown_token.clone();

        // Convert core MqttConfig to mockforge_mqtt::MqttConfig
        let broker_config = mockforge_mqtt::broker::MqttConfig {
            port: mqtt_config.port,
            host: mqtt_config.host.clone(),
            max_connections: mqtt_config.max_connections,
            max_packet_size: mqtt_config.max_packet_size,
            keep_alive_secs: mqtt_config.keep_alive_secs,
            version: mockforge_mqtt::broker::MqttVersion::default(),
        };

        Some(tokio::spawn(async move {
            use mockforge_mqtt::start_mqtt_server;

            println!("📡 MQTT broker listening on {}:{}", mqtt_config.host, mqtt_config.port);

            // Start the MQTT server
            tokio::select! {
                result = start_mqtt_server(broker_config) => {
                    result.map_err(|e| format!("MQTT server error: {:?}", e))
                }
                _ = mqtt_shutdown.cancelled() => {
                    println!("🛑 Shutting down MQTT broker...");
                    Ok(())
                }
            }
        }))
    } else {
        None
    };
    #[cfg(not(feature = "mqtt"))]
    let _mqtt_handle: Option<tokio::task::JoinHandle<Result<(), String>>> = None;

    // Auto-start tunnel if deceptive deploy is enabled with auto_tunnel
    let tunnel_handle = if config.deceptive_deploy.enabled && config.deceptive_deploy.auto_tunnel {
        use mockforge_tunnel::{TunnelConfig, TunnelManager, TunnelProvider};
        use std::sync::Arc;
        use tokio::time::{sleep, Duration};

        let local_url = format!("http://localhost:{}", http_port);
        let deploy_config = config.deceptive_deploy.clone();
        let tunnel_shutdown = shutdown_token.clone();

        Some(tokio::spawn(async move {
            // Wait a bit for the server to be ready
            sleep(Duration::from_secs(2)).await;

            let provider = TunnelProvider::SelfHosted; // Default to self-hosted
            let mut tunnel_config = TunnelConfig::new(&local_url).with_provider(provider);

            // Use custom domain if specified
            if let Some(domain) = deploy_config.custom_domain {
                tunnel_config.custom_domain = Some(domain);
            }

            // Get tunnel server URL from environment or use default
            if let Ok(server_url) = std::env::var("MOCKFORGE_TUNNEL_SERVER_URL") {
                tunnel_config.server_url = Some(server_url);
            }

            // Get auth token from environment if available
            if let Ok(auth_token) = std::env::var("MOCKFORGE_TUNNEL_AUTH_TOKEN") {
                tunnel_config.auth_token = Some(auth_token);
            }

            match TunnelManager::new(&tunnel_config) {
                Ok(manager) => {
                    println!("🌐 Starting tunnel for deceptive deploy...");
                    match manager.create_tunnel(&tunnel_config).await {
                        Ok(status) => {
                            println!("✅ Tunnel created successfully!");
                            println!("   Public URL: {}", status.public_url);
                            println!("   Tunnel ID: {}", status.tunnel_id);
                            println!(
                                "💡 Your mock API is now accessible at: {}",
                                status.public_url
                            );

                            // Wait for shutdown signal
                            tokio::select! {
                                _ = tunnel_shutdown.cancelled() => {
                                    println!("🛑 Stopping tunnel...");
                                    if let Err(e) = manager.stop_tunnel().await {
                                        eprintln!("⚠️  Warning: Failed to stop tunnel: {}", e);
                                    }
                                    Ok::<(), anyhow::Error>(())
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("⚠️  Warning: Failed to create tunnel: {}", e);
                            eprintln!("💡 You can start a tunnel manually with: mockforge tunnel start --local-url {}", local_url);
                            Ok(())
                        }
                    }
                }
                Err(e) => {
                    eprintln!("⚠️  Warning: Failed to initialize tunnel manager: {}", e);
                    eprintln!("💡 You can start a tunnel manually with: mockforge tunnel start --local-url {}", local_url);
                    Ok(())
                }
            }
        }))
    } else {
        None
    };

    // Start Kafka broker (if enabled)
    #[cfg(feature = "kafka")]
    let _kafka_handle = if config.kafka.enabled {
        let kafka_config = config.kafka.clone();
        let kafka_shutdown = shutdown_token.clone();

        Some(tokio::spawn(async move {
            use mockforge_kafka::KafkaMockBroker;

            println!("📨 Kafka broker listening on {}:{}", kafka_config.host, kafka_config.port);

            // Create and start the Kafka broker
            match KafkaMockBroker::new(kafka_config.clone()).await {
                Ok(broker) => {
                    tokio::select! {
                        result = broker.start() => {
                            result.map_err(|e| format!("Kafka broker error: {:?}", e))
                        }
                        _ = kafka_shutdown.cancelled() => {
                            println!("🛑 Shutting down Kafka broker...");
                            Ok(())
                        }
                    }
                }
                Err(e) => Err(format!("Failed to initialize Kafka broker: {:?}", e)),
            }
        }))
    } else {
        None
    };
    #[cfg(not(feature = "kafka"))]
    let _kafka_handle: Option<tokio::task::JoinHandle<Result<(), String>>> = None;

    // Start AMQP broker (if enabled)
    #[cfg(feature = "amqp")]
    let _amqp_handle = if config.amqp.enabled {
        let amqp_config = config.amqp.clone();
        let amqp_shutdown = shutdown_token.clone();

        Some(tokio::spawn(async move {
            use mockforge_amqp::{AmqpBroker, AmqpSpecRegistry};
            use std::sync::Arc;

            println!("🐰 AMQP broker listening on {}:{}", amqp_config.host, amqp_config.port);

            // Create spec registry
            let spec_registry = Arc::new(
                AmqpSpecRegistry::new(amqp_config.clone())
                    .await
                    .map_err(|e| format!("Failed to create AMQP spec registry: {:?}", e))?,
            );

            // Load fixtures if configured
            if let Some(ref fixtures_dir) = amqp_config.fixtures_dir {
                if fixtures_dir.exists() {
                    println!("   Loading AMQP fixtures from {:?}", fixtures_dir);
                }
            }

            // Create and start the AMQP broker
            let broker = AmqpBroker::new(amqp_config.clone(), spec_registry);
            tokio::select! {
                result = broker.start() => {
                    result.map_err(|e| format!("AMQP broker error: {:?}", e))
                }
                _ = amqp_shutdown.cancelled() => {
                    println!("🛑 Shutting down AMQP broker...");
                    Ok(())
                }
            }
        }))
    } else {
        None
    };
    #[cfg(not(feature = "amqp"))]
    let _amqp_handle: Option<tokio::task::JoinHandle<Result<(), String>>> = None;

    // Start TCP server (if enabled)
    #[cfg(feature = "tcp")]
    let _tcp_handle = if config.tcp.enabled {
        use mockforge_tcp::{TcpConfig as TcpServerConfig, TcpServer, TcpSpecRegistry};
        use std::sync::Arc;

        let tcp_config = config.tcp.clone();
        let tcp_shutdown = shutdown_token.clone();

        // Convert core TcpConfig to mockforge_tcp::TcpConfig
        let server_config = TcpServerConfig {
            port: tcp_config.port,
            host: tcp_config.host.clone(),
            max_connections: tcp_config.max_connections,
            timeout_secs: tcp_config.timeout_secs,
            fixtures_dir: tcp_config.fixtures_dir.clone(),
            echo_mode: tcp_config.echo_mode,
            enable_tls: tcp_config.enable_tls,
            tls_cert_path: tcp_config.tls_cert_path.clone(),
            tls_key_path: tcp_config.tls_key_path.clone(),
            read_buffer_size: 8192, // Default buffer sizes
            write_buffer_size: 8192,
            delimiter: None, // Stream mode by default
        };

        Some(tokio::spawn(async move {
            let mut registry = TcpSpecRegistry::new();

            // Load fixtures if configured
            if let Some(ref fixtures_dir) = server_config.fixtures_dir {
                if fixtures_dir.exists() {
                    if let Err(e) = registry.load_fixtures(fixtures_dir) {
                        eprintln!(
                            "⚠️  Warning: Failed to load TCP fixtures from {:?}: {}",
                            fixtures_dir, e
                        );
                    } else {
                        println!("   Loaded TCP fixtures from {:?}", fixtures_dir);
                    }
                }
            }

            let registry_arc = Arc::new(registry);

            println!("🔌 TCP server listening on {}:{}", server_config.host, server_config.port);

            match TcpServer::new(server_config, registry_arc) {
                Ok(server) => {
                    tokio::select! {
                        result = server.start() => {
                            result.map_err(|e| format!("TCP server error: {}", e))
                        }
                        _ = tcp_shutdown.cancelled() => {
                            println!("🛑 Shutting down TCP server...");
                            Ok(())
                        }
                    }
                }
                Err(e) => Err(format!("Failed to initialize TCP server: {}", e)),
            }
        }))
    } else {
        None
    };
    #[cfg(not(feature = "tcp"))]
    let _tcp_handle: Option<tokio::task::JoinHandle<Result<(), String>>> = None;

    // Start Admin UI server (if enabled)
    let admin_handle = if config.admin.enabled {
        let admin_port = config.admin.port;
        let http_port = config.http.port;
        let ws_port = config.websocket.port;
        let grpc_port = config.grpc.port;
        let prometheus_url = config.admin.prometheus_url.clone();
        let admin_shutdown = shutdown_token.clone();
        // Clone all host values before the async move closure
        let admin_host = config.admin.host.clone();
        let http_host = config.http.host.clone();
        let ws_host = config.websocket.host.clone();
        let grpc_host = config.grpc.host.clone();
        Some(tokio::spawn(async move {
            println!("🎛️ Admin UI listening on http://{}:{}", admin_host, admin_port);

            // Parse addresses with proper error handling
            use crate::progress::parse_address;
            let addr = match parse_address(&format!("{}:{}", admin_host, admin_port), "admin UI") {
                Ok(addr) => addr,
                Err(e) => {
                    return Err(format!(
                        "Failed to bind Admin UI to {}:{}: {}",
                        admin_host, admin_port, e.message
                    ))
                }
            };

            let http_addr =
                match parse_address(&format!("{}:{}", http_host, http_port), "HTTP server") {
                    Ok(addr) => Some(addr),
                    Err(e) => {
                        return Err(format!(
                            "Failed to parse HTTP server address {}:{}: {}",
                            http_host, http_port, e.message
                        ))
                    }
                };
            let ws_addr =
                match parse_address(&format!("{}:{}", ws_host, ws_port), "WebSocket server") {
                    Ok(addr) => Some(addr),
                    Err(e) => {
                        return Err(format!(
                            "Failed to parse WebSocket server address {}:{}: {}",
                            ws_host, ws_port, e.message
                        ))
                    }
                };
            let grpc_addr =
                match parse_address(&format!("{}:{}", grpc_host, grpc_port), "gRPC server") {
                    Ok(addr) => Some(addr),
                    Err(e) => {
                        return Err(format!(
                            "Failed to parse gRPC server address {}:{}: {}",
                            grpc_host, grpc_port, e.message
                        ))
                    }
                };

            tokio::select! {
                result = mockforge_ui::start_admin_server(
                    addr,
                    http_addr,
                    ws_addr,
                    grpc_addr,
                    None,
                    true,
                    prometheus_url,
                ) => {
                    result.map_err(|e| format!("Admin UI server error: {}", e))
                }
                _ = admin_shutdown.cancelled() => {
                    Ok(())
                }
            }
        }))
    } else {
        None
    };

    // Start Prometheus metrics server (if enabled)
    let metrics_handle = if config.observability.prometheus.enabled {
        let metrics_port = config.observability.prometheus.port;
        let metrics_registry = metrics_registry.clone();
        let metrics_shutdown = shutdown_token.clone();
        Some(tokio::spawn(async move {
            println!(
                "📊 Prometheus metrics server listening on http://0.0.0.0:{}/metrics",
                metrics_port
            );
            let app = prometheus_router(metrics_registry);
            let addr = SocketAddr::from(([0, 0, 0, 0], metrics_port));
            let listener = TcpListener::bind(addr)
                .await
                .map_err(|e| format!("Failed to bind metrics server to {}: {}", addr, e))?;
            tokio::select! {
                result = serve(listener, app) => {
                    result.map_err(|e| format!("Metrics server error: {}", e))
                }
                _ = metrics_shutdown.cancelled() => {
                    Ok(())
                }
            }
        }))
    } else {
        None
    };

    // Give servers a moment to start, then mark service as ready
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    health_manager.set_ready().await;
    tracing::info!("Service marked as ready - all servers initialized");

    // Wait for all servers or shutdown signal, handling errors properly
    let result = tokio::select! {
        result = http_handle => {
            match result {
                Ok(Ok(())) => {
                    println!("📡 HTTP server stopped gracefully");
                    None
                }
                Ok(Err(e)) => {
                    eprintln!("❌ {}", e);
                    Some(e)
                }
                Err(e) => {
                    let error = format!("HTTP server task panicked: {}", e);
                    eprintln!("❌ {}", error);
                    Some(error)
                }
            }
        }
        result = ws_handle => {
            match result {
                Ok(Ok(())) => {
                    println!("🔌 WebSocket server stopped gracefully");
                    None
                }
                Ok(Err(e)) => {
                    eprintln!("❌ {}", e);
                    Some(e)
                }
                Err(e) => {
                    let error = format!("WebSocket server task panicked: {}", e);
                    eprintln!("❌ {}", error);
                    Some(error)
                }
            }
        }
        result = grpc_handle => {
            match result {
                Ok(Ok(())) => {
                    println!("⚡ gRPC server stopped gracefully");
                    None
                }
                Ok(Err(e)) => {
                    eprintln!("❌ {}", e);
                    Some(e)
                }
                Err(e) => {
                    let error = format!("gRPC server task panicked: {}", e);
                    eprintln!("❌ {}", error);
                    Some(error)
                }
            }
        }
        result = async {
            if let Some(handle) = admin_handle {
                Some(handle.await)
            } else {
                std::future::pending::<Option<Result<Result<(), String>, tokio::task::JoinError>>>().await
            }
        } => {
            match result {
                Some(Ok(Ok(()))) => {
                    println!("🎛️ Admin UI stopped gracefully");
                    None
                }
                Some(Ok(Err(e))) => {
                    eprintln!("❌ {}", e);
                    Some(e)
                }
                Some(Err(e)) => {
                    let error = format!("Admin UI task panicked: {}", e);
                    eprintln!("❌ {}", error);
                    Some(error)
                }
                None => None
            }
        }
        result = async {
            if let Some(handle) = metrics_handle {
                Some(handle.await)
            } else {
                std::future::pending::<Option<Result<Result<(), String>, tokio::task::JoinError>>>().await
            }
        } => {
            match result {
                Some(Ok(Ok(()))) => {
                    println!("📊 Metrics server stopped gracefully");
                    None
                }
                Some(Ok(Err(e))) => {
                    eprintln!("❌ {}", e);
                    Some(e)
                }
                Some(Err(e)) => {
                    let error = format!("Metrics server task panicked: {}", e);
                    eprintln!("❌ {}", error);
                    Some(error)
                }
                None => None
            }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("🛑 Received shutdown signal");
            // Trigger health manager shutdown
            health_manager.trigger_shutdown().await;
            None
        }
    };

    // Trigger shutdown for all remaining tasks
    println!("👋 Shutting down remaining servers...");
    shutdown_token.cancel();

    // Give tasks a moment to shut down gracefully
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Return error if any server failed
    if let Some(error) = result {
        Err(error.into())
    } else {
        Ok(())
    }
}

/// Handle contract-diff commands
async fn handle_contract_diff(
    diff_command: ContractDiffCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use contract_diff_commands::{
        handle_contract_diff_analyze, handle_contract_diff_apply_patch,
        handle_contract_diff_compare, handle_contract_diff_generate_patch,
    };
    use mockforge_core::ai_contract_diff::ContractDiffConfig;

    match diff_command {
        ContractDiffCommands::Analyze {
            spec,
            request_path,
            capture_id,
            output,
            llm_provider,
            llm_model,
            llm_api_key,
            confidence_threshold,
        } => {
            // Build config from CLI args
            let config = if llm_provider.is_some()
                || llm_model.is_some()
                || llm_api_key.is_some()
                || confidence_threshold.is_some()
            {
                let mut cfg = ContractDiffConfig::default();
                if let Some(provider) = llm_provider {
                    cfg.llm_provider = provider;
                }
                if let Some(model) = llm_model {
                    cfg.llm_model = model;
                }
                if let Some(api_key) = llm_api_key {
                    cfg.api_key = Some(api_key);
                }
                if let Some(threshold) = confidence_threshold {
                    cfg.confidence_threshold = threshold;
                }
                Some(cfg)
            } else {
                None
            };

            handle_contract_diff_analyze(spec, request_path, capture_id, output, config).await?;
        }
        ContractDiffCommands::Compare {
            old_spec,
            new_spec,
            output,
        } => {
            handle_contract_diff_compare(old_spec, new_spec, output).await?;
        }
        ContractDiffCommands::GeneratePatch {
            spec,
            request_path,
            capture_id,
            output,
            llm_provider,
            llm_model,
            llm_api_key,
        } => {
            // Build config from CLI args
            let config = if llm_provider.is_some() || llm_model.is_some() || llm_api_key.is_some() {
                let mut cfg = ContractDiffConfig::default();
                if let Some(provider) = llm_provider {
                    cfg.llm_provider = provider;
                }
                if let Some(model) = llm_model {
                    cfg.llm_model = model;
                }
                if let Some(api_key) = llm_api_key {
                    cfg.api_key = Some(api_key);
                }
                Some(cfg)
            } else {
                None
            };

            handle_contract_diff_generate_patch(spec, request_path, capture_id, output, config)
                .await?;
        }
        ContractDiffCommands::ApplyPatch {
            spec,
            patch,
            output,
        } => {
            handle_contract_diff_apply_patch(spec, patch, output).await?;
        }
    }

    Ok(())
}

async fn handle_data(
    data_command: DataCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match data_command {
        DataCommands::Template {
            template,
            rows,
            format,
            output,
            rag,
            rag_provider,
            rag_model,
            rag_endpoint,
            rag_timeout,
            rag_max_retries,
        } => {
            println!("🎯 Generating {} rows using '{}' template", rows, template);
            println!("📄 Output format: {}", format);
            if rag {
                println!("🧠 RAG mode enabled");
                if let Some(provider) = &rag_provider {
                    println!("🤖 RAG Provider: {}", provider);
                }
                if let Some(model) = &rag_model {
                    println!("🧠 RAG Model: {}", model);
                }
            }
            if let Some(output_path) = &output {
                println!("💾 Output file: {}", output_path.display());
            }

            // Generate data using the specified template
            let result = generate_from_template(
                &template,
                rows,
                rag,
                rag_provider,
                rag_model,
                rag_endpoint,
                rag_timeout,
                rag_max_retries,
            )
            .await?;

            // Format and output the result
            output_result(result, format, output).await?;
        }
        DataCommands::Schema {
            schema,
            rows,
            format,
            output,
        } => {
            println!("📋 Generating {} rows from schema: {}", rows, schema.display());
            println!("📄 Output format: {}", format);
            if let Some(output_path) = &output {
                println!("💾 Output file: {}", output_path.display());
            }

            // Generate data from JSON schema
            let result = generate_from_json_schema_file(&schema, rows).await?;

            // Format and output the result
            output_result(result, format, output).await?;
        }
        DataCommands::MockOpenapi {
            spec,
            rows,
            format,
            output,
            realistic,
            include_optional,
            validate,
            array_size,
            max_array_size,
        } => {
            println!("🚀 Generating mock data from OpenAPI spec: {}", spec.display());
            println!("📊 Rows per schema: {}", rows);
            println!("📄 Output format: {}", format);
            if realistic {
                println!("🎭 Realistic data generation enabled");
            }
            if include_optional {
                println!("📝 Including optional fields");
            }
            if validate {
                println!("✅ Schema validation enabled");
            }
            println!("📏 Array size: {} (max: {})", array_size, max_array_size);
            if let Some(output_path) = &output {
                println!("💾 Output file: {}", output_path.display());
            }

            // Generate mock data from OpenAPI spec
            let result = generate_mock_data_from_openapi(
                &spec,
                rows,
                realistic,
                include_optional,
                validate,
                array_size,
                max_array_size,
            )
            .await?;

            // Format and output the result
            output_mock_data_result(result, format, output).await?;
        }
        DataCommands::MockServer {
            spec,
            port,
            host,
            cors,
            log_requests,
            delay,
            realistic,
            include_optional,
            validate,
        } => {
            println!("🌐 Starting mock server based on OpenAPI spec: {}", spec.display());
            println!("🔗 Server will run on {}:{}", host, port);
            if cors {
                println!("🌍 CORS enabled");
            }
            if log_requests {
                println!("📝 Request logging enabled");
            }
            if !delay.is_empty() {
                println!("⏱️ Response delays configured: {:?}", delay);
            }
            if realistic {
                println!("🎭 Realistic data generation enabled");
            }
            if include_optional {
                println!("📝 Including optional fields");
            }
            if validate {
                println!("✅ Schema validation enabled");
            }

            // Start the mock server
            start_mock_server_from_spec(
                &spec,
                port,
                &host,
                cors,
                log_requests,
                delay,
                realistic,
                include_optional,
                validate,
            )
            .await?;
        }
    }

    Ok(())
}

async fn handle_admin(
    port: u16,
    _config: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🎛️ Starting MockForge Admin UI...");

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
    )
    .await?;

    println!("✅ Admin UI started successfully!");
    println!("🌐 Access at: http://localhost:{}/", port);

    // Keep running until shutdown signal
    tokio::signal::ctrl_c().await?;
    println!("👋 Shutting down admin UI...");

    Ok(())
}

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

    println!("\n⚡ MockForge Quick Mock Mode");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📁 Loading data from: {}", file.display());

    // Load JSON file
    let json_str = fs::read_to_string(&file)
        .map_err(|e| format!("Failed to read file '{}': {}", file.display(), e))?;

    let json_data: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse JSON from '{}': {}", file.display(), e))?;

    println!("✓ JSON loaded successfully");

    // Create quick mock state
    println!("🔍 Auto-detecting routes from JSON keys...");
    let state = QuickMockState::from_json(json_data)
        .await
        .map_err(|e| format!("Failed to create quick mock state: {}", e))?;

    let resource_names = state.resource_names().await;
    println!("✓ Detected {} resource(s):", resource_names.len());
    for resource in &resource_names {
        println!("  • /{}", resource);
    }

    // Build router
    let app = build_quick_router(state).await;

    println!();
    println!("🚀 Quick Mock Server Configuration:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
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
    println!("📚 Available Endpoints:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    for resource in &resource_names {
        println!("   GET    /{}          - List all", resource);
        println!("   GET    /{}/:id      - Get by ID", resource);
        println!("   POST   /{}          - Create new", resource);
        println!("   PUT    /{}/:id      - Update by ID", resource);
        println!("   DELETE /{}/:id      - Delete by ID", resource);
        println!();
    }
    println!("   GET    /__quick/info       - API information");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Start server
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    let listener = TcpListener::bind(addr).await?;

    println!();
    println!("✅ Server started successfully!");
    println!("💡 Press Ctrl+C to stop");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Serve with graceful shutdown
    serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.unwrap_or_else(|e| {
                eprintln!("⚠️  Warning: Failed to install CTRL+C signal handler: {}", e);
                eprintln!("💡 Server may not shut down gracefully on SIGINT");
            });
        })
        .await?;

    println!("\n👋 Server stopped\n");

    Ok(())
}

async fn handle_sync(
    workspace_dir: PathBuf,
    _config: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("\n🔄 Starting MockForge Sync Daemon...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📁 Workspace directory: {}", workspace_dir.display());
    println!();
    println!("ℹ️  What the sync daemon does:");
    println!("   • Monitors the workspace directory for .yaml/.yml file changes");
    println!("   • Automatically imports new or modified request files");
    println!("   • Syncs changes bidirectionally between files and workspace");
    println!("   • Skips hidden files (starting with .)");
    println!();
    println!("🔍 Monitoring for file changes...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Create sync service
    let sync_service = mockforge_core::SyncService::new(&workspace_dir);

    // Start the sync service
    sync_service.start().await?;

    println!("✅ Sync daemon started successfully!");
    println!("💡 Press Ctrl+C to stop\n");

    // Keep running until shutdown signal
    tokio::signal::ctrl_c().await?;
    println!("\n🛑 Received shutdown signal");

    // Stop the sync service
    sync_service.stop().await?;
    println!("👋 Sync daemon stopped\n");

    Ok(())
}

/// Load RAG configuration from environment variables and CLI options
fn load_rag_config(
    provider_override: Option<String>,
    model_override: Option<String>,
    endpoint_override: Option<String>,
    timeout_override: Option<u64>,
    max_retries_override: Option<usize>,
) -> RagConfig {
    let provider = provider_override
        .or_else(|| std::env::var("MOCKFORGE_RAG_PROVIDER").ok())
        .unwrap_or_else(|| "openai".to_string());

    let llm_provider = match provider.to_lowercase().as_str() {
        "anthropic" => LlmProvider::Anthropic,
        "ollama" => LlmProvider::Ollama,
        "openai_compatible" => LlmProvider::OpenAICompatible,
        _ => LlmProvider::OpenAI,
    };

    let embedding_provider = match std::env::var("MOCKFORGE_EMBEDDING_PROVIDER")
        .unwrap_or_else(|_| "openai".to_string())
        .to_lowercase()
        .as_str()
    {
        "openai_compatible" => EmbeddingProvider::OpenAICompatible,
        _ => EmbeddingProvider::OpenAI,
    };

    RagConfig {
        provider: llm_provider.clone(),
        api_endpoint: endpoint_override
            .or_else(|| std::env::var("MOCKFORGE_RAG_API_ENDPOINT").ok())
            .unwrap_or_else(|| match llm_provider {
                LlmProvider::OpenAI => "https://api.openai.com/v1/chat/completions".to_string(),
                LlmProvider::Anthropic => "https://api.anthropic.com/v1/messages".to_string(),
                LlmProvider::Ollama => "http://localhost:11434/api/generate".to_string(),
                LlmProvider::OpenAICompatible => {
                    "http://localhost:8000/v1/chat/completions".to_string()
                }
            }),
        api_key: std::env::var("MOCKFORGE_RAG_API_KEY").ok(),
        model: model_override
            .or_else(|| std::env::var("MOCKFORGE_RAG_MODEL").ok())
            .unwrap_or_else(|| match llm_provider {
                LlmProvider::OpenAI => "gpt-3.5-turbo".to_string(),
                LlmProvider::Anthropic => "claude-3-sonnet-20240229".to_string(),
                LlmProvider::Ollama => "llama2".to_string(),
                LlmProvider::OpenAICompatible => "gpt-3.5-turbo".to_string(),
            }),
        max_tokens: std::env::var("MOCKFORGE_RAG_MAX_TOKENS")
            .unwrap_or_else(|_| "1000".to_string())
            .parse()
            .unwrap_or(1000),
        temperature: std::env::var("MOCKFORGE_RAG_TEMPERATURE")
            .unwrap_or_else(|_| "0.7".to_string())
            .parse()
            .unwrap_or(0.7),
        context_window: std::env::var("MOCKFORGE_RAG_CONTEXT_WINDOW")
            .unwrap_or_else(|_| "4000".to_string())
            .parse()
            .unwrap_or(4000),
        semantic_search_enabled: std::env::var("MOCKFORGE_SEMANTIC_SEARCH")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true),
        embedding_provider,
        embedding_model: std::env::var("MOCKFORGE_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-ada-002".to_string()),
        embedding_endpoint: std::env::var("MOCKFORGE_EMBEDDING_ENDPOINT").ok(),
        similarity_threshold: std::env::var("MOCKFORGE_SIMILARITY_THRESHOLD")
            .unwrap_or_else(|_| "0.7".to_string())
            .parse()
            .unwrap_or(0.7),
        max_chunks: std::env::var("MOCKFORGE_MAX_CHUNKS")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .unwrap_or(5),
        request_timeout_seconds: timeout_override
            .or_else(|| {
                std::env::var("MOCKFORGE_RAG_TIMEOUT_SECONDS").ok().and_then(|s| s.parse().ok())
            })
            .unwrap_or(30),
        max_retries: max_retries_override
            .or_else(|| {
                std::env::var("MOCKFORGE_RAG_MAX_RETRIES").ok().and_then(|s| s.parse().ok())
            })
            .unwrap_or(3),
    }
}

/// Generate data from a predefined template
#[allow(clippy::too_many_arguments)]
async fn generate_from_template(
    template: &str,
    _rows: usize,
    rag_enabled: bool,
    rag_provider: Option<String>,
    rag_model: Option<String>,
    rag_endpoint: Option<String>,
    rag_timeout: Option<u64>,
    rag_max_retries: Option<usize>,
) -> Result<mockforge_data::GenerationResult, Box<dyn std::error::Error + Send + Sync>> {
    use mockforge_data::schema::templates;

    let config = mockforge_data::DataConfig {
        rows: _rows,
        rag_enabled,
        ..Default::default()
    };

    let schema = match template.to_lowercase().as_str() {
        "user" | "users" => templates::user_schema(),
        "product" | "products" => templates::product_schema(),
        "order" | "orders" => templates::order_schema(),
        _ => {
            return Err(format!(
                "Unknown template: {}. Available templates: user, product, order",
                template
            )
            .into());
        }
    };

    let mut generator = mockforge_data::DataGenerator::new(schema, config)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    // Configure RAG if enabled
    if rag_enabled {
        let rag_config = load_rag_config(
            rag_provider.clone(),
            rag_model.clone(),
            rag_endpoint.clone(),
            rag_timeout,
            rag_max_retries,
        );
        generator
            .configure_rag(rag_config)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    }

    generator
        .generate()
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

/// Generate data from a JSON schema file
async fn generate_from_json_schema_file(
    schema_path: &PathBuf,
    rows: usize,
) -> Result<mockforge_data::GenerationResult, Box<dyn std::error::Error + Send + Sync>> {
    // Read the JSON schema file
    let schema_content = tokio::fs::read_to_string(schema_path).await?;
    let schema_json: serde_json::Value = serde_json::from_str(&schema_content)?;

    // Generate data from the schema
    mockforge_data::generate_from_json_schema(&schema_json, rows)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

/// Output the generation result in the specified format
async fn output_result(
    result: mockforge_data::GenerationResult,
    format: String,
    output_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let output_content = match format.to_lowercase().as_str() {
        "json" => result.to_json_string()?,
        "jsonl" | "jsonlines" => result.to_jsonl_string()?,
        "csv" => {
            // For CSV, we'll need to convert JSON to CSV format
            // This is a simplified implementation - in a real system you'd use a proper CSV library
            let mut csv_output = String::new();

            if let Some(first_row) = result.data.first() {
                if let Some(obj) = first_row.as_object() {
                    // Add header row
                    let headers: Vec<String> = obj.keys().map(|k| k.to_string()).collect();
                    csv_output.push_str(&headers.join(","));
                    csv_output.push('\n');

                    // Add data rows
                    for row in &result.data {
                        if let Some(obj) = row.as_object() {
                            let values: Vec<String> = headers
                                .iter()
                                .map(|header| {
                                    obj.get(header)
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string()
                                })
                                .collect();
                            csv_output.push_str(&values.join(","));
                            csv_output.push('\n');
                        }
                    }
                }
            }
            csv_output
        }
        _ => result.to_json_string()?, // Default to JSON
    };

    // Output to file or stdout
    if let Some(path) = output_path {
        tokio::fs::write(&path, &output_content).await?;
        println!("💾 Data written to: {}", path.display());
    } else {
        println!("{}", output_content);
    }

    println!("✅ Generated {} rows in {}ms", result.count, result.generation_time_ms);

    if !result.warnings.is_empty() {
        println!("⚠️  Warnings:");
        for warning in result.warnings {
            println!("   - {}", warning);
        }
    }

    Ok(())
}

/// Generate mock data from OpenAPI specification
async fn generate_mock_data_from_openapi(
    spec_path: &PathBuf,
    rows: usize,
    realistic: bool,
    include_optional: bool,
    validate: bool,
    array_size: usize,
    max_array_size: usize,
) -> Result<mockforge_data::MockDataResult, Box<dyn std::error::Error + Send + Sync>> {
    // Read the OpenAPI specification file
    let spec_content = tokio::fs::read_to_string(spec_path).await?;

    // Parse JSON or YAML
    let spec_json: serde_json::Value = if spec_path.extension().and_then(|s| s.to_str())
        == Some("yaml")
        || spec_path.extension().and_then(|s| s.to_str()) == Some("yml")
    {
        serde_yaml::from_str(&spec_content)?
    } else {
        serde_json::from_str(&spec_content)?
    };

    // Create generator configuration
    let config = mockforge_data::MockGeneratorConfig::new()
        .realistic_mode(realistic)
        .include_optional_fields(include_optional)
        .validate_generated_data(validate)
        .default_array_size(array_size)
        .max_array_size(max_array_size);

    // Generate mock data
    let mut generator = mockforge_data::MockDataGenerator::with_config(config);
    generator
        .generate_from_openapi_spec(&spec_json)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

/// Output mock data result in the specified format
async fn output_mock_data_result(
    result: mockforge_data::MockDataResult,
    format: String,
    output_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let output_content = match format.to_lowercase().as_str() {
        "json" => serde_json::to_string_pretty(&result)?,
        "jsonl" | "jsonlines" => {
            // Convert to JSONL format
            let mut jsonl_output = String::new();

            // Add schemas
            for (schema_name, schema_data) in &result.schemas {
                let schema_line = json!({
                    "type": "schema",
                    "name": schema_name,
                    "data": schema_data
                });
                jsonl_output.push_str(&serde_json::to_string(&schema_line)?);
                jsonl_output.push('\n');
            }

            // Add responses
            for (endpoint, response) in &result.responses {
                let response_line = json!({
                    "type": "response",
                    "endpoint": endpoint,
                    "status": response.status,
                    "headers": response.headers,
                    "body": response.body
                });
                jsonl_output.push_str(&serde_json::to_string(&response_line)?);
                jsonl_output.push('\n');
            }

            jsonl_output
        }
        "csv" => {
            // For CSV, we'll create a simplified format
            let mut csv_output = String::new();
            csv_output.push_str("type,name,endpoint,status,data\n");

            // Add schemas
            for (schema_name, schema_data) in &result.schemas {
                csv_output.push_str(&format!(
                    "schema,{},\"\",\"\",{}\n",
                    schema_name,
                    serde_json::to_string(schema_data)?.replace("\"", "\"\"")
                ));
            }

            // Add responses
            for (endpoint, response) in &result.responses {
                csv_output.push_str(&format!(
                    "response,\"\",{},{},{}\n",
                    endpoint.replace("\"", "\"\""),
                    response.status,
                    serde_json::to_string(&response.body)?.replace("\"", "\"\"")
                ));
            }

            csv_output
        }
        _ => serde_json::to_string_pretty(&result)?, // Default to JSON
    };

    // Output to file or stdout
    if let Some(path) = output_path {
        tokio::fs::write(&path, &output_content).await?;
        println!("💾 Mock data written to: {}", path.display());
    } else {
        println!("{}", output_content);
    }

    println!(
        "✅ Generated mock data for {} schemas and {} endpoints",
        result.schemas.len(),
        result.responses.len()
    );

    if !result.warnings.is_empty() {
        println!("⚠️  Warnings:");
        for warning in result.warnings {
            println!("   - {}", warning);
        }
    }

    Ok(())
}

/// Start mock server from OpenAPI specification
async fn start_mock_server_from_spec(
    spec_path: &PathBuf,
    port: u16,
    host: &str,
    cors: bool,
    log_requests: bool,
    delays: Vec<String>,
    realistic: bool,
    include_optional: bool,
    validate: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Read the OpenAPI specification file
    let spec_content = tokio::fs::read_to_string(spec_path).await?;

    // Parse JSON or YAML
    let spec_json: serde_json::Value = if spec_path.extension().and_then(|s| s.to_str())
        == Some("yaml")
        || spec_path.extension().and_then(|s| s.to_str()) == Some("yml")
    {
        serde_yaml::from_str(&spec_content)?
    } else {
        serde_json::from_str(&spec_content)?
    };

    // Create server configuration
    let mut config = mockforge_data::MockServerConfig::new(spec_json)
        .port(port)
        .host(host.to_string())
        .enable_cors(cors)
        .log_requests(log_requests)
        .generator_config(
            mockforge_data::MockGeneratorConfig::new()
                .realistic_mode(realistic)
                .include_optional_fields(include_optional)
                .validate_generated_data(validate),
        );

    // Add response delays
    for delay_spec in delays {
        if let Some((endpoint, delay_ms)) = delay_spec.split_once(':') {
            if let Ok(delay) = delay_ms.parse::<u64>() {
                config = config.response_delay(endpoint.to_string(), delay);
            }
        }
    }

    // Start the mock server
    println!("🚀 Starting mock server...");
    println!("📡 Server will be available at: http://{}:{}", host, port);
    println!("📋 OpenAPI spec: {}", spec_path.display());
    println!("🛑 Press Ctrl+C to stop the server");

    mockforge_data::start_mock_server_with_config(config)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

/// Handle shell completions generation
/// Handle chaos engineering commands
async fn handle_chaos_command(chaos_command: ChaosCommands) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match chaos_command {
        ChaosCommands::Profile { profile_command } => {
            match profile_command {
                ProfileCommands::Apply { name, base_url } => {
                    println!("🔧 Applying chaos profile: {}", name);
                    let client = reqwest::Client::new();
                    let url = format!("{}/api/chaos/profiles/{}/apply", base_url, name);
                    let response = client.post(&url).send().await?;
                    if response.status().is_success() {
                        println!("✅ Profile '{}' applied successfully", name);
                    } else {
                        let error_text = response.text().await.unwrap_or_default();
                        eprintln!("❌ Failed to apply profile: {}", error_text);
                        std::process::exit(1);
                    }
                }
                ProfileCommands::Export { name, format, output, base_url } => {
                    println!("📤 Exporting profile: {}", name);
                    let client = reqwest::Client::new();
                    let url = format!("{}/api/chaos/profiles/{}/export?format={}", base_url, name, format);
                    let response = client.get(&url).send().await?;
                    if response.status().is_success() {
                        let content = response.text().await?;
                        if let Some(output_path) = output {
                            tokio::fs::write(&output_path, content).await?;
                            println!("✅ Profile exported to: {}", output_path.display());
                        } else {
                            println!("{}", content);
                        }
                    } else {
                        let error_text = response.text().await.unwrap_or_default();
                        eprintln!("❌ Failed to export profile: {}", error_text);
                        std::process::exit(1);
                    }
                }
                ProfileCommands::Import { file, base_url } => {
                    println!("📥 Importing profile from: {}", file.display());
                    let content = tokio::fs::read_to_string(&file).await?;
                    let format = if file.extension().and_then(|s| s.to_str()) == Some("yaml") ||
                                   file.extension().and_then(|s| s.to_str()) == Some("yml") {
                        "yaml"
                    } else {
                        "json"
                    };
                    let client = reqwest::Client::new();
                    let url = format!("{}/api/chaos/profiles/import", base_url);
                    let response = client
                        .post(&url)
                        .json(&serde_json::json!({
                            "content": content,
                            "format": format
                        }))
                        .send()
                        .await?;
                    if response.status().is_success() {
                        println!("✅ Profile imported successfully");
                    } else {
                        let error_text = response.text().await.unwrap_or_default();
                        eprintln!("❌ Failed to import profile: {}", error_text);
                        std::process::exit(1);
                    }
                }
                ProfileCommands::List { base_url } => {
                    println!("📋 Listing available chaos profiles...");
                    let client = reqwest::Client::new();
                    let url = format!("{}/api/chaos/profiles", base_url);
                    let response = client.get(&url).send().await?;
                    if response.status().is_success() {
                        let profiles: Vec<serde_json::Value> = response.json().await?;
                        println!("\nAvailable Profiles:");
                        println!("{:-<80}", "");
                        for profile in profiles {
                            let name = profile["name"].as_str().unwrap_or("unknown");
                            let description = profile["description"].as_str().unwrap_or("");
                            let builtin = profile["builtin"].as_bool().unwrap_or(false);
                            let tags = profile["tags"].as_array()
                                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                                .unwrap_or_default();
                            println!("  • {} {}", name, if builtin { "(built-in)" } else { "(custom)" });
                            if !description.is_empty() {
                                println!("    {}", description);
                            }
                            if !tags.is_empty() {
                                println!("    Tags: {}", tags);
                            }
                            println!();
                        }
                    } else {
                        let error_text = response.text().await.unwrap_or_default();
                        eprintln!("❌ Failed to list profiles: {}", error_text);
                        std::process::exit(1);
                    }
                }
            }
        }
    }
    Ok(())
}

fn handle_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();
    generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
}

/// Handle mock generation from configuration
async fn handle_generate(
    config_path: Option<PathBuf>,
    spec_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    verbose: bool,
    dry_run: bool,
    watch: bool,
    watch_debounce: u64,
    progress: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use mockforge_core::{discover_config_file, load_generate_config_with_fallback};
    use progress::{CliError, ExitCode, LogLevel, ProgressManager};
    use std::time::Instant;

    // Initialize progress manager
    let mut progress_mgr = ProgressManager::new(verbose);

    // If watch mode is enabled, set up file watching
    if watch {
        let files_to_watch = if let Some(spec) = &spec_path {
            vec![spec.clone()]
        } else if let Some(config) = &config_path {
            vec![config.clone()]
        } else {
            // Try to discover config file
            match discover_config_file() {
                Ok(path) => vec![path],
                Err(_) => {
                    return Err(CliError::new(
                        "No configuration file found for watch mode".to_string(),
                        ExitCode::ConfigurationError,
                    )
                    .with_suggestion(
                        "Provide --config or --spec flag, or create mockforge.toml".to_string(),
                    )
                    .display_and_exit());
                }
            }
        };

        progress_mgr.log(LogLevel::Info, "🔄 Starting watch mode...");
        progress_mgr.log(
            LogLevel::Info,
            &format!("👀 Watching {} file(s) for changes", files_to_watch.len()),
        );

        // Execute initial generation
        if let Err(e) = execute_generation(
            &mut progress_mgr,
            config_path.clone(),
            spec_path.clone(),
            output_path.clone(),
            verbose,
            dry_run,
            progress,
        )
        .await
        {
            progress_mgr.log(LogLevel::Error, &format!("Initial generation failed: {}", e));
            return Err(e);
        }

        // Set up watch loop
        let callback = move || {
            let config_path = config_path.clone();
            let spec_path = spec_path.clone();
            let output_path = output_path.clone();
            let verbose = verbose;
            let dry_run = dry_run;
            let progress = progress;

            async move {
                let mut progress_mgr = ProgressManager::new(verbose);
                execute_generation(
                    &mut progress_mgr,
                    config_path,
                    spec_path,
                    output_path,
                    verbose,
                    dry_run,
                    progress,
                )
                .await
            }
        };

        progress::watch::watch_files(files_to_watch, callback, watch_debounce).await?;
        return Ok(());
    }

    // Single generation run
    execute_generation(
        &mut progress_mgr,
        config_path,
        spec_path,
        output_path,
        verbose,
        dry_run,
        progress,
    )
    .await
}

/// Load and validate a configuration file
async fn load_and_validate_config(
    path: &PathBuf,
    verbose: bool,
    progress_mgr: &mut crate::progress::ProgressManager,
) -> mockforge_core::GenerateConfig {
    use crate::progress::{utils, LogLevel};
    use mockforge_core::load_generate_config_with_fallback;

    if verbose {
        progress_mgr
            .log(LogLevel::Debug, &format!("📄 Loading configuration from: {}", path.display()));
    }
    // Validate config file exists
    if let Err(e) = utils::validate_file_path(path) {
        e.display_and_exit();
    }
    load_generate_config_with_fallback(path).await
}

/// Execute the actual generation process with progress tracking
async fn execute_generation(
    progress_mgr: &mut crate::progress::ProgressManager,
    config_path: Option<PathBuf>,
    spec_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    verbose: bool,
    dry_run: bool,
    show_progress: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use mockforge_core::{discover_config_file, GenerateConfig};
    use progress::{utils, CliError, ExitCode, LogLevel};
    use std::time::Instant;

    let start_time = Instant::now();

    progress_mgr.log(LogLevel::Info, "🔧 Generating mocks from configuration...");

    // Step 1: Discover or load config file
    let (config_file, mut config) = if let Some(path) = &config_path {
        let config = load_and_validate_config(path, verbose, progress_mgr).await;
        (Some(path.clone()), config)
    } else {
        match discover_config_file() {
            Ok(path) => {
                let config = load_and_validate_config(&path, verbose, progress_mgr).await;
                (Some(path), config)
            }
            Err(_) => {
                // If no config file found, check if spec_path was provided as CLI argument
                if spec_path.is_none() {
                    progress_mgr
                        .log(LogLevel::Warning, "ℹ️  No configuration file found, using defaults");
                    return Err(CliError::new(
                        "No configuration file found and no spec provided. Please create mockforge.toml, mockforge.yaml, or mockforge.json, or provide --spec flag.".to_string(),
                        ExitCode::ConfigurationError,
                    ).with_suggestion(
                        "Create a configuration file or use --spec to specify an OpenAPI specification".to_string()
                    ).display_and_exit());
                }
                // If spec_path is provided, we can continue without a config file
                progress_mgr
                    .log(LogLevel::Warning, "ℹ️  No configuration file found, using defaults");
                // Use default configuration directly
                (None, GenerateConfig::default())
            }
        }
    };

    // Step 3: Apply CLI argument overrides
    if let Some(spec) = &spec_path {
        config.input.spec = Some(spec.clone());
    }

    if let Some(output) = &output_path {
        config.output.path = output.clone();
    }

    // Step 4: Validate configuration
    // Use require_registry helper (works with references) for better error handling
    let spec = progress::require_registry(&config.input.spec, "spec")?;

    if !spec.exists() {
        return Err(CliError::new(
            format!("Specification file not found: {}", spec.display()),
            ExitCode::FileNotFound,
        )
        .with_suggestion("Check the file path and ensure the specification file exists".to_string())
        .display_and_exit());
    }

    // Enhanced validation with detailed error messages
    if verbose {
        progress_mgr.log(LogLevel::Debug, "🔍 Validating specification...");
    }

    let spec_content = match tokio::fs::read_to_string(spec).await {
        Ok(content) => content,
        Err(e) => CliError::new(
            format!("Failed to read specification file: {}", e),
            ExitCode::FileNotFound,
        )
        .display_and_exit(),
    };

    // Detect format and validate
    let format = match mockforge_core::spec_parser::SpecFormat::detect(&spec_content, Some(spec)) {
        Ok(fmt) => fmt,
        Err(e) => {
            return Err(CliError::new(
                format!("Failed to detect specification format: {}", e),
                ExitCode::ConfigurationError,
            )
            .with_suggestion(
                "Ensure your file is a valid OpenAPI, GraphQL, or protobuf specification"
                    .to_string(),
            )
            .display_and_exit());
        }
    };

    if verbose {
        progress_mgr
            .log(LogLevel::Debug, &format!("📋 Detected format: {}", format.display_name()));
    }

    // Validate based on format
    match format {
        mockforge_core::spec_parser::SpecFormat::OpenApi20
        | mockforge_core::spec_parser::SpecFormat::OpenApi30
        | mockforge_core::spec_parser::SpecFormat::OpenApi31 => {
            // Optimize parsing: try JSON first, then YAML (avoids double parsing)
            let json_value: serde_json::Value =
                match serde_json::from_str::<serde_json::Value>(&spec_content) {
                    Ok(val) => val,
                    Err(_) => {
                        // Try YAML if JSON parsing fails
                        match serde_yaml::from_str(&spec_content) {
                            Ok(val) => val,
                            Err(e) => CliError::new(
                                format!("Invalid JSON or YAML in OpenAPI spec: {}", e),
                                ExitCode::ConfigurationError,
                            )
                            .display_and_exit(),
                        }
                    }
                };

            let validation =
                mockforge_core::spec_parser::OpenApiValidator::validate(&json_value, format);
            if !validation.is_valid {
                let error_details: Vec<String> = validation
                    .errors
                    .iter()
                    .map(|e| {
                        let mut msg = e.message.clone();
                        if let Some(path) = &e.path {
                            msg.push_str(&format!(" (at {})", path));
                        }
                        if let Some(suggestion) = &e.suggestion {
                            msg.push_str(&format!(". Hint: {}", suggestion));
                        }
                        msg
                    })
                    .collect();

                let error_msg = error_details.join("\n  ");
                return Err(CliError::new(
                    format!("Invalid OpenAPI specification:\n  {}", error_msg),
                    ExitCode::ConfigurationError,
                )
                .with_suggestion("Fix the validation errors above and try again".to_string())
                .display_and_exit());
            }

            if !validation.warnings.is_empty() && verbose {
                progress_mgr.log(LogLevel::Warning, "⚠️  Validation warnings:");
                for warning in &validation.warnings {
                    progress_mgr.log(LogLevel::Warning, &format!("  - {}", warning));
                }
            }

            if verbose {
                progress_mgr.log(LogLevel::Success, "✅ OpenAPI specification is valid");
            }
        }
        mockforge_core::spec_parser::SpecFormat::GraphQL => {
            let validation = mockforge_core::spec_parser::GraphQLValidator::validate(&spec_content);
            if !validation.is_valid {
                let error_details: Vec<String> = validation
                    .errors
                    .iter()
                    .map(|e| {
                        let mut msg = e.message.clone();
                        if let Some(suggestion) = &e.suggestion {
                            msg.push_str(&format!(". Hint: {}", suggestion));
                        }
                        msg
                    })
                    .collect();

                let error_msg = error_details.join("\n  ");
                return Err(CliError::new(
                    format!("Invalid GraphQL schema:\n  {}", error_msg),
                    ExitCode::ConfigurationError,
                )
                .with_suggestion("Fix the validation errors above and try again".to_string())
                .display_and_exit());
            }

            if !validation.warnings.is_empty() && verbose {
                progress_mgr.log(LogLevel::Warning, "⚠️  Validation warnings:");
                for warning in &validation.warnings {
                    progress_mgr.log(LogLevel::Warning, &format!("  - {}", warning));
                }
            }

            if verbose {
                progress_mgr.log(LogLevel::Success, "✅ GraphQL schema is valid");
            }
        }
        mockforge_core::spec_parser::SpecFormat::Protobuf => {
            if verbose {
                progress_mgr
                    .log(LogLevel::Info, "📋 Protobuf validation will be performed during parsing");
            }
        }
    }

    // Validate output directory
    if let Err(e) = utils::validate_output_dir(&config.output.path) {
        e.display_and_exit();
    }

    if verbose {
        progress_mgr.log(LogLevel::Debug, &format!("📝 Input spec: {}", spec.display()));
        progress_mgr
            .log(LogLevel::Debug, &format!("📂 Output path: {}", config.output.path.display()));
        if let Some(filename) = &config.output.filename {
            progress_mgr.log(LogLevel::Debug, &format!("📄 Output filename: {}", filename));
        }
        if let Some(options) = &config.options {
            progress_mgr.log(LogLevel::Debug, &format!("⚙️  Client: {:?}", options.client));
            progress_mgr.log(LogLevel::Debug, &format!("⚙️  Mode: {:?}", options.mode));
            progress_mgr.log(LogLevel::Debug, &format!("⚙️  Runtime: {:?}", options.runtime));
        }
        if !config.plugins.is_empty() {
            progress_mgr.log(LogLevel::Debug, "🔌 Plugins:");
            for (name, plugin) in &config.plugins {
                match plugin {
                    mockforge_core::PluginConfig::Simple(pkg) => {
                        progress_mgr.log(LogLevel::Debug, &format!("  - {}: {}", name, pkg));
                    }
                    mockforge_core::PluginConfig::Advanced { package, options } => {
                        progress_mgr.log(
                            LogLevel::Debug,
                            &format!("  - {}: {} (with options)", name, package),
                        );
                        if !options.is_empty() {
                            for (k, v) in options {
                                progress_mgr.log(LogLevel::Debug, &format!("    - {}: {}", k, v));
                            }
                        }
                    }
                }
            }
        }
    }

    if dry_run {
        progress_mgr.log(LogLevel::Success, "✅ Configuration is valid (dry run)");
        return Ok(());
    }

    // Create progress bar for generation steps
    let total_steps = 5u64;
    let progress_bar = if show_progress {
        Some(progress_mgr.create_main_progress(total_steps, "Generating mocks"))
    } else {
        None
    };

    // Step 5: Create output directory
    progress_mgr.log_step(1, total_steps as usize, "Preparing output directory");
    if config.output.clean && config.output.path.exists() {
        if verbose {
            progress_mgr.log(
                LogLevel::Debug,
                &format!("🧹 Cleaning output directory: {}", config.output.path.display()),
            );
        }
        tokio::fs::remove_dir_all(&config.output.path).await?;
    }

    tokio::fs::create_dir_all(&config.output.path).await?;
    if let Some(ref pb) = progress_bar {
        pb.inc(1u64);
    }

    // Step 6: Load and process OpenAPI spec
    progress_mgr.log_step(2, total_steps as usize, "Loading OpenAPI specification");
    let spec_content = tokio::fs::read_to_string(spec).await?;
    let spec_size = utils::format_file_size(spec_content.len() as u64);
    progress_mgr.log(LogLevel::Info, &format!("📖 Loaded OpenAPI specification ({})", spec_size));

    // Parse OpenAPI spec for file naming context
    let parsed_spec =
        OpenApiSpec::from_string(&spec_content, spec.extension().and_then(|e| e.to_str()))
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Failed to parse OpenAPI specification: {}", e).into()
            })?;

    // Build file naming context from OpenAPI spec (for file naming templates)
    let naming_context = if config.output.file_naming_template.is_some() {
        Some(build_file_naming_context(&parsed_spec))
    } else {
        None
    };

    if let Some(ref pb) = progress_bar {
        pb.inc(1u64);
    }

    // Step 7: Generate mock server code
    progress_mgr.log_step(3, total_steps as usize, "Generating mock server code");

    // Determine output filename with extension handling
    let base_filename =
        config.output.filename.clone().unwrap_or_else(|| "generated_mock".to_string());

    // Determine extension based on config or default
    let extension = config.output.extension.clone().unwrap_or_else(|| "rs".to_string());

    // Build initial file path
    let mut output_file = config.output.path.join(format!("{}.{}", base_filename, extension));

    // Generate mock server code using the codegen module
    let codegen_config = mockforge_core::codegen::CodegenConfig {
        mock_data_strategy: mockforge_core::codegen::MockDataStrategy::ExamplesOrRandom,
        port: None, // Will use default 3000
        enable_cors: false,
        default_delay_ms: None,
    };

    let raw_mock_code = mockforge_core::codegen::generate_mock_server_code(
        &parsed_spec,
        &extension,
        &codegen_config,
    )
    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
        format!("Failed to generate mock server code: {}", e).into()
    })?;

    // Create GeneratedFile for processing
    let mut generated_file = GeneratedFile {
        path: output_file
            .strip_prefix(&config.output.path)
            .unwrap_or(&output_file)
            .to_path_buf(),
        content: raw_mock_code,
        extension: extension.clone(),
        exportable: matches!(extension.as_str(), "ts" | "tsx" | "js" | "jsx" | "mjs"),
    };

    // Apply output control options (banner, extension, file naming template with context)
    generated_file =
        process_generated_file(generated_file, &config.output, Some(spec), naming_context.as_ref());

    // Update output_file path after processing
    output_file = config.output.path.join(&generated_file.path);

    // Write the processed file
    tokio::fs::write(&output_file, generated_file.content.clone()).await?;

    // Track generated files for barrel generation
    let mut all_generated_files = vec![generated_file];

    if let Some(ref pb) = progress_bar {
        pb.inc(1u64);
    }

    // Step 8: Generate additional files if needed
    progress_mgr.log_step(4, total_steps as usize, "Generating additional files");

    // Create a basic README
    let readme_content = format!(
        r#"# Generated Mock Server

This mock server was generated by MockForge from the OpenAPI specification:
- Source: {}
- Generated: {}

## Usage

```bash
# Start the mock server
cargo run

# Or use MockForge CLI
mockforge serve --spec {}
```

## Files Generated

- `{}` - Main mock server implementation
- `README.md` - This file
"#,
        spec.display(),
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        spec.display(),
        {
            use crate::progress::get_file_name;
            get_file_name(&output_file).unwrap_or_else(|e| {
                eprintln!("{}", e.message);
                if let Some(suggestion) = e.suggestion {
                    eprintln!("💡 {}", suggestion);
                }
                std::process::exit(e.exit_code as i32);
            })
        }
    );

    let readme_file = config.output.path.join("README.md");
    tokio::fs::write(&readme_file, readme_content).await?;

    if let Some(ref pb) = progress_bar {
        pb.inc(1u64);
    }

    // Step 9: Generate barrel files if requested
    if config.output.barrel_type != mockforge_core::BarrelType::None {
        if verbose {
            progress_mgr.log(
                LogLevel::Debug,
                &format!("📦 Generating barrel files (type: {:?})", config.output.barrel_type),
            );
        }

        match BarrelGenerator::generate_barrel_files(
            &config.output.path,
            &all_generated_files,
            config.output.barrel_type,
        ) {
            Ok(barrel_files) => {
                for (barrel_path, barrel_content) in barrel_files {
                    tokio::fs::write(&barrel_path, barrel_content).await?;
                    if verbose {
                        progress_mgr.log(
                            LogLevel::Debug,
                            &format!("📄 Generated barrel file: {}", barrel_path.display()),
                        );
                    }
                }
            }
            Err(e) => {
                progress_mgr
                    .log(LogLevel::Warning, &format!("⚠️  Failed to generate barrel files: {}", e));
            }
        }
    }

    // Step 10: Finalize
    progress_mgr.log_step(5, total_steps as usize, "Finalizing generation");

    let duration = start_time.elapsed();
    let duration_str = utils::format_duration(duration);

    // Count total files (generated + barrel files + README)
    let total_files = all_generated_files.len() + 1; // +1 for README

    progress_mgr
        .log(LogLevel::Success, &format!("✅ Mock generation completed in {}", duration_str));
    progress_mgr.log(
        LogLevel::Info,
        &format!("📁 Output directory: {}", config.output.path.display()),
    );
    progress_mgr.log(LogLevel::Info, &format!("📄 Generated files: {} files", total_files));

    if let Some(ref pb) = progress_bar {
        pb.finish();
    }

    Ok(())
}

/// Handle project initialization
async fn handle_init(
    name: String,
    no_examples: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::fs;

    println!("🚀 Initializing MockForge project...");

    // Determine project directory
    let project_dir = if name == "." {
        std::env::current_dir()?
    } else {
        PathBuf::from(&name)
    };

    // Create project directory if it doesn't exist
    if !project_dir.exists() {
        fs::create_dir_all(&project_dir)?;
        println!("📁 Created directory: {}", project_dir.display());
    }

    // Create config file
    let config_path = project_dir.join("mockforge.yaml");
    if config_path.exists() {
        println!("⚠️  Configuration file already exists: {}", config_path.display());
    } else {
        let config_content = r#"# MockForge Configuration
# Full configuration reference: https://docs.mockforge.dev/config

# HTTP Server
http:
  port: 3000
  host: "0.0.0.0"
  openapi_spec: "./examples/openapi.json"
  cors_enabled: true
  request_timeout_secs: 30
  request_validation: "enforce"
  aggregate_validation_errors: true
  validate_responses: false
  response_template_expand: false
  validation_overrides: {}
  skip_admin_validation: true

# WebSocket Server
websocket:
  port: 3001
  host: "0.0.0.0"
  connection_timeout_secs: 300

# gRPC Server
grpc:
  port: 50051
  host: "0.0.0.0"

# Admin UI
admin:
  enabled: true
  port: 9080
  host: "127.0.0.1"
  api_enabled: true
  auth_required: false
  prometheus_url: "http://localhost:9090"

# Core Features
core:
  latency_enabled: true
  failures_enabled: false
  overrides_enabled: true
  traffic_shaping_enabled: false

# Observability
observability:
  prometheus:
    enabled: true
    port: 9090
    host: "0.0.0.0"
    path: "/metrics"
  opentelemetry: null
  recorder: null
  chaos: null

# Data Generation
data:
  default_rows: 100
  default_format: "json"
  locale: "en"
  templates: {}
  rag:
    enabled: false
    provider: "openai"

# Logging
logging:
  level: "info"
  json_format: false
  max_file_size_mb: 10
  max_files: 5
"#;
        fs::write(&config_path, config_content)?;
        println!("✅ Created mockforge.yaml");
    }

    // Create examples directory if not skipped
    if !no_examples {
        let examples_dir = project_dir.join("examples");
        fs::create_dir_all(&examples_dir)?;
        println!("📁 Created examples directory");

        // Create example OpenAPI spec
        let openapi_path = examples_dir.join("openapi.json");
        let openapi_content = r#"{
  "openapi": "3.0.0",
  "info": {
    "title": "Example API",
    "version": "1.0.0"
  },
  "paths": {
    "/health": {
      "get": {
        "summary": "Health check",
        "responses": {
          "200": {
            "description": "OK",
            "content": {
              "application/json": {
                "schema": {
                  "type": "object",
                  "properties": {
                    "status": {
                      "type": "string"
                    }
                  }
                }
              }
            }
          }
        }
      }
    },
    "/users": {
      "get": {
        "summary": "List users",
        "responses": {
          "200": {
            "description": "OK",
            "content": {
              "application/json": {
                "schema": {
                  "type": "array",
                  "items": {
                    "type": "object",
                    "properties": {
                      "id": {
                        "type": "integer"
                      },
                      "name": {
                        "type": "string"
                      },
                      "email": {
                        "type": "string"
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}"#;
        fs::write(&openapi_path, openapi_content)?;
        println!("✅ Created examples/openapi.json");

        // Create example fixture
        let fixtures_dir = project_dir.join("fixtures");
        fs::create_dir_all(&fixtures_dir)?;
        let fixture_path = fixtures_dir.join("users.json");
        let fixture_content = r#"[
  {
    "id": 1,
    "name": "Alice Johnson",
    "email": "alice@example.com"
  },
  {
    "id": 2,
    "name": "Bob Smith",
    "email": "bob@example.com"
  }
]"#;
        fs::write(&fixture_path, fixture_content)?;
        println!("✅ Created fixtures/users.json");
    }

    println!("\n🎉 MockForge project initialized successfully!");
    println!("\nNext steps:");
    println!("  1. cd {}", if name == "." { "." } else { &name });
    println!("  2. Edit mockforge.yaml to configure your mock servers");
    if !no_examples {
        println!("  3. Review examples/openapi.json for API specifications");
    }
    println!("  4. Run: mockforge serve --config mockforge.yaml");
    println!("\nNote: CLI arguments override config file settings");

    Ok(())
}

/// Handle config commands
async fn handle_config(
    config_command: ConfigCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match config_command {
        ConfigCommands::Validate { config } => {
            handle_config_validate(config).await?;
        }
    }
    Ok(())
}

/// Handle config validation
async fn handle_config_validate(
    config_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔍 Validating MockForge configuration...");

    // Auto-discover config file if not provided
    let config_file = if let Some(path) = config_path {
        path
    } else {
        discover_config_file()?
    };

    println!("📄 Checking configuration file: {}", config_file.display());

    // Check if file exists
    if !config_file.exists() {
        return Err(format!("Configuration file not found: {}", config_file.display()).into());
    }

    // Read and parse YAML/JSON
    let config_content = tokio::fs::read_to_string(&config_file).await?;
    let is_yaml = config_file
        .extension()
        .and_then(|s| s.to_str())
        .map(|ext| ext == "yaml" || ext == "yml")
        .unwrap_or(true);

    // First, try to parse with ServerConfig for full schema validation
    let config_result = if is_yaml {
        serde_yaml::from_str::<mockforge_core::ServerConfig>(&config_content)
            .map_err(|e| format_yaml_error(&config_content, e))
    } else {
        serde_json::from_str::<mockforge_core::ServerConfig>(&config_content)
            .map_err(|e| format_json_error(&config_content, e))
    };

    match config_result {
        Ok(config) => {
            // Successfully parsed - now validate content
            let mut warnings = Vec::new();
            let mut errors = Vec::new();

            // Validate HTTP section
            if config.http.host.is_empty() {
                errors.push("HTTP host is empty".to_string());
            }
            if config.http.port == 0 {
                errors.push("HTTP port cannot be 0".to_string());
            }

            // Check OpenAPI spec if provided
            if let Some(ref spec_path) = config.http.openapi_spec {
                if !std::path::Path::new(spec_path).exists() {
                    errors.push(format!("OpenAPI spec file not found: {}", spec_path));
                } else {
                    println!("   ✓ OpenAPI spec: {}", spec_path);
                }
            } else {
                warnings.push(
                    "No OpenAPI spec configured. HTTP endpoints will need to be defined manually."
                        .to_string(),
                );
            }

            // Validate request validation mode
            let valid_modes = ["off", "warn", "enforce"];
            if let Some(validation) = &config.http.validation {
                if !valid_modes.contains(&validation.mode.as_str()) {
                    errors.push(format!(
                        "Invalid request validation mode '{}'. Must be one of: off, warn, enforce",
                        validation.mode
                    ));
                }
            }

            // Validate HTTP auth if configured
            if let Some(ref auth) = config.http.auth {
                if let Some(ref jwt) = auth.jwt {
                    if let Some(ref secret) = jwt.secret {
                        if secret.is_empty() {
                            errors.push(
                                "HTTP JWT auth is configured but secret is empty".to_string(),
                            );
                        }
                    } else if jwt.rsa_public_key.is_none() && jwt.ecdsa_public_key.is_none() {
                        errors.push("HTTP JWT auth requires at least one key (secret, rsa_public_key, or ecdsa_public_key)".to_string());
                    }
                }
                if let Some(ref basic) = auth.basic_auth {
                    if basic.credentials.is_empty() {
                        warnings.push(
                            "HTTP Basic auth is configured but no credentials are defined"
                                .to_string(),
                        );
                    }
                }
            }

            // Validate WebSocket section
            if config.websocket.port == 0 {
                errors.push("WebSocket port cannot be 0".to_string());
            }
            if config.websocket.port == config.http.port {
                errors.push("WebSocket port conflicts with HTTP port".to_string());
            }

            // Validate gRPC section
            if config.grpc.port == 0 {
                errors.push("gRPC port cannot be 0".to_string());
            }
            if config.grpc.port == config.http.port || config.grpc.port == config.websocket.port {
                errors.push("gRPC port conflicts with HTTP or WebSocket port".to_string());
            }

            // Validate chaining configuration
            if config.chaining.enabled {
                if config.chaining.max_chain_length == 0 {
                    errors.push("Chaining is enabled but max_chain_length is 0".to_string());
                }
                if config.chaining.global_timeout_secs == 0 {
                    warnings.push("Chaining global timeout is 0 (no timeout)".to_string());
                }
                println!(
                    "   ✓ Request chaining: enabled (max length: {})",
                    config.chaining.max_chain_length
                );
            }

            // Validate admin UI configuration
            if config.admin.enabled {
                if config.admin.port == 0 {
                    errors.push("Admin UI is enabled but port is 0".to_string());
                }
                if config.admin.port == config.http.port
                    || config.admin.port == config.websocket.port
                    || config.admin.port == config.grpc.port
                {
                    errors.push("Admin UI port conflicts with another service port".to_string());
                }
                if config.admin.auth_required
                    && (config.admin.username.is_none() || config.admin.password.is_none())
                {
                    errors.push(
                        "Admin UI auth is required but username/password not configured"
                            .to_string(),
                    );
                }
            } else {
                warnings
                    .push("Admin UI is disabled. Enable with 'admin.enabled: true'.".to_string());
            }

            // Validate observability
            if config.observability.prometheus.enabled && config.observability.prometheus.port == 0
            {
                errors.push("Prometheus metrics enabled but port is 0".to_string());
            }

            if let Some(ref otel) = config.observability.opentelemetry {
                if otel.enabled {
                    if otel.service_name.is_empty() {
                        warnings.push("OpenTelemetry service name is empty".to_string());
                    }
                    println!("   ✓ OpenTelemetry: enabled (service: {})", otel.service_name);
                }
            }

            if let Some(ref recorder) = config.observability.recorder {
                if recorder.enabled {
                    if recorder.database_path.is_empty() {
                        errors.push("Recorder is enabled but database path is empty".to_string());
                    }
                    println!("   ✓ Recorder: enabled (db: {})", recorder.database_path);
                }
            }

            // Print results
            if !errors.is_empty() {
                println!("\n❌ Configuration has errors:");
                for error in &errors {
                    println!("   ✗ {}", error);
                }
                return Err("Configuration validation failed".into());
            }

            println!("\n✅ Configuration is valid");
            println!("\n📊 Summary:");
            println!("   HTTP server: {}:{}", config.http.host, config.http.port);
            println!("   WebSocket server: {}:{}", config.websocket.host, config.websocket.port);
            println!("   gRPC server: {}:{}", config.grpc.host, config.grpc.port);

            if config.admin.enabled {
                println!("   Admin UI: http://{}:{}", config.admin.host, config.admin.port);
            }

            if config.observability.prometheus.enabled {
                println!(
                    "   Prometheus metrics: http://{}:{}/metrics",
                    config.http.host, config.observability.prometheus.port
                );
            }

            if !warnings.is_empty() {
                println!("\n⚠️  Warnings:");
                for warning in warnings {
                    println!("   - {}", warning);
                }
            }

            Ok(())
        }
        Err(error_msg) => {
            println!("❌ Configuration validation failed:\n");
            println!("{}", error_msg);
            Err("Invalid configuration".into())
        }
    }
}

/// Format YAML parsing errors with line numbers and better field path extraction
fn format_yaml_error(content: &str, error: serde_yaml::Error) -> String {
    let mut message = String::from("❌ Configuration parsing error:\n\n");

    // Extract field path from error message if possible
    let error_str = error.to_string();
    let field_path = extract_field_path(&error_str);

    if let Some(location) = error.location() {
        let line = location.line();
        let column = location.column();

        message.push_str(&format!("📍 Location: line {}, column {}\n\n", line, column));

        // Show the problematic line with context
        let lines: Vec<&str> = content.lines().collect();
        let start = line.saturating_sub(2);
        let end = (line + 1).min(lines.len());

        for (idx, line_content) in lines[start..end].iter().enumerate() {
            let line_num = start + idx + 1;
            if line_num == line {
                message.push_str(&format!("  > {} | {}\n", line_num, line_content));
                if column > 0 {
                    message.push_str(&format!(
                        "    {}^\n",
                        " ".repeat(column + 5 + line_num.to_string().len())
                    ));
                }
            } else {
                message.push_str(&format!("    {} | {}\n", line_num, line_content));
            }
        }

        message.push_str("\n");
    }

    // Show the error with field path if extracted
    if let Some(path) = &field_path {
        message.push_str(&format!("🔍 Field path: {}\n", path));
        message.push_str(&format!("❌ Error: {}\n\n", error));
    } else {
        message.push_str(&format!("❌ Error: {}\n\n", error));
    }

    // Add helpful suggestions based on error type and field path
    if error_str.contains("duplicate key") {
        message.push_str("💡 Tip: You have a duplicate key in your YAML. Each key must be unique within its section.\n");
    } else if error_str.contains("invalid type") {
        message.push_str("💡 Tip: Check that your values match the expected types (strings, numbers, booleans, arrays, objects).\n");
        if let Some(path) = &field_path {
            message.push_str(&format!("   Check the type for field: {}\n", path));
        }
    } else if error_str.contains("missing field") {
        // Most fields in MockForge are optional with defaults
        message.push_str("💡 Tip: This field is usually optional and has a default value.\n");
        message.push_str(
            "   Most configuration fields can be omitted - MockForge will use sensible defaults.\n",
        );
        if let Some(path) = &field_path {
            message.push_str(&format!("   \n   To fix: Either add the field at path '{}' or remove it entirely (defaults will be used).\n", path));
            message.push_str(
                "   See config.template.yaml for all available options and their defaults.\n",
            );
        } else {
            message.push_str(
                "   See config.template.yaml for all available options and their defaults.\n",
            );
        }
    } else if error_str.contains("unknown field") {
        message.push_str("💡 Tip: You may have a typo in a field name.\n");
        if let Some(path) = &field_path {
            message.push_str(&format!("   Unknown field at path: {}\n", path));
            message.push_str(
                "   Check the spelling against the documentation or config.template.yaml.\n",
            );
        } else {
            message.push_str(
                "   Check the spelling against the documentation or config.template.yaml.\n",
            );
        }
    } else if error_str.contains("expected") {
        message.push_str("💡 Tip: There's a type mismatch or syntax error.\n");
        if let Some(path) = &field_path {
            message.push_str(&format!("   Check the value type for field: {}\n", path));
        }
    }

    message.push_str("\n📚 For a complete example configuration, see: config.template.yaml\n");
    message.push_str("   Or run: mockforge init .\n");

    message
}

/// Format JSON parsing errors with line numbers and better field path extraction
fn format_json_error(content: &str, error: serde_json::Error) -> String {
    let mut message = String::from("❌ Configuration parsing error:\n\n");

    // Extract field path from error message if possible
    let error_str = error.to_string();
    let field_path = extract_field_path(&error_str);

    let line = error.line();
    let column = error.column();

    message.push_str(&format!("📍 Location: line {}, column {}\n\n", line, column));

    // Show the problematic line with context
    let lines: Vec<&str> = content.lines().collect();
    let start = line.saturating_sub(2);
    let end = (line + 1).min(lines.len());

    for (idx, line_content) in lines[start..end].iter().enumerate() {
        let line_num = start + idx + 1;
        if line_num == line {
            message.push_str(&format!("  > {} | {}\n", line_num, line_content));
            if column > 0 {
                message.push_str(&format!(
                    "    {}^\n",
                    " ".repeat(column + 5 + line_num.to_string().len())
                ));
            }
        } else {
            message.push_str(&format!("    {} | {}\n", line_num, line_content));
        }
    }

    message.push_str("\n");

    // Show the error with field path if extracted
    if let Some(path) = &field_path {
        message.push_str(&format!("🔍 Field path: {}\n", path));
        message.push_str(&format!("❌ Error: {}\n\n", error));
    } else {
        message.push_str(&format!("❌ Error: {}\n\n", error));
    }

    // Add helpful suggestions based on error type
    if error_str.contains("trailing comma") {
        message.push_str(
            "💡 Tip: JSON doesn't allow trailing commas. Remove the comma after the last item.\n",
        );
    } else if error_str.contains("missing field") {
        message.push_str("💡 Tip: This field is usually optional and has a default value.\n");
        message.push_str(
            "   Most configuration fields can be omitted - MockForge will use sensible defaults.\n",
        );
        if let Some(path) = &field_path {
            message.push_str(&format!("   \n   To fix: Either add the field at path '{}' or remove it entirely (defaults will be used).\n", path));
        }
        message.push_str(
            "   See config.template.yaml for all available options and their defaults.\n",
        );
    } else if error_str.contains("duplicate field") {
        message.push_str(
            "💡 Tip: You have a duplicate key. Each key must be unique within its object.\n",
        );
    } else if error_str.contains("expected") {
        message
            .push_str("💡 Tip: Check for missing or extra brackets, braces, quotes, or commas.\n");
        if let Some(path) = &field_path {
            message.push_str(&format!("   Or check the value type for field: {}\n", path));
        }
    } else if error_str.contains("unknown field") {
        message.push_str("💡 Tip: You may have a typo in a field name.\n");
        if let Some(path) = &field_path {
            message.push_str(&format!("   Unknown field at path: {}\n", path));
        }
        message
            .push_str("   Check the spelling against the documentation or config.template.yaml.\n");
    }

    message.push_str("\n📚 For a complete example configuration, see: config.template.yaml\n");
    message.push_str("   Or run: mockforge init .\n");

    message
}

/// Extract field path from serde error messages
/// Examples:
///   "missing field `host` at line 2 column 1" -> Some("host")
///   "unknown field `foo`, expected one of `bar`, `baz`" -> Some("foo")
///   "invalid type: string \"x\", expected u16 at line 5" -> None (type error, not field path)
fn extract_field_path(error_msg: &str) -> Option<String> {
    // Try to extract field name from "missing field `X`" or "unknown field `X`"
    if let Some(start) = error_msg.find("field `") {
        let after_field = &error_msg[start + 7..];
        if let Some(end) = after_field.find('`') {
            let field_name = &after_field[..end];

            // Try to find parent path context if available
            // Serde errors sometimes include path context like "http.host"
            if let Some(path_context_start) = error_msg.rfind(" at ") {
                let path_context = &error_msg[..path_context_start];
                // Look for common patterns like "http.host" or "admin.port"
                for section in ["http", "admin", "websocket", "grpc", "core", "logging"] {
                    if path_context.contains(section) {
                        return Some(format!("{}.{}", section, field_name));
                    }
                }
            }

            return Some(field_name.to_string());
        }
    }

    // Try to extract from "invalid type" with context
    if let Some(start) = error_msg.find("invalid type") {
        // Look backwards for field context
        if let Some(field_start) = error_msg[..start].rfind("field `") {
            let after_field = &error_msg[field_start + 7..];
            if let Some(end) = after_field.find('`') {
                return Some(after_field[..end].to_string());
            }
        }
    }

    None
}

/// Discover configuration file in current directory and parents
fn discover_config_file() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let current_dir = std::env::current_dir()?;
    let config_names = vec![
        "mockforge.yaml",
        "mockforge.yml",
        ".mockforge.yaml",
        ".mockforge.yml",
    ];

    // Check current directory
    for name in &config_names {
        let path = current_dir.join(name);
        if path.exists() {
            return Ok(path);
        }
    }

    // Check parent directories (up to 5 levels)
    let mut dir = current_dir.clone();
    for _ in 0..5 {
        if let Some(parent) = dir.parent() {
            for name in &config_names {
                let path = parent.join(name);
                if path.exists() {
                    return Ok(path);
                }
            }
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }

    Err("No configuration file found. Expected one of: mockforge.yaml, mockforge.yml, .mockforge.yaml, .mockforge.yml".into())
}

/// Handle AI testing commands
async fn handle_test_ai(
    ai_command: AiTestCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match ai_command {
        AiTestCommands::IntelligentMock {
            prompt,
            rag_provider,
            rag_model,
            output,
        } => {
            println!("🧠 Testing Intelligent Mock Generation");
            println!("📝 Prompt: {}", prompt);

            // Load RAG configuration
            let rag_config = load_rag_config(rag_provider, rag_model, None, None, None);

            // Create intelligent mock generator
            use mockforge_data::{IntelligentMockConfig, IntelligentMockGenerator, ResponseMode};

            let config = IntelligentMockConfig::new(ResponseMode::Intelligent)
                .with_prompt(prompt)
                .with_rag_config(rag_config);

            let mut generator = IntelligentMockGenerator::new(config)?;

            // Generate mock data
            println!("🎯 Generating mock data...");
            let result = generator.generate().await?;

            // Output result
            let output_str = serde_json::to_string_pretty(&result)?;
            if let Some(path) = output {
                tokio::fs::write(&path, &output_str).await?;
                println!("💾 Output written to: {}", path.display());
            } else {
                println!("\n📄 Generated Mock Data:");
                println!("{}", output_str);
            }

            println!("✅ Intelligent mock generation completed successfully!");
        }

        AiTestCommands::Drift {
            initial_data,
            iterations,
            output,
        } => {
            println!("📊 Testing Data Drift Simulation");
            println!("📁 Initial data: {}", initial_data.display());
            println!("🔄 Iterations: {}", iterations);

            // Read initial data
            let data_content = tokio::fs::read_to_string(&initial_data).await?;
            let mut current_data: serde_json::Value = serde_json::from_str(&data_content)?;

            // Create a simple drift configuration
            use mockforge_data::drift::{DriftRule, DriftStrategy};
            use mockforge_data::DataDriftConfig;

            let rule = DriftRule::new("value".to_string(), DriftStrategy::Linear).with_rate(1.0);
            let drift_config = DataDriftConfig::new().with_rule(rule);

            let engine = mockforge_data::DataDriftEngine::new(drift_config)?;

            // Simulate drift iterations
            println!("\n🎯 Simulating drift:");
            let mut results = vec![current_data.clone()];

            for i in 1..=iterations {
                current_data = engine.apply_drift(current_data).await?;
                results.push(current_data.clone());
                println!("   Iteration {}: {:?}", i, current_data);
            }

            // Output results
            let output_str = serde_json::to_string_pretty(&results)?;
            if let Some(path) = output {
                tokio::fs::write(&path, &output_str).await?;
                println!("\n💾 Output written to: {}", path.display());
            } else {
                println!("\n📄 Final Drifted Data:");
                println!("{}", serde_json::to_string_pretty(&current_data)?);
            }

            println!("✅ Data drift simulation completed successfully!");
        }

        AiTestCommands::EventStream {
            narrative,
            event_count,
            rag_provider,
            rag_model,
            output,
        } => {
            println!("🌊 Testing AI Event Stream Generation");
            println!("📖 Narrative: {}", narrative);
            println!("🔢 Event count: {}", event_count);

            // Load RAG configuration
            let rag_config = load_rag_config(rag_provider, rag_model, None, None, None);

            // Create replay augmentation config
            use mockforge_data::{EventStrategy, ReplayAugmentationConfig, ReplayMode};

            let config = ReplayAugmentationConfig {
                mode: ReplayMode::Generated,
                strategy: EventStrategy::CountBased,
                narrative: Some(narrative),
                event_count: Some(event_count),
                rag_config: Some(rag_config),
                ..Default::default()
            };

            let mut engine = mockforge_data::ReplayAugmentationEngine::new(config)?;

            // Generate event stream
            println!("🎯 Generating event stream...");
            let events = engine.generate_stream().await?;

            // Output results
            let output_str = serde_json::to_string_pretty(&events)?;
            if let Some(path) = output {
                tokio::fs::write(&path, &output_str).await?;
                println!("💾 Output written to: {}", path.display());
            } else {
                println!("\n📄 Generated Events:");
                for (i, event) in events.iter().enumerate() {
                    println!("\nEvent {}:", i + 1);
                    println!("  Type: {}", event.event_type);
                    println!("  Timestamp: {}", event.timestamp);
                    println!("  Data: {}", serde_json::to_string_pretty(&event.data)?);
                }
            }

            println!("\n✅ Event stream generation completed successfully!");
            println!("   Generated {} events", events.len());
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_generate_tests(
    database: PathBuf,
    format: String,
    output: Option<PathBuf>,
    protocol: Option<String>,
    method: Option<String>,
    path: Option<String>,
    status_code: Option<u16>,
    limit: usize,
    suite_name: String,
    base_url: String,
    ai_descriptions: bool,
    llm_provider: String,
    llm_model: String,
    llm_endpoint: Option<String>,
    llm_api_key: Option<String>,
    validate_body: bool,
    validate_status: bool,
    validate_headers: bool,
    validate_timing: bool,
    max_duration_ms: Option<u64>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use mockforge_recorder::{
        LlmConfig, Protocol, QueryFilter, RecorderDatabase, TestFormat, TestGenerationConfig,
        TestGenerator,
    };

    println!("🧪 Generating tests from recorded API interactions");
    println!("📁 Database: {}", database.display());
    println!("📝 Format: {}", format);
    println!("🎯 Suite name: {}", suite_name);

    // Open database with proper error handling for path conversion
    use crate::progress::{CliError, ExitCode};
    let db_path = database.to_str().ok_or_else(|| {
        CliError::new(
            format!("Invalid database path: {}", database.display()),
            ExitCode::FileNotFound,
        )
        .with_suggestion(
            "Ensure the database path contains only valid UTF-8 characters".to_string(),
        )
    })?;
    let db = RecorderDatabase::new(db_path).await?;
    println!("✅ Database opened successfully");

    // Parse test format
    let test_format = match format.as_str() {
        "rust_reqwest" => TestFormat::RustReqwest,
        "http_file" => TestFormat::HttpFile,
        "curl" => TestFormat::Curl,
        "postman" => TestFormat::Postman,
        "k6" => TestFormat::K6,
        "python_pytest" => TestFormat::PythonPytest,
        "javascript_jest" => TestFormat::JavaScriptJest,
        "go_test" => TestFormat::GoTest,
        _ => {
            eprintln!("❌ Invalid format: {}. Supported formats: rust_reqwest, http_file, curl, postman, k6, python_pytest, javascript_jest, go_test", format);
            return Err("Invalid format".into());
        }
    };

    // Parse protocol filter
    let protocol_filter = protocol.as_ref().and_then(|p| match p.to_lowercase().as_str() {
        "http" => Some(Protocol::Http),
        "grpc" => Some(Protocol::Grpc),
        "websocket" => Some(Protocol::WebSocket),
        "graphql" => Some(Protocol::GraphQL),
        _ => None,
    });

    // Create LLM config if AI descriptions enabled
    let llm_config = if ai_descriptions {
        let endpoint = llm_endpoint.unwrap_or_else(|| {
            if llm_provider == "ollama" {
                "http://localhost:11434/api/generate".to_string()
            } else {
                "https://api.openai.com/v1/chat/completions".to_string()
            }
        });

        Some(LlmConfig {
            provider: llm_provider.clone(),
            api_endpoint: endpoint,
            api_key: llm_api_key,
            model: llm_model.clone(),
            temperature: 0.3,
        })
    } else {
        None
    };

    // Create test generation config
    let config = TestGenerationConfig {
        format: test_format,
        include_assertions: true,
        validate_body,
        validate_status,
        validate_headers,
        validate_timing,
        max_duration_ms,
        suite_name: suite_name.clone(),
        base_url: Some(base_url.clone()),
        ai_descriptions,
        llm_config,
        group_by_endpoint: true,
        include_setup_teardown: true,
        generate_fixtures: ai_descriptions,
        suggest_edge_cases: ai_descriptions,
        analyze_test_gaps: ai_descriptions,
        deduplicate_tests: true,
        optimize_test_order: false,
    };

    // Create query filter
    let filter = QueryFilter {
        protocol: protocol_filter,
        method: method.clone(),
        path: path.clone(),
        status_code: status_code.map(|c| c as i32),
        trace_id: None,
        min_duration_ms: None,
        max_duration_ms: None,
        tags: None,
        limit: Some(limit as i32),
        offset: Some(0),
    };

    println!("🔍 Searching for recordings...");
    if let Some(p) = &protocol {
        println!("   Protocol: {}", p);
    }
    if let Some(m) = &method {
        println!("   Method: {}", m);
    }
    if let Some(p) = &path {
        println!("   Path: {}", p);
    }
    if let Some(s) = status_code {
        println!("   Status code: {}", s);
    }
    println!("   Limit: {}", limit);

    // Generate tests
    let generator = TestGenerator::new(db, config);
    println!("\n🎨 Generating tests...");

    if ai_descriptions {
        println!("🤖 Using {} ({}) for AI descriptions", llm_provider, llm_model);
    }

    let result = generator.generate_from_filter(filter).await?;

    println!("\n✅ Test generation completed successfully!");
    println!("   Generated {} tests", result.metadata.test_count);
    println!("   Covering {} endpoints", result.metadata.endpoint_count);
    println!("   Protocols: {:?}", result.metadata.protocols);

    // Output test file
    if let Some(output_path) = output {
        tokio::fs::write(&output_path, &result.test_file).await?;
        println!("\n💾 Tests written to: {}", output_path.display());
    } else {
        println!("\n📄 Generated Test File:");
        println!("{}", "=".repeat(60));
        println!("{}", result.test_file);
        println!("{}", "=".repeat(60));
    }

    // Print summary of generated tests
    println!("\n📊 Test Summary:");
    for (i, test) in result.tests.iter().enumerate() {
        println!("   {}. {} - {} {}", i + 1, test.name, test.method, test.endpoint);
        if ai_descriptions
            && !test.description.is_empty()
            && test.description != format!("Test {} {}", test.method, test.endpoint)
        {
            println!("      Description: {}", test.description);
        }
    }

    println!("\n🎉 Done! You can now run the generated tests.");

    Ok(())
}

async fn handle_orchestrate(
    command: OrchestrateCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match command {
        OrchestrateCommands::Start { file, base_url } => {
            println!("🚀 Starting chaos orchestration from: {}", file.display());

            // Read orchestration file
            let content = std::fs::read_to_string(&file)?;
            let format = if file.extension().and_then(|s| s.to_str()) == Some("json") {
                "json"
            } else {
                "yaml"
            };

            // Send to API
            let client = reqwest::Client::new();
            let url = format!("{}/api/chaos/orchestration/import", base_url);

            let response = client
                .post(&url)
                .json(&serde_json::json!({
                    "content": content,
                    "format": format
                }))
                .send()
                .await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                println!("✅ {}", result["message"].as_str().unwrap_or("Orchestration imported"));

                // Now start it
                let _start_url = format!("{}/api/chaos/orchestration/start", base_url);
                // Note: This is a simplified version - would need to parse and send proper request
                println!("   Use the API to start the orchestration");
            } else {
                eprintln!("❌ Failed to import orchestration: {}", response.status());
            }
        }

        OrchestrateCommands::Status { base_url } => {
            println!("📊 Checking orchestration status...");

            let client = reqwest::Client::new();
            let url = format!("{}/api/chaos/orchestration/status", base_url);

            let response = client.get(&url).send().await?;

            if response.status().is_success() {
                let status: serde_json::Value = response.json().await?;

                if status["is_running"].as_bool().unwrap_or(false) {
                    println!("✅ Orchestration is running");
                    println!("   Name: {}", status["name"].as_str().unwrap_or("Unknown"));
                    println!(
                        "   Progress: {:.1}%",
                        status["progress"].as_f64().unwrap_or(0.0) * 100.0
                    );
                } else {
                    println!("⏸️  No orchestration currently running");
                }
            } else {
                eprintln!("❌ Failed to get status: {}", response.status());
            }
        }

        OrchestrateCommands::Stop { base_url } => {
            println!("🛑 Stopping orchestration...");

            let client = reqwest::Client::new();
            let url = format!("{}/api/chaos/orchestration/stop", base_url);

            let response = client.post(&url).send().await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                println!("✅ {}", result["message"].as_str().unwrap_or("Orchestration stopped"));
            } else {
                eprintln!("❌ Failed to stop orchestration: {}", response.status());
            }
        }

        OrchestrateCommands::Validate { file } => {
            println!("🔍 Validating orchestration file: {}", file.display());

            // Check if file exists
            if !file.exists() {
                eprintln!("❌ File not found: {}", file.display());
                return Err("File not found".into());
            }

            // Read and parse file
            let content = std::fs::read_to_string(&file)?;
            let is_json = file.extension().and_then(|s| s.to_str()) == Some("json");

            let parse_result: Result<serde_json::Value, String> = if is_json {
                serde_json::from_str::<serde_json::Value>(&content)
                    .map_err(|e| format_json_error(&content, e))
            } else {
                // Parse as YAML, then convert to JSON Value for uniform handling
                serde_yaml::from_str::<serde_yaml::Value>(&content)
                    .map_err(|e| format_yaml_error(&content, e))
                    .and_then(|yaml_val| {
                        serde_json::to_value(yaml_val)
                            .map_err(|e| format!("Failed to convert YAML to JSON: {}", e))
                    })
            };

            match parse_result {
                Ok(value) => {
                    // Validate structure
                    let mut errors = Vec::new();
                    let mut warnings = Vec::new();

                    // Check for required fields
                    if value.get("name").is_none() {
                        errors.push("Missing required field 'name'".to_string());
                    } else if !value["name"].is_string() {
                        errors.push("Field 'name' must be a string".to_string());
                    }

                    // Validate steps array
                    match value.get("steps") {
                        None => {
                            errors.push("Missing required field 'steps'".to_string());
                        }
                        Some(steps) => {
                            if let Some(steps_arr) = steps.as_array() {
                                if steps_arr.is_empty() {
                                    warnings.push(
                                        "Steps array is empty - orchestration won't do anything"
                                            .to_string(),
                                    );
                                }

                                // Validate each step
                                for (idx, step) in steps_arr.iter().enumerate() {
                                    let step_num = idx + 1;

                                    if !step.is_object() {
                                        errors.push(format!("Step #{} is not an object", step_num));
                                        continue;
                                    }

                                    // Check step name
                                    if step.get("name").is_none() {
                                        errors.push(format!(
                                            "Step #{} is missing 'name' field",
                                            step_num
                                        ));
                                    }

                                    // Check scenario
                                    match step.get("scenario") {
                                        None => {
                                            errors.push(format!(
                                                "Step #{} is missing 'scenario' field",
                                                step_num
                                            ));
                                        }
                                        Some(scenario) => {
                                            if scenario.get("name").is_none() {
                                                errors.push(format!(
                                                    "Step #{} scenario is missing 'name' field",
                                                    step_num
                                                ));
                                            }
                                            if scenario.get("config").is_none() {
                                                errors.push(format!(
                                                    "Step #{} scenario is missing 'config' field",
                                                    step_num
                                                ));
                                            }
                                        }
                                    }

                                    // Check duration
                                    if step.get("duration_seconds").is_none() {
                                        warnings.push(format!("Step #{} is missing 'duration_seconds' - using default", step_num));
                                    } else if !step["duration_seconds"].is_number() {
                                        errors.push(format!(
                                            "Step #{} 'duration_seconds' must be a number",
                                            step_num
                                        ));
                                    }

                                    // Check delay
                                    if let Some(delay) = step.get("delay_before_seconds") {
                                        if !delay.is_number() {
                                            errors.push(format!(
                                                "Step #{} 'delay_before_seconds' must be a number",
                                                step_num
                                            ));
                                        }
                                    }
                                }
                            } else {
                                errors.push("Field 'steps' must be an array".to_string());
                            }
                        }
                    }

                    // Print results
                    if !errors.is_empty() {
                        println!("❌ Orchestration file has errors:");
                        for error in &errors {
                            println!("   ✗ {}", error);
                        }
                        return Err("Validation failed".into());
                    }

                    println!("✅ Orchestration file is valid");

                    // Show summary
                    if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
                        println!("\n📊 Summary:");
                        println!("   Name: {}", name);
                        if let Some(desc) = value.get("description").and_then(|v| v.as_str()) {
                            println!("   Description: {}", desc);
                        }
                        if let Some(steps) = value.get("steps").and_then(|v| v.as_array()) {
                            println!("   Steps: {}", steps.len());
                        }
                    }

                    if !warnings.is_empty() {
                        println!("\n⚠️  Warnings:");
                        for warning in warnings {
                            println!("   - {}", warning);
                        }
                    }
                }
                Err(error_msg) => {
                    println!("❌ Orchestration file validation failed:\n");
                    println!("{}", error_msg);
                    return Err("Invalid orchestration file".into());
                }
            }
        }

        OrchestrateCommands::Template { output, format } => {
            println!("📝 Generating orchestration template...");

            let template = if format == "json" {
                serde_json::to_string_pretty(&serde_json::json!({
                    "name": "example_orchestration",
                    "description": "Example chaos orchestration",
                    "steps": [
                        {
                            "name": "warmup",
                            "scenario": {
                                "name": "network_degradation",
                                "config": {
                                    "enabled": true,
                                    "latency": {
                                        "enabled": true,
                                        "fixed_delay_ms": 100
                                    }
                                }
                            },
                            "duration_seconds": 60,
                            "delay_before_seconds": 0,
                            "continue_on_failure": false
                        },
                        {
                            "name": "peak_load",
                            "scenario": {
                                "name": "peak_traffic",
                                "config": {
                                    "enabled": true,
                                    "rate_limit": {
                                        "enabled": true,
                                        "requests_per_second": 100
                                    }
                                }
                            },
                            "duration_seconds": 120,
                            "delay_before_seconds": 10,
                            "continue_on_failure": true
                        }
                    ],
                    "parallel": false,
                    "loop_orchestration": false,
                    "max_iterations": 1,
                    "tags": ["example", "test"]
                }))?
            } else {
                "name: example_orchestration
description: Example chaos orchestration
steps:
  - name: warmup
    scenario:
      name: network_degradation
      config:
        enabled: true
        latency:
          enabled: true
          fixed_delay_ms: 100
    duration_seconds: 60
    delay_before_seconds: 0
    continue_on_failure: false
  - name: peak_load
    scenario:
      name: peak_traffic
      config:
        enabled: true
        rate_limit:
          enabled: true
          requests_per_second: 100
    duration_seconds: 120
    delay_before_seconds: 10
    continue_on_failure: true
parallel: false
loop_orchestration: false
max_iterations: 1
tags:
  - example
  - test
"
                .to_string()
            };

            std::fs::write(&output, template)?;
            println!("✅ Template saved to: {}", output.display());
        }
    }

    Ok(())
}

/// Handle AI-powered spec suggestion command
#[allow(clippy::too_many_arguments)]
async fn handle_suggest(
    from: Option<PathBuf>,
    from_description: Option<String>,
    format: String,
    output: Option<PathBuf>,
    num_suggestions: usize,
    include_examples: bool,
    domain: Option<String>,
    llm_provider: String,
    llm_model: Option<String>,
    llm_endpoint: Option<String>,
    llm_api_key: Option<String>,
    temperature: f64,
    print_json: bool,
) -> anyhow::Result<()> {
    use mockforge_core::intelligent_behavior::{
        config::BehaviorModelConfig, OutputFormat, SpecSuggestionEngine, SuggestionConfig,
        SuggestionInput,
    };

    // Determine output format
    let output_format = format.parse::<OutputFormat>().map_err(|e| anyhow::anyhow!("{}", e))?;

    // Build LLM config
    let default_model = match llm_provider.to_lowercase().as_str() {
        "openai" => "gpt-4o-mini",
        "anthropic" => "claude-3-5-sonnet-20241022",
        "ollama" => "llama3.1",
        _ => "gpt-4o-mini",
    };

    let llm_config = BehaviorModelConfig {
        llm_provider: llm_provider.clone(),
        model: llm_model.unwrap_or_else(|| default_model.to_string()),
        api_endpoint: llm_endpoint,
        api_key: llm_api_key,
        temperature,
        max_tokens: 4000,
        ..Default::default()
    };

    // Build suggestion config
    let suggestion_config = SuggestionConfig {
        llm_config,
        output_format,
        num_suggestions,
        include_examples,
        domain_hint: domain,
    };

    // Parse input
    let input = if let Some(description) = from_description {
        SuggestionInput::Description { text: description }
    } else if let Some(input_path) = from {
        let content = tokio::fs::read_to_string(&input_path).await?;
        let json_value: serde_json::Value = serde_json::from_str(&content)?;

        // Try to detect input type
        if let Some(method) = json_value.get("method").and_then(|v| v.as_str()) {
            // Single endpoint format
            SuggestionInput::Endpoint {
                method: method.to_string(),
                path: json_value
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'path' field in endpoint input"))?
                    .to_string(),
                request: json_value.get("request").cloned(),
                response: json_value.get("response").cloned(),
                description: json_value
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from),
            }
        } else if json_value.get("openapi").is_some() || json_value.get("paths").is_some() {
            // Partial OpenAPI spec
            SuggestionInput::PartialSpec { spec: json_value }
        } else if let Some(paths_array) = json_value.get("paths").and_then(|v| v.as_array()) {
            // List of paths
            let paths = paths_array.iter().filter_map(|v| v.as_str().map(String::from)).collect();
            SuggestionInput::Paths { paths }
        } else {
            return Err(anyhow::anyhow!(
                "Unable to detect input type. Expected 'method' field for endpoint, \
                 'openapi' for spec, or 'paths' array"
            ));
        }
    } else {
        return Err(anyhow::anyhow!(
            "Must provide either --from <file> or --from-description <text>"
        ));
    };

    println!("🤖 Generating API specification suggestions...");
    println!("   Provider: {}", llm_provider);
    println!("   Model: {}", suggestion_config.llm_config.model);
    println!("   Suggestions: {}", num_suggestions);
    if let Some(ref d) = suggestion_config.domain_hint {
        println!("   Domain: {}", d);
    }
    println!();

    // Create engine and generate suggestions
    let engine = SpecSuggestionEngine::new(suggestion_config);
    let result = engine.suggest(&input).await?;

    // Print results
    if print_json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("✅ Generated {} endpoint suggestions", result.metadata.endpoint_count);
        if let Some(domain) = &result.metadata.detected_domain {
            println!("   Detected domain: {}", domain);
        }
        println!();

        // Print endpoint suggestions
        println!("📝 Suggested Endpoints:");
        for (i, suggestion) in result.suggestions.iter().enumerate() {
            println!("\n{}. {} {}", i + 1, suggestion.method, suggestion.path);
            println!("   {}", suggestion.description);
            if !suggestion.parameters.is_empty() {
                println!("   Parameters:");
                for param in &suggestion.parameters {
                    let req = if param.required {
                        "required"
                    } else {
                        "optional"
                    };
                    println!(
                        "     - {} ({}): {} [{}]",
                        param.name, param.location, param.data_type, req
                    );
                }
            }
            if !suggestion.reasoning.is_empty() {
                println!("   💡 {}", suggestion.reasoning);
            }
        }
        println!();

        // Save specs to file(s)
        if let Some(base_path) = output {
            match output_format {
                OutputFormat::OpenAPI => {
                    if let Some(spec) = &result.openapi_spec {
                        let yaml = serde_yaml::to_string(spec)?;
                        tokio::fs::write(&base_path, yaml).await?;
                        println!("✅ OpenAPI spec saved to: {}", base_path.display());
                    } else {
                        println!("⚠️  No OpenAPI spec generated");
                    }
                }
                OutputFormat::MockForge => {
                    if let Some(config) = &result.mockforge_config {
                        let yaml = serde_yaml::to_string(config)?;
                        tokio::fs::write(&base_path, yaml).await?;
                        println!("✅ MockForge config saved to: {}", base_path.display());
                    } else {
                        println!("⚠️  No MockForge config generated");
                    }
                }
                OutputFormat::Both => {
                    // Save both with different extensions
                    let openapi_path = base_path.with_extension("openapi.yaml");
                    let mockforge_path = base_path.with_extension("mockforge.yaml");

                    if let Some(spec) = &result.openapi_spec {
                        let yaml = serde_yaml::to_string(spec)?;
                        tokio::fs::write(&openapi_path, yaml).await?;
                        println!("✅ OpenAPI spec saved to: {}", openapi_path.display());
                    }

                    if let Some(config) = &result.mockforge_config {
                        let yaml = serde_yaml::to_string(config)?;
                        tokio::fs::write(&mockforge_path, yaml).await?;
                        println!("✅ MockForge config saved to: {}", mockforge_path.display());
                    }
                }
            }
        } else {
            println!("💡 Tip: Use --output <file> to save the generated specification");
        }
    }

    Ok(())
}
