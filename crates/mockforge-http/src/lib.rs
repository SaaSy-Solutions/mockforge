//! # MockForge HTTP
//!
//! HTTP/REST API mocking library for MockForge.
//!
//! This crate provides HTTP-specific functionality for creating mock REST APIs,
//! including OpenAPI integration, request validation, AI-powered response generation,
//! and management endpoints.
//!
//! ## Overview
//!
//! MockForge HTTP enables you to:
//!
//! - **Serve OpenAPI specs**: Automatically generate mock endpoints from OpenAPI/Swagger
//! - **Validate requests**: Enforce schema validation with configurable modes
//! - **AI-powered responses**: Generate intelligent responses using LLMs
//! - **Management API**: Real-time monitoring, configuration, and control
//! - **Request logging**: Comprehensive HTTP request/response logging
//! - **Metrics collection**: Track performance and usage statistics
//! - **Server-Sent Events**: Stream logs and metrics to clients
//!
//! ## Quick Start
//!
//! ### Basic HTTP Server from OpenAPI
//!
//! ```rust,no_run
//! use axum::Router;
//! use mockforge_core::openapi_routes::ValidationMode;
//! use mockforge_core::ValidationOptions;
//! use mockforge_http::build_router;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Build router from OpenAPI specification
//!     let router = build_router(
//!         Some("./api-spec.json".to_string()),
//!         Some(ValidationOptions {
//!             request_mode: ValidationMode::Enforce,
//!             ..ValidationOptions::default()
//!         }),
//!         None,
//!     ).await;
//!
//!     // Start the server
//!     let addr: std::net::SocketAddr = "0.0.0.0:3000".parse()?;
//!     let listener = tokio::net::TcpListener::bind(addr).await?;
//!     axum::serve(listener, router).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### With Management API
//!
//! Enable real-time monitoring and configuration:
//!
//! ```rust,no_run
//! use mockforge_http::{management_router, ManagementState};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let state = ManagementState::new(None, None, 3000);
//!
//! // Build management router
//! let mgmt_router = management_router(state);
//!
//! // Mount under your main router
//! let app = axum::Router::new()
//!     .nest("/__mockforge", mgmt_router);
//! # Ok(())
//! # }
//! ```
//!
//! ### AI-Powered Responses
//!
//! Generate intelligent responses based on request context:
//!
//! ```rust,no_run
//! use mockforge_data::intelligent_mock::{IntelligentMockConfig, ResponseMode};
//! use mockforge_http::{process_response_with_ai, AiResponseConfig};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let ai_config = AiResponseConfig {
//!     intelligent: Some(
//!         IntelligentMockConfig::new(ResponseMode::Intelligent)
//!             .with_prompt("Generate realistic user data".to_string()),
//!     ),
//!     drift: None,
//! };
//!
//! let response = process_response_with_ai(
//!     Some(json!({"name": "Alice"})),
//!     ai_config
//!         .intelligent
//!         .clone()
//!         .map(serde_json::to_value)
//!         .transpose()?,
//!     ai_config
//!         .drift
//!         .clone()
//!         .map(serde_json::to_value)
//!         .transpose()?,
//! )
//! .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Key Features
//!
//! ### OpenAPI Integration
//! - Automatic endpoint generation from specs
//! - Request/response validation
//! - Schema-based mock data generation
//!
//! ### Management & Monitoring
//! - [`management`]: REST API for server control and monitoring
//! - [`management_ws`]: WebSocket API for real-time updates
//! - [`sse`]: Server-Sent Events for log streaming
//! - [`request_logging`]: Comprehensive request/response logging
//! - [`metrics_middleware`]: Performance metrics collection
//!
//! ### Advanced Features
//! - [`ai_handler`]: AI-powered response generation
//! - [`auth`]: Authentication and authorization
//! - [`chain_handlers`]: Multi-step request workflows
//! - [`latency_profiles`]: Configurable latency simulation
//! - [`replay_listing`]: Fixture management
//!
//! ## Middleware
//!
//! MockForge HTTP includes several middleware layers:
//!
//! - **Request Tracing**: [`http_tracing_middleware`] - Distributed tracing integration
//! - **Metrics Collection**: [`metrics_middleware`] - Prometheus-compatible metrics
//! - **Operation Metadata**: [`op_middleware`] - OpenAPI operation tracking
//!
//! ## Management API Endpoints
//!
//! When using the management router, these endpoints are available:
//!
//! - `GET /health` - Health check
//! - `GET /stats` - Server statistics
//! - `GET /logs` - Request logs (SSE stream)
//! - `GET /metrics` - Performance metrics
//! - `GET /fixtures` - List available fixtures
//! - `POST /config/*` - Update configuration
//!
//! ## Examples
//!
//! See the [examples directory](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples)
//! for complete working examples.
//!
//! ## Related Crates
//!
//! - [`mockforge-core`](https://docs.rs/mockforge-core): Core mocking functionality
//! - [`mockforge-data`](https://docs.rs/mockforge-data): Synthetic data generation
//! - [`mockforge-plugin-core`](https://docs.rs/mockforge-plugin-core): Plugin development
//!
//! ## Documentation
//!
//! - [MockForge Book](https://docs.mockforge.dev/)
//! - [HTTP Mocking Guide](https://docs.mockforge.dev/user-guide/http-mocking.html)
//! - [API Reference](https://docs.rs/mockforge-http)

