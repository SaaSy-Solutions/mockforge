use clap::{Parser, Subcommand};
use mockforge_core::{apply_env_overrides, load_config_with_fallback, ServerConfig, init_global_logger};
use mockforge_data::{dataset::DatasetMetadata, schema::templates, DataConfig, DataGenerator};
use tracing::*;

#[derive(Parser, Debug)]
#[command(name = "mockforge")]
#[command(about = "MockForge - Advanced API Mocking Platform")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the mock servers (HTTP, WebSocket, gRPC)
    Serve {
        /// Path to OpenAPI spec (json or yaml)
        #[arg(long)]
        spec: Option<String>,
        /// Configuration file path
        #[arg(short, long)]
        config: Option<String>,
        #[arg(long, default_value_t = 3000)]
        http_port: u16,
        #[arg(long, default_value_t = 3001)]
        ws_port: u16,
        #[arg(long, default_value_t = 50051)]
        grpc_port: u16,
        /// Enable admin UI
        #[arg(long)]
        admin: bool,
        #[arg(long, default_value_t = 8080)]
        admin_port: u16,
        /// Force embedding Admin UI under HTTP server
        #[arg(long)]
        admin_embed: bool,
        /// Explicit mount path for embedded Admin UI (implies --admin-embed)
        #[arg(long)]
        admin_mount_path: Option<String>,
        /// Force standalone Admin UI on separate port (overrides embed)
        #[arg(long)]
        admin_standalone: bool,
        /// Disable Admin API endpoints (UI loads but API routes are absent)
        #[arg(long)]
        disable_admin_api: bool,
        /// Request validation mode: off, warn, enforce
        #[arg(long, value_parser = ["off","warn","enforce"], default_value = "enforce")]
        validation: String,
        /// Aggregate request validation errors into JSON array
        #[arg(long, default_value_t = true)]
        aggregate_errors: bool,
        /// Validate responses (warn-only)
        #[arg(long, default_value_t = false)]
        validate_responses: bool,
        /// Expand templating tokens in responses/examples
        #[arg(long, default_value_t = false)]
        response_template_expand: bool,
        /// Validation error HTTP status code (e.g., 400 or 422)
        #[arg(long)]
        validation_status: Option<u16>,
        /// Enable latency simulation
        #[arg(long)]
        latency_enabled: bool,
        /// Enable failure injection
        #[arg(long)]
        failures_enabled: bool,
    },
    /// Generate synthetic data
    Data {
        #[command(subcommand)]
        data_command: DataCommands,
    },
    /// Start admin UI server only
    Admin {
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
}

