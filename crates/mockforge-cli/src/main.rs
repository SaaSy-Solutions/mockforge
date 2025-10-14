use axum::serve;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use mockforge_core::encryption::init_key_store;
use mockforge_core::{apply_env_overrides, load_config_with_fallback, ServerConfig};
use mockforge_data::rag::{EmbeddingProvider, LlmProvider, RagConfig};
use mockforge_observability::prometheus::{prometheus_router, MetricsRegistry};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;

mod plugin_commands;
mod smtp_commands;
mod workspace_commands;

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

        /// HTTP server port
        #[arg(long, default_value = "3000", help_heading = "Server Ports")]
        http_port: u16,

        /// WebSocket server port
        #[arg(long, default_value = "3001", help_heading = "Server Ports")]
        ws_port: u16,

        /// gRPC server port
        #[arg(long, default_value = "50051", help_heading = "Server Ports")]
        grpc_port: u16,

        /// SMTP server port
        #[arg(long, default_value = "1025", help_heading = "Server Ports")]
        smtp_port: u16,

        /// Enable admin UI
        #[arg(long, help_heading = "Admin & UI")]
        admin: bool,

        /// Admin UI port (when running standalone)
        #[arg(long, default_value = "9080", help_heading = "Admin & UI")]
        admin_port: u16,

        /// Enable Prometheus metrics endpoint
        #[arg(long, help_heading = "Observability & Metrics")]
        metrics: bool,

        /// Metrics server port
        #[arg(long, default_value = "9090", help_heading = "Observability & Metrics")]
        metrics_port: u16,

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
    },

    /// SMTP server management and mailbox operations
    ///
    /// Examples:
    ///   mockforge smtp mailbox list
    ///   mockforge smtp mailbox show email-123
    ///   mockforge smtp mailbox clear
    ///   mockforge smtp fixtures list
    ///   mockforge smtp send --to user@example.com --subject "Test"
    #[command(verbatim_doc_comment)]
    Smtp {
        #[command(subcommand)]
        smtp_command: SmtpCommands,
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

    /// Configuration management
    Config {
        #[command(subcommand)]
        config_command: ConfigCommands,
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
        #[arg(short, long)]
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
        #[arg(short, long, default_value = "5")]
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
            http_port,
            ws_port,
            grpc_port,
            smtp_port: _smtp_port,
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
            ai_enabled,
            rag_provider,
            rag_model,
            rag_api_key,
            dry_run,
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
                http_port,
                ws_port,
                grpc_port,
                _smtp_port,
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
                traffic_shaping,
                bandwidth_limit,
                burst_size,
                network_profile,
                chaos_random,
                chaos_random_error_rate,
                chaos_random_delay_rate,
                chaos_random_min_delay,
                chaos_random_max_delay,
                ai_enabled,
                rag_provider,
                rag_model,
                rag_api_key,
                dry_run,
            )
            .await?;
        }
        Commands::Smtp { smtp_command } => {
            smtp_commands::handle_smtp_command(smtp_command).await?;
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
        Commands::Completions { shell } => {
            handle_completions(shell);
        }
        Commands::Init { name, no_examples } => {
            handle_init(name, no_examples).await?;
        }
        Commands::Config { config_command } => {
            handle_config(config_command).await?;
        }
        Commands::TestAi { ai_command } => {
            handle_test_ai(ai_command).await?;
        }

        Commands::Plugin { plugin_command } => {
            plugin_commands::handle_plugin_command(plugin_command).await?;
        }
        Commands::Workspace { workspace_command } => {
            workspace_commands::handle_workspace_command(workspace_command).await?;
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
    http_port: u16,
    ws_port: u16,
    grpc_port: u16,
    admin: bool,
    admin_port: u16,
    metrics: bool,
    metrics_port: u16,
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
    traffic_shaping: bool,
    bandwidth_limit: u64,
    burst_size: u64,
    ai_enabled: bool,
    rag_provider: Option<String>,
    rag_model: Option<String>,
    rag_api_key: Option<String>,
    network_profile: Option<String>,
    chaos_random: bool,
    #[allow(dead_code)]
    chaos_random_error_rate: f64,
    #[allow(dead_code)]
    chaos_random_delay_rate: f64,
    #[allow(dead_code)]
    chaos_random_min_delay: u64,
    #[allow(dead_code)]
    chaos_random_max_delay: u64,
    dry_run: bool,
}

/// Build ServerConfig from CLI arguments, config file, and environment variables
/// Precedence: CLI args > Config file > Environment variables > Defaults
async fn build_server_config_from_cli(serve_args: &ServeArgs) -> ServerConfig {
    // Step 1: Load config from file if provided, otherwise use defaults
    let mut config = if let Some(path) = &serve_args.config_path {
        println!("📄 Loading configuration from: {}", path.display());
        load_config_with_fallback(path.clone()).await
    } else {
        ServerConfig::default()
    };

    // Step 2: Apply environment variable overrides
    config = apply_env_overrides(config);

    // Step 3: Apply CLI argument overrides (CLI takes highest precedence)

    // HTTP configuration
    config.http.port = serve_args.http_port;
    if let Some(spec_path) = &serve_args.spec {
        config.http.openapi_spec = Some(spec_path.to_string_lossy().to_string());
    }

    // WebSocket configuration
    config.websocket.port = serve_args.ws_port;
    if let Some(replay_path) = &serve_args.ws_replay_file {
        config.websocket.replay_file = Some(replay_path.to_string_lossy().to_string());
    }

    // gRPC configuration
    config.grpc.port = serve_args.grpc_port;

    // Admin configuration
    config.admin.enabled = serve_args.admin;
    config.admin.port = serve_args.admin_port;

    // Prometheus metrics configuration
    config.observability.prometheus.enabled = serve_args.metrics;
    config.observability.prometheus.port = serve_args.metrics_port;

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

/// Validate server configuration before starting
async fn validate_serve_config(
    config_path: &Option<PathBuf>,
    spec_path: &Option<PathBuf>,
    ports: &[(u16, &str)],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::fs;
    use std::net::TcpListener;

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

    // Check port availability
    let mut unavailable_ports = Vec::new();
    for (port, name) in ports {
        // Try to bind to the port to check if it's available
        match TcpListener::bind(("127.0.0.1", *port)) {
            Ok(_) => {
                // Port is available
            }
            Err(e) => {
                unavailable_ports.push((*port, *name, e));
            }
        }
    }

    if !unavailable_ports.is_empty() {
        let mut error_msg = String::from("One or more ports are already in use:\n\n");
        for (port, name, err) in &unavailable_ports {
            error_msg.push_str(&format!("  • {} port {}: {}\n", name, port, err));
        }
        error_msg.push_str("\nPossible solutions:\n");
        error_msg.push_str("  1. Stop the process using these ports\n");
        error_msg
            .push_str("  2. Use different ports with flags like --http-port, --ws-port, etc.\n");
        error_msg.push_str("  3. Find the process using the port with: lsof -i :<port> or netstat -tulpn | grep <port>\n");

        return Err(error_msg.into());
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
    http_port: u16,
    ws_port: u16,
    grpc_port: u16,
    _smtp_port: u16,
    admin: bool,
    admin_port: u16,
    metrics: bool,
    metrics_port: u16,
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
    traffic_shaping: bool,
    bandwidth_limit: u64,
    burst_size: u64,
    network_profile: Option<String>,
    chaos_random: bool,
    chaos_random_error_rate: f64,
    chaos_random_delay_rate: f64,
    chaos_random_min_delay: u64,
    chaos_random_max_delay: u64,
    ai_enabled: bool,
    rag_provider: Option<String>,
    rag_model: Option<String>,
    rag_api_key: Option<String>,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Perform early validation
    let mut ports_to_check = vec![
        (http_port, "HTTP"),
        (ws_port, "WebSocket"),
        (grpc_port, "gRPC"),
    ];

    if admin {
        ports_to_check.push((admin_port, "Admin UI"));
    }

    if metrics {
        ports_to_check.push((metrics_port, "Metrics"));
    }

    let serve_args = ServeArgs {
        config_path: config_path.clone(),
        http_port,
        ws_port,
        grpc_port,
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
    };

    validate_serve_config(&serve_args.config_path, &serve_args.spec, &ports_to_check).await?;

    if serve_args.dry_run {
        println!("✅ Configuration validation passed!");
        println!("✅ All required ports are available");
        if serve_args.config_path.is_some() {
            println!("✅ Configuration file is valid");
        }
        if serve_args.spec.is_some() {
            println!("✅ OpenAPI spec file is valid");
        }
        if serve_args.spec.is_some() {
            println!("✅ OpenAPI spec file is valid");
        }
        println!("\n🎉 Dry run successful - no issues found!");
        return Ok(());
    }

    let mut config = build_server_config_from_cli(&serve_args).await;

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

    // Build HTTP router with OpenAPI spec, chain support, multi-tenant, and traffic shaping if enabled
    let multi_tenant_config = if config.multi_tenant.enabled {
        Some(config.multi_tenant.clone())
    } else {
        None
    };

    let http_app = if config.core.traffic_shaping_enabled {
        use mockforge_core::TrafficShaper;
        let traffic_shaper = Some(TrafficShaper::new(config.core.traffic_shaping.clone()));
        mockforge_http::build_router_with_traffic_shaping_and_multi_tenant(
            config.http.openapi_spec.clone(),
            None,
            traffic_shaper,
            true,
            multi_tenant_config,
        )
        .await
    } else {
        // Use chain-enabled router for standard operation
        mockforge_http::build_router_with_chains_and_multi_tenant(
            config.http.openapi_spec.clone(),
            None,
            None, // Use default chain config
            multi_tenant_config,
            Some(config.routes.clone()),
            config.http.cors.clone(),
        )
        .await
    };

    println!(
        "✅ HTTP server configured with health check at http://localhost:{}/health",
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

    // Start HTTP server
    let http_port = config.http.port;
    let http_shutdown = shutdown_token.clone();
    let http_handle = tokio::spawn(async move {
        println!("📡 HTTP server listening on http://localhost:{}", http_port);
        tokio::select! {
            result = mockforge_http::serve_router(http_port, http_app) => {
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

    // Start SMTP server (if enabled)
    let smtp_handle = if config.smtp.enabled {
        let smtp_config = config.smtp.clone();
        let smtp_shutdown = shutdown_token.clone();

        Some(tokio::spawn(async move {
            use mockforge_smtp::{SmtpServer, SmtpSpecRegistry};
            use std::sync::Arc;

            println!("📧 SMTP server listening on {}:{}", smtp_config.host, smtp_config.port);

            // Create registry and load fixtures
            let mut registry =
                SmtpSpecRegistry::with_mailbox_size(smtp_config.max_mailbox_messages);

            if let Some(fixtures_dir) = &smtp_config.fixtures_dir {
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

            // Convert core SmtpConfig to mockforge_smtp::SmtpConfig
            let smtp_server_config = mockforge_smtp::SmtpConfig {
                port: smtp_config.port,
                host: smtp_config.host.clone(),
                hostname: smtp_config.hostname.clone(),
                fixtures_dir: smtp_config.fixtures_dir.clone(),
                timeout_secs: smtp_config.timeout_secs,
                max_connections: smtp_config.max_connections,
                enable_mailbox: smtp_config.enable_mailbox,
                max_mailbox_messages: smtp_config.max_mailbox_messages,
            };

            let server = SmtpServer::new(smtp_server_config, Arc::new(registry));

            tokio::select! {
                result = server.start() => {
                    result.map_err(|e| format!("SMTP server error: {}", e))
                }
                _ = smtp_shutdown.cancelled() => {
                    Ok(())
                }
            }
        }))
    } else {
        None
    };

    // Start Admin UI server (if enabled)
    let admin_handle = if config.admin.enabled {
        let admin_port = config.admin.port;
        let http_port = config.http.port;
        let ws_port = config.websocket.port;
        let grpc_port = config.grpc.port;
        let prometheus_url = config.admin.prometheus_url.clone();
        let admin_shutdown = shutdown_token.clone();
        Some(tokio::spawn(async move {
            println!("🎛️ Admin UI listening on http://localhost:{}", admin_port);
            let addr = format!("127.0.0.1:{}", admin_port).parse().unwrap();
            tokio::select! {
                result = mockforge_ui::start_admin_server(
                    addr,
                    Some(format!("127.0.0.1:{}", http_port).parse().unwrap()),
                    Some(format!("127.0.0.1:{}", ws_port).parse().unwrap()),
                    Some(format!("127.0.0.1:{}", grpc_port).parse().unwrap()),
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
        result = async {
            if let Some(handle) = smtp_handle {
                Some(handle.await)
            } else {
                std::future::pending::<Option<Result<Result<(), String>, tokio::task::JoinError>>>().await
            }
        } => {
            match result {
                Some(Ok(Ok(()))) => {
                    println!("📧 SMTP server stopped gracefully");
                    None
                }
                Some(Ok(Err(e))) => {
                    eprintln!("❌ {}", e);
                    Some(e)
                }
                Some(Err(e)) => {
                    let error = format!("SMTP server task panicked: {}", e);
                    eprintln!("❌ {}", error);
                    Some(error)
                }
                None => None
            }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("🛑 Received shutdown signal");
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
    rows: usize,
    rag_enabled: bool,
    rag_provider: Option<String>,
    rag_model: Option<String>,
    rag_endpoint: Option<String>,
    rag_timeout: Option<u64>,
    rag_max_retries: Option<usize>,
) -> Result<mockforge_data::GenerationResult, Box<dyn std::error::Error + Send + Sync>> {
    use mockforge_data::schema::templates;

    let config = mockforge_data::DataConfig {
        rows,
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

/// Handle shell completions generation
fn handle_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();
    generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
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

/// Format YAML parsing errors with line numbers
fn format_yaml_error(content: &str, error: serde_yaml::Error) -> String {
    let mut message = String::from("Invalid YAML syntax:\n");

    if let Some(location) = error.location() {
        let line = location.line();
        let column = location.column();

        message.push_str(&format!("  at line {}, column {}\n\n", line, column));

        // Show the problematic line with context
        let lines: Vec<&str> = content.lines().collect();
        let start = line.saturating_sub(2);
        let end = (line + 1).min(lines.len());

        for (idx, line_content) in lines[start..end].iter().enumerate() {
            let line_num = start + idx + 1;
            if line_num == line {
                message.push_str(&format!("  > {} | {}\n", line_num, line_content));
                message.push_str(&format!(
                    "  {}^\n",
                    " ".repeat(column + 5 + line_num.to_string().len())
                ));
            } else {
                message.push_str(&format!("    {} | {}\n", line_num, line_content));
            }
        }

        message.push_str(&format!("\n  Error: {}\n", error));
    } else {
        message.push_str(&format!("  {}\n", error));
    }

    // Add helpful suggestions based on common errors
    let error_str = error.to_string();
    if error_str.contains("duplicate key") {
        message.push_str("\n💡 Tip: You have a duplicate key in your YAML. Each key must be unique within its section.\n");
    } else if error_str.contains("invalid type") {
        message.push_str("\n💡 Tip: Check that your values match the expected types (strings, numbers, booleans, arrays, objects).\n");
    } else if error_str.contains("missing field") {
        message.push_str("\n💡 Tip: A required field is missing. Check the documentation for required configuration fields.\n");
    } else if error_str.contains("unknown field") {
        message.push_str("\n💡 Tip: You may have a typo in a field name. Check the spelling against the documentation.\n");
    }

    message
}

/// Format JSON parsing errors with line numbers
fn format_json_error(content: &str, error: serde_json::Error) -> String {
    let mut message = String::from("Invalid JSON syntax:\n");

    let line = error.line();
    let column = error.column();

    message.push_str(&format!("  at line {}, column {}\n\n", line, column));

    // Show the problematic line with context
    let lines: Vec<&str> = content.lines().collect();
    let start = line.saturating_sub(2);
    let end = (line + 1).min(lines.len());

    for (idx, line_content) in lines[start..end].iter().enumerate() {
        let line_num = start + idx + 1;
        if line_num == line {
            message.push_str(&format!("  > {} | {}\n", line_num, line_content));
            message
                .push_str(&format!("  {}^\n", " ".repeat(column + 5 + line_num.to_string().len())));
        } else {
            message.push_str(&format!("    {} | {}\n", line_num, line_content));
        }
    }

    message.push_str(&format!("\n  Error: {}\n", error));

    // Add helpful suggestions
    let error_str = error.to_string();
    if error_str.contains("trailing comma") {
        message.push_str(
            "\n💡 Tip: JSON doesn't allow trailing commas. Remove the comma after the last item.\n",
        );
    } else if error_str.contains("expected") {
        message.push_str(
            "\n💡 Tip: Check for missing or extra brackets, braces, quotes, or commas.\n",
        );
    } else if error_str.contains("duplicate field") {
        message.push_str(
            "\n💡 Tip: You have a duplicate key. Each key must be unique within its object.\n",
        );
    }

    message
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

    // Open database
    let db = RecorderDatabase::new(database.to_str().unwrap()).await?;
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
