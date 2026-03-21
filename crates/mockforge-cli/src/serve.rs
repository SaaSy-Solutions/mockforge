use axum::serve as axum_serve;
use mockforge_chaos::api::create_chaos_api_router;
use mockforge_chaos::config::ChaosConfig;
use mockforge_core::encryption::init_key_store;
use mockforge_core::{apply_env_overrides, OpenApiSpec, ServerConfig};
use mockforge_observability::prometheus::{prometheus_router, MetricsRegistry};
use std::any::Any;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::TcpListener;

/// Arguments for building server configuration
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct ServeArgs {
    pub config_path: Option<PathBuf>,
    pub profile: Option<String>,
    pub http_port: Option<u16>,
    pub ws_port: Option<u16>,
    pub grpc_port: Option<u16>,
    pub tcp_port: Option<u16>,
    pub admin: bool,
    pub admin_port: Option<u16>,
    pub metrics: bool,
    pub metrics_port: Option<u16>,
    pub tracing: bool,
    pub tracing_service_name: String,
    pub tracing_environment: String,
    pub jaeger_endpoint: String,
    pub tracing_sampling_rate: f64,
    pub recorder: bool,
    pub recorder_db: String,
    pub recorder_no_api: bool,
    pub recorder_api_port: Option<u16>,
    pub recorder_max_requests: i64,
    pub recorder_retention_days: i64,
    pub chaos: bool,
    pub chaos_scenario: Option<String>,
    pub chaos_latency_ms: Option<u64>,
    pub chaos_latency_range: Option<String>,
    pub chaos_latency_probability: f64,
    pub chaos_http_errors: Option<String>,
    pub chaos_http_error_probability: f64,
    pub chaos_rate_limit: Option<u32>,
    pub chaos_bandwidth_limit: Option<u64>,
    pub chaos_packet_loss: Option<f64>,
    pub spec: Vec<PathBuf>,
    pub spec_dir: Option<PathBuf>,
    pub merge_conflicts: String,
    pub api_versioning: String,
    pub base_path: Option<String>,
    pub tls_enabled: bool,
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
    pub tls_ca: Option<PathBuf>,
    pub tls_min_version: String,
    pub mtls: String,
    pub ws_replay_file: Option<PathBuf>,
    pub graphql: Option<PathBuf>,
    pub graphql_port: Option<u16>,
    pub graphql_upstream: Option<String>,
    pub traffic_shaping: bool,
    pub bandwidth_limit: u64,
    pub burst_size: u64,
    pub ai_enabled: bool,
    pub rag_provider: Option<String>,
    pub rag_model: Option<String>,
    pub rag_api_key: Option<String>,
    pub network_profile: Option<String>,
    pub chaos_random: bool,
    /// Random chaos: error injection rate (0.0-1.0)
    pub chaos_random_error_rate: f64,
    /// Random chaos: delay injection rate (0.0-1.0)
    pub chaos_random_delay_rate: f64,
    /// Random chaos: minimum delay in milliseconds
    pub chaos_random_min_delay: u64,
    /// Random chaos: maximum delay in milliseconds
    pub chaos_random_max_delay: u64,
    pub reality_level: Option<u8>,
    pub dry_run: bool,
    pub progress: bool,
    pub verbose: bool,
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
    // Handle spec files - use first spec for backward compatibility with config
    // Full multi-spec handling will be done in HTTP server integration
    if let Some(spec_path) = serve_args.spec.first() {
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

    // Reality level configuration
    if let Some(level_value) = serve_args.reality_level {
        if let Some(level) = mockforge_core::RealityLevel::from_value(level_value) {
            config.reality.level = level;
            config.reality.enabled = true;
            println!("🎚️  Reality level set to {} ({})", level.value(), level.name());

            // Apply reality configuration to subsystems
            let reality_engine = mockforge_core::RealityEngine::with_level(level);
            reality_engine.apply_to_config(&mut config).await;
        } else {
            eprintln!(
                "⚠️  Invalid reality level: {}. Must be between 1 and 5. Using default.",
                level_value
            );
        }
    } else if config.reality.enabled {
        // Apply reality configuration from config file if enabled
        let level = config.reality.level;
        let reality_engine = mockforge_core::RealityEngine::with_level(level);
        reality_engine.apply_to_config(&mut config).await;
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
    spec_paths: &[PathBuf],
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

    // Validate spec files if provided
    for spec in spec_paths {
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
    use tracing_opentelemetry::OpenTelemetryLayer;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

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

    // Initialize the tracer (this sets up the global tracer provider)
    // The global tracer provider is what the OpenTelemetry layer will use
    let _tracer = init_tracer(tracing_config)?;

    // Create OpenTelemetry layer that uses the global tracer provider
    // The layer() function automatically uses the global tracer provider set by init_tracer
    let otel_layer = OpenTelemetryLayer::default();

    // Parse log level
    let log_level = logging_config.level.clone();
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // Build the subscriber with OpenTelemetry layer
    // We need to reinitialize the subscriber to add the OpenTelemetry layer
    let registry = tracing_subscriber::registry().with(env_filter).with(otel_layer);

    // Add console layer based on config
    if logging_config.json_format {
        use tracing_subscriber::fmt;
        registry.with(fmt::layer().json()).init();
    } else {
        use tracing_subscriber::fmt;
        registry.with(fmt::layer()).init();
    }

    tracing::info!("OpenTelemetry tracing initialized successfully with layer integration");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
/// Start the MockForge server with the given configuration
///
/// This function is public so it can be called from other commands like deploy
pub async fn handle_serve(
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
    spec: Vec<PathBuf>,
    spec_dir: Option<PathBuf>,
    merge_conflicts: String,
    api_versioning: String,
    base_path: Option<String>,
    tls_enabled: bool,
    tls_cert: Option<PathBuf>,
    tls_key: Option<PathBuf>,
    tls_ca: Option<PathBuf>,
    tls_min_version: String,
    mtls: String,
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
    _chaos_profile: Option<String>,
    ai_enabled: bool,
    reality_level: Option<u8>,
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

    // If no spec files provided, check MOCKFORGE_CONFIG env var (inline spec content)
    // or MOCKFORGE_OPENAPI_SPEC_URL env var (URL or local path to spec)
    let spec = if spec.is_empty() {
        if let Ok(config_json) = std::env::var("MOCKFORGE_CONFIG") {
            // MOCKFORGE_CONFIG contains the OpenAPI spec as JSON
            let spec_dir = std::path::Path::new("/tmp/mockforge-specs");
            let _ = tokio::fs::create_dir_all(spec_dir).await;
            let spec_path = spec_dir.join("spec.json");
            match tokio::fs::write(&spec_path, config_json.as_bytes()).await {
                Ok(()) => {
                    tracing::info!("Loaded spec from MOCKFORGE_CONFIG env var");
                    vec![spec_path]
                }
                Err(e) => {
                    tracing::error!("Failed to write spec from MOCKFORGE_CONFIG: {}", e);
                    vec![]
                }
            }
        } else if let Ok(spec_url) = std::env::var("MOCKFORGE_OPENAPI_SPEC_URL") {
            if spec_url.starts_with("http://") || spec_url.starts_with("https://") {
                tracing::info!("Downloading spec from URL: {}", spec_url);
                match reqwest::get(&spec_url).await {
                    Ok(response) if response.status().is_success() => {
                        let spec_dir = std::path::Path::new("/tmp/mockforge-specs");
                        let _ = tokio::fs::create_dir_all(spec_dir).await;
                        let spec_path = spec_dir.join("spec.json");
                        match response.bytes().await {
                            Ok(bytes) => match tokio::fs::write(&spec_path, &bytes).await {
                                Ok(()) => {
                                    tracing::info!("Spec downloaded to {}", spec_path.display());
                                    vec![spec_path]
                                }
                                Err(e) => {
                                    tracing::error!("Failed to write spec file: {}", e);
                                    vec![]
                                }
                            },
                            Err(e) => {
                                tracing::error!("Failed to read spec response: {}", e);
                                vec![]
                            }
                        }
                    }
                    Ok(response) => {
                        tracing::error!("Failed to download spec: HTTP {}", response.status());
                        vec![]
                    }
                    Err(e) => {
                        tracing::error!("Failed to download spec: {}", e);
                        vec![]
                    }
                }
            } else {
                vec![PathBuf::from(spec_url)]
            }
        } else {
            vec![]
        }
    } else {
        spec
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
        spec_dir,
        merge_conflicts,
        api_versioning,
        base_path,
        tls_enabled,
        tls_cert,
        tls_key,
        tls_ca,
        tls_min_version,
        mtls,
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
        reality_level: reality_level.or_else(|| {
            // Check environment variable as fallback
            std::env::var("MOCKFORGE_REALITY_LEVEL").ok().and_then(|v| v.parse::<u8>().ok())
        }),
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

    // Skip port check for dry-run mode since we're not actually binding
    if !serve_args.dry_run {
        if let Err(port_error) = ensure_ports_available(&final_ports) {
            return Err(port_error.into());
        }
    }

    if serve_args.dry_run {
        println!("✅ Configuration validation passed!");
        if serve_args.config_path.is_some() {
            println!("✅ Configuration file is valid");
        }
        if !serve_args.spec.is_empty() {
            println!("✅ OpenAPI spec file(s) are valid");
        }
        if serve_args.spec_dir.is_some() {
            println!("✅ OpenAPI spec directory is valid");
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
        use mockforge_chaos::core_network_profiles::NetworkProfileCatalog;
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
        use mockforge_chaos::core_chaos_utilities::ChaosConfig;

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

    // Initialize the global request logger early, BEFORE any server tasks are spawned.
    // This ensures HTTP request logs are captured from the very first request,
    // not just after the admin UI router happens to initialize.
    mockforge_core::init_global_logger(1000);

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

    // Initialize key store at startup (lightweight operation, keep synchronous)
    init_key_store();

    // Initialize request capture manager lazily (defer until first use)
    // This is lightweight but can be deferred to improve startup time
    tokio::spawn(async {
        use mockforge_core::request_capture::init_global_capture_manager;
        init_global_capture_manager(1000); // Keep last 1000 requests
        tracing::info!(
            "Request capture manager initialized for contract diff analysis (lazy-loaded)"
        );
    });

    // Initialize SIEM emitter lazily (defer until first use to improve startup time)
    let siem_config = config.security.monitoring.siem.clone();
    if siem_config.enabled {
        use mockforge_core::security::init_global_siem_emitter;
        // Spawn async task to initialize SIEM emitter in background (non-blocking)
        tokio::spawn(async move {
            if let Err(e) = init_global_siem_emitter(siem_config.clone()).await {
                tracing::warn!("Failed to initialize SIEM emitter: {}", e);
            } else {
                tracing::info!(
                    "SIEM emitter initialized with {} destinations (lazy-loaded)",
                    siem_config.destinations.len()
                );
            }
        });
    }

    // Initialize access review system if enabled
    let _access_review_scheduler_handle = if config.security.monitoring.access_review.enabled {
        use mockforge_core::security::{
            access_review::AccessReviewEngine,
            access_review_notifications::{AccessReviewNotificationService, NotificationConfig},
            access_review_scheduler::AccessReviewScheduler,
            access_review_service::AccessReviewService,
            api_tokens::InMemoryApiTokenStorage,
            justification_storage::InMemoryJustificationStorage,
            mfa_tracking::InMemoryMfaStorage,
        };
        use std::sync::Arc;
        use tokio::sync::RwLock;

        // Create storage backends (in-memory for now, can be replaced with database-backed implementations)
        let _token_storage: Arc<dyn mockforge_core::security::ApiTokenStorage> =
            Arc::new(InMemoryApiTokenStorage::new());
        let _mfa_storage: Arc<dyn mockforge_core::security::MfaStorage> =
            Arc::new(InMemoryMfaStorage::new());
        let _justification_storage: Arc<dyn mockforge_core::security::JustificationStorage> =
            Arc::new(InMemoryJustificationStorage::new());

        // Create a simple user data provider (placeholder - would use CollabUserDataProvider if collab is enabled)
        // For now, we'll create a minimal implementation that can be extended
        struct SimpleUserDataProvider;
        #[async_trait::async_trait]
        impl mockforge_core::security::UserDataProvider for SimpleUserDataProvider {
            async fn get_all_users(
                &self,
            ) -> Result<Vec<mockforge_core::security::UserAccessInfo>, mockforge_security_core::error::Error>
            {
                // Return empty list - would be populated from actual user management system
                Ok(Vec::new())
            }
            async fn get_privileged_users(
                &self,
            ) -> Result<Vec<mockforge_core::security::PrivilegedAccessInfo>, mockforge_security_core::error::Error>
            {
                Ok(Vec::new())
            }
            async fn get_api_tokens(
                &self,
            ) -> Result<Vec<mockforge_core::security::ApiTokenInfo>, mockforge_security_core::error::Error>
            {
                Ok(Vec::new())
            }
            async fn get_user(
                &self,
                _user_id: uuid::Uuid,
            ) -> Result<Option<mockforge_core::security::UserAccessInfo>, mockforge_security_core::error::Error>
            {
                Ok(None)
            }
            async fn get_last_login(
                &self,
                _user_id: uuid::Uuid,
            ) -> Result<Option<chrono::DateTime<chrono::Utc>>, mockforge_security_core::error::Error> {
                Ok(None)
            }
            async fn revoke_user_access(
                &self,
                _user_id: uuid::Uuid,
                _reason: String,
            ) -> Result<(), mockforge_security_core::error::Error> {
                Ok(())
            }
            async fn update_user_permissions(
                &self,
                _user_id: uuid::Uuid,
                _roles: Vec<String>,
                _permissions: Vec<String>,
            ) -> Result<(), mockforge_security_core::error::Error> {
                Ok(())
            }
        }

        let user_provider = SimpleUserDataProvider;

        // Create access review engine and service
        let review_config = config.security.monitoring.access_review.clone();
        let review_config_for_scheduler = review_config.clone();
        let engine = AccessReviewEngine::new(review_config.clone());
        let review_service = AccessReviewService::new(engine, Box::new(user_provider));
        let review_service_arc = Arc::new(RwLock::new(review_service));

        // Create notification service
        let notification_config = NotificationConfig {
            enabled: review_config.notifications.enabled,
            channels: review_config
                .notifications
                .channels
                .iter()
                .map(|c| match c.as_str() {
                    "email" => mockforge_core::security::access_review_notifications::NotificationChannel::Email,
                    "slack" => mockforge_core::security::access_review_notifications::NotificationChannel::Slack,
                    "webhook" => mockforge_core::security::access_review_notifications::NotificationChannel::Webhook,
                    _ => mockforge_core::security::access_review_notifications::NotificationChannel::InApp,
                })
                .collect(),
            recipients: review_config.notifications.recipients,
            channel_config: std::collections::HashMap::new(),
        };
        let notification_service =
            Arc::new(AccessReviewNotificationService::new(notification_config));

        // Initialize global access review service for HTTP API
        use mockforge_core::security::init_global_access_review_service;
        if let Err(e) = init_global_access_review_service(review_service_arc.clone()).await {
            tracing::warn!("Failed to initialize global access review service: {}", e);
        } else {
            tracing::info!("Global access review service initialized");
        }

        // Create and start scheduler
        let scheduler = AccessReviewScheduler::with_notifications(
            review_service_arc,
            review_config_for_scheduler,
            Some(notification_service),
        );
        let handle = scheduler.start();

        tracing::info!("Access review scheduler started");
        Some(handle)
    } else {
        None
    };

    // Initialize privileged access manager if enabled
    let _privileged_access_manager = if config.security.monitoring.privileged_access.require_mfa {
        use mockforge_core::security::{
            justification_storage::InMemoryJustificationStorage, mfa_tracking::InMemoryMfaStorage,
            privileged_access::PrivilegedAccessManager,
        };
        use std::sync::Arc;

        let privileged_config = config.security.monitoring.privileged_access.clone();
        let mfa_storage: Arc<dyn mockforge_core::security::MfaStorage> =
            Arc::new(InMemoryMfaStorage::new());
        let justification_storage: Arc<dyn mockforge_core::security::JustificationStorage> =
            Arc::new(InMemoryJustificationStorage::new());

        let manager = PrivilegedAccessManager::new(
            privileged_config,
            Some(mfa_storage),
            Some(justification_storage),
        );

        // Start session cleanup task
        let manager_for_cleanup = Arc::new(RwLock::new(manager));
        let cleanup_manager = manager_for_cleanup.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Every 5 minutes
            loop {
                interval.tick().await;
                if let Err(e) = cleanup_manager.write().await.cleanup_expired_sessions().await {
                    tracing::warn!("Failed to cleanup expired privileged sessions: {}", e);
                }
            }
        });

        // Initialize global privileged access manager for HTTP API
        use mockforge_core::security::init_global_privileged_access_manager;
        if let Err(e) = init_global_privileged_access_manager(manager_for_cleanup.clone()).await {
            tracing::warn!("Failed to initialize global privileged access manager: {}", e);
        } else {
            tracing::info!("Global privileged access manager initialized");
        }

        tracing::info!("Privileged access manager initialized");
        Some(manager_for_cleanup)
    } else {
        None
    };

    // Initialize change management engine if enabled
    let _change_management_engine = if config.security.monitoring.change_management.enabled {
        use mockforge_core::security::change_management::ChangeManagementEngine;
        use std::sync::Arc;

        let change_config = config.security.monitoring.change_management.clone();
        let engine = ChangeManagementEngine::new(change_config);
        let engine_arc = Arc::new(RwLock::new(engine));

        // Initialize global change management engine for HTTP API
        use mockforge_core::security::init_global_change_management_engine;
        if let Err(e) = init_global_change_management_engine(engine_arc.clone()).await {
            tracing::warn!("Failed to initialize global change management engine: {}", e);
        } else {
            tracing::info!("Global change management engine initialized");
        }

        tracing::info!("Change management engine initialized");
        Some(engine_arc)
    } else {
        None
    };

    // Initialize compliance dashboard engine if enabled
    let _compliance_dashboard_engine = if config.security.monitoring.compliance_dashboard.enabled {
        use mockforge_core::security::compliance_dashboard::ComplianceDashboardEngine;
        use std::sync::Arc;

        let dashboard_config = config.security.monitoring.compliance_dashboard.clone();
        let engine = ComplianceDashboardEngine::new(dashboard_config);
        let engine_arc = Arc::new(RwLock::new(engine));

        // Initialize global compliance dashboard engine for HTTP API
        use mockforge_core::security::init_global_compliance_dashboard_engine;
        if let Err(e) = init_global_compliance_dashboard_engine(engine_arc.clone()).await {
            tracing::warn!("Failed to initialize global compliance dashboard engine: {}", e);
        } else {
            tracing::info!("Global compliance dashboard engine initialized");
        }

        tracing::info!("Compliance dashboard engine initialized");
        Some(engine_arc)
    } else {
        None
    };

    // Initialize risk assessment engine if enabled
    let _risk_assessment_engine = if config.security.monitoring.risk_assessment.enabled {
        use mockforge_core::security::risk_assessment::RiskAssessmentEngine;
        use std::sync::Arc;

        let risk_config = config.security.monitoring.risk_assessment.clone();
        let engine = RiskAssessmentEngine::new(risk_config);
        let engine_arc = Arc::new(RwLock::new(engine));

        // Initialize global risk assessment engine for HTTP API
        use mockforge_core::security::init_global_risk_assessment_engine;
        if let Err(e) = init_global_risk_assessment_engine(engine_arc.clone()).await {
            tracing::warn!("Failed to initialize global risk assessment engine: {}", e);
        } else {
            tracing::info!("Global risk assessment engine initialized");
        }

        tracing::info!("Risk assessment engine initialized");
        Some(engine_arc)
    } else {
        None
    };

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
            // TLS defaults (not yet exposed in core config)
            tls_enabled: false,
            tls_port: 8883,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
            tls_client_auth: false,
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
            let mut interval = tokio::time::interval(Duration::from_secs(1));
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

    // Initialize MockAI in parallel with router building to improve startup time
    // This allows MockAI initialization to happen concurrently with HTTP router setup
    let mockai = if config.mockai.enabled {
        use mockforge_core::intelligent_behavior::MockAI;
        use std::sync::Arc;
        use tokio::sync::RwLock;
        use tracing::{debug, info};

        let behavior_config = config.mockai.intelligent_behavior.clone();
        let spec_path = config.http.openapi_spec.clone();

        // Create MockAI with a default instance first (fast), then upgrade in background
        // This allows the server to start immediately while MockAI initializes
        let mockai_arc = Arc::new(RwLock::new(MockAI::new(behavior_config.clone())));
        let mockai_for_upgrade = mockai_arc.clone();
        let behavior_config_for_upgrade = behavior_config.clone();

        // Spawn task to upgrade MockAI with OpenAPI spec if available (non-blocking)
        tokio::spawn(async move {
            if let Some(ref spec_path) = spec_path {
                match OpenApiSpec::from_file(spec_path).await {
                    Ok(openapi_spec) => {
                        match MockAI::from_openapi(&openapi_spec, behavior_config_for_upgrade).await
                        {
                            Ok(instance) => {
                                *mockai_for_upgrade.write().await = instance;
                                info!("✅ MockAI upgraded with OpenAPI spec (background initialization)");
                            }
                            Err(e) => {
                                debug!(
                                    "MockAI not available (no OpenAI API key configured): {}",
                                    e
                                );
                                // Keep default instance
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Failed to load OpenAPI spec for MockAI: {}", e);
                        // Keep default instance
                    }
                }
            }
        });

        Some(mockai_arc)
    } else {
        None
    };

    // Create ValidationOptions from config for template expansion
    use mockforge_core::openapi_routes::{ValidationMode, ValidationOptions};
    let request_mode = if let Some(ref validation) = config.http.validation {
        match validation.mode.as_str() {
            "off" | "disable" | "disabled" => ValidationMode::Disabled,
            "warn" | "warning" => ValidationMode::Warn,
            _ => ValidationMode::Enforce,
        }
    } else {
        ValidationMode::Enforce
    };

    let validation_options = ValidationOptions {
        request_mode,
        aggregate_errors: config.http.aggregate_validation_errors,
        validate_responses: config.http.validate_responses,
        overrides: std::collections::HashMap::new(),
        admin_skip_prefixes: vec!["/__mockforge".to_string(), "/health".to_string()],
        response_template_expand: config.http.response_template_expand,
        validation_status: config.http.validation_status,
    };

    // Process multiple specs if provided
    let final_spec_path = if !serve_args.spec.is_empty() || serve_args.spec_dir.is_some() {
        use mockforge_core::openapi::multi_spec::{
            group_specs_by_api_version, group_specs_by_openapi_version, load_specs_from_directory,
            load_specs_from_files, merge_specs, ConflictStrategy,
        };

        // Load specs
        let specs = if !serve_args.spec.is_empty() {
            load_specs_from_files(serve_args.spec.clone())
                .await
                .map_err(|e| format!("Failed to load spec files: {}", e))?
        } else if let Some(ref spec_dir) = serve_args.spec_dir {
            load_specs_from_directory(spec_dir)
                .await
                .map_err(|e| format!("Failed to load specs from directory: {}", e))?
        } else {
            Vec::new()
        };

        if specs.is_empty() {
            config.http.openapi_spec.clone()
        } else {
            // Determine conflict strategy
            let conflict_strategy = ConflictStrategy::from(serve_args.merge_conflicts.as_str());

            // Group by OpenAPI doc version first
            let openapi_groups = group_specs_by_openapi_version(specs);

            // Process each OpenAPI version group
            let mut merged_specs: Vec<(String, mockforge_core::openapi::spec::OpenApiSpec)> =
                Vec::new();
            for (_openapi_version, version_specs) in openapi_groups {
                // Apply API versioning grouping if enabled
                let api_versioning = serve_args.api_versioning.as_str();
                match api_versioning {
                    "info" | "path-prefix" => {
                        // Group by API version
                        let api_groups = group_specs_by_api_version(version_specs);
                        for (api_version, api_specs) in api_groups {
                            // Merge specs in this API version group
                            match merge_specs(api_specs, conflict_strategy) {
                                Ok(merged) => merged_specs.push((api_version, merged)),
                                Err(e) => {
                                    return Err(format!("Failed to merge specs: {}", e).into());
                                }
                            }
                        }
                    }
                    _ => {
                        // Merge all specs in this OpenAPI version group
                        match merge_specs(version_specs, conflict_strategy) {
                            Ok(merged) => merged_specs.push(("default".to_string(), merged)),
                            Err(e) => {
                                return Err(format!("Failed to merge specs: {}", e).into());
                            }
                        }
                    }
                }
            }

            // If we have multiple merged specs (different API versions), we need to handle them
            // For now, merge them all into one (or we could create separate routers with path prefixes)
            if merged_specs.len() == 1 {
                // Single merged spec - write to temp file
                let merged = &merged_specs[0].1;
                let raw_doc = merged
                    .raw_document
                    .as_ref()
                    .ok_or_else(|| "Merged spec has no raw document".to_string())?;
                let merged_json = serde_json::to_string_pretty(raw_doc)
                    .map_err(|e| format!("Failed to serialize merged spec: {}", e))?;

                // Use persistent temp file (won't be deleted automatically)
                let temp_dir = std::env::temp_dir();
                let temp_path =
                    temp_dir.join(format!("mockforge_merged_spec_{}.json", uuid::Uuid::new_v4()));
                std::fs::write(&temp_path, merged_json.as_bytes())
                    .map_err(|e| format!("Failed to write merged spec: {}", e))?;

                Some(temp_path.to_string_lossy().to_string())
            } else if merged_specs.is_empty() {
                config.http.openapi_spec.clone()
            } else if serve_args.api_versioning == "path-prefix" {
                let mut prefixed_specs: Vec<(PathBuf, mockforge_core::openapi::spec::OpenApiSpec)> =
                    Vec::new();

                for (api_version, spec) in merged_specs {
                    let version_suffix = api_version.trim().trim_start_matches('v');
                    let prefix = format!("/v{}", version_suffix);
                    let mut spec_json = spec.raw_document.clone().ok_or_else(|| {
                        format!("Merged spec for version '{}' has no raw document", api_version)
                    })?;

                    if let Some(paths_obj) =
                        spec_json.get_mut("paths").and_then(|p| p.as_object_mut())
                    {
                        let old_paths = std::mem::take(paths_obj);
                        let mut new_paths = serde_json::Map::new();
                        for (path, value) in old_paths {
                            let normalized_path = if path.starts_with('/') {
                                path
                            } else {
                                format!("/{}", path)
                            };
                            new_paths.insert(format!("{}{}", prefix, normalized_path), value);
                        }
                        *paths_obj = new_paths;
                    }

                    let prefixed_spec = mockforge_core::openapi::spec::OpenApiSpec::from_json(
                        spec_json,
                    )
                    .map_err(|e| {
                        format!(
                            "Failed to build prefixed spec for API version '{}': {}",
                            api_version, e
                        )
                    })?;

                    prefixed_specs
                        .push((PathBuf::from(format!("api-{}", api_version)), prefixed_spec));
                }

                match merge_specs(prefixed_specs, conflict_strategy) {
                    Ok(final_merged) => {
                        let raw_doc = final_merged.raw_document.as_ref().ok_or_else(|| {
                            "Final merged prefixed spec has no raw document".to_string()
                        })?;
                        let merged_json = serde_json::to_string_pretty(raw_doc).map_err(|e| {
                            format!("Failed to serialize prefixed merged spec: {}", e)
                        })?;

                        let temp_dir = std::env::temp_dir();
                        let temp_path = temp_dir.join(format!(
                            "mockforge_merged_prefixed_spec_{}.json",
                            uuid::Uuid::new_v4()
                        ));
                        std::fs::write(&temp_path, merged_json.as_bytes())
                            .map_err(|e| format!("Failed to write prefixed merged spec: {}", e))?;

                        Some(temp_path.to_string_lossy().to_string())
                    }
                    Err(e) => {
                        return Err(format!(
                            "Failed to merge path-prefixed API version specs: {}",
                            e
                        )
                        .into());
                    }
                }
            } else {
                // Multiple merged specs - for now, merge them all
                let all_specs: Vec<_> =
                    merged_specs.into_iter().map(|(_, s)| (PathBuf::from("merged"), s)).collect();
                match merge_specs(all_specs, conflict_strategy) {
                    Ok(final_merged) => {
                        let raw_doc = final_merged
                            .raw_document
                            .as_ref()
                            .ok_or_else(|| "Final merged spec has no raw document".to_string())?;
                        let merged_json = serde_json::to_string_pretty(raw_doc)
                            .map_err(|e| format!("Failed to serialize final merged spec: {}", e))?;

                        // Use persistent temp file (won't be deleted automatically)
                        let temp_dir = std::env::temp_dir();
                        let temp_path = temp_dir
                            .join(format!("mockforge_merged_spec_{}.json", uuid::Uuid::new_v4()));
                        std::fs::write(&temp_path, merged_json.as_bytes())
                            .map_err(|e| format!("Failed to write merged spec: {}", e))?;

                        Some(temp_path.to_string_lossy().to_string())
                    }
                    Err(e) => {
                        return Err(
                            format!("Failed to merge multiple API version specs: {}", e).into()
                        );
                    }
                }
            }
        }
    } else {
        config.http.openapi_spec.clone()
    };

    // Apply --base-path override: prefix all paths in the OpenAPI spec
    let final_spec_path = if let Some(ref bp) = serve_args.base_path {
        if let Some(ref spec) = final_spec_path {
            let spec_content = tokio::fs::read_to_string(spec)
                .await
                .map_err(|e| format!("Failed to read spec for --base-path injection: {}", e))?;
            let mut spec_json: serde_json::Value =
                if spec.ends_with(".yaml") || spec.ends_with(".yml") {
                    serde_yaml::from_str(&spec_content)
                        .map_err(|e| format!("Failed to parse spec YAML: {}", e))?
                } else {
                    serde_json::from_str(&spec_content)
                        .map_err(|e| format!("Failed to parse spec JSON: {}", e))?
                };

            // Normalize base path: ensure leading slash, strip trailing slash
            let base = {
                let mut p = bp.clone();
                if !p.starts_with('/') {
                    p.insert(0, '/');
                }
                p.trim_end_matches('/').to_string()
            };

            // Rewrite the paths object: prefix every path key with the base path
            if let Some(paths_obj) = spec_json.get("paths").and_then(|v| v.as_object()).cloned() {
                let mut new_paths = serde_json::Map::new();
                for (path_key, path_value) in paths_obj {
                    let prefixed = format!("{}{}", base, path_key);
                    new_paths.insert(prefixed, path_value);
                }
                spec_json["paths"] = serde_json::Value::Object(new_paths);
            }

            let modified_json = serde_json::to_string_pretty(&spec_json)
                .map_err(|e| format!("Failed to serialize spec with base path: {}", e))?;
            let temp_dir = std::env::temp_dir();
            let temp_path =
                temp_dir.join(format!("mockforge_basepath_spec_{}.json", uuid::Uuid::new_v4()));
            std::fs::write(&temp_path, modified_json.as_bytes())
                .map_err(|e| format!("Failed to write base-path spec: {}", e))?;

            tracing::info!("Applied --base-path '{}': prefixed all spec paths with '{}'", bp, base);
            Some(temp_path.to_string_lossy().to_string())
        } else {
            final_spec_path
        }
    } else {
        final_spec_path
    };

    // Configure traffic shaping for HTTP middleware when enabled
    let traffic_shaping_enabled = config.core.traffic_shaping_enabled;
    let traffic_shaper = if traffic_shaping_enabled {
        Some(mockforge_core::TrafficShaper::new(config.core.traffic_shaping.clone()))
    } else {
        None
    };

    // Use composable router builder
    #[allow(deprecated)]
    let mut http_app = mockforge_http::HttpRouterBuilder::new()
        .spec_path_opt(final_spec_path)
        .validation_options(validation_options)
        .with_multi_tenant_opt(multi_tenant_config)
        .route_configs(config.routes.clone())
        .cors_config_opt(config.http.cors.clone())
        .smtp_registry_opt(smtp_registry.as_ref().cloned())
        .mqtt_broker_opt(mqtt_broker_for_http)
        .with_traffic_shaping_opt(traffic_shaper, traffic_shaping_enabled)
        .health_manager(health_manager_for_router)
        .with_mockai_opt(mockai.clone())
        .with_deceptive_deploy(config.deceptive_deploy.clone())
        .build()
        .await;

    // Integrate chaos engineering API router
    // Convert from ServerConfig's ChaosEngConfig to mockforge-chaos's ChaosConfig
    let chaos_config = if let Some(ref chaos_eng_config) = config.observability.chaos {
        // Convert ChaosEngConfig to ChaosConfig
        let chaos_cfg = ChaosConfig {
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
    // Note: Temporarily passing None to avoid type mismatch between different versions of MockAI
    if mockai.is_some() && chaos_config.enabled {
        tracing::warn!(
            "Chaos API is running without MockAI-assisted fault generation due temporary cross-crate type compatibility limits"
        );
    }
    let (chaos_router, chaos_config_arc, latency_tracker, chaos_api_state) =
        create_chaos_api_router(chaos_config.clone(), None);
    http_app = http_app.merge(chaos_router);
    println!("✅ Chaos Engineering API available at /api/chaos/*");

    // Store chaos_api_state for passing to admin server (Phase 3)
    let chaos_api_state_for_admin = chaos_api_state.clone();

    // Integrate chaos middleware if chaos is enabled
    if chaos_config.enabled {
        use axum::middleware::from_fn;
        use mockforge_chaos::middleware::{chaos_middleware_with_state, ChaosMiddleware};
        use std::sync::{Arc, OnceLock};

        // Create chaos middleware with shared config for hot-reload support
        // Pass the shared config Arc from chaos_api_state
        let chaos_middleware_instance =
            Arc::new(ChaosMiddleware::new(chaos_config_arc.clone(), latency_tracker));

        // Initialize middleware injectors from actual config (async, but we spawn it)
        let middleware_init = chaos_middleware_instance.clone();
        tokio::spawn(async move {
            middleware_init.init_from_config().await;
        });

        // Store the middleware in a static OnceLock to avoid Send issues with closures
        // This middleware will record latencies for the latency graph
        static CHAOS_MIDDLEWARE: OnceLock<Arc<ChaosMiddleware>> = OnceLock::new();
        let _ = CHAOS_MIDDLEWARE.set(chaos_middleware_instance.clone());

        // Use a closure that accesses the static - this is Send-safe because
        // the static is accessed inside the async block, not captured in the closure.
        // The RNG used by the middleware is thread-local and created fresh each time,
        // so it's safe even though the compiler can't prove it statically.
        // SAFETY: rand::rng() uses thread-local storage, so each thread gets its own RNG instance.
        // The RNG is created fresh on each call and never sent across threads, so this is Send-safe.
        // We use a wrapper to assert Send safety for the future.
        struct SendSafeWrapper<F>(F);
        unsafe impl<F> Send for SendSafeWrapper<F> {}
        impl<F: std::future::Future<Output = axum::response::Response>> std::future::Future
            for SendSafeWrapper<F>
        {
            type Output = axum::response::Response;
            fn poll(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Self::Output> {
                unsafe { std::pin::Pin::new_unchecked(&mut self.get_unchecked_mut().0).poll(cx) }
            }
        }

        http_app =
            http_app.layer(from_fn(|req: axum::extract::Request, next: axum::middleware::Next| {
                SendSafeWrapper(async move {
                    if let Some(state) = CHAOS_MIDDLEWARE.get() {
                        chaos_middleware_with_state(state.clone(), req, next).await
                    } else {
                        // Chaos middleware not initialized, pass through
                        next.run(req).await
                    }
                })
            }));
        println!("✅ Chaos middleware integrated - latency recording enabled");
    }

    // Note: OData URI rewrite is applied at the service level in serve_router_with_tls()

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
    let metrics_registry = Arc::new(MetricsRegistry::new());

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

    // Build TLS config: CLI flags take precedence over config file
    let mut http_tls_config = config.http.tls.clone();

    // Override with CLI flags if provided
    if serve_args.tls_enabled {
        http_tls_config = Some(mockforge_core::config::HttpTlsConfig {
            enabled: true,
            cert_file: serve_args
                .tls_cert
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| {
                    http_tls_config.as_ref().map(|t| t.cert_file.clone()).unwrap_or_default()
                }),
            key_file: serve_args
                .tls_key
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| {
                    http_tls_config.as_ref().map(|t| t.key_file.clone()).unwrap_or_default()
                }),
            ca_file: serve_args
                .tls_ca
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .or_else(|| http_tls_config.as_ref().and_then(|t| t.ca_file.clone())),
            min_version: serve_args.tls_min_version.clone(),
            cipher_suites: http_tls_config
                .as_ref()
                .map(|t| t.cipher_suites.clone())
                .unwrap_or_default(),
            require_client_cert: serve_args.mtls == "required",
            mtls_mode: serve_args.mtls.clone(),
        });
    } else if let Some(ref mut tls) = http_tls_config {
        // Update mtls_mode from CLI if provided, even if TLS wasn't enabled via CLI
        if serve_args.mtls != "off" {
            tls.mtls_mode = serve_args.mtls.clone();
            if serve_args.mtls == "required" {
                tls.require_client_cert = true;
            }
        }
    }

    let http_tls_config_final = http_tls_config.clone();
    let http_shutdown = shutdown_token.clone();
    let http_handle = tokio::spawn(async move {
        if let Some(ref tls) = http_tls_config_final {
            if tls.enabled {
                println!("🔒 HTTPS server listening on https://localhost:{}", http_port);
            } else {
                println!("📡 HTTP server listening on http://localhost:{}", http_port);
            }
        } else {
            println!("📡 HTTP server listening on http://localhost:{}", http_port);
        }
        tokio::select! {
            result = mockforge_http::serve_router_with_tls(http_port, http_app, http_tls_config_final) => {
                result.map_err(|e| format!("HTTP server error: {}", e))
            }
            _ = http_shutdown.cancelled() => {
                Ok(())
            }
        }
    });

    // Start WebSocket server
    let ws_port = config.websocket.port;
    let ws_host = config.websocket.host.clone();
    let ws_shutdown = shutdown_token.clone();
    let ws_handle = tokio::spawn(async move {
        println!("🔌 WebSocket server listening on ws://{}:{}", ws_host, ws_port);
        tokio::select! {
            result = mockforge_ws::start_with_latency_and_host(ws_port, &ws_host, None) => {
                result.map_err(|e| format!("WebSocket server error: {}", e))
            }
            _ = ws_shutdown.cancelled() => {
                Ok(())
            }
        }
    });

    // Start gRPC server (only if enabled and port is not 0)
    let grpc_port = config.grpc.port;
    let grpc_enabled = config.grpc.enabled;
    let grpc_shutdown = shutdown_token.clone();
    let grpc_handle = if grpc_enabled && grpc_port != 0 {
        tokio::spawn(async move {
            println!("⚡ gRPC server listening on localhost:{}", grpc_port);
            tokio::select! {
                result = mockforge_grpc::start(grpc_port) => {
                    result.map_err(|e| format!("gRPC server error: {}", e))
                }
                _ = grpc_shutdown.cancelled() => {
                    Ok(())
                }
            }
        })
    } else {
        // gRPC disabled or port is 0, create a no-op handle
        tracing::debug!("gRPC server disabled (enabled: {}, port: {})", grpc_enabled, grpc_port);
        tokio::spawn(async move {
            // Wait for shutdown signal, then return Ok
            grpc_shutdown.cancelled().await;
            Ok(())
        })
    };

    #[cfg(feature = "smtp")]
    let smtp_handle = if let Some(ref smtp_registry) = smtp_registry {
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
    let mqtt_handle = if let Some(ref _mqtt_registry) = mqtt_registry {
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
            // TLS defaults (not yet exposed in core config)
            tls_enabled: false,
            tls_port: 8883,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
            tls_client_auth: false,
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
    let mqtt_handle: Option<tokio::task::JoinHandle<Result<(), String>>> = None;

    // Auto-start tunnel if deceptive deploy is enabled with auto_tunnel
    let _tunnel_handle = if config.deceptive_deploy.enabled && config.deceptive_deploy.auto_tunnel {
        use mockforge_tunnel::{TunnelConfig, TunnelManager, TunnelProvider};

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

                            // Update deployment metadata with tunnel URL
                            let metadata_path = std::path::Path::new(".mockforge/deployment.json");
                            if metadata_path.exists() {
                                if let Ok(metadata_content) = std::fs::read_to_string(metadata_path)
                                {
                                    if let Ok(mut metadata) =
                                        serde_json::from_str::<serde_json::Value>(&metadata_content)
                                    {
                                        metadata["tunnel_url"] =
                                            serde_json::Value::String(status.public_url.clone());
                                        if let Ok(updated_json) =
                                            serde_json::to_string_pretty(&metadata)
                                        {
                                            if let Err(e) =
                                                std::fs::write(metadata_path, updated_json)
                                            {
                                                tracing::warn!("Failed to update deployment metadata with tunnel URL: {}", e);
                                            } else {
                                                tracing::info!("Updated deployment metadata with tunnel URL: {}", status.public_url);
                                            }
                                        }
                                    }
                                }
                            }

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
    let kafka_handle = if config.kafka.enabled {
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
    let kafka_handle: Option<tokio::task::JoinHandle<Result<(), String>>> = None;

    // Start AMQP broker (if enabled)
    #[cfg(feature = "amqp")]
    let amqp_handle = if config.amqp.enabled {
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
    let amqp_handle: Option<tokio::task::JoinHandle<Result<(), String>>> = None;

    // Start TCP server (if enabled)
    #[cfg(feature = "tcp")]
    let tcp_handle = if config.tcp.enabled {
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
    let tcp_handle: Option<tokio::task::JoinHandle<Result<(), String>>> = None;

    // Create latency injector if latency is enabled (for hot-reload support)
    use mockforge_core::latency::{FaultConfig, LatencyInjector};
    use tokio::sync::RwLock;

    let latency_injector_for_admin = if config.core.latency_enabled {
        let latency_profile = config.core.default_latency.clone();
        // Create a basic fault config (can be enhanced later)
        let fault_config = FaultConfig::default();
        Some(Arc::new(RwLock::new(LatencyInjector::new(latency_profile, fault_config))))
    } else {
        None
    };

    // Create recorder instance if configured
    let recorder_for_admin: Option<Arc<mockforge_recorder::Recorder>> =
        if let Some(ref recorder_config) = config.observability.recorder {
            if recorder_config.enabled {
                match mockforge_recorder::RecorderDatabase::new(&recorder_config.database_path)
                    .await
                {
                    Ok(db) => {
                        tracing::info!(
                            "Admin: recorder initialized from {}",
                            recorder_config.database_path
                        );
                        Some(Arc::new(mockforge_recorder::Recorder::new(db)))
                    }
                    Err(e) => {
                        tracing::warn!("Admin: failed to initialize recorder database: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

    // Create VBR engine with in-memory backend (lightweight, no disk side-effects)
    let vbr_engine_for_admin: Option<Arc<mockforge_vbr::VbrEngine>> = {
        let vbr_config = mockforge_vbr::VbrConfig::new()
            .with_storage_backend(mockforge_vbr::StorageBackend::Memory);
        match mockforge_vbr::VbrEngine::new(vbr_config).await {
            Ok(engine) => {
                tracing::info!("Admin: VBR engine initialized (in-memory)");
                Some(Arc::new(engine))
            }
            Err(e) => {
                tracing::warn!("Admin: failed to initialize VBR engine: {}", e);
                None
            }
        }
    };

    // Create empty federation instance for admin dashboard
    let federation_for_admin: Option<Arc<mockforge_federation::Federation>> =
        Some(Arc::new(mockforge_federation::Federation::empty()));

    // Clone references for admin server
    let chaos_api_state_for_admin_clone = chaos_api_state_for_admin.clone();
    let latency_injector_for_admin_clone = latency_injector_for_admin.clone();
    let mockai_for_admin = mockai.clone();
    let continuum_config_for_admin = config.reality_continuum.clone();
    let time_travel_manager_for_admin = time_travel_manager.clone();

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
        // Clone subsystem references for admin server
        let chaos_state = chaos_api_state_for_admin_clone.clone();
        let latency_injector = latency_injector_for_admin_clone.clone();
        let mockai_ref = mockai_for_admin.clone();
        let continuum_config = continuum_config_for_admin.clone();
        let time_travel_manager_clone = time_travel_manager_for_admin.clone();
        let recorder_clone = recorder_for_admin.clone();
        let federation_clone = federation_for_admin.clone();
        let vbr_engine_clone = vbr_engine_for_admin.clone();
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

            // Initialize continuum engine from config
            let continuum_config = Some(continuum_config);
            let virtual_clock_for_continuum = Some(time_travel_manager_clone.clock());

            tokio::select! {
                result = mockforge_ui::start_admin_server(
                    addr,
                    http_addr,
                    ws_addr,
                    grpc_addr,
                    None, // graphql_server_addr
                    true, // api_enabled
                    prometheus_url,
                    Some(chaos_state),
                    latency_injector,
                    mockai_ref,
                    continuum_config,
                    virtual_clock_for_continuum,
                    recorder_clone,
                    federation_clone,
                    vbr_engine_clone,
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
                result = axum_serve(listener, app) => {
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
    tokio::time::sleep(Duration::from_millis(500)).await;
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
        // Monitor optional protocol handles — errors are no longer silent
        result = async {
            if let Some(handle) = smtp_handle {
                Some(("SMTP", handle.await))
            } else {
                std::future::pending().await
            }
        } => {
            match result {
                Some((name, Ok(Ok(())))) => { println!("📧 {} server stopped gracefully", name); None }
                Some((name, Ok(Err(e)))) => { eprintln!("❌ {} server error: {}", name, e); Some(e) }
                Some((name, Err(e))) => { let error = format!("{} server task panicked: {}", name, e); eprintln!("❌ {}", error); Some(error) }
                None => None
            }
        }
        result = async {
            if let Some(handle) = mqtt_handle {
                Some(("MQTT", handle.await))
            } else {
                std::future::pending().await
            }
        } => {
            match result {
                Some((name, Ok(Ok(())))) => { println!("📡 {} broker stopped gracefully", name); None }
                Some((name, Ok(Err(e)))) => { eprintln!("❌ {} broker error: {}", name, e); Some(e) }
                Some((name, Err(e))) => { let error = format!("{} broker task panicked: {}", name, e); eprintln!("❌ {}", error); Some(error) }
                None => None
            }
        }
        result = async {
            if let Some(handle) = kafka_handle {
                Some(("Kafka", handle.await))
            } else {
                std::future::pending().await
            }
        } => {
            match result {
                Some((name, Ok(Ok(())))) => { println!("📨 {} broker stopped gracefully", name); None }
                Some((name, Ok(Err(e)))) => { eprintln!("❌ {} broker error: {}", name, e); Some(e) }
                Some((name, Err(e))) => { let error = format!("{} broker task panicked: {}", name, e); eprintln!("❌ {}", error); Some(error) }
                None => None
            }
        }
        result = async {
            if let Some(handle) = amqp_handle {
                Some(("AMQP", handle.await))
            } else {
                std::future::pending().await
            }
        } => {
            match result {
                Some((name, Ok(Ok(())))) => { println!("🐰 {} broker stopped gracefully", name); None }
                Some((name, Ok(Err(e)))) => { eprintln!("❌ {} broker error: {}", name, e); Some(e) }
                Some((name, Err(e))) => { let error = format!("{} broker task panicked: {}", name, e); eprintln!("❌ {}", error); Some(error) }
                None => None
            }
        }
        result = async {
            if let Some(handle) = tcp_handle {
                Some(("TCP", handle.await))
            } else {
                std::future::pending().await
            }
        } => {
            match result {
                Some((name, Ok(Ok(())))) => { println!("🔌 {} server stopped gracefully", name); None }
                Some((name, Ok(Err(e)))) => { eprintln!("❌ {} server error: {}", name, e); Some(e) }
                Some((name, Err(e))) => { let error = format!("{} server task panicked: {}", name, e); eprintln!("❌ {}", error); Some(error) }
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
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Return error if any server failed
    if let Some(error) = result {
        Err(error.into())
    } else {
        Ok(())
    }
}