#[derive(Subcommand, Debug)]
enum DataCommands {
    /// Generate data using built-in templates
    Template {
        /// Template type (user, product, order)
        #[arg(value_enum)]
        template: TemplateType,
        /// Number of rows to generate
        #[arg(short, long, default_value_t = 100)]
        rows: usize,
        /// Output format (json, jsonl, csv, yaml)
        #[arg(short, long, default_value = "json")]
        format: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
        /// Enable RAG mode
        #[arg(long)]
        rag: bool,
    },
    /// Generate data from JSON schema
    Schema {
        /// Path to JSON schema file
        #[arg(short, long)]
        schema: String,
        /// Number of rows to generate
        #[arg(short, long, default_value_t = 100)]
        rows: usize,
        /// Output format
        #[arg(short, long, default_value = "json")]
        format: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Generate data from OpenAPI spec
    OpenApi {
        /// Path to OpenAPI spec file
        #[arg(short, long)]
        spec: String,
        /// Number of rows to generate
        #[arg(short, long, default_value_t = 100)]
        rows: usize,
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum TemplateType {
    User,
    Product,
    Order,
}

async fn handle_data_command(
    command: DataCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match command {
        DataCommands::Template {
            template,
            rows,
            format,
            output,
            rag,
        } => {
            let schema = match template {
                TemplateType::User => templates::user_schema(),
                TemplateType::Product => templates::product_schema(),
                TemplateType::Order => templates::order_schema(),
            };

            let config = DataConfig {
                rows,
                rag_enabled: rag,
                ..Default::default()
            };

            let mut generator = DataGenerator::new(schema, config)?;
            let result = generator.generate().await?;

            match format.as_str() {
                "json" => {
                    let json_output = result.to_json_string()?;
                    handle_output(&json_output, &output).await?;
                }
                "jsonl" => {
                    let jsonl_output = result.to_jsonl_string()?;
                    handle_output(&jsonl_output, &output).await?;
                }
                "csv" => {
                    // Convert GenerationResult to Dataset for CSV output
                    let metadata = DatasetMetadata::new(
                        "generated_dataset".to_string(),
                        "json_schema".to_string(),
                        &result,
                        DataConfig::default(),
                    );
                    let dataset = mockforge_data::Dataset::new(metadata, result.data);
                    let csv_output = dataset.to_csv_string()?;
                    handle_output(&csv_output, &output).await?;
                }
                _ => {
                    eprintln!("Unsupported format: {}. Supported: json, jsonl, csv", format);
                    std::process::exit(1);
                }
            }
        }
        DataCommands::Schema {
            schema: schema_path,
            rows,
            format,
            output,
        } => {
            let schema_content = tokio::fs::read_to_string(&schema_path).await?;
            let schema_value: serde_json::Value = serde_json::from_str(&schema_content)?;

            let result = mockforge_data::generate_from_json_schema(&schema_value, rows).await?;

            let output_content = match format.as_str() {
                "json" => result.to_json_string()?,
                "jsonl" => result.to_jsonl_string()?,
                "csv" => {
                    // Convert GenerationResult to Dataset for CSV output
                    let metadata = DatasetMetadata::new(
                        "generated_dataset".to_string(),
                        "json_schema".to_string(),
                        &result,
                        DataConfig::default(),
                    );
                    let dataset = mockforge_data::Dataset::new(metadata, result.data);
                    dataset.to_csv_string()?
                }
                _ => {
                    eprintln!("Unsupported format: {}. Supported: json, jsonl, csv", format);
                    std::process::exit(1);
                }
            };

            handle_output(&output_content, &output).await?;
        }
        DataCommands::OpenApi {
            spec: spec_path,
            rows,
            output,
        } => {
            let spec_content = tokio::fs::read_to_string(&spec_path).await?;
            let spec_value: serde_json::Value = serde_json::from_str(&spec_content)?;

            let result = mockforge_data::generate_from_openapi(&spec_value, rows).await?;

            let output_content = result.to_json_string()?;
            handle_output(&output_content, &output).await?;
        }
    }

    Ok(())
}

async fn handle_output(
    content: &str,
    output_path: &Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match output_path {
        Some(path) => {
            tokio::fs::write(path, content).await?;
            println!("Data written to {}", path);
        }
        None => {
            println!("{}", content);
        }
    }
    Ok(())
}

async fn start_servers_with_config(
    config: ServerConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "MockForge servers starting — http:{} ws:{} grpc:{} admin:{}",
        config.http.port, config.websocket.port, config.grpc.port, config.admin.port
    );

    let mut tasks = vec![];

    // Start HTTP server (optionally with embedded Admin UI)
    let http_config = config.http.clone();
    let admin_mount_path = config.admin.mount_path.clone();
    let admin_api_enabled = config.admin.api_enabled;
    let http_port_for_addr = http_config.port;
    let ws_port_for_addr = config.websocket.port;
    let grpc_port_for_addr = config.grpc.port;
    let http_latency_profile = if config.core.latency_enabled {
        Some(config.core.default_latency.clone())
    } else {
        None
    };
    let http_latency_injector = if config.core.latency_enabled {
        Some(mockforge_core::latency::LatencyInjector::new(
            config.core.default_latency.clone(),
            Default::default(),
        ))
    } else {
        None
    };

    // Create failure injector if failures are enabled
    let http_failure_injector = if config.core.failures_enabled {
        Some(mockforge_core::create_failure_injector(
            config.core.failures_enabled,
            config.core.failure_config.clone(),
        ))
    } else {
        None
    };

    let http_task = tokio::spawn(async move {
        if let Some(mount_path) = admin_mount_path {
            // Build base HTTP app and mount admin UI under the configured path
            let mut overrides = std::collections::HashMap::new();
            for (k, v) in &http_config.validation_overrides {
                let mode = match v.as_str() {
                    "off" => mockforge_core::openapi_routes::ValidationMode::Disabled,
                    "warn" => mockforge_core::openapi_routes::ValidationMode::Warn,
                    _ => mockforge_core::openapi_routes::ValidationMode::Enforce,
                };
                overrides.insert(k.clone(), mode);
            }
            let opts = Some(mockforge_core::openapi_routes::ValidationOptions {
                request_mode: match http_config.request_validation.as_str() {
                    "off" => mockforge_core::openapi_routes::ValidationMode::Disabled,
                    "warn" => mockforge_core::openapi_routes::ValidationMode::Warn,
                    _ => mockforge_core::openapi_routes::ValidationMode::Enforce,
                },
                aggregate_errors: http_config.aggregate_validation_errors,
                validate_responses: http_config.validate_responses,
                overrides,
                admin_skip_prefixes: if http_config.skip_admin_validation {
                    vec![mount_path.clone(), "/__mockforge".into()]
                } else {
                    vec![]
                },
                response_template_expand: http_config.response_template_expand,
                validation_status: http_config.validation_status,
            });
            // Expose admin mount prefix to HTTP builder (used to set env for skip prefixes as well)
            std::env::set_var("MOCKFORGE_ADMIN_MOUNT_PREFIX", &mount_path);

            let mut app = mockforge_http::build_router_with_injectors(http_config.openapi_spec, opts, http_latency_injector, http_failure_injector.clone()).await;

            // Compute server addresses for Admin state
            let http_addr: std::net::SocketAddr =
                format!("127.0.0.1:{}", http_port_for_addr).parse().unwrap();
            let ws_addr: std::net::SocketAddr =
                format!("127.0.0.1:{}", ws_port_for_addr).parse().unwrap();
            let grpc_addr: std::net::SocketAddr =
                format!("127.0.0.1:{}", grpc_port_for_addr).parse().unwrap();

            let admin_router = mockforge_ui::create_admin_router(
                Some(http_addr),
                Some(ws_addr),
                Some(grpc_addr),
                admin_api_enabled,
            );
            app = app.nest(mount_path.as_str(), admin_router);

            if let Err(e) = mockforge_http::serve_router(http_port_for_addr, app).await {
                error!("HTTP server error: {}", e);
            }
        } else if let Err(e) = {
            let mut overrides = std::collections::HashMap::new();
            for (k, v) in &http_config.validation_overrides {
                let mode = match v.as_str() {
                    "off" => mockforge_core::openapi_routes::ValidationMode::Disabled,
                    "warn" => mockforge_core::openapi_routes::ValidationMode::Warn,
                    _ => mockforge_core::openapi_routes::ValidationMode::Enforce,
                };
                overrides.insert(k.clone(), mode);
            }
            let opts = Some(mockforge_core::openapi_routes::ValidationOptions {
                request_mode: match http_config.request_validation.as_str() {
                    "off" => mockforge_core::openapi_routes::ValidationMode::Disabled,
                    "warn" => mockforge_core::openapi_routes::ValidationMode::Warn,
                    _ => mockforge_core::openapi_routes::ValidationMode::Enforce,
                },
                aggregate_errors: http_config.aggregate_validation_errors,
                validate_responses: http_config.validate_responses,
                overrides,
                admin_skip_prefixes: if http_config.skip_admin_validation {
                    vec!["/__mockforge".into()]
                } else {
                    vec![]
                },
                response_template_expand: http_config.response_template_expand,
                validation_status: http_config.validation_status,
            });
            mockforge_http::start_with_injectors(http_config.port, http_config.openapi_spec, opts, http_latency_profile, http_failure_injector).await
        } {
            error!("HTTP server error: {}", e);
        }
    });
    tasks.push(http_task);

    // Start WebSocket server
    let ws_config = config.websocket.clone();
    let ws_latency = if config.core.latency_enabled {
        Some(config.core.default_latency.clone())
    } else {
        None
    };
    let ws_task = tokio::spawn(async move {
        if let Err(e) = mockforge_ws::start_with_latency(ws_config.port, ws_latency).await {
            error!("WebSocket server error: {}", e);
        }
    });
    tasks.push(ws_task);

    // Start gRPC server
    let grpc_config = config.grpc.clone();
    let grpc_latency = if config.core.latency_enabled {
        Some(config.core.default_latency.clone())
    } else {
        None
    };
    let grpc_task = tokio::spawn(async move {
        // Create gRPC config with environment variable support
        let proto_dir = std::env::var("MOCKFORGE_PROTO_DIR")
            .unwrap_or_else(|_| "proto".to_string());
        let grpc_dynamic_config = mockforge_grpc::DynamicGrpcConfig {
            proto_dir,
            enable_reflection: false,
            excluded_services: Vec::new(),
        };
        
        if let Err(e) = mockforge_grpc::start_with_config(grpc_config.port, grpc_latency, grpc_dynamic_config).await {
            error!("gRPC server error: {}", e);
        }
    });
    tasks.push(grpc_task);

    // Start admin UI as standalone if enabled and not mounted under HTTP
    if config.admin.enabled && config.admin.mount_path.is_none() {
        let admin_config = config.admin.clone();
        let http_addr = format!("127.0.0.1:{}", config.http.port).parse().unwrap();
        let ws_addr = format!("127.0.0.1:{}", config.websocket.port).parse().unwrap();
        let grpc_addr = format!("127.0.0.1:{}", config.grpc.port).parse().unwrap();

        let admin_task = tokio::spawn(async move {
            let admin_addr = format!("127.0.0.1:{}", admin_config.port).parse().unwrap();
            if let Err(e) = mockforge_ui::start_admin_server(
                admin_addr,
                Some(http_addr),
                Some(ws_addr),
                Some(grpc_addr),
                admin_config.api_enabled,
            )
            .await
            {
                error!("Admin UI server error: {}", e);
            }
        });
        tasks.push(admin_task);
        info!("Admin UI available at http://127.0.0.1:{}", config.admin.port);
    }

    info!("MockForge servers running:");
    info!("  HTTP: http://127.0.0.1:{}", config.http.port);
    info!("  WebSocket: ws://127.0.0.1:{}", config.websocket.port);
    info!("  gRPC: localhost:{}", config.grpc.port);
    if config.admin.enabled {
        if let Some(ref mount) = config.admin.mount_path {
            info!("  Admin UI (embedded): http://127.0.0.1:{}{}/", config.http.port, mount);
        } else {
            info!("  Admin UI: http://127.0.0.1:{}/", config.admin.port);
        }
    }

    // Wait for all tasks
    for task in tasks {
        let _ = task.await;
    }

    Ok(())
}

async fn start_admin_only(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting MockForge Admin UI on port {}", port);
    let addr = format!("127.0.0.1:{}", port).parse().unwrap();

    mockforge_ui::start_admin_server(addr, None, None, None, true).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve {
            spec,
            config,
            http_port,
            ws_port,
            grpc_port,
            admin,
            admin_port,
            admin_embed,
            admin_mount_path,
            admin_standalone,
            disable_admin_api,
            validation,
            aggregate_errors,
            validate_responses,
            response_template_expand,
            validation_status,
            latency_enabled,
            failures_enabled,
        } => {
            // Initialize centralized request logger
            init_global_logger(2000); // Keep last 2000 requests in memory
            info!("Initialized centralized request logger");

            // Load configuration
            let mut server_config = if let Some(ref config_path) = config {
                load_config_with_fallback(&config_path).await
            } else {
                ServerConfig::default()
            };

            // Apply command line overrides
            if let Some(spec_path) = spec {
                server_config.http.openapi_spec = Some(spec_path);
            }
            server_config.http.port = http_port;
            server_config.websocket.port = ws_port;
            server_config.grpc.port = grpc_port;
            server_config.admin.enabled = admin;
            server_config.admin.port = admin_port;
            if disable_admin_api {
                server_config.admin.api_enabled = false;
            }
            if admin_embed || admin_mount_path.is_some() {
                server_config.admin.mount_path =
                    Some(admin_mount_path.unwrap_or_else(|| "/admin".to_string()));
            }
            if admin_standalone {
                server_config.admin.mount_path = None;
            }

            // Apply CLI latency overrides
            server_config.core.latency_enabled = latency_enabled;
            server_config.core.failures_enabled = failures_enabled;

            // Apply environment variable overrides
            let server_config = apply_env_overrides(server_config);

            // Export validation flags as env for HTTP layer or pass via options at build
            std::env::set_var("MOCKFORGE_REQUEST_VALIDATION", &validation);
            std::env::set_var(
                "MOCKFORGE_AGGREGATE_ERRORS",
                if aggregate_errors { "true" } else { "false" },
            );
            std::env::set_var(
                "MOCKFORGE_RESPONSE_VALIDATION",
                if validate_responses { "true" } else { "false" },
            );
            if response_template_expand {
                std::env::set_var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND", "true");
            }
            if let Some(code) = validation_status {
                std::env::set_var("MOCKFORGE_VALIDATION_STATUS", code.to_string());
            }
            if let Some(ref p) = server_config.http.openapi_spec {
                let _ = p;
            }
            // If config file path is provided, pass it to HTTP so Admin API can persist changes
            if let Some(ref config_path) = config {
                std::env::set_var("MOCKFORGE_CONFIG_PATH", config_path);
            }

            start_servers_with_config(server_config).await?;
        }
        Commands::Data { data_command } => {
            handle_data_command(data_command).await?;
        }
        Commands::Admin { port } => {
            start_admin_only(port).await?;
        }
    }

    Ok(())
}
