//! Test server binary for Playwright/Vitest integration
//!
//! This binary starts a MockForge server for use in JavaScript/TypeScript tests.
//! It's designed to be spawned from Playwright's webServer or Vitest's globalSetup.

use mockforge_test::MockForgeServer;
use std::path::PathBuf;
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting MockForge test server...");

    // Create proto directory in /tmp to avoid gRPC errors
    let proto_dir = PathBuf::from("/tmp/proto");
    if !proto_dir.exists() {
        std::fs::create_dir_all(&proto_dir)?;
        info!("Created proto directory at /tmp/proto");
    }

    // Get the path to the OpenAPI spec file (in the test-integration directory)
    let spec_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-api.json");

    if !spec_file.exists() {
        eprintln!("Warning: OpenAPI spec not found at {:?}", spec_file);
    } else {
        info!("Using OpenAPI spec: {}", spec_file.display());
    }

    // Create workspace directory structure for different test scenarios
    let workspace_dir = PathBuf::from("/tmp/mockforge-test-workspaces");
    if workspace_dir.exists() {
        std::fs::remove_dir_all(&workspace_dir)?;
    }
    std::fs::create_dir_all(&workspace_dir)?;

    // Create workspace subdirectories for different scenarios
    let scenarios = vec![
        "default",
        "test-scenario",
        "user-authenticated",
        "user-unauthenticated",
        "server-errors",
        "slow-responses",
    ];
    for scenario in scenarios {
        let scenario_dir = workspace_dir.join(scenario);
        std::fs::create_dir_all(&scenario_dir)?;
        info!("Created workspace: {}", scenario);
    }

    info!("Workspace directory: {}", workspace_dir.display());

    // Create a mockforge.yaml config file in /tmp that references our OpenAPI spec
    // This is required because MockForge loads routes from the config, not just --spec flag
    let config_content = format!(
        r#"# Auto-generated MockForge config for test integration
http:
  port: 3000
  openapi_spec: "{}"
websocket:
  port: 0
grpc:
  port: 0
admin:
  enabled: false
metrics:
  enabled: false
"#,
        spec_file.display()
    );

    let config_path = PathBuf::from("/tmp/mockforge-test-config.yaml");
    std::fs::write(&config_path, config_content)?;
    info!("Created MockForge config: {}", config_path.display());

    // Determine binary path - use local build if not in PATH
    let binary_path = if which::which("mockforge").is_ok() {
        None // Use mockforge from PATH
    } else {
        // Use locally built binary
        let local_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("target/debug/mockforge");

        if local_binary.exists() {
            info!("Using local MockForge binary at: {}", local_binary.display());
            Some(local_binary)
        } else {
            None // Will fail with BinaryNotFound if not found
        }
    };

    // Start MockForge server on port 3000
    // Note: We use the config file to load the OpenAPI spec and configure the server
    let mut builder = MockForgeServer::builder()
        .http_port(3000) // Config also specifies 3000
        .health_timeout(Duration::from_secs(60)) // Increased timeout for config loading
        .working_dir("/tmp") // Use /tmp to avoid loading mockforge.yaml from project dir
        .extra_arg("--config")
        .extra_arg(config_path.to_str().unwrap())
        .extra_arg("--ws-port")
        .extra_arg("0")
        .extra_arg("--grpc-port")
        .extra_arg("0")
        .extra_arg("--metrics-port")
        .extra_arg("0")
        .extra_arg("--admin-port")
        .extra_arg("0");

    if let Some(path) = binary_path {
        builder = builder.binary_path(path);
    }

    let server = builder.build().await?;

    info!("✅ MockForge server started successfully!");
    info!("   HTTP Port: {}", server.http_port());
    info!("   Base URL: {}", server.base_url());
    info!("   PID: {}", server.pid());
    info!("");
    info!("Server is ready for testing. Press Ctrl+C to stop.");

    // Keep running until interrupted
    tokio::signal::ctrl_c().await?;

    info!("Shutting down MockForge server...");
    server.stop()?;
    info!("Server stopped.");

    Ok(())
}
