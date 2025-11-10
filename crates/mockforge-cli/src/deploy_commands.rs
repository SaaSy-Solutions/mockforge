//! Deploy commands for production-like mock API deployment

use mockforge_core::config::{DeceptiveDeployConfig, ServerConfig};
use mockforge_core::load_config;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use tracing::{info, warn};

#[derive(clap::Subcommand)]
pub enum DeploySubcommand {
    /// Deploy mock APIs with production-like configuration
    ///
    /// Examples:
    ///   mockforge deploy --config config.yaml
    ///   mockforge deploy --config config.yaml --spec api.yaml
    ///   mockforge deploy --config config.yaml --auto-tunnel
    ///   mockforge deploy --config config.yaml --start-server
    ///   mockforge deploy --config config.yaml --dry-run
    Deploy {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// OpenAPI spec file path
        #[arg(short, long)]
        spec: Option<PathBuf>,

        /// Auto-start tunnel for public URL
        #[arg(long)]
        auto_tunnel: bool,

        /// Custom domain for deployment
        #[arg(long)]
        custom_domain: Option<String>,

        /// Use production preset configuration
        #[arg(long)]
        production_preset: bool,

        /// Start the server after validation (default: false)
        #[arg(long)]
        start_server: bool,

        /// Validate configuration without starting server
        #[arg(long)]
        dry_run: bool,
    },

