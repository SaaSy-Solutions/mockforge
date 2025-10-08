use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use mockforge_core::encryption::init_key_store;
use mockforge_data;
use mockforge_data::rag::{EmbeddingProvider, LlmProvider, RagConfig};
use mockforge_grpc;
use mockforge_http;
use mockforge_ui;
use mockforge_ws;
use std::path::PathBuf;

mod plugin_commands;

#[derive(Parser)]
#[command(name = "mockforge")]
#[command(about = "MockForge - Comprehensive API Mocking Framework")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start mock servers (HTTP, WebSocket, gRPC)
    Serve {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// HTTP server port
        #[arg(long, default_value = "3000")]
        http_port: u16,

        /// WebSocket server port
        #[arg(long, default_value = "3001")]
        ws_port: u16,

        /// gRPC server port
        #[arg(long, default_value = "50051")]
        grpc_port: u16,

        /// Enable admin UI
        #[arg(long)]
        admin: bool,

        /// Admin UI port (when running standalone)
        #[arg(long, default_value = "9080")]
        admin_port: u16,

        /// Enable Prometheus metrics endpoint
        #[arg(long)]
        metrics: bool,

        /// Metrics server port
        #[arg(long, default_value = "9090")]
        metrics_port: u16,

        /// Enable OpenTelemetry distributed tracing
        #[arg(long)]
        tracing: bool,

        /// Service name for traces
        #[arg(long, default_value = "mockforge")]
        tracing_service_name: String,

        /// Tracing environment (development, staging, production)
        #[arg(long, default_value = "development")]
        tracing_environment: String,

        /// Jaeger endpoint for trace export
        #[arg(long, default_value = "http://localhost:14268/api/traces")]
        jaeger_endpoint: String,

        /// Tracing sampling rate (0.0 to 1.0)
        #[arg(long, default_value = "1.0")]
        tracing_sampling_rate: f64,

        /// Enable API Flight Recorder
        #[arg(long)]
        recorder: bool,

        /// Recorder database file path
        #[arg(long, default_value = "./mockforge-recordings.db")]
        recorder_db: String,

        /// Disable recorder management API
        #[arg(long)]
        recorder_no_api: bool,

        /// Recorder management API port (defaults to main port)
        #[arg(long)]
        recorder_api_port: Option<u16>,

        /// Maximum number of recorded requests (0 for unlimited)
        #[arg(long, default_value = "10000")]
        recorder_max_requests: i64,

        /// Auto-delete recordings older than N days (0 to disable)
        #[arg(long, default_value = "7")]
        recorder_retention_days: i64,

        /// Enable chaos engineering
        #[arg(long)]
        chaos: bool,

        /// Chaos scenario (network_degradation, service_instability, cascading_failure, peak_traffic, slow_backend)
        #[arg(long)]
        chaos_scenario: Option<String>,

        /// Chaos latency: fixed delay in milliseconds
        #[arg(long)]
        chaos_latency_ms: Option<u64>,

        /// Chaos latency: random delay range (min-max) in milliseconds
        #[arg(long)]
        chaos_latency_range: Option<String>,

        /// Chaos latency probability (0.0-1.0)
        #[arg(long, default_value = "1.0")]
        chaos_latency_probability: f64,

        /// Chaos fault injection: HTTP error codes (comma-separated)
        #[arg(long)]
        chaos_http_errors: Option<String>,

        /// Chaos fault injection: HTTP error probability (0.0-1.0)
        #[arg(long, default_value = "0.1")]
        chaos_http_error_probability: f64,

        /// Chaos rate limit: requests per second
        #[arg(long)]
        chaos_rate_limit: Option<u32>,

        /// Chaos traffic shaping: bandwidth limit (bytes/sec)
        #[arg(long)]
        chaos_bandwidth_limit: Option<u64>,

        /// Chaos traffic shaping: packet loss percentage (0-100)
        #[arg(long)]
        chaos_packet_loss: Option<f64>,

        /// Enable gRPC-specific chaos engineering
        #[arg(long)]
        chaos_grpc: bool,

        /// gRPC chaos: status codes to inject (comma-separated)
        #[arg(long)]
        chaos_grpc_status_codes: Option<String>,

        /// gRPC chaos: stream interruption probability (0.0-1.0)
        #[arg(long, default_value = "0.1")]
        chaos_grpc_stream_interruption_probability: f64,

        /// Enable WebSocket-specific chaos engineering
        #[arg(long)]
        chaos_websocket: bool,

        /// WebSocket chaos: close codes to inject (comma-separated)
        #[arg(long)]
        chaos_websocket_close_codes: Option<String>,

        /// WebSocket chaos: message drop probability (0.0-1.0)
        #[arg(long, default_value = "0.05")]
        chaos_websocket_message_drop_probability: f64,

        /// WebSocket chaos: message corruption probability (0.0-1.0)
        #[arg(long, default_value = "0.05")]
        chaos_websocket_message_corruption_probability: f64,

        /// Enable GraphQL-specific chaos engineering
        #[arg(long)]
        chaos_graphql: bool,

        /// GraphQL chaos: error codes to inject (comma-separated)
        #[arg(long)]
        chaos_graphql_error_codes: Option<String>,

        /// GraphQL chaos: partial data probability (0.0-1.0)
        #[arg(long, default_value = "0.1")]
        chaos_graphql_partial_data_probability: f64,

        /// GraphQL chaos: enable resolver-level latency injection
        #[arg(long)]
        chaos_graphql_resolver_latency: bool,

        /// Enable circuit breaker pattern
        #[arg(long)]
        circuit_breaker: bool,

        /// Circuit breaker: failure threshold
        #[arg(long, default_value = "5")]
        circuit_breaker_failure_threshold: u64,

        /// Circuit breaker: success threshold
        #[arg(long, default_value = "2")]
        circuit_breaker_success_threshold: u64,

        /// Circuit breaker: timeout in milliseconds
        #[arg(long, default_value = "60000")]
        circuit_breaker_timeout_ms: u64,

        /// Circuit breaker: failure rate threshold percentage (0-100)
        #[arg(long, default_value = "50.0")]
        circuit_breaker_failure_rate: f64,

        /// Enable bulkhead pattern
        #[arg(long)]
        bulkhead: bool,

        /// Bulkhead: maximum concurrent requests
        #[arg(long, default_value = "100")]
        bulkhead_max_concurrent: u32,

        /// Bulkhead: maximum queue size
        #[arg(long, default_value = "10")]
        bulkhead_max_queue: u32,

        /// Bulkhead: queue timeout in milliseconds
        #[arg(long, default_value = "5000")]
        bulkhead_queue_timeout_ms: u64,

        /// OpenAPI spec file for HTTP server
        #[arg(short, long)]
        spec: Option<PathBuf>,

        /// WebSocket replay file
        #[arg(long)]
        ws_replay_file: Option<PathBuf>,

        /// Enable traffic shaping
        #[arg(long)]
        traffic_shaping: bool,

        /// Traffic shaping bandwidth limit (bytes per second)
        #[arg(long, default_value = "1000000")]
        bandwidth_limit: u64,

        /// Traffic shaping burst size (bytes)
        #[arg(long, default_value = "10000")]
        burst_size: u64,

        /// Enable AI-powered features
        #[arg(long)]
        ai_enabled: bool,

        /// AI/RAG provider (openai, anthropic, ollama, openai_compatible)
        #[arg(long)]
        rag_provider: Option<String>,

        /// AI/RAG model name
        #[arg(long)]
        rag_model: Option<String>,

        /// AI/RAG API key (or set MOCKFORGE_RAG_API_KEY)
        #[arg(long, env = "MOCKFORGE_RAG_API_KEY")]
        rag_api_key: Option<String>,
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

    /// Start sync daemon for background directory synchronization
    Sync {
        /// Workspace directory to monitor
        #[arg(short, long)]
        workspace_dir: PathBuf,

        /// Configuration file path
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

    /// Chaos experiment orchestration
    Orchestrate {
        #[command(subcommand)]
        orchestrate_command: OrchestrateCommands,
    },

    /// Generate tests from recorded API interactions
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
        #[arg(long, env = "MOCKFORGE_LLM_API_KEY")]
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
}

#[derive(Subcommand)]
enum OrchestrateCommands {
    /// Start a chaos orchestration from file
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
enum DataCommands {
    /// Generate data from built-in templates
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

    // Initialize tracing
    tracing_subscriber::fmt::init();

    match cli.command {
        Commands::Serve {
            config,
            http_port,
            ws_port,
            grpc_port,
            admin,
            admin_port,
            metrics,
            metrics_port,
            spec,
            ws_replay_file,
            traffic_shaping,
            bandwidth_limit,
            burst_size,
            ai_enabled,
            rag_provider,
            rag_model,
            rag_api_key,
        } => {
            handle_serve(
                config,
                http_port,
                ws_port,
                grpc_port,
                admin,
                admin_port,
                metrics,
                metrics_port,
                spec,
                ws_replay_file,
                traffic_shaping,
                bandwidth_limit,
                burst_size,
                ai_enabled,
                rag_provider,
                rag_model,
                rag_api_key,
            )
            .await?;
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

        Commands::Orchestrate { orchestrate_command } => {
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
            ).await?;
        }
    }

    Ok(())
}

async fn handle_serve(
    _config: Option<PathBuf>,
    http_port: u16,
    ws_port: u16,
    grpc_port: u16,
    admin: bool,
    admin_port: u16,
    metrics: bool,
    metrics_port: u16,
    spec: Option<PathBuf>,
    _ws_replay_file: Option<PathBuf>,
    traffic_shaping: bool,
    bandwidth_limit: u64,
    burst_size: u64,
    ai_enabled: bool,
    rag_provider: Option<String>,
    rag_model: Option<String>,
    rag_api_key: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚀 Starting MockForge servers...");
    println!("📡 HTTP server on port {}", http_port);
    println!("🔌 WebSocket server on port {}", ws_port);
    println!("⚡ gRPC server on port {}", grpc_port);
    if admin {
        println!("🎛️ Admin UI on port {}", admin_port);
    }
    if metrics {
        println!("📊 Metrics endpoint on port {}", metrics_port);
    }
    if ai_enabled {
        println!("🧠 AI features enabled");
        if let Some(provider) = &rag_provider {
            println!("🤖 AI Provider: {}", provider);
        }
        if let Some(model) = &rag_model {
            println!("🧠 AI Model: {}", model);
        }
    }

    // Set AI environment variables if provided
    if let Some(api_key) = rag_api_key {
        std::env::set_var("MOCKFORGE_RAG_API_KEY", api_key);
    }
    if let Some(provider) = rag_provider {
        std::env::set_var("MOCKFORGE_RAG_PROVIDER", provider);
    }
    if let Some(model) = rag_model {
        std::env::set_var("MOCKFORGE_RAG_MODEL", model);
    }

    // Initialize key store at startup
    init_key_store();

    // Build HTTP router with OpenAPI spec, chain support, and traffic shaping if enabled
    let http_app = if traffic_shaping {
        use mockforge_core::{
            BandwidthConfig, BurstLossConfig, TrafficShaper, TrafficShapingConfig,
        };
        let config = TrafficShapingConfig {
            bandwidth: BandwidthConfig::new(bandwidth_limit, burst_size),
            burst_loss: BurstLossConfig::default(), // Disable burst loss for now
        };
        let traffic_shaper = Some(TrafficShaper::new(config));
        mockforge_http::build_router_with_traffic_shaping(
            spec.as_ref().map(|p| p.to_string_lossy().to_string()),
            None,
            traffic_shaper,
            true,
        )
        .await
    } else {
        // Use chain-enabled router for standard operation
        mockforge_http::build_router_with_chains(
            spec.as_ref().map(|p| p.to_string_lossy().to_string()),
            None,
            None, // Use default chain config
        )
        .await
    };

    println!(
        "✅ HTTP server configured with health check at http://localhost:{}/health",
        http_port
    );
    println!("✅ WebSocket server configured at ws://localhost:{}/ws", ws_port);
    println!("✅ gRPC server configured at localhost:{}", grpc_port);
    if admin {
        println!("✅ Admin UI configured at http://localhost:{}", admin_port);
    }

    println!("💡 Press Ctrl+C to stop");

    // Start HTTP server
    let http_handle = tokio::spawn(async move {
        println!("📡 HTTP server listening on http://localhost:{}", http_port);
        if let Err(e) = mockforge_http::serve_router(http_port, http_app).await {
            eprintln!("❌ HTTP server error: {}", e);
        }
    });

    // Start WebSocket server
    let ws_handle = tokio::spawn(async move {
        println!("🔌 WebSocket server listening on ws://localhost:{}", ws_port);
        if let Err(e) = mockforge_ws::start_with_latency(ws_port, None).await {
            eprintln!("❌ WebSocket server error: {}", e);
        }
    });

    // Start gRPC server
    let grpc_handle = tokio::spawn(async move {
        println!("⚡ gRPC server listening on localhost:{}", grpc_port);
        if let Err(e) = mockforge_grpc::start(grpc_port).await {
            eprintln!("❌ gRPC server error: {}", e);
        }
    });

    // Start Admin UI server (if enabled)
    let admin_handle = if admin {
        Some(tokio::spawn(async move {
            println!("🎛️ Admin UI listening on http://localhost:{}", admin_port);
            let addr = format!("127.0.0.1:{}", admin_port).parse().unwrap();
            if let Err(e) = mockforge_ui::start_admin_server(
                addr,
                Some(format!("127.0.0.1:{}", http_port).parse().unwrap()),
                Some(format!("127.0.0.1:{}", ws_port).parse().unwrap()),
                Some(format!("127.0.0.1:{}", grpc_port).parse().unwrap()),
                None,
                true,
            )
            .await
            {
                eprintln!("❌ Admin UI server error: {}", e);
            }
        }))
    } else {
        None
    };

    // Start Prometheus metrics server (if enabled)
    let metrics_handle = if metrics {
        use mockforge_observability::{get_global_registry, prometheus::prometheus_router};
        use std::sync::Arc;

        Some(tokio::spawn(async move {
            let registry = Arc::new(get_global_registry().clone());
            let app = prometheus_router(registry);
            let addr = format!("0.0.0.0:{}", metrics_port);

            println!("📊 Metrics endpoint available at http://localhost:{}/metrics", metrics_port);

            match tokio::net::TcpListener::bind(&addr).await {
                Ok(listener) => {
                    if let Err(e) = axum::serve(listener, app).await {
                        eprintln!("❌ Metrics server error: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to bind metrics server: {}", e);
                }
            }
        }))
    } else {
        None
    };

    // Wait for all servers or shutdown signal
    tokio::select! {
        _ = http_handle => {
            println!("📡 HTTP server stopped");
        }
        _ = ws_handle => {
            println!("🔌 WebSocket server stopped");
        }
        _ = grpc_handle => {
            println!("⚡ gRPC server stopped");
        }
        _ = async {
            if let Some(handle) = admin_handle {
                handle.await.unwrap();
            } else {
                std::future::pending::<()>().await;
            }
        } => {
            println!("🎛️ Admin UI stopped");
        }
        _ = tokio::signal::ctrl_c() => {
            println!("🛑 Received shutdown signal");
        }
    }

    println!("👋 Shutting down servers...");

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
    mockforge_ui::start_admin_server(
        addr, None, // http_server_addr
        None, // ws_server_addr
        None, // grpc_server_addr
        None, // graphql_server_addr
        true, // api_enabled
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
    println!("🔄 Starting MockForge Sync Daemon...");
    println!("📁 Monitoring workspace directory: {}", workspace_dir.display());

    // Create sync service
    let sync_service = mockforge_core::SyncService::new(&workspace_dir);

    // Start the sync service
    sync_service.start().await?;

    println!("✅ Sync daemon started successfully!");
    println!("🔍 Monitoring for workspace sync changes...");
    println!("💡 Press Ctrl+C to stop");

    // Keep running until shutdown signal
    tokio::signal::ctrl_c().await?;
    println!("🛑 Received shutdown signal");

    // Stop the sync service
    sync_service.stop().await?;
    println!("👋 Sync daemon stopped");

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
http:
  port: 3000
  endpoints: []

websocket:
  port: 3001

grpc:
  port: 50051

admin:
  enabled: true
  port: 9080
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
    println!("  4. Run: mockforge serve");

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

    // Read and parse YAML
    let config_content = tokio::fs::read_to_string(&config_file).await?;
    let config: serde_json::Value = serde_yaml::from_str(&config_content)
        .map_err(|e| format!("Invalid YAML syntax: {}", e))?;

    // Basic validation
    let mut endpoints_count = 0;
    let mut chains_count = 0;
    let mut warnings = Vec::new();

    // Validate HTTP section
    if let Some(http) = config.get("http") {
        if let Some(endpoints) = http.get("endpoints") {
            if let Some(arr) = endpoints.as_array() {
                endpoints_count = arr.len();
            }
        }
    } else {
        warnings.push("No HTTP configuration found");
    }

    // Validate chains section
    if let Some(chains) = config.get("chains") {
        if let Some(arr) = chains.as_array() {
            chains_count = arr.len();
        }
    }

    // Check for admin section
    if config.get("admin").is_none() {
        warnings.push("No admin UI configuration found");
    }

    println!("✅ Configuration is valid");
    println!("\n📊 Summary:");
    println!("   Found {} HTTP endpoints", endpoints_count);
    println!("   Found {} chains", chains_count);

    if !warnings.is_empty() {
        println!("\n⚠️  Warnings:");
        for warning in warnings {
            println!("   - {}", warning);
        }
    }

    Ok(())
}

/// Discover configuration file in current directory and parents
fn discover_config_file() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let current_dir = std::env::current_dir()?;
    let config_names = vec!["mockforge.yaml", "mockforge.yml", ".mockforge.yaml", ".mockforge.yml"];

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
            use mockforge_data::{DataDriftConfig, DriftRule, DriftStrategy};

            let rule = DriftRule::new("value".to_string(), DriftStrategy::Linear)
                .with_rate(1.0);
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

            let config = ReplayAugmentationConfig::new(
                ReplayMode::Generated,
                EventStrategy::CountBased,
            )
            .with_narrative(narrative)
            .with_event_count(event_count)
            .with_rag_config(rag_config);

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
        TestGenerator, TestGenerationConfig, TestFormat, QueryFilter, RecorderDatabase, Protocol,
        LlmConfig,
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
    let protocol_filter = protocol.as_ref().and_then(|p| {
        match p.to_lowercase().as_str() {
            "http" => Some(Protocol::Http),
            "grpc" => Some(Protocol::Grpc),
            "websocket" => Some(Protocol::WebSocket),
            "graphql" => Some(Protocol::GraphQL),
            _ => None,
        }
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
    };

    // Create query filter
    let filter = QueryFilter {
        protocol: protocol_filter,
        method: method.clone(),
        path: path.clone(),
        status_code: status_code.map(|c| c as i32),
        min_duration_ms: None,
        max_duration_ms: None,
        trace_id: None,
        limit: Some(limit as i64),
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
        if ai_descriptions && !test.description.is_empty() && test.description != format!("Test {} {}", test.method, test.endpoint) {
            println!("      Description: {}", test.description);
        }
    }

    println!("\n🎉 Done! You can now run the generated tests.");

    Ok(())
}

async fn handle_orchestrate(command: OrchestrateCommands) -> Result<(), Box<dyn std::error::Error>> {
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
                let start_url = format!("{}/api/chaos/orchestration/start", base_url);
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
                    println!("   Progress: {:.1}%", status["progress"].as_f64().unwrap_or(0.0) * 100.0);
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

            // Read and parse file
            let content = std::fs::read_to_string(&file)?;

            let result = if file.extension().and_then(|s| s.to_str()) == Some("json") {
                serde_json::from_str::<serde_json::Value>(&content)
                    .map(|_| ())
                    .map_err(|e| format!("Invalid JSON: {}", e))
            } else {
                serde_yaml::from_str::<serde_yaml::Value>(&content)
                    .map(|_| ())
                    .map_err(|e| format!("Invalid YAML: {}", e))
            };

            match result {
                Ok(_) => println!("✅ Orchestration file is valid"),
                Err(e) => eprintln!("❌ {}", e),
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
".to_string()
            };

            std::fs::write(&output, template)?;
            println!("✅ Template saved to: {}", output.display());
        }
    }

    Ok(())
}
