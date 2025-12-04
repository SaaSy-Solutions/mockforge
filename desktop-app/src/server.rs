//! Mock server management for desktop app
//!
//! This module handles starting and stopping the embedded MockForge server

use mockforge_core::ServerConfig;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Manages the embedded MockForge server instance
pub struct MockServerManager {
    /// Server task handles
    server_handles: Vec<JoinHandle<Result<(), String>>>,
    /// Server configuration
    config: Option<ServerConfig>,
    /// Server status
    is_running: bool,
    /// Shutdown token
    shutdown_token: Option<CancellationToken>,
}

impl MockServerManager {
    pub fn new() -> Self {
        Self {
            server_handles: Vec::new(),
            config: None,
            is_running: false,
            shutdown_token: None,
        }
    }

    /// Start the mock server with the given configuration
    pub async fn start(&mut self, config: ServerConfig) -> Result<(), String> {
        if self.is_running {
            return Err("Server is already running".to_string());
        }

        // Create shutdown token
        let shutdown_token = CancellationToken::new();
        let shutdown_token_clone = shutdown_token.clone();

        // Clone config for the async task
        let config_clone = config.clone();

        // Start servers in background tasks
        let handles = start_embedded_servers(config_clone, shutdown_token_clone).await?;

        self.server_handles = handles;
        self.config = Some(config);
        self.is_running = true;
        self.shutdown_token = Some(shutdown_token);

        Ok(())
    }

    /// Stop the mock server
    pub async fn stop(&mut self) -> Result<(), String> {
        if !self.is_running {
            return Err("Server is not running".to_string());
        }

        // Cancel shutdown token to signal all servers to stop
        if let Some(token) = self.shutdown_token.take() {
            token.cancel();
        }

        // Wait for all server tasks to complete
        for handle in self.server_handles.drain(..) {
            let _ = handle.await;
        }

        self.is_running = false;
        self.config = None;

        Ok(())
    }

    /// Check if server is running
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Get server configuration
    pub fn config(&self) -> Option<&ServerConfig> {
        self.config.as_ref()
    }
}

/// Start all embedded MockForge servers
async fn start_embedded_servers(
    config: ServerConfig,
    shutdown_token: CancellationToken,
) -> Result<Vec<JoinHandle<Result<(), String>>>, String> {
    use mockforge_grpc;
    use mockforge_http;
    use mockforge_ws;
    use std::sync::Arc;

    let mut handles = Vec::new();

    // Create health manager for health checks
    use mockforge_http::HealthManager;
    use std::time::Duration;
    let health_manager = Arc::new(HealthManager::with_init_timeout(Duration::from_secs(60)));

    // Build HTTP router using the same approach as CLI
    // Simplified version for desktop app - we don't need all the complex features
    // Note: Using None for route_configs, cors_config, and deceptive_deploy_config to avoid type mismatch
    // between path dependency (desktop-app) and published version (mockforge-http)
    // Routes and CORS can be configured via OpenAPI spec instead
    let http_app = mockforge_http::build_router_with_chains_and_multi_tenant(
        config.http.openapi_spec.clone(),
        None, // validation options
        None, // chain config
        None, // multi-tenant config
        None, // route_configs - skip custom routes to avoid type mismatch
        None, // cors_config - skip to avoid type mismatch, use defaults
        None,                                  // ai_generator
        None,                                  // smtp_registry
        None,                                  // mqtt_broker
        None,                                  // traffic_shaper
        false,                                 // traffic_shaping_enabled
        Some(health_manager.clone()),          // health_manager
        None,                                  // mockai
        None,                                  // deceptive_deploy_config - skip to avoid type mismatch
        None,                                  // proxy_config
    )
    .await;

    // Start HTTP server
    let http_port = config.http.port;
    // Skip TLS config to avoid type mismatch between path dependency and published version
    // TLS can be configured via environment or config file if needed
    let http_shutdown = shutdown_token.clone();
    let http_handle = tokio::spawn(async move {
        tracing::info!("HTTP server listening on http://localhost:{}", http_port);
        tokio::select! {
            // Pass None for TLS config to avoid type mismatch
            result = mockforge_http::serve_router_with_tls(http_port, http_app, None) => {
                result.map_err(|e| format!("HTTP server error: {}", e))
            }
            _ = http_shutdown.cancelled() => {
                tracing::info!("HTTP server shutting down...");
                Ok(())
            }
        }
    });
    handles.push(http_handle);

    // Start WebSocket server if enabled
    if config.websocket.port > 0 {
        let ws_port = config.websocket.port;
        let ws_shutdown = shutdown_token.clone();
        let ws_handle = tokio::spawn(async move {
            tracing::info!("WebSocket server listening on ws://localhost:{}", ws_port);
            tokio::select! {
                result = mockforge_ws::start_with_latency(ws_port, None) => {
                    result.map_err(|e| format!("WebSocket server error: {}", e))
                }
                _ = ws_shutdown.cancelled() => {
                    tracing::info!("WebSocket server shutting down...");
                    Ok(())
                }
            }
        });
        handles.push(ws_handle);
    }

    // Start gRPC server if enabled
    if config.grpc.port > 0 {
        let grpc_port = config.grpc.port;
        let grpc_shutdown = shutdown_token.clone();
        let grpc_handle = tokio::spawn(async move {
            tracing::info!("gRPC server listening on localhost:{}", grpc_port);
            tokio::select! {
                result = mockforge_grpc::start(grpc_port) => {
                    result.map_err(|e| format!("gRPC server error: {}", e))
                }
                _ = grpc_shutdown.cancelled() => {
                    tracing::info!("gRPC server shutting down...");
                    Ok(())
                }
            }
        });
        handles.push(grpc_handle);
    }

    Ok(handles)
}