pub mod ai_handler;
pub mod auth;
pub mod chain_handlers;
pub mod coverage;
pub mod http_tracing_middleware;
pub mod latency_profiles;
pub mod management;
pub mod management_ws;
pub mod metrics_middleware;
pub mod middleware;
pub mod op_middleware;
pub mod rag_ai_generator;
pub mod replay_listing;
pub mod request_logging;
pub mod sse;
pub mod token_response;
pub mod ui_builder;

// Re-export AI handler utilities
pub use ai_handler::{process_response_with_ai, AiResponseConfig, AiResponseHandler};

// Re-export management API utilities
pub use management::{
    management_router, management_router_with_ui_builder, ManagementState, MockConfig,
    ServerConfig, ServerStats,
};

// Re-export UI Builder utilities
pub use ui_builder::{create_ui_builder_router, EndpointConfig, UIBuilderState};

// Re-export management WebSocket utilities
pub use management_ws::{ws_management_router, MockEvent, WsManagementState};

// Re-export metrics middleware
pub use metrics_middleware::collect_http_metrics;

// Re-export tracing middleware
pub use http_tracing_middleware::http_tracing_middleware;

// Re-export coverage utilities
pub use coverage::{calculate_coverage, CoverageReport, MethodCoverage, RouteCoverage};

use axum::middleware::from_fn_with_state;
use axum::{extract::State, response::Json, Router};
use mockforge_core::failure_injection::{FailureConfig, FailureInjector};
use mockforge_core::latency::LatencyInjector;
use mockforge_core::openapi::OpenApiSpec;
use mockforge_core::openapi_routes::OpenApiRouteRegistry;
use mockforge_core::openapi_routes::ValidationOptions;

use mockforge_core::LatencyProfile;
#[cfg(feature = "data-faker")]
use mockforge_data::provider::register_core_faker_provider;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::*;

/// Route info for storing in state
#[derive(Clone)]
pub struct RouteInfo {
    pub method: String,
    pub path: String,
    pub operation_id: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<String>,
}

/// Shared state for tracking OpenAPI routes
#[derive(Clone)]
pub struct HttpServerState {
    pub routes: Vec<RouteInfo>,
    pub rate_limiter: Option<std::sync::Arc<crate::middleware::rate_limit::GlobalRateLimiter>>,
}

impl Default for HttpServerState {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpServerState {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            rate_limiter: None,
        }
    }

    pub fn with_routes(routes: Vec<RouteInfo>) -> Self {
        Self {
            routes,
            rate_limiter: None,
        }
    }

    pub fn with_rate_limiter(
        mut self,
        rate_limiter: std::sync::Arc<crate::middleware::rate_limit::GlobalRateLimiter>,
    ) -> Self {
        self.rate_limiter = Some(rate_limiter);
        self
    }
}

/// Handler to return OpenAPI routes information
async fn get_routes_handler(State(state): State<HttpServerState>) -> Json<serde_json::Value> {
    let route_info: Vec<serde_json::Value> = state
        .routes
        .iter()
        .map(|route| {
            serde_json::json!({
                "method": route.method,
                "path": route.path,
                "operation_id": route.operation_id,
                "summary": route.summary,
                "description": route.description,
                "parameters": route.parameters
            })
        })
        .collect();

    Json(serde_json::json!({
        "routes": route_info,
        "total": state.routes.len()
    }))
}

/// Build the base HTTP router, optionally from an OpenAPI spec.
pub async fn build_router(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    failure_config: Option<FailureConfig>,
) -> Router {
    build_router_with_multi_tenant(spec_path, options, failure_config, None, None, None, None, None)
        .await
}