    /// Get deployment status
    Status {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Stop the deployed mock API
    Stop {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
}

pub async fn handle_deploy_command(cmd: DeploySubcommand) -> anyhow::Result<()> {
    match cmd {
        DeploySubcommand::Deploy {
            config,
            spec,
            auto_tunnel,
            custom_domain,
            production_preset,
            start_server,
            dry_run,
        } => {
            deploy_mock_api(
                config,
                spec,
                auto_tunnel,
                custom_domain,
                production_preset,
                start_server,
                dry_run,
            )
            .await
        }
        DeploySubcommand::Status { config } => get_deployment_status(config).await,
        DeploySubcommand::Stop { config } => stop_deployment(config).await,
    }
}

/// Deployment metadata stored in .mockforge/deployment.json
#[derive(Debug, Serialize, Deserialize)]
struct DeploymentMetadata {
    /// Process ID of the running server
    pid: Option<u32>,
    /// HTTP server port
    http_port: u16,
    /// Admin UI port (if enabled)
    admin_port: Option<u16>,
    /// Tunnel URL (if auto_tunnel is enabled)
    tunnel_url: Option<String>,
    /// Configuration file path
    config_path: String,
    /// OpenAPI spec path
    spec_path: Option<String>,
    /// Deployment timestamp
    deployed_at: String,
}

async fn deploy_mock_api(
    config_path: Option<PathBuf>,
    spec_path: Option<PathBuf>,
    auto_tunnel: bool,
    custom_domain: Option<String>,
    production_preset: bool,
    start_server: bool,
    dry_run: bool,
) -> anyhow::Result<()> {
    info!("🚀 Starting deceptive deploy...");

    // Clone config_path early since we'll need it later
    let config_path_clone = config_path.clone();

    // Load configuration
    let mut server_config = if let Some(config_path) = config_path {
        load_config(&config_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?
    } else {
        // Try to find default config file
        let default_paths = [
            PathBuf::from("mockforge.yaml"),
            PathBuf::from("config.yaml"),
            PathBuf::from("mockforge.yml"),
            PathBuf::from("config.yml"),
        ];

        let mut found_config = None;
        for path in &default_paths {
            if path.exists() {
                found_config = Some(path.clone());
                break;
            }
        }

        if let Some(path) = found_config {
            load_config(&path)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?
        } else {
            warn!("No config file found, using defaults");
            ServerConfig::default()
        }
    };

    // Enable deceptive deploy if not already enabled
    if !server_config.deceptive_deploy.enabled {
        if production_preset {
            server_config.deceptive_deploy = DeceptiveDeployConfig::production_preset();
            info!("Applied production preset configuration");
        } else {
            server_config.deceptive_deploy.enabled = true;
            info!("Enabled deceptive deploy mode");
        }
    }

    // Override auto_tunnel if specified
    if auto_tunnel {
        server_config.deceptive_deploy.auto_tunnel = true;
    }

    // Override custom domain if specified
    if let Some(domain) = custom_domain {
        server_config.deceptive_deploy.custom_domain = Some(domain);
    }

    // Override spec path if specified
    // Clone spec_path early since we'll need it later for handle_serve
    let spec_path_for_serve = spec_path.clone();
    if let Some(spec) = spec_path {
        server_config.http.openapi_spec = Some(spec.to_string_lossy().to_string());
    }

    // Validate that we have an OpenAPI spec
    if server_config.http.openapi_spec.is_none() {
        return Err(anyhow::anyhow!(
            "OpenAPI spec is required for deployment. Use --spec to specify a spec file."
        ));
    }

    // Generate deployment report
    info!("✅ Configuration loaded and validated");
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📋 Deceptive Deploy Configuration");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("   Enabled:         {}", server_config.deceptive_deploy.enabled);
    println!("   Auto tunnel:     {}", server_config.deceptive_deploy.auto_tunnel);
    if let Some(domain) = &server_config.deceptive_deploy.custom_domain {
        println!("   Custom domain:   {}", domain);
    }
    if !server_config.deceptive_deploy.headers.is_empty() {
        println!(
            "   Production headers: {} configured",
            server_config.deceptive_deploy.headers.len()
        );
        for (key, value) in &server_config.deceptive_deploy.headers {
            println!(
                "     - {}: {}",
                key,
                if value.len() > 50 {
                    format!("{}...", &value[..50])
                } else {
                    value.clone()
                }
            );
        }
    }
    if let Some(rate_limit) = &server_config.deceptive_deploy.rate_limit {
        println!(
            "   Rate limiting:   {} req/min (burst: {})",
            rate_limit.requests_per_minute, rate_limit.burst
        );
    }
    if let Some(cors) = &server_config.deceptive_deploy.cors {
        println!(
            "   CORS:            {} origins, {} methods",
            cors.allowed_origins.len(),
            cors.allowed_methods.len()
        );
    }
    if server_config.deceptive_deploy.oauth.is_some() {
        println!("   OAuth:           Configured");
    }
    println!("   HTTP Port:       {}", server_config.http.port);
    if server_config.admin.enabled {
        println!("   Admin Port:      {}", server_config.admin.port);
    }
    if let Some(spec) = &server_config.http.openapi_spec {
        println!("   OpenAPI Spec:    {}", spec);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // If dry-run, exit here
    if dry_run {
        info!("🔍 Dry-run mode: Configuration validated successfully");
        info!("💡 Remove --dry-run to actually deploy");
        return Ok(());
    }

    // If not starting server, just save config and exit
    if !start_server {
        info!("🎯 Configuration ready for deployment");
        info!("💡 Use 'mockforge deploy --start-server' to start the server");
        if server_config.deceptive_deploy.auto_tunnel {
            info!("🌐 Tunnel will be started automatically when server is ready");
        }
        return Ok(());
    }

    // Start the server
    info!("🚀 Starting server...");

    // Determine config file path for saving
    let effective_config_path = if let Some(ref path) = config_path_clone {
        path.clone()
    } else {
        let default_paths = [
            PathBuf::from("mockforge.yaml"),
            PathBuf::from("config.yaml"),
            PathBuf::from("mockforge.yml"),
            PathBuf::from("config.yml"),
        ];
        default_paths
            .iter()
            .find(|p| p.exists())
            .cloned()
            .unwrap_or_else(|| PathBuf::from("mockforge.yaml"))
    };

    // Save the config if we modified it (e.g., enabled deceptive deploy)
    // Check if config file exists and if our config differs
    let config_was_modified = if effective_config_path.exists() {
        // Load existing config to compare
        if let Ok(existing_config) = load_config(&effective_config_path).await {
            // Compare key fields since DeceptiveDeployConfig doesn't implement PartialEq
            existing_config.deceptive_deploy.enabled != server_config.deceptive_deploy.enabled
                || existing_config.deceptive_deploy.auto_tunnel
                    != server_config.deceptive_deploy.auto_tunnel
                || existing_config.deceptive_deploy.custom_domain
                    != server_config.deceptive_deploy.custom_domain
        } else {
            true // If we can't load it, assume modified
        }
    } else {
        true // New config file
    };

    if config_was_modified {
        // Save the updated config
        use serde_yaml;
        let config_yaml = serde_yaml::to_string(&server_config)?;
        fs::write(&effective_config_path, config_yaml)?;
        info!("💾 Saved updated configuration to {}", effective_config_path.display());
    }

    // Save deployment metadata
    let deployment_meta = DeploymentMetadata {
        pid: Some(process::id()),
        http_port: server_config.http.port,
        admin_port: if server_config.admin.enabled {
            Some(server_config.admin.port)
        } else {
            None
        },
        tunnel_url: None, // Will be updated when tunnel starts
        config_path: effective_config_path.to_string_lossy().to_string(),
        spec_path: server_config.http.openapi_spec.clone(),
        deployed_at: chrono::Utc::now().to_rfc3339(),
    };

    // Create .mockforge directory if it doesn't exist
    let mockforge_dir = Path::new(".mockforge");
    if !mockforge_dir.exists() {
        fs::create_dir_all(mockforge_dir)?;
    }

    // Save metadata
    let metadata_path = mockforge_dir.join("deployment.json");
    let metadata_json = serde_json::to_string_pretty(&deployment_meta)?;
    fs::write(&metadata_path, metadata_json)?;
    info!("💾 Saved deployment metadata to {}", metadata_path.display());

    // Start the server by calling handle_serve directly
    // This reuses the existing serve command logic and keeps everything in one process
    info!("🎯 Starting MockForge server...");
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🚀 Deceptive Deploy - Starting Server");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Import handle_serve from main module
    use crate::handle_serve;

    // Call handle_serve with minimal parameters, using config defaults
    // The config file we saved contains all the deceptive deploy settings
    handle_serve(
        Some(effective_config_path.clone()), // config_path
        None,                                // profile
        None,                                // http_port (use config)
        None,                                // ws_port (use config)
        None,                                // grpc_port (use config)
        None,                                // smtp_port
        None,                                // tcp_port
        server_config.admin.enabled,         // admin
        None,                                // admin_port (use config)
        false,                               // metrics
        None,                                // metrics_port
        false,                               // tracing
        "mockforge".to_string(),             // tracing_service_name
        "production".to_string(),            // tracing_environment
        String::new(),                       // jaeger_endpoint
        1.0,                                 // tracing_sampling_rate
        false,                               // recorder
        String::new(),                       // recorder_db
        false,                               // recorder_no_api
        None,                                // recorder_api_port
        0,                                   // recorder_max_requests
        0,                                   // recorder_retention_days
        false,                               // chaos
        None,                                // chaos_scenario
        None,                                // chaos_latency_ms
        None,                                // chaos_latency_range
        0.0,                                 // chaos_latency_probability
        None,                                // chaos_http_errors
        0.0,                                 // chaos_http_error_probability
        None,                                // chaos_rate_limit
        None,                                // chaos_bandwidth_limit
        None,                                // chaos_packet_loss
        spec_path_for_serve,                 // spec
        None,                                // ws_replay_file
        None,                                // graphql
        None,                                // graphql_port
        None,                                // graphql_upstream
        false,                               // traffic_shaping
        0,                                   // bandwidth_limit
        0,                                   // burst_size
        None,                                // network_profile
        false,                               // chaos_random
        0.0,                                 // chaos_random_error_rate
        0.0,                                 // chaos_random_delay_rate
        0,                                   // chaos_random_min_delay
        0,                                   // chaos_random_max_delay
        None,                                // chaos_profile
        false,                               // ai_enabled
        None,                                // reality_level
        None,                                // rag_provider
        None,                                // rag_model
        None,                                // rag_api_key
        false,                               // dry_run
        false,                               // progress
        false,                               // verbose
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to start server: {}", e))?;

    Ok(())
}

async fn get_deployment_status(config_path: Option<PathBuf>) -> anyhow::Result<()> {
    info!("📊 Getting deployment status...");

    // Check for deployment metadata
    let metadata_path = Path::new(".mockforge/deployment.json");
    if !metadata_path.exists() {
        warn!("❌ No deployment found");
        warn!("💡 Run 'mockforge deploy' to create a deployment");
        return Ok(());
    }

    // Load deployment metadata
    let metadata_json = fs::read_to_string(metadata_path)?;
    let metadata: DeploymentMetadata = serde_json::from_str(&metadata_json)?;

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Deployment Status");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Check if process is still running
    let is_running = if let Some(pid) = metadata.pid {
        // Check if process exists (simple check - on Unix systems)
        #[cfg(unix)]
        {
            use std::process::Command;
            Command::new("kill")
                .args(&["-0", &pid.to_string()])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }
        #[cfg(not(unix))]
        {
            // On non-Unix, we can't easily check if process exists
            // Assume it might be running
            true
        }
    } else {
        false
    };

    if is_running {
        println!("   Status:         ✅ Running");
    } else {
        println!("   Status:         ⏸️  Stopped");
    }

    if let Some(pid) = metadata.pid {
        println!("   Process ID:     {}", pid);
    }
    println!("   HTTP Port:      {}", metadata.http_port);
    if let Some(admin_port) = metadata.admin_port {
        println!("   Admin Port:     {}", admin_port);
    }
    if let Some(tunnel_url) = &metadata.tunnel_url {
        println!("   Tunnel URL:     {}", tunnel_url);
    }
    println!("   Config:          {}", metadata.config_path);
    if let Some(spec) = &metadata.spec_path {
        println!("   OpenAPI Spec:   {}", spec);
    }
    println!("   Deployed:       {}", metadata.deployed_at);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    Ok(())
}

async fn stop_deployment(_config_path: Option<PathBuf>) -> anyhow::Result<()> {
    info!("🛑 Stopping deployment...");

    // Check for deployment metadata
    let metadata_path = Path::new(".mockforge/deployment.json");
    if !metadata_path.exists() {
        warn!("❌ No deployment found to stop");
        return Ok(());
    }

    // Load deployment metadata
    let metadata_json = fs::read_to_string(metadata_path)?;
    let metadata: DeploymentMetadata = serde_json::from_str(&metadata_json)?;

    // Stop the process if it's running
    if let Some(pid) = metadata.pid {
        #[cfg(unix)]
        {
            use std::process::Command;
            info!("🛑 Stopping process {}...", pid);

            // Try graceful shutdown first (SIGTERM)
            let term_result = Command::new("kill").args(&["-TERM", &pid.to_string()]).output();

            match term_result {
                Ok(output) if output.status.success() => {
                    info!("✅ Sent SIGTERM to process {}", pid);
                    // Wait a bit for graceful shutdown
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                    // Check if process is still running
                    let still_running = Command::new("kill")
                        .args(&["-0", &pid.to_string()])
                        .output()
                        .map(|o| o.status.success())
                        .unwrap_or(false);

                    if still_running {
                        warn!("⚠️  Process still running, sending SIGKILL...");
                        let _ = Command::new("kill").args(&["-KILL", &pid.to_string()]).output();
                    }
                }
                Ok(_) => {
                    warn!("⚠️  Process {} may not be running", pid);
                }
                Err(e) => {
                    warn!("⚠️  Failed to stop process {}: {}", pid, e);
                }
            }
        }
        #[cfg(not(unix))]
        {
            warn!("⚠️  Process stopping not supported on this platform");
            warn!("💡 Please stop the server manually (Ctrl+C or kill process {})", pid);
        }
    }

    // Clean up deployment metadata
    if metadata_path.exists() {
        fs::remove_file(metadata_path)?;
        info!("🗑️  Removed deployment metadata");
    }

    info!("✅ Deployment stopped");

    Ok(())
}