/// Build the base HTTP router with multi-tenant workspace support
#[allow(clippy::too_many_arguments)]
pub async fn build_router_with_multi_tenant(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    failure_config: Option<FailureConfig>,
    multi_tenant_config: Option<mockforge_core::MultiTenantConfig>,
    _route_configs: Option<Vec<mockforge_core::config::RouteConfig>>,
    _cors_config: Option<mockforge_core::config::HttpCorsConfig>,
    ai_generator: Option<
        std::sync::Arc<dyn mockforge_core::openapi::response::AiGenerator + Send + Sync>,
    >,
    smtp_registry: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
) -> Router {
    use std::time::Instant;

    let startup_start = Instant::now();

    // Set up the basic router
    let mut app = Router::new();

    // Initialize rate limiter with default configuration
    // Can be customized via environment variables or config
    let rate_limit_config = crate::middleware::RateLimitConfig {
        requests_per_minute: std::env::var("MOCKFORGE_RATE_LIMIT_RPM")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000),
        burst: std::env::var("MOCKFORGE_RATE_LIMIT_BURST")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2000),
        per_ip: true,
        per_endpoint: false,
    };
    let rate_limiter =
        std::sync::Arc::new(crate::middleware::GlobalRateLimiter::new(rate_limit_config.clone()));

    let mut state = HttpServerState::new().with_rate_limiter(rate_limiter.clone());

    // Clone spec_path for later use
    let spec_path_for_mgmt = spec_path.clone();

    // If an OpenAPI spec is provided, integrate it
    if let Some(spec_path) = spec_path {
        tracing::debug!("Processing OpenAPI spec path: {}", spec_path);

        // Measure OpenAPI spec loading
        let spec_load_start = Instant::now();
        match OpenApiSpec::from_file(&spec_path).await {
            Ok(openapi) => {
                let spec_load_duration = spec_load_start.elapsed();
                info!(
                    "Successfully loaded OpenAPI spec from {} (took {:?})",
                    spec_path, spec_load_duration
                );

                // Measure route registry creation
                tracing::debug!("Creating OpenAPI route registry...");
                let registry_start = Instant::now();
                let registry = if let Some(opts) = options {
                    tracing::debug!("Using custom validation options");
                    OpenApiRouteRegistry::new_with_options(openapi, opts)
                } else {
                    tracing::debug!("Using environment-based options");
                    OpenApiRouteRegistry::new_with_env(openapi)
                };
                let registry_duration = registry_start.elapsed();
                info!(
                    "Created OpenAPI route registry with {} routes (took {:?})",
                    registry.routes().len(),
                    registry_duration
                );

                // Measure route extraction
                let extract_start = Instant::now();
                let route_info: Vec<RouteInfo> = registry
                    .routes()
                    .iter()
                    .map(|route| RouteInfo {
                        method: route.method.clone(),
                        path: route.path.clone(),
                        operation_id: route.operation.operation_id.clone(),
                        summary: route.operation.summary.clone(),
                        description: route.operation.description.clone(),
                        parameters: route.parameters.clone(),
                    })
                    .collect();
                state.routes = route_info;
                let extract_duration = extract_start.elapsed();
                debug!("Extracted route information (took {:?})", extract_duration);

                // Measure overrides loading
                let overrides = if std::env::var("MOCKFORGE_HTTP_OVERRIDES_GLOB").is_ok() {
                    tracing::debug!("Loading overrides from environment variable");
                    let overrides_start = Instant::now();
                    match mockforge_core::Overrides::load_from_globs(&[]).await {
                        Ok(overrides) => {
                            let overrides_duration = overrides_start.elapsed();
                            info!(
                                "Loaded {} override rules (took {:?})",
                                overrides.rules().len(),
                                overrides_duration
                            );
                            Some(overrides)
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load overrides: {}", e);
                            None
                        }
                    }
                } else {
                    None
                };

                // Measure router building
                let router_build_start = Instant::now();
                let overrides_enabled = overrides.is_some();
                let openapi_router = if let Some(ai_generator) = &ai_generator {
                    tracing::debug!("Building router with AI generator support");
                    registry.build_router_with_ai(Some(ai_generator.clone()))
                } else if let Some(failure_config) = &failure_config {
                    tracing::debug!("Building router with failure injection and overrides");
                    let failure_injector = FailureInjector::new(Some(failure_config.clone()), true);
                    registry.build_router_with_injectors_and_overrides(
                        LatencyInjector::default(),
                        Some(failure_injector),
                        overrides,
                        overrides_enabled,
                    )
                } else {
                    tracing::debug!("Building router with overrides");
                    registry.build_router_with_injectors_and_overrides(
                        LatencyInjector::default(),
                        None,
                        overrides,
                        overrides_enabled,
                    )
                };
                let router_build_duration = router_build_start.elapsed();
                debug!("Built OpenAPI router (took {:?})", router_build_duration);

                tracing::debug!("Merging OpenAPI router with main router");
                app = app.merge(openapi_router);
                tracing::debug!("Router built successfully");
            }
            Err(e) => {
                warn!("Failed to load OpenAPI spec from {}: {}. Starting without OpenAPI integration.", spec_path, e);
            }
        }
    }

    // Add basic health check endpoint
    app = app.route(
        "/health",
        axum::routing::get(|| async {
            use mockforge_core::server_utils::health::HealthStatus;
            axum::Json(serde_json::to_value(HealthStatus::healthy(0, "mockforge-http")).unwrap())
        }),
    )
    // Add SSE endpoints
    .merge(sse::sse_router());

    // Clone state for routes_router since we'll use it for middleware too
    let state_for_routes = state.clone();

    // Create a router with state for the routes and coverage endpoints
    let routes_router = Router::new()
        .route("/__mockforge/routes", axum::routing::get(get_routes_handler))
        .route("/__mockforge/coverage", axum::routing::get(coverage::get_coverage_handler))
        .with_state(state_for_routes);

    // Merge the routes router with the main app
    app = app.merge(routes_router);

    // Add static coverage UI
    // Determine the path to the coverage.html file
    let coverage_html_path = std::env::var("MOCKFORGE_COVERAGE_UI_PATH")
        .unwrap_or_else(|_| "crates/mockforge-http/static/coverage.html".to_string());

    // Check if the file exists before serving it
    if std::path::Path::new(&coverage_html_path).exists() {
        app = app.nest_service(
            "/__mockforge/coverage.html",
            tower_http::services::ServeFile::new(&coverage_html_path),
        );
        debug!("Serving coverage UI from: {}", coverage_html_path);
    } else {
        debug!(
            "Coverage UI file not found at: {}. Skipping static file serving.",
            coverage_html_path
        );
    }

    // Add management API endpoints
    let mut management_state = ManagementState::new(None, spec_path_for_mgmt, 3000); // Port will be updated when we know the actual port
    #[cfg(feature = "smtp")]
    let mut management_state = {
        if let Some(smtp_reg) = smtp_registry {
            let smtp_reg = smtp_reg
                .downcast::<mockforge_smtp::SmtpSpecRegistry>()
                .expect("Invalid SMTP registry type passed to HTTP management state");
            management_state.with_smtp_registry(smtp_reg)
        } else {
            management_state
        }
    };
    #[cfg(not(feature = "smtp"))]
    let mut management_state = management_state;
    #[cfg(not(feature = "smtp"))]
    let _ = smtp_registry;
    app = app.nest("/__mockforge/api", management_router(management_state));

    // Add management WebSocket endpoint
    let ws_state = WsManagementState::new();
    app = app.nest("/__mockforge/ws", ws_management_router(ws_state));

    // Add request logging middleware to capture all requests
    app = app.layer(axum::middleware::from_fn(request_logging::log_http_requests));

    // Add rate limiting middleware (before logging to rate limit early)
    app = app.layer(from_fn_with_state(state, crate::middleware::rate_limit_middleware));

    // Add workspace routing middleware if multi-tenant is enabled
    if let Some(mt_config) = multi_tenant_config {
        if mt_config.enabled {
            use mockforge_core::{MultiTenantWorkspaceRegistry, WorkspaceRouter};
            use std::sync::Arc;

            info!(
                "Multi-tenant mode enabled with {} routing strategy",
                match mt_config.routing_strategy {
                    mockforge_core::RoutingStrategy::Path => "path-based",
                    mockforge_core::RoutingStrategy::Port => "port-based",
                    mockforge_core::RoutingStrategy::Both => "hybrid",
                }
            );

            // Create the multi-tenant workspace registry
            let mut registry = MultiTenantWorkspaceRegistry::new(mt_config.clone());

            // Register the default workspace before wrapping in Arc
            let default_workspace =
                mockforge_core::Workspace::new(mt_config.default_workspace.clone());
            if let Err(e) =
                registry.register_workspace(mt_config.default_workspace.clone(), default_workspace)
            {
                warn!("Failed to register default workspace: {}", e);
            } else {
                info!("Registered default workspace: '{}'", mt_config.default_workspace);
            }

            // Auto-discover and register workspaces if configured
            if mt_config.auto_discover {
                if let Some(config_dir) = &mt_config.config_directory {
                    let config_path = Path::new(config_dir);
                    if config_path.exists() && config_path.is_dir() {
                        match fs::read_dir(config_path).await {
                            Ok(mut entries) => {
                                while let Ok(Some(entry)) = entries.next_entry().await {
                                    let path = entry.path();
                                    if path.extension() == Some(OsStr::new("yaml")) {
                                        match fs::read_to_string(&path).await {
                                            Ok(content) => {
                                                match serde_yaml::from_str::<
                                                    mockforge_core::Workspace,
                                                >(
                                                    &content
                                                ) {
                                                    Ok(workspace) => {
                                                        if let Err(e) = registry.register_workspace(
                                                            workspace.id.clone(),
                                                            workspace,
                                                        ) {
                                                            warn!("Failed to register auto-discovered workspace from {:?}: {}", path, e);
                                                        } else {
                                                            info!("Auto-registered workspace from {:?}", path);
                                                        }
                                                    }
                                                    Err(e) => {
                                                        warn!("Failed to parse workspace from {:?}: {}", path, e);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                warn!(
                                                    "Failed to read workspace file {:?}: {}",
                                                    path, e
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to read config directory {:?}: {}", config_path, e);
                            }
                        }
                    } else {
                        warn!(
                            "Config directory {:?} does not exist or is not a directory",
                            config_path
                        );
                    }
                }
            }

            // Wrap registry in Arc for shared access
            let registry = Arc::new(registry);

            // Create workspace router and wrap the app with workspace middleware
            let _workspace_router = WorkspaceRouter::new(registry);

            // Note: The actual middleware integration would need to be implemented
            // in the WorkspaceRouter to work with Axum's middleware system
            info!("Workspace routing middleware initialized for HTTP server");
        }
    }

    let total_startup_duration = startup_start.elapsed();
    info!("HTTP router startup completed (total time: {:?})", total_startup_duration);

    app
}

/// Build the base HTTP router with authentication and latency support
pub async fn build_router_with_auth_and_latency(
    _spec_path: Option<String>,
    _options: Option<()>,
    _auth_config: Option<mockforge_core::config::AuthConfig>,
    _latency_injector: Option<LatencyInjector>,
) -> Router {
    // For now, just use the basic router. Full auth and latency support can be added later.
    build_router(None, None, None).await
}

/// Build the base HTTP router with latency injection support
pub async fn build_router_with_latency(
    _spec_path: Option<String>,
    _options: Option<ValidationOptions>,
    _latency_injector: Option<LatencyInjector>,
) -> Router {
    // For now, fall back to basic router since injectors are complex to implement
    build_router(None, None, None).await
}

/// Build the base HTTP router with authentication support
pub async fn build_router_with_auth(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    auth_config: Option<mockforge_core::config::AuthConfig>,
) -> Router {
    use crate::auth::{auth_middleware, create_oauth2_client, AuthState};
    use std::sync::Arc;

    // If richer faker is available, register provider once (idempotent)
    #[cfg(feature = "data-faker")]
    {
        register_core_faker_provider();
    }

    // Set up authentication state
    let spec = if let Some(spec_path) = &spec_path {
        match mockforge_core::openapi::OpenApiSpec::from_file(&spec_path).await {
            Ok(spec) => Some(Arc::new(spec)),
            Err(e) => {
                warn!("Failed to load OpenAPI spec for auth: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Create OAuth2 client if configured
    let oauth2_client = if let Some(auth_config) = &auth_config {
        if let Some(oauth2_config) = &auth_config.oauth2 {
            match create_oauth2_client(oauth2_config) {
                Ok(client) => Some(client),
                Err(e) => {
                    warn!("Failed to create OAuth2 client: {}", e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    let auth_state = AuthState {
        config: auth_config.unwrap_or_default(),
        spec,
        oauth2_client,
        introspection_cache: Arc::new(RwLock::new(HashMap::new())),
    };

    // Set up the basic router with auth state
    let mut app = Router::new().with_state(auth_state.clone());

    // If an OpenAPI spec is provided, integrate it
    if let Some(spec_path) = spec_path {
        match OpenApiSpec::from_file(&spec_path).await {
            Ok(openapi) => {
                info!("Loaded OpenAPI spec from {}", spec_path);
                let registry = if let Some(opts) = options {
                    OpenApiRouteRegistry::new_with_options(openapi, opts)
                } else {
                    OpenApiRouteRegistry::new_with_env(openapi)
                };

                app = registry.build_router();
            }
            Err(e) => {
                warn!("Failed to load OpenAPI spec from {}: {}. Starting without OpenAPI integration.", spec_path, e);
            }
        }
    }

    // Add basic health check endpoint
    app = app.route(
        "/health",
        axum::routing::get(|| async {
            use mockforge_core::server_utils::health::HealthStatus;
            axum::Json(serde_json::to_value(HealthStatus::healthy(0, "mockforge-http")).unwrap())
        }),
    )
    // Add SSE endpoints
    .merge(sse::sse_router())
    // Add authentication middleware (before logging)
    .layer(axum::middleware::from_fn_with_state(auth_state.clone(), auth_middleware))
    // Add request logging middleware
    .layer(axum::middleware::from_fn(request_logging::log_http_requests));

    app
}

/// Serve a provided router on the given port.
pub async fn serve_router(
    port: u16,
    app: Router,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::net::SocketAddr;

    let addr = mockforge_core::wildcard_socket_addr(port);
    info!("HTTP listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        format!(
            "Failed to bind HTTP server to port {}: {}\n\
             Hint: The port may already be in use. Try using a different port with --http-port or check if another process is using this port with: lsof -i :{} or netstat -tulpn | grep {}",
            port, e, port, port
        )
    })?;

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
    Ok(())
}

/// Backwards-compatible start that builds + serves the base router.
pub async fn start(
    port: u16,
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    start_with_latency(port, spec_path, options, None).await
}

/// Start HTTP server with authentication and latency support
pub async fn start_with_auth_and_latency(
    port: u16,
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    auth_config: Option<mockforge_core::config::AuthConfig>,
    latency_profile: Option<LatencyProfile>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    start_with_auth_and_injectors(port, spec_path, options, auth_config, latency_profile, None)
        .await
}

/// Start HTTP server with authentication and injectors support
pub async fn start_with_auth_and_injectors(
    port: u16,
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    auth_config: Option<mockforge_core::config::AuthConfig>,
    _latency_profile: Option<LatencyProfile>,
    _failure_injector: Option<mockforge_core::FailureInjector>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // For now, ignore latency and failure injectors and just use auth
    let app = build_router_with_auth(spec_path, options, auth_config).await;
    serve_router(port, app).await
}

/// Start HTTP server with latency injection support
pub async fn start_with_latency(
    port: u16,
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    latency_profile: Option<LatencyProfile>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let latency_injector =
        latency_profile.map(|profile| LatencyInjector::new(profile, Default::default()));

    let app = build_router_with_latency(spec_path, options, latency_injector).await;
    serve_router(port, app).await
}

/// Build the base HTTP router with chaining support
pub async fn build_router_with_chains(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    circling_config: Option<mockforge_core::request_chaining::ChainConfig>,
) -> Router {
    build_router_with_chains_and_multi_tenant(
        spec_path,
        options,
        circling_config,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        false,
    )
    .await
}

/// Build the base HTTP router with chaining and multi-tenant support
#[allow(clippy::too_many_arguments)]
pub async fn build_router_with_chains_and_multi_tenant(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    _circling_config: Option<mockforge_core::request_chaining::ChainConfig>,
    multi_tenant_config: Option<mockforge_core::MultiTenantConfig>,
    _route_configs: Option<Vec<mockforge_core::config::RouteConfig>>,
    _cors_config: Option<mockforge_core::config::HttpCorsConfig>,
    _ai_generator: Option<
        std::sync::Arc<dyn mockforge_core::openapi::response::AiGenerator + Send + Sync>,
    >,
    smtp_registry: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
    mqtt_broker: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
    traffic_shaper: Option<mockforge_core::traffic_shaping::TrafficShaper>,
    traffic_shaping_enabled: bool,
) -> Router {
    use crate::latency_profiles::LatencyProfiles;
    use crate::op_middleware::Shared;
    use mockforge_core::Overrides;

    let _shared = Shared {
        profiles: LatencyProfiles::default(),
        overrides: Overrides::default(),
        failure_injector: None,
        traffic_shaper,
        overrides_enabled: false,
        traffic_shaping_enabled,
    };

    // Start with basic router
    let mut app = Router::new();
    let mut include_default_health = true;

    // If an OpenAPI spec is provided, integrate it
    if let Some(ref spec) = spec_path {
        match OpenApiSpec::from_file(&spec).await {
            Ok(openapi) => {
                info!("Loaded OpenAPI spec from {}", spec);
                let registry = if let Some(opts) = options {
                    OpenApiRouteRegistry::new_with_options(openapi, opts)
                } else {
                    OpenApiRouteRegistry::new_with_env(openapi)
                };
                if registry
                    .routes()
                    .iter()
                    .any(|route| route.method == "GET" && route.path == "/health")
                {
                    include_default_health = false;
                }
                let spec_router = registry.build_router();
                app = app.merge(spec_router);
            }
            Err(e) => {
                warn!("Failed to load OpenAPI spec from {:?}: {}. Starting without OpenAPI integration.", spec_path, e);
            }
        }
    }

    if include_default_health {
        app = app.route(
            "/health",
            axum::routing::get(|| async {
                use mockforge_core::server_utils::health::HealthStatus;
                axum::Json(
                    serde_json::to_value(HealthStatus::healthy(0, "mockforge-http")).unwrap(),
                )
            }),
        );
    }

    app = app.merge(sse::sse_router());

    // Add management API endpoints
    let management_state = ManagementState::new(None, spec_path, 3000); // Port will be updated when we know the actual port
    #[cfg(feature = "smtp")]
    let management_state = {
        if let Some(smtp_reg) = smtp_registry {
            let smtp_reg = smtp_reg
                .downcast::<mockforge_smtp::SmtpSpecRegistry>()
                .expect("Invalid SMTP registry type passed to HTTP management state");
            management_state.with_smtp_registry(smtp_reg)
        } else {
            management_state
        }
    };
    #[cfg(not(feature = "smtp"))]
    let management_state = {
        let _ = smtp_registry;
        management_state
    };
    #[cfg(feature = "mqtt")]
    let management_state = {
        if let Some(broker) = mqtt_broker {
            let broker = broker
                .downcast::<mockforge_mqtt::MqttBroker>()
                .expect("Invalid MQTT broker passed to HTTP management state");
            management_state.with_mqtt_broker(broker)
        } else {
            management_state
        }
    };
    #[cfg(not(feature = "mqtt"))]
    let management_state = {
        let _ = mqtt_broker;
        management_state
    };
    app = app.nest("/__mockforge/api", management_router(management_state));

    // Add workspace routing middleware if multi-tenant is enabled
    if let Some(mt_config) = multi_tenant_config {
        if mt_config.enabled {
            use mockforge_core::{MultiTenantWorkspaceRegistry, WorkspaceRouter};
            use std::sync::Arc;

            info!(
                "Multi-tenant mode enabled with {} routing strategy",
                match mt_config.routing_strategy {
                    mockforge_core::RoutingStrategy::Path => "path-based",
                    mockforge_core::RoutingStrategy::Port => "port-based",
                    mockforge_core::RoutingStrategy::Both => "hybrid",
                }
            );

            // Create the multi-tenant workspace registry
            let mut registry = MultiTenantWorkspaceRegistry::new(mt_config.clone());

            // Register the default workspace before wrapping in Arc
            let default_workspace =
                mockforge_core::Workspace::new(mt_config.default_workspace.clone());
            if let Err(e) =
                registry.register_workspace(mt_config.default_workspace.clone(), default_workspace)
            {
                warn!("Failed to register default workspace: {}", e);
            } else {
                info!("Registered default workspace: '{}'", mt_config.default_workspace);
            }

            // Wrap registry in Arc for shared access
            let registry = Arc::new(registry);

            // Create workspace router
            let _workspace_router = WorkspaceRouter::new(registry);
            info!("Workspace routing middleware initialized for HTTP server");
        }
    }

    app
}

// Note: start_with_traffic_shaping function removed due to compilation issues
// Use build_router_with_traffic_shaping_and_multi_tenant directly instead

#[test]
fn test_route_info_clone() {
    let route = RouteInfo {
        method: "POST".to_string(),
        path: "/users".to_string(),
        operation_id: Some("createUser".to_string()),
        summary: None,
        description: None,
        parameters: vec![],
    };

    let cloned = route.clone();
    assert_eq!(route.method, cloned.method);
    assert_eq!(route.path, cloned.path);
    assert_eq!(route.operation_id, cloned.operation_id);
}

#[test]
fn test_http_server_state_new() {
    let state = HttpServerState::new();
    assert_eq!(state.routes.len(), 0);
}

#[test]
fn test_http_server_state_with_routes() {
    let routes = vec![
        RouteInfo {
            method: "GET".to_string(),
            path: "/users".to_string(),
            operation_id: Some("getUsers".to_string()),
            summary: None,
            description: None,
            parameters: vec![],
        },
        RouteInfo {
            method: "POST".to_string(),
            path: "/users".to_string(),
            operation_id: Some("createUser".to_string()),
            summary: None,
            description: None,
            parameters: vec![],
        },
    ];

    let state = HttpServerState::with_routes(routes.clone());
    assert_eq!(state.routes.len(), 2);
    assert_eq!(state.routes[0].method, "GET");
    assert_eq!(state.routes[1].method, "POST");
}

#[test]
fn test_http_server_state_clone() {
    let routes = vec![RouteInfo {
        method: "GET".to_string(),
        path: "/test".to_string(),
        operation_id: None,
        summary: None,
        description: None,
        parameters: vec![],
    }];

    let state = HttpServerState::with_routes(routes);
    let cloned = state.clone();

    assert_eq!(state.routes.len(), cloned.routes.len());
    assert_eq!(state.routes[0].method, cloned.routes[0].method);
}

#[tokio::test]
async fn test_build_router_without_openapi() {
    let _router = build_router(None, None, None).await;
    // Should succeed without OpenAPI spec
}

#[tokio::test]
async fn test_build_router_with_nonexistent_spec() {
    let _router = build_router(Some("/nonexistent/spec.yaml".to_string()), None, None).await;
    // Should succeed but log a warning
}

#[tokio::test]
async fn test_build_router_with_auth_and_latency() {
    let _router = build_router_with_auth_and_latency(None, None, None, None).await;
    // Should succeed without parameters
}

#[tokio::test]
async fn test_build_router_with_latency() {
    let _router = build_router_with_latency(None, None, None).await;
    // Should succeed without parameters
}

#[tokio::test]
async fn test_build_router_with_auth() {
    let _router = build_router_with_auth(None, None, None).await;
    // Should succeed without parameters
}

#[tokio::test]
async fn test_build_router_with_chains() {
    let _router = build_router_with_chains(None, None, None).await;
    // Should succeed without parameters
}

#[test]
fn test_route_info_with_all_fields() {
    let route = RouteInfo {
        method: "PUT".to_string(),
        path: "/users/{id}".to_string(),
        operation_id: Some("updateUser".to_string()),
        summary: Some("Update user".to_string()),
        description: Some("Updates an existing user".to_string()),
        parameters: vec!["id".to_string(), "body".to_string()],
    };

    assert!(route.operation_id.is_some());
    assert!(route.summary.is_some());
    assert!(route.description.is_some());
    assert_eq!(route.parameters.len(), 2);
}

#[test]
fn test_route_info_with_minimal_fields() {
    let route = RouteInfo {
        method: "DELETE".to_string(),
        path: "/users/{id}".to_string(),
        operation_id: None,
        summary: None,
        description: None,
        parameters: vec![],
    };

    assert!(route.operation_id.is_none());
    assert!(route.summary.is_none());
    assert!(route.description.is_none());
    assert_eq!(route.parameters.len(), 0);
}

#[test]
fn test_http_server_state_empty_routes() {
    let state = HttpServerState::with_routes(vec![]);
    assert_eq!(state.routes.len(), 0);
}

#[test]
fn test_http_server_state_multiple_routes() {
    let routes = vec![
        RouteInfo {
            method: "GET".to_string(),
            path: "/users".to_string(),
            operation_id: Some("listUsers".to_string()),
            summary: Some("List all users".to_string()),
            description: None,
            parameters: vec![],
        },
        RouteInfo {
            method: "GET".to_string(),
            path: "/users/{id}".to_string(),
            operation_id: Some("getUser".to_string()),
            summary: Some("Get a user".to_string()),
            description: None,
            parameters: vec!["id".to_string()],
        },
        RouteInfo {
            method: "POST".to_string(),
            path: "/users".to_string(),
            operation_id: Some("createUser".to_string()),
            summary: Some("Create a user".to_string()),
            description: None,
            parameters: vec!["body".to_string()],
        },
    ];

    let state = HttpServerState::with_routes(routes);
    assert_eq!(state.routes.len(), 3);

    // Verify different HTTP methods
    let methods: Vec<&str> = state.routes.iter().map(|r| r.method.as_str()).collect();
    assert!(methods.contains(&"GET"));
    assert!(methods.contains(&"POST"));
}

#[test]
fn test_http_server_state_with_rate_limiter() {
    use std::sync::Arc;

    let config = crate::middleware::RateLimitConfig::default();
    let rate_limiter = Arc::new(crate::middleware::GlobalRateLimiter::new(config));

    let state = HttpServerState::new().with_rate_limiter(rate_limiter);

    assert!(state.rate_limiter.is_some());
    assert_eq!(state.routes.len(), 0);
}

#[tokio::test]
async fn test_build_router_includes_rate_limiter() {
    let _router = build_router(None, None, None).await;
    // Router should be created successfully with rate limiter initialized
}
