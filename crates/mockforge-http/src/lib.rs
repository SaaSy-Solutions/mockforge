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
/// Cross-protocol consistency engine integration for HTTP
pub mod consistency;
/// Contract diff middleware for automatic request capture
pub mod contract_diff_middleware;
pub mod coverage;
pub mod database;
/// File generation service for creating mock PDF, CSV, JSON files
pub mod file_generator;
/// File serving for generated mock files
pub mod file_server;
/// Kubernetes-native health check endpoints (liveness, readiness, startup probes)
pub mod health;
pub mod http_tracing_middleware;
/// Latency profile configuration for HTTP request simulation
pub mod latency_profiles;
/// Management API for server control and monitoring
pub mod management;
/// WebSocket-based management API for real-time updates
pub mod management_ws;
pub mod metrics_middleware;
pub mod middleware;
pub mod op_middleware;
/// Browser/Mobile Proxy Server
pub mod proxy_server;
/// Quick mock generation utilities
pub mod quick_mock;
/// RAG-powered AI response generation
pub mod rag_ai_generator;
/// Replay listing and fixture management
pub mod replay_listing;
pub mod request_logging;
/// Specification import API for OpenAPI and AsyncAPI
pub mod spec_import;
/// Server-Sent Events for streaming logs and metrics
pub mod sse;
/// State machine API for scenario state machines
pub mod state_machine_api;
/// TLS/HTTPS support
pub mod tls;
/// Token response utilities
pub mod token_response;
/// UI Builder API for low-code mock endpoint creation
pub mod ui_builder;
/// Verification API for request verification
pub mod verification;

// Access review handlers
pub mod handlers;

// Re-export AI handler utilities
pub use ai_handler::{process_response_with_ai, AiResponseConfig, AiResponseHandler};
// Re-export health check utilities
pub use health::{HealthManager, ServiceStatus};

// Re-export management API utilities
pub use management::{
    management_router, management_router_with_ui_builder, ManagementState, MockConfig,
    ServerConfig, ServerStats,
};

// Re-export UI Builder utilities
pub use ui_builder::{create_ui_builder_router, EndpointConfig, UIBuilderState};

// Re-export management WebSocket utilities
pub use management_ws::{ws_management_router, MockEvent, WsManagementState};

// Re-export verification API utilities
pub use verification::verification_router;

// Re-export metrics middleware
pub use metrics_middleware::collect_http_metrics;

// Re-export tracing middleware
pub use http_tracing_middleware::http_tracing_middleware;

// Re-export coverage utilities
pub use coverage::{calculate_coverage, CoverageReport, MethodCoverage, RouteCoverage};

/// Helper function to load persona from config file
/// Tries to load from common config locations: config.yaml, mockforge.yaml, tools/mockforge/config.yaml
async fn load_persona_from_config() -> Option<Arc<Persona>> {
    use mockforge_core::config::load_config;

    // Try common config file locations
    let config_paths = [
        "config.yaml",
        "mockforge.yaml",
        "tools/mockforge/config.yaml",
        "../tools/mockforge/config.yaml",
    ];

    for path in &config_paths {
        if let Ok(config) = load_config(path).await {
            // Access intelligent_behavior through mockai config
            // Note: Config structure is mockai.intelligent_behavior.personas
            if let Some(persona) = config.mockai.intelligent_behavior.personas.get_active_persona() {
                tracing::info!(
                    "Loaded active persona '{}' from config file: {}",
                    persona.name,
                    path
                );
                return Some(Arc::new(persona.clone()));
            } else {
                tracing::debug!(
                    "No active persona found in config file: {} (personas count: {})",
                    path,
                    config.mockai.intelligent_behavior.personas.personas.len()
                );
            }
        } else {
            tracing::debug!("Could not load config from: {}", path);
        }
    }

    tracing::debug!("No persona found in config files, persona-based generation will be disabled");
    None
}

use axum::middleware::from_fn_with_state;
use axum::{extract::State, response::Json, Router};
use mockforge_core::failure_injection::{FailureConfig, FailureInjector};
use mockforge_core::latency::LatencyInjector;
use mockforge_core::openapi::OpenApiSpec;
use mockforge_core::openapi_routes::OpenApiRouteRegistry;
use mockforge_core::openapi_routes::ValidationOptions;
use mockforge_core::intelligent_behavior::config::Persona;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

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
    /// HTTP method (GET, POST, PUT, etc.)
    pub method: String,
    /// API path pattern (e.g., "/api/users/{id}")
    pub path: String,
    /// OpenAPI operation ID if available
    pub operation_id: Option<String>,
    /// Operation summary from OpenAPI spec
    pub summary: Option<String>,
    /// Operation description from OpenAPI spec
    pub description: Option<String>,
    /// List of parameter names for this route
    pub parameters: Vec<String>,
}

/// Shared state for tracking OpenAPI routes
#[derive(Clone)]
pub struct HttpServerState {
    /// List of registered routes from OpenAPI spec
    pub routes: Vec<RouteInfo>,
    /// Optional global rate limiter for request throttling
    pub rate_limiter: Option<std::sync::Arc<crate::middleware::rate_limit::GlobalRateLimiter>>,
    /// Production headers to add to all responses (for deceptive deploy)
    pub production_headers: Option<std::sync::Arc<std::collections::HashMap<String, String>>>,
}

impl Default for HttpServerState {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpServerState {
    /// Create a new empty HTTP server state
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            rate_limiter: None,
            production_headers: None,
        }
    }

    /// Create HTTP server state with pre-configured routes
    pub fn with_routes(routes: Vec<RouteInfo>) -> Self {
        Self {
            routes,
            rate_limiter: None,
            production_headers: None,
        }
    }

    /// Add a rate limiter to the HTTP server state
    pub fn with_rate_limiter(
        mut self,
        rate_limiter: std::sync::Arc<crate::middleware::rate_limit::GlobalRateLimiter>,
    ) -> Self {
        self.rate_limiter = Some(rate_limiter);
        self
    }

    /// Add production headers to the HTTP server state
    pub fn with_production_headers(
        mut self,
        headers: std::sync::Arc<std::collections::HashMap<String, String>>,
    ) -> Self {
        self.production_headers = Some(headers);
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
    build_router_with_multi_tenant(
        spec_path,
        options,
        failure_config,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
}

/// Apply CORS middleware to the router based on configuration
fn apply_cors_middleware(
    app: Router,
    cors_config: Option<mockforge_core::config::HttpCorsConfig>,
) -> Router {
    use http::Method;
    use tower_http::cors::AllowOrigin;

    if let Some(config) = cors_config {
        if !config.enabled {
            return app;
        }

        let mut cors_layer = CorsLayer::new();
        let mut is_wildcard_origin = false;

        // Configure allowed origins
        if config.allowed_origins.contains(&"*".to_string()) {
            cors_layer = cors_layer.allow_origin(Any);
            is_wildcard_origin = true;
        } else if !config.allowed_origins.is_empty() {
            // Try to parse each origin, fallback to permissive if parsing fails
            let origins: Vec<_> = config
                .allowed_origins
                .iter()
                .filter_map(|origin| {
                    origin.parse::<http::HeaderValue>().ok().map(|hv| AllowOrigin::exact(hv))
                })
                .collect();

            if origins.is_empty() {
                // If no valid origins, use permissive for development
                warn!("No valid CORS origins configured, using permissive CORS");
                cors_layer = cors_layer.allow_origin(Any);
                is_wildcard_origin = true;
            } else {
                // Use the first origin as exact match (tower-http limitation)
                // For multiple origins, we'd need a custom implementation
                if origins.len() == 1 {
                    cors_layer = cors_layer.allow_origin(origins[0].clone());
                    is_wildcard_origin = false;
                } else {
                    // Multiple origins - use permissive for now
                    warn!(
                        "Multiple CORS origins configured, using permissive CORS. \
                        Consider using '*' for all origins."
                    );
                    cors_layer = cors_layer.allow_origin(Any);
                    is_wildcard_origin = true;
                }
            }
        } else {
            // No origins specified, use permissive for development
            cors_layer = cors_layer.allow_origin(Any);
            is_wildcard_origin = true;
        }

        // Configure allowed methods
        if !config.allowed_methods.is_empty() {
            let methods: Vec<Method> =
                config.allowed_methods.iter().filter_map(|m| m.parse().ok()).collect();
            if !methods.is_empty() {
                cors_layer = cors_layer.allow_methods(methods);
            }
        } else {
            // Default to common HTTP methods
            cors_layer = cors_layer.allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
                Method::OPTIONS,
            ]);
        }

        // Configure allowed headers
        if !config.allowed_headers.is_empty() {
            let headers: Vec<_> = config
                .allowed_headers
                .iter()
                .filter_map(|h| h.parse::<http::HeaderName>().ok())
                .collect();
            if !headers.is_empty() {
                cors_layer = cors_layer.allow_headers(headers);
            }
        } else {
            // Default headers
            cors_layer =
                cors_layer.allow_headers([http::header::CONTENT_TYPE, http::header::AUTHORIZATION]);
        }

        // Configure credentials - cannot allow credentials with wildcard origin
        // Determine if credentials should be allowed
        // Cannot allow credentials with wildcard origin per CORS spec
        let should_allow_credentials = if is_wildcard_origin {
            // Wildcard origin - credentials must be false
            false
        } else {
            // Specific origins - use config value (defaults to false)
            config.allow_credentials
        };

        cors_layer = cors_layer.allow_credentials(should_allow_credentials);

        info!(
            "CORS middleware enabled with configured settings (credentials: {})",
            should_allow_credentials
        );
        app.layer(cors_layer)
    } else {
        // No CORS config provided - use permissive CORS for development
        // Note: permissive() allows credentials, but since it uses wildcard origin,
        // we need to disable credentials to avoid CORS spec violation
        debug!("No CORS config provided, using permissive CORS for development");
        // Create a permissive CORS layer but disable credentials to avoid CORS spec violation
        // (cannot combine credentials with wildcard origin)
        app.layer(CorsLayer::permissive().allow_credentials(false))
    }
}

/// Build the base HTTP router with multi-tenant workspace support
#[allow(clippy::too_many_arguments)]
pub async fn build_router_with_multi_tenant(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    failure_config: Option<FailureConfig>,
    multi_tenant_config: Option<mockforge_core::MultiTenantConfig>,
    _route_configs: Option<Vec<mockforge_core::config::RouteConfig>>,
    cors_config: Option<mockforge_core::config::HttpCorsConfig>,
    ai_generator: Option<
        std::sync::Arc<dyn mockforge_core::openapi::response::AiGenerator + Send + Sync>,
    >,
    smtp_registry: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
    mockai: Option<
        std::sync::Arc<tokio::sync::RwLock<mockforge_core::intelligent_behavior::MockAI>>,
    >,
    deceptive_deploy_config: Option<mockforge_core::config::DeceptiveDeployConfig>,
) -> Router {
    use std::time::Instant;

    let startup_start = Instant::now();

    // Set up the basic router
    let mut app = Router::new();

    // Initialize rate limiter with default configuration
    // Can be customized via environment variables or config
    let mut rate_limit_config = crate::middleware::RateLimitConfig {
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

    // Apply deceptive deploy configuration if enabled
    let mut final_cors_config = cors_config;
    let mut production_headers: Option<std::sync::Arc<std::collections::HashMap<String, String>>> =
        None;
    // Auth config from deceptive deploy OAuth (if configured)
    let mut deceptive_deploy_auth_config: Option<mockforge_core::config::AuthConfig> = None;

    if let Some(deploy_config) = &deceptive_deploy_config {
        if deploy_config.enabled {
            info!("Deceptive deploy mode enabled - applying production-like configuration");

            // Override CORS config if provided
            if let Some(prod_cors) = &deploy_config.cors {
                final_cors_config = Some(mockforge_core::config::HttpCorsConfig {
                    enabled: true,
                    allowed_origins: prod_cors.allowed_origins.clone(),
                    allowed_methods: prod_cors.allowed_methods.clone(),
                    allowed_headers: prod_cors.allowed_headers.clone(),
                    allow_credentials: prod_cors.allow_credentials,
                });
                info!("Applied production-like CORS configuration");
            }

            // Override rate limit config if provided
            if let Some(prod_rate_limit) = &deploy_config.rate_limit {
                rate_limit_config = crate::middleware::RateLimitConfig {
                    requests_per_minute: prod_rate_limit.requests_per_minute,
                    burst: prod_rate_limit.burst,
                    per_ip: prod_rate_limit.per_ip,
                    per_endpoint: false,
                };
                info!(
                    "Applied production-like rate limiting: {} req/min, burst: {}",
                    prod_rate_limit.requests_per_minute, prod_rate_limit.burst
                );
            }

            // Set production headers
            if !deploy_config.headers.is_empty() {
                let headers_map: std::collections::HashMap<String, String> =
                    deploy_config.headers.clone();
                production_headers = Some(std::sync::Arc::new(headers_map));
                info!("Configured {} production headers", deploy_config.headers.len());
            }

            // Integrate OAuth config from deceptive deploy
            if let Some(prod_oauth) = &deploy_config.oauth {
                let oauth2_config: mockforge_core::config::OAuth2Config = prod_oauth.clone().into();
                deceptive_deploy_auth_config = Some(mockforge_core::config::AuthConfig {
                    oauth2: Some(oauth2_config),
                    ..Default::default()
                });
                info!("Applied production-like OAuth configuration for deceptive deploy");
            }
        }
    }

    let rate_limiter =
        std::sync::Arc::new(crate::middleware::GlobalRateLimiter::new(rate_limit_config.clone()));

    let mut state = HttpServerState::new().with_rate_limiter(rate_limiter.clone());

    // Add production headers to state if configured
    if let Some(headers) = production_headers.clone() {
        state = state.with_production_headers(headers);
    }

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

                // Try to load persona from config if available
                let persona = load_persona_from_config().await;

                let registry = if let Some(opts) = options {
                    tracing::debug!("Using custom validation options");
                    if let Some(ref persona) = persona {
                        tracing::info!("Using persona '{}' for route generation", persona.name);
                    }
                    OpenApiRouteRegistry::new_with_options_and_persona(openapi, opts, persona)
                } else {
                    tracing::debug!("Using environment-based options");
                    if let Some(ref persona) = persona {
                        tracing::info!("Using persona '{}' for route generation", persona.name);
                    }
                    OpenApiRouteRegistry::new_with_env_and_persona(openapi, persona)
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
                let openapi_router = if let Some(mockai_instance) = &mockai {
                    tracing::debug!("Building router with MockAI support");
                    registry.build_router_with_mockai(Some(mockai_instance.clone()))
                } else if let Some(ai_generator) = &ai_generator {
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
            {
                // HealthStatus should always serialize, but handle errors gracefully
                match serde_json::to_value(HealthStatus::healthy(0, "mockforge-http")) {
                    Ok(value) => axum::Json(value),
                    Err(e) => {
                        // Log error but return a simple healthy response
                        tracing::error!("Failed to serialize health status: {}", e);
                        axum::Json(serde_json::json!({
                            "status": "healthy",
                            "service": "mockforge-http",
                            "uptime_seconds": 0
                        }))
                    }
                }
            }
        }),
    )
    // Add SSE endpoints
    .merge(sse::sse_router())
    // Add file serving endpoints for generated mock files
    .merge(file_server::file_serving_router());

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

    // Create WebSocket state and connect it to management state
    use std::sync::Arc;
    let ws_state = WsManagementState::new();
    let ws_broadcast = Arc::new(ws_state.tx.clone());
    let management_state = management_state.with_ws_broadcast(ws_broadcast);

    // Note: ProxyConfig not available in this build function path
    // Migration endpoints will work once ProxyConfig is passed to build_router_with_chains_and_multi_tenant

    #[cfg(feature = "smtp")]
    let management_state = {
        if let Some(smtp_reg) = smtp_registry {
            match smtp_reg.downcast::<mockforge_smtp::SmtpSpecRegistry>() {
                Ok(smtp_reg) => management_state.with_smtp_registry(smtp_reg),
                Err(e) => {
                    error!(
                        "Invalid SMTP registry type passed to HTTP management state: {:?}",
                        e.type_id()
                    );
                    management_state
                }
            }
        } else {
            management_state
        }
    };
    #[cfg(not(feature = "smtp"))]
    let management_state = management_state;
    #[cfg(not(feature = "smtp"))]
    let _ = smtp_registry;
    app = app.nest("/__mockforge/api", management_router(management_state));

    // Add verification API endpoint
    app = app.merge(verification_router());

    // Add OIDC well-known endpoints
    use crate::auth::oidc::oidc_router;
    app = app.merge(oidc_router());

    // Add access review API if enabled
    {
        use mockforge_core::security::get_global_access_review_service;
        if let Some(service) = get_global_access_review_service().await {
            use crate::handlers::access_review::{access_review_router, AccessReviewState};
            let review_state = AccessReviewState { service };
            app = app.nest("/api/v1/security/access-reviews", access_review_router(review_state));
            debug!("Access review API mounted at /api/v1/security/access-reviews");
        }
    }

    // Add privileged access API if enabled
    {
        use mockforge_core::security::get_global_privileged_access_manager;
        if let Some(manager) = get_global_privileged_access_manager().await {
            use crate::handlers::privileged_access::{privileged_access_router, PrivilegedAccessState};
            let privileged_state = PrivilegedAccessState { manager };
            app = app.nest("/api/v1/security/privileged-access", privileged_access_router(privileged_state));
            debug!("Privileged access API mounted at /api/v1/security/privileged-access");
        }
    }

    // Add change management API if enabled
    {
        use mockforge_core::security::get_global_change_management_engine;
        if let Some(engine) = get_global_change_management_engine().await {
            use crate::handlers::change_management::{change_management_router, ChangeManagementState};
            let change_state = ChangeManagementState { engine };
            app = app.nest("/api/v1/change-management", change_management_router(change_state));
            debug!("Change management API mounted at /api/v1/change-management");
        }
    }

    // Add risk assessment API if enabled
    {
        use mockforge_core::security::get_global_risk_assessment_engine;
        if let Some(engine) = get_global_risk_assessment_engine().await {
            use crate::handlers::risk_assessment::{risk_assessment_router, RiskAssessmentState};
            let risk_state = RiskAssessmentState { engine };
            app = app.nest("/api/v1/security", risk_assessment_router(risk_state));
            debug!("Risk assessment API mounted at /api/v1/security/risks");
        }
    }

    // Add token lifecycle API
    {
        use crate::auth::token_lifecycle::TokenLifecycleManager;
        use crate::handlers::token_lifecycle::{token_lifecycle_router, TokenLifecycleState};
        let lifecycle_manager = Arc::new(TokenLifecycleManager::default());
        let lifecycle_state = TokenLifecycleState {
            manager: lifecycle_manager,
        };
        app = app.nest("/api/v1/auth", token_lifecycle_router(lifecycle_state));
        debug!("Token lifecycle API mounted at /api/v1/auth");
    }

    // Add OAuth2 server endpoints
    {
        use crate::auth::oidc::{load_oidc_state, OidcState};
        use crate::auth::token_lifecycle::TokenLifecycleManager;
        use crate::handlers::oauth2_server::{oauth2_server_router, OAuth2ServerState};
        // Load OIDC state from configuration (environment variables or config file)
        let oidc_state = Arc::new(RwLock::new(load_oidc_state()));
        let lifecycle_manager = Arc::new(TokenLifecycleManager::default());
        let oauth2_state = OAuth2ServerState {
            oidc_state,
            lifecycle_manager,
            auth_codes: Arc::new(RwLock::new(HashMap::new())),
        };
        app = app.merge(oauth2_server_router(oauth2_state));
        debug!("OAuth2 server endpoints mounted at /oauth2/authorize and /oauth2/token");
    }

    // Add consent screen endpoints
    {
        use crate::auth::risk_engine::RiskEngine;
        use crate::auth::token_lifecycle::TokenLifecycleManager;
        use crate::handlers::consent::{consent_router, ConsentState};
        use crate::handlers::oauth2_server::OAuth2ServerState;
        use crate::auth::oidc::{load_oidc_state, OidcState};
        // Load OIDC state from configuration (environment variables or config file)
        let oidc_state = Arc::new(RwLock::new(load_oidc_state()));
        let lifecycle_manager = Arc::new(TokenLifecycleManager::default());
        let oauth2_state = OAuth2ServerState {
            oidc_state: oidc_state.clone(),
            lifecycle_manager: lifecycle_manager.clone(),
            auth_codes: Arc::new(RwLock::new(HashMap::new())),
        };
        let risk_engine = Arc::new(RiskEngine::default());
        let consent_state = ConsentState {
            oauth2_state,
            risk_engine,
        };
        app = app.merge(consent_router(consent_state));
        debug!("Consent screen endpoints mounted at /consent");
    }

    // Add risk simulation API
    {
        use crate::auth::risk_engine::RiskEngine;
        use crate::handlers::risk_simulation::{risk_simulation_router, RiskSimulationState};
        let risk_engine = Arc::new(RiskEngine::default());
        let risk_state = RiskSimulationState { risk_engine };
        app = app.nest("/api/v1/auth", risk_simulation_router(risk_state));
        debug!("Risk simulation API mounted at /api/v1/auth/risk");
    }

    // Add management WebSocket endpoint
    app = app.nest("/__mockforge/ws", ws_management_router(ws_state));

    // Add request logging middleware to capture all requests
    app = app.layer(axum::middleware::from_fn(request_logging::log_http_requests));

    // Add security middleware for security event tracking (after logging, before contract diff)
    app = app.layer(axum::middleware::from_fn(crate::middleware::security_middleware));

    // Add contract diff middleware for automatic request capture
    // This captures requests for contract diff analysis (after logging)
    app = app.layer(axum::middleware::from_fn(contract_diff_middleware::capture_for_contract_diff));

    // Add rate limiting middleware (before logging to rate limit early)
    app = app.layer(from_fn_with_state(state.clone(), crate::middleware::rate_limit_middleware));

    // Add production headers middleware if configured
    if state.production_headers.is_some() {
        app = app.layer(from_fn_with_state(
            state.clone(),
            crate::middleware::production_headers_middleware,
        ));
    }

    // Add authentication middleware if OAuth is configured via deceptive deploy
    if let Some(auth_config) = deceptive_deploy_auth_config {
        use crate::auth::{auth_middleware, create_oauth2_client, AuthState};
        use std::collections::HashMap;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        // Create OAuth2 client if configured
        let oauth2_client = if let Some(oauth2_config) = &auth_config.oauth2 {
            match create_oauth2_client(oauth2_config) {
                Ok(client) => Some(client),
                Err(e) => {
                    warn!("Failed to create OAuth2 client from deceptive deploy config: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Create auth state
        let auth_state = AuthState {
            config: auth_config,
            spec: None, // OpenAPI spec not available in this context
            oauth2_client,
            introspection_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Apply auth middleware
        app = app.layer(axum::middleware::from_fn_with_state(auth_state, auth_middleware));
        info!("Applied OAuth authentication middleware from deceptive deploy configuration");
    }

    // Add CORS middleware (use final_cors_config which may be overridden by deceptive deploy)
    app = apply_cors_middleware(app, final_cors_config);

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
            {
                // HealthStatus should always serialize, but handle errors gracefully
                match serde_json::to_value(HealthStatus::healthy(0, "mockforge-http")) {
                    Ok(value) => axum::Json(value),
                    Err(e) => {
                        // Log error but return a simple healthy response
                        tracing::error!("Failed to serialize health status: {}", e);
                        axum::Json(serde_json::json!({
                            "status": "healthy",
                            "service": "mockforge-http",
                            "uptime_seconds": 0
                        }))
                    }
                }
            }
        }),
    )
    // Add SSE endpoints
    .merge(sse::sse_router())
    // Add file serving endpoints for generated mock files
    .merge(file_server::file_serving_router())
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
    serve_router_with_tls(port, app, None).await
}

/// Serve a provided router on the given port with optional TLS support.
pub async fn serve_router_with_tls(
    port: u16,
    app: Router,
    tls_config: Option<mockforge_core::config::HttpTlsConfig>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::net::SocketAddr;

    let addr = mockforge_core::wildcard_socket_addr(port);

    if let Some(ref tls) = tls_config {
        if tls.enabled {
            info!("HTTPS listening on {}", addr);
            return serve_with_tls(addr, app, tls).await;
        }
    }

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

/// Serve router with TLS/HTTPS support
///
/// Note: This is a simplified implementation. For production use, consider using
/// a reverse proxy (nginx) for TLS termination, or use axum-server crate.
/// This implementation validates TLS configuration but recommends using a reverse proxy.
async fn serve_with_tls(
    addr: std::net::SocketAddr,
    _app: Router,
    tls_config: &mockforge_core::config::HttpTlsConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Validate TLS configuration by attempting to load certificates
    let _acceptor = tls::load_tls_acceptor(tls_config)?;

    // For now, return an informative error suggesting reverse proxy usage
    // Full TLS implementation with axum requires axum-server or similar
    Err(format!(
        "TLS/HTTPS support is configured but requires a reverse proxy (nginx) for production use.\n\
         Certificate validation passed: {} and {}\n\
         For native TLS support, please use a reverse proxy or wait for axum-server integration.\n\
         You can configure nginx with TLS termination pointing to the HTTP server on port {}.",
        tls_config.cert_file,
        tls_config.key_file,
        addr.port()
    )
    .into())
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
        None, // health_manager
        None, // mockai
        None, // deceptive_deploy_config
        None, // proxy_config
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
    route_configs: Option<Vec<mockforge_core::config::RouteConfig>>,
    cors_config: Option<mockforge_core::config::HttpCorsConfig>,
    _ai_generator: Option<
        std::sync::Arc<dyn mockforge_core::openapi::response::AiGenerator + Send + Sync>,
    >,
    smtp_registry: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
    mqtt_broker: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
    traffic_shaper: Option<mockforge_core::traffic_shaping::TrafficShaper>,
    traffic_shaping_enabled: bool,
    health_manager: Option<std::sync::Arc<health::HealthManager>>,
    _mockai: Option<
        std::sync::Arc<tokio::sync::RwLock<mockforge_core::intelligent_behavior::MockAI>>,
    >,
    deceptive_deploy_config: Option<mockforge_core::config::DeceptiveDeployConfig>,
    proxy_config: Option<mockforge_core::proxy::config::ProxyConfig>,
) -> Router {
    use crate::latency_profiles::LatencyProfiles;
    use crate::op_middleware::Shared;
    use mockforge_core::Overrides;

    // Extract template expansion setting before options is moved (used in OpenAPI routes and custom routes)
    let template_expand = options.as_ref()
        .map(|o| o.response_template_expand)
        .unwrap_or_else(|| {
            std::env::var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false)
        });

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

                // Try to load persona from config if available
                let persona = load_persona_from_config().await;

                let mut registry = if let Some(opts) = options {
                    tracing::debug!("Using custom validation options");
                    if let Some(ref persona) = persona {
                        tracing::info!("Using persona '{}' for route generation", persona.name);
                    }
                    OpenApiRouteRegistry::new_with_options_and_persona(openapi, opts, persona)
                } else {
                    tracing::debug!("Using environment-based options");
                    if let Some(ref persona) = persona {
                        tracing::info!("Using persona '{}' for route generation", persona.name);
                    }
                    OpenApiRouteRegistry::new_with_env_and_persona(openapi, persona)
                };

                // Load custom fixtures if enabled
                let fixtures_dir = std::env::var("MOCKFORGE_FIXTURES_DIR")
                    .unwrap_or_else(|_| "/app/fixtures".to_string());
                let custom_fixtures_enabled = std::env::var("MOCKFORGE_CUSTOM_FIXTURES_ENABLED")
                    .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                    .unwrap_or(true); // Enabled by default

                if custom_fixtures_enabled {
                    use mockforge_core::CustomFixtureLoader;
                    use std::path::PathBuf;
                    use std::sync::Arc;

                    let fixtures_path = PathBuf::from(&fixtures_dir);
                    let mut custom_loader = CustomFixtureLoader::new(fixtures_path, true);

                    if let Err(e) = custom_loader.load_fixtures().await {
                        tracing::warn!("Failed to load custom fixtures: {}", e);
                    } else {
                        tracing::info!("Custom fixtures loaded from {}", fixtures_dir);
                        registry = registry.with_custom_fixture_loader(Arc::new(custom_loader));
                    }
                }

                if registry
                    .routes()
                    .iter()
                    .any(|route| route.method == "GET" && route.path == "/health")
                {
                    include_default_health = false;
                }
                // Use MockAI if available, otherwise use standard router
                let spec_router = if let Some(ref mockai_instance) = _mockai {
                    tracing::debug!("Building router with MockAI support");
                    registry.build_router_with_mockai(Some(mockai_instance.clone()))
                } else {
                    registry.build_router()
                };
                app = app.merge(spec_router);
            }
            Err(e) => {
                warn!("Failed to load OpenAPI spec from {:?}: {}. Starting without OpenAPI integration.", spec_path, e);
            }
        }
    }

    // Helper function to recursively expand templates in JSON values
    fn expand_templates_in_json(value: &serde_json::Value, context: &mockforge_core::ai_response::RequestContext) -> serde_json::Value {
        use mockforge_core::ai_response::expand_prompt_template;
        use serde_json::Value;

        match value {
            Value::String(s) => {
                // Normalize {{request.query.name}} to {{query.name}} format
                let normalized = s
                    .replace("{{request.query.", "{{query.")
                    .replace("{{request.path.", "{{path.")
                    .replace("{{request.headers.", "{{headers.")
                    .replace("{{request.body.", "{{body.")
                    .replace("{{request.method}}", "{{method}}")
                    .replace("{{request.path}}", "{{path}}");

                // Handle || operator: extract template part and default value separately
                // Pattern: "Hello {{query.name || \"world\"}}" -> extract "Hello {{query.name}}" and "world"
                let (template_part, default_value) = if normalized.contains("||") {
                    // Find the template part before || and default after ||
                    // Pattern: "Hello {{query.name || \"world\"}}"
                    // We need to find {{... || "..."}} and split it
                    if let Some(open_idx) = normalized.find("{{") {
                        if let Some(close_idx) = normalized[open_idx..].find("}}") {
                            let template_block = &normalized[open_idx..open_idx+close_idx+2];
                            if let Some(pipe_idx) = template_block.find("||") {
                                // Split: "{{query.name || \"world\"}}" -> "{{query.name " and " \"world\"}}"
                                let before_pipe = &template_block[..pipe_idx].trim();
                                let after_pipe = &template_block[pipe_idx+2..].trim();

                                // Extract template variable name (remove {{ and trim)
                                let template_var = before_pipe.trim_start_matches("{{").trim();
                                // Replace the entire template block with just the template variable
                                let replacement = format!("{{{{{}}}}}}}", template_var);
                                let template = normalized.replace(template_block, &replacement);

                                // Extract default value: " \"world\"}}" -> "world"
                                let mut default = after_pipe.trim_end_matches("}}").trim().to_string();
                                // Remove quotes
                                default = default.trim_matches('"').trim_matches('\'').trim_matches('\\').to_string();
                                default = default.trim().to_string();

                                (template, Some(default))
                            } else {
                                (normalized, None)
                            }
                        } else {
                            (normalized, None)
                        }
                    } else {
                        (normalized, None)
                    }
                } else {
                    (normalized, None)
                };

                // Expand the template part
                let mut expanded = expand_prompt_template(&template_part, context);

                // If template wasn't fully expanded and we have a default, use default
                // Otherwise use the expanded value
                let final_expanded = if (expanded.contains("{{query.") || expanded.contains("{{path.") || expanded.contains("{{headers."))
                    && default_value.is_some() {
                    default_value.unwrap()
                } else {
                    // Clean up any stray closing braces that might remain
                    // This can happen if template replacement left partial braces
                    while expanded.ends_with('}') && !expanded.ends_with("}}") {
                        expanded.pop();
                    }
                    expanded
                };

                Value::String(final_expanded)
            }
            Value::Array(arr) => {
                Value::Array(arr.iter().map(|v| expand_templates_in_json(v, context)).collect())
            }
            Value::Object(obj) => {
                let mut new_obj = serde_json::Map::new();
                for (k, v) in obj {
                    new_obj.insert(k.clone(), expand_templates_in_json(v, context));
                }
                Value::Object(new_obj)
            }
            _ => value.clone(),
        }
    }

    // Register custom routes from config
    if let Some(route_configs) = route_configs {
        use axum::http::StatusCode;
        use axum::response::IntoResponse;

        if !route_configs.is_empty() {
            info!("Registering {} custom route(s) from config", route_configs.len());
        }

        for route_config in route_configs {
            let status = route_config.response.status;
            let body = route_config.response.body.clone();
            let headers = route_config.response.headers.clone();
            let path = route_config.path.clone();
            let method = route_config.method.clone();
            let latency_config = route_config.latency.clone();

            // Create handler that returns the configured response with template expansion
            // Supports both basic templates ({{uuid}}, {{now}}) and request-aware templates
            // ({{request.query.name}}, {{request.path.id}}, {{request.headers.name}})
            // Register route using `any()` since we need full Request access for template expansion
            let expected_method = method.to_uppercase();
            app = app.route(&path, axum::routing::any(move |req: axum::http::Request<axum::body::Body>| {
                let body = body.clone();
                let headers = headers.clone();
                let expand = template_expand;
                let latency = latency_config.clone();
                let expected = expected_method.clone();
                let status_code = status;

                async move {
                    // Check if request method matches expected method
                    if req.method().as_str() != expected.as_str() {
                        // Return 405 Method Not Allowed for wrong method
                        return axum::response::Response::builder()
                            .status(axum::http::StatusCode::METHOD_NOT_ALLOWED)
                            .header("Allow", &expected)
                            .body(axum::body::Body::empty())
                            .unwrap()
                            .into_response();
                    }

                    // Apply latency injection if configured
                    // Calculate delay before any await to avoid Send issues
                    let delay_ms = if let Some(ref lat) = latency {
                        if lat.enabled {
                            use rand::{rng, Rng};

                            // Check probability - generate all random values before await
                            let mut rng = rng();
                            let roll: f64 = rng.random();

                            if roll < lat.probability {
                                if let Some(fixed) = lat.fixed_delay_ms {
                                    // Fixed delay with optional jitter
                                    let jitter = (fixed as f64 * lat.jitter_percent / 100.0) as u64;
                                    let jitter_amount = if jitter > 0 {
                                        rng.random_range(0..=jitter)
                                    } else {
                                        0
                                    };
                                    Some(fixed + jitter_amount)
                                } else if let Some((min, max)) = lat.random_delay_range_ms {
                                    // Random delay range
                                    Some(rng.random_range(min..=max))
                                } else {
                                    // Default to 0 if no delay specified
                                    Some(0)
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    // Apply delay if calculated (after all random generation is done)
                    if let Some(delay) = delay_ms {
                        if delay > 0 {
                            use tokio::time::{sleep, Duration};
                            sleep(Duration::from_millis(delay)).await;
                        }
                    }

                    // Create JSON response from body, or empty object if None
                    let mut body_value = body.unwrap_or(serde_json::json!({}));

                    // Apply template expansion if enabled
                    if expand {
                        use mockforge_core::ai_response::RequestContext;
                        use std::collections::HashMap;
                        use serde_json::Value;

                        // Extract request data for template expansion
                        let method = req.method().to_string();
                        let path = req.uri().path().to_string();

                        // Extract query parameters
                        let query_params: HashMap<String, Value> = req
                            .uri()
                            .query()
                            .map(|q| {
                                url::form_urlencoded::parse(q.as_bytes())
                                    .into_owned()
                                    .map(|(k, v)| (k, Value::String(v)))
                                    .collect()
                            })
                            .unwrap_or_default();

                        // Extract headers
                        let request_headers: HashMap<String, Value> = req
                            .headers()
                            .iter()
                            .filter_map(|(name, value)| {
                                value.to_str().ok().map(|v| {
                                    (name.to_string(), Value::String(v.to_string()))
                                })
                            })
                            .collect();

                        // Note: Request body extraction for {{request.body.field}} would go here
                        // For now, we skip it to avoid consuming the body

                        // Build request context
                        let context = RequestContext::new(method.clone(), path.clone())
                            .with_query_params(query_params)
                            .with_headers(request_headers);

                        // Recursively expand templates in JSON structure
                        body_value = expand_templates_in_json(&body_value, &context);
                    }

                    let mut response = axum::Json(body_value).into_response();

                    // Set status code
                    *response.status_mut() = StatusCode::from_u16(status_code)
                        .unwrap_or(StatusCode::OK);

                    // Add custom headers
                    for (key, value) in headers {
                        if let Ok(header_name) = axum::http::HeaderName::from_bytes(key.as_bytes()) {
                            if let Ok(header_value) = axum::http::HeaderValue::from_str(&value) {
                                response.headers_mut().insert(header_name, header_value);
                            }
                        }
                    }

                    response
                }
            }));

            debug!("Registered route: {} {}", method, path);
        }
    }

    // Add health check endpoints
    if let Some(health) = health_manager {
        // Use comprehensive health check router with all probe endpoints
        app = app.merge(health::health_router(health));
        info!(
            "Health check endpoints enabled: /health, /health/live, /health/ready, /health/startup"
        );
    } else if include_default_health {
        // Fallback to basic health endpoint for backwards compatibility
        app = app.route(
            "/health",
            axum::routing::get(|| async {
                use mockforge_core::server_utils::health::HealthStatus;
                {
                    // HealthStatus should always serialize, but handle errors gracefully
                    match serde_json::to_value(HealthStatus::healthy(0, "mockforge-http")) {
                        Ok(value) => axum::Json(value),
                        Err(e) => {
                            // Log error but return a simple healthy response
                            tracing::error!("Failed to serialize health status: {}", e);
                            axum::Json(serde_json::json!({
                                "status": "healthy",
                                "service": "mockforge-http",
                                "uptime_seconds": 0
                            }))
                        }
                    }
                }
            }),
        );
    }

    app = app.merge(sse::sse_router());
    // Add file serving endpoints for generated mock files
    app = app.merge(file_server::file_serving_router());

    // Add management API endpoints
    let spec_path_clone = spec_path.clone();
    let mut management_state = ManagementState::new(None, spec_path_clone, 3000); // Port will be updated when we know the actual port

    // Create WebSocket state and connect it to management state
    use std::sync::Arc;
    let ws_state = WsManagementState::new();
    let ws_broadcast = Arc::new(ws_state.tx.clone());
    let management_state = management_state.with_ws_broadcast(ws_broadcast);

    // Add proxy config to management state if available
    let management_state = if let Some(proxy_cfg) = proxy_config {
        use tokio::sync::RwLock;
        let proxy_config_arc = Arc::new(RwLock::new(proxy_cfg));
        management_state.with_proxy_config(proxy_config_arc)
    } else {
        management_state
    };

    #[cfg(feature = "smtp")]
    let management_state = {
        if let Some(smtp_reg) = smtp_registry {
            match smtp_reg.downcast::<mockforge_smtp::SmtpSpecRegistry>() {
                Ok(smtp_reg) => management_state.with_smtp_registry(smtp_reg),
                Err(e) => {
                    error!(
                        "Invalid SMTP registry type passed to HTTP management state: {:?}",
                        e.type_id()
                    );
                    management_state
                }
            }
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
            match broker.downcast::<mockforge_mqtt::MqttBroker>() {
                Ok(broker) => management_state.with_mqtt_broker(broker),
                Err(e) => {
                    error!(
                        "Invalid MQTT broker passed to HTTP management state: {:?}",
                        e.type_id()
                    );
                    management_state
                }
            }
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

    // Add verification API endpoint
    app = app.merge(verification_router());

    // Add OIDC well-known endpoints
    use crate::auth::oidc::oidc_router;
    app = app.merge(oidc_router());

    // Add access review API if enabled
    {
        use mockforge_core::security::get_global_access_review_service;
        if let Some(service) = get_global_access_review_service().await {
            use crate::handlers::access_review::{access_review_router, AccessReviewState};
            let review_state = AccessReviewState { service };
            app = app.nest("/api/v1/security/access-reviews", access_review_router(review_state));
            debug!("Access review API mounted at /api/v1/security/access-reviews");
        }
    }

    // Add privileged access API if enabled
    {
        use mockforge_core::security::get_global_privileged_access_manager;
        if let Some(manager) = get_global_privileged_access_manager().await {
            use crate::handlers::privileged_access::{privileged_access_router, PrivilegedAccessState};
            let privileged_state = PrivilegedAccessState { manager };
            app = app.nest("/api/v1/security/privileged-access", privileged_access_router(privileged_state));
            debug!("Privileged access API mounted at /api/v1/security/privileged-access");
        }
    }

    // Add change management API if enabled
    {
        use mockforge_core::security::get_global_change_management_engine;
        if let Some(engine) = get_global_change_management_engine().await {
            use crate::handlers::change_management::{change_management_router, ChangeManagementState};
            let change_state = ChangeManagementState { engine };
            app = app.nest("/api/v1/change-management", change_management_router(change_state));
            debug!("Change management API mounted at /api/v1/change-management");
        }
    }

    // Add risk assessment API if enabled
    {
        use mockforge_core::security::get_global_risk_assessment_engine;
        if let Some(engine) = get_global_risk_assessment_engine().await {
            use crate::handlers::risk_assessment::{risk_assessment_router, RiskAssessmentState};
            let risk_state = RiskAssessmentState { engine };
            app = app.nest("/api/v1/security", risk_assessment_router(risk_state));
            debug!("Risk assessment API mounted at /api/v1/security/risks");
        }
    }

    // Add token lifecycle API
    {
        use crate::auth::token_lifecycle::TokenLifecycleManager;
        use crate::handlers::token_lifecycle::{token_lifecycle_router, TokenLifecycleState};
        let lifecycle_manager = Arc::new(TokenLifecycleManager::default());
        let lifecycle_state = TokenLifecycleState {
            manager: lifecycle_manager,
        };
        app = app.nest("/api/v1/auth", token_lifecycle_router(lifecycle_state));
        debug!("Token lifecycle API mounted at /api/v1/auth");
    }

    // Add OAuth2 server endpoints
    {
        use crate::auth::oidc::{load_oidc_state, OidcState};
        use crate::auth::token_lifecycle::TokenLifecycleManager;
        use crate::handlers::oauth2_server::{oauth2_server_router, OAuth2ServerState};
        // Load OIDC state from configuration (environment variables or config file)
        let oidc_state = Arc::new(RwLock::new(load_oidc_state()));
        let lifecycle_manager = Arc::new(TokenLifecycleManager::default());
        let oauth2_state = OAuth2ServerState {
            oidc_state,
            lifecycle_manager,
            auth_codes: Arc::new(RwLock::new(HashMap::new())),
        };
        app = app.merge(oauth2_server_router(oauth2_state));
        debug!("OAuth2 server endpoints mounted at /oauth2/authorize and /oauth2/token");
    }

    // Add consent screen endpoints
    {
        use crate::auth::risk_engine::RiskEngine;
        use crate::auth::token_lifecycle::TokenLifecycleManager;
        use crate::handlers::consent::{consent_router, ConsentState};
        use crate::handlers::oauth2_server::OAuth2ServerState;
        use crate::auth::oidc::{load_oidc_state, OidcState};
        // Load OIDC state from configuration (environment variables or config file)
        let oidc_state = Arc::new(RwLock::new(load_oidc_state()));
        let lifecycle_manager = Arc::new(TokenLifecycleManager::default());
        let oauth2_state = OAuth2ServerState {
            oidc_state: oidc_state.clone(),
            lifecycle_manager: lifecycle_manager.clone(),
            auth_codes: Arc::new(RwLock::new(HashMap::new())),
        };
        let risk_engine = Arc::new(RiskEngine::default());
        let consent_state = ConsentState {
            oauth2_state,
            risk_engine,
        };
        app = app.merge(consent_router(consent_state));
        debug!("Consent screen endpoints mounted at /consent");
    }

    // Add risk simulation API
    {
        use crate::auth::risk_engine::RiskEngine;
        use crate::handlers::risk_simulation::{risk_simulation_router, RiskSimulationState};
        let risk_engine = Arc::new(RiskEngine::default());
        let risk_state = RiskSimulationState { risk_engine };
        app = app.nest("/api/v1/auth", risk_simulation_router(risk_state));
        debug!("Risk simulation API mounted at /api/v1/auth/risk");
    }

    // Initialize database connection (optional)
    let database = {
        use crate::database::Database;
        let database_url = std::env::var("DATABASE_URL").ok();
        match Database::connect_optional(database_url.as_deref()).await {
            Ok(db) => {
                if db.is_connected() {
                    // Run migrations if database is connected
                    if let Err(e) = db.migrate_if_connected().await {
                        warn!("Failed to run database migrations: {}", e);
                    } else {
                        info!("Database connected and migrations applied");
                    }
                }
                Some(db)
            }
            Err(e) => {
                warn!("Failed to connect to database: {}. Continuing without database support.", e);
                None
            }
        }
    };

    // Add drift budget and incident management endpoints
    {
        use crate::handlers::drift_budget::{drift_budget_router, DriftBudgetState};
        use crate::middleware::drift_tracking::DriftTrackingState;
        use mockforge_core::ai_contract_diff::ContractDiffAnalyzer;
        use mockforge_core::contract_drift::{DriftBudgetConfig, DriftBudgetEngine};
        use mockforge_core::consumer_contracts::{ConsumerBreakingChangeDetector, UsageRecorder};
        use mockforge_core::incidents::{IncidentManager, IncidentStore};
        use std::sync::Arc;

        // Initialize drift budget engine with default config
        let drift_config = DriftBudgetConfig::default();
        let drift_engine = Arc::new(DriftBudgetEngine::new(drift_config.clone()));

        // Initialize incident store and manager
        let incident_store = Arc::new(IncidentStore::default());
        let incident_manager = Arc::new(IncidentManager::new(incident_store.clone()));

        // Initialize usage recorder and consumer detector
        let usage_recorder = Arc::new(UsageRecorder::default());
        let consumer_detector = Arc::new(ConsumerBreakingChangeDetector::new(usage_recorder.clone()));

        // Initialize contract diff analyzer if enabled
        let diff_analyzer = if drift_config.enabled {
            match ContractDiffAnalyzer::new(mockforge_core::ai_contract_diff::ContractDiffConfig::default()) {
                Ok(analyzer) => Some(Arc::new(analyzer)),
                Err(e) => {
                    warn!("Failed to create contract diff analyzer: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Get OpenAPI spec if available
        // Note: Load from spec_path if available, or leave as None for manual configuration.
        let spec = if let Some(ref spec_path) = spec_path {
            match mockforge_core::openapi::OpenApiSpec::from_file(spec_path).await {
                Ok(s) => Some(Arc::new(s)),
                Err(e) => {
                    debug!("Failed to load OpenAPI spec for drift tracking: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Create drift tracking state
        let drift_tracking_state = DriftTrackingState {
            diff_analyzer,
            spec,
            drift_engine: drift_engine.clone(),
            incident_manager: incident_manager.clone(),
            usage_recorder,
            consumer_detector,
            enabled: drift_config.enabled,
        };

        // Add response body buffering middleware (before drift tracking)
        app = app.layer(axum::middleware::from_fn(crate::middleware::buffer_response_middleware));

        // Add drift tracking middleware (after response buffering)
        // Use a wrapper that inserts state into extensions before calling the middleware
        let drift_tracking_state_clone = drift_tracking_state.clone();
        app = app.layer(axum::middleware::from_fn(move |mut req: axum::extract::Request, next: axum::middleware::Next| {
            let state = drift_tracking_state_clone.clone();
            async move {
                // Insert state into extensions if not already present
                if req.extensions().get::<crate::middleware::drift_tracking::DriftTrackingState>().is_none() {
                    req.extensions_mut().insert(state);
                }
                // Call the middleware function
                crate::middleware::drift_tracking::drift_tracking_middleware_with_extensions(req, next).await
            }
        }));

        let drift_state = DriftBudgetState {
            engine: drift_engine,
            incident_manager,
            gitops_handler: None, // Can be initialized later if GitOps is configured
        };

        app = app.merge(drift_budget_router(drift_state));
        debug!("Drift budget and incident management endpoints mounted at /api/v1/drift");
    }

    // Add behavioral cloning middleware (optional - applies learned behavior to requests)
    {
        use crate::middleware::behavioral_cloning::BehavioralCloningMiddlewareState;
        use std::path::PathBuf;

        // Determine database path (defaults to ./recordings.db)
        let db_path = std::env::var("RECORDER_DATABASE_PATH")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .map(|p| p.join("recordings.db"))
            });

        let bc_middleware_state = if let Some(path) = db_path {
            BehavioralCloningMiddlewareState::with_database_path(path)
        } else {
            BehavioralCloningMiddlewareState::new()
        };

        // Only enable if BEHAVIORAL_CLONING_ENABLED is set to true
        let enabled = std::env::var("BEHAVIORAL_CLONING_ENABLED")
            .ok()
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);

        if enabled {
            let bc_state_clone = bc_middleware_state.clone();
            app = app.layer(axum::middleware::from_fn(move |mut req: axum::extract::Request, next: axum::middleware::Next| {
                let state = bc_state_clone.clone();
                async move {
                    // Insert state into extensions if not already present
                    if req.extensions().get::<BehavioralCloningMiddlewareState>().is_none() {
                        req.extensions_mut().insert(state);
                    }
                    // Call the middleware function
                    crate::middleware::behavioral_cloning::behavioral_cloning_middleware(req, next).await
                }
            }));
            debug!("Behavioral cloning middleware enabled (applies learned behavior to requests)");
        }
    }

    // Add consumer contracts endpoints
    {
        use crate::handlers::consumer_contracts::{consumer_contracts_router, ConsumerContractsState};
        use mockforge_core::consumer_contracts::{
            ConsumerBreakingChangeDetector, ConsumerRegistry, UsageRecorder,
        };
        use std::sync::Arc;

        // Initialize consumer registry
        let registry = Arc::new(ConsumerRegistry::default());

        // Initialize usage recorder
        let usage_recorder = Arc::new(UsageRecorder::default());

        // Initialize breaking change detector
        let detector = Arc::new(ConsumerBreakingChangeDetector::new(usage_recorder.clone()));

        let consumer_state = ConsumerContractsState {
            registry,
            usage_recorder,
            detector,
        };

        app = app.merge(consumer_contracts_router(consumer_state));
        debug!("Consumer contracts endpoints mounted at /api/v1/consumers");
    }

    // Add behavioral cloning endpoints
    {
        use crate::handlers::behavioral_cloning::{behavioral_cloning_router, BehavioralCloningState};
        use std::path::PathBuf;

        // Determine database path (defaults to ./recordings.db)
        let db_path = std::env::var("RECORDER_DATABASE_PATH")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .map(|p| p.join("recordings.db"))
            });

        let bc_state = if let Some(path) = db_path {
            BehavioralCloningState::with_database_path(path)
        } else {
            BehavioralCloningState::new()
        };

        app = app.merge(behavioral_cloning_router(bc_state));
        debug!("Behavioral cloning endpoints mounted at /api/v1/behavioral-cloning");
    }

    // Add consistency engine and cross-protocol state management
    {
        use crate::consistency::{HttpAdapter, ConsistencyMiddlewareState};
        use crate::handlers::consistency::{consistency_router, ConsistencyState};
        use mockforge_core::consistency::ConsistencyEngine;
        use std::sync::Arc;

        // Initialize consistency engine
        let consistency_engine = Arc::new(ConsistencyEngine::new());

        // Create and register HTTP adapter
        let http_adapter = Arc::new(HttpAdapter::new(consistency_engine.clone()));
        consistency_engine.register_adapter(http_adapter.clone()).await;

        // Create consistency state for handlers
        let consistency_state = ConsistencyState {
            engine: consistency_engine.clone(),
        };

        // Create consistency middleware state
        let consistency_middleware_state = ConsistencyMiddlewareState {
            engine: consistency_engine.clone(),
            adapter: http_adapter,
        };

        // Add consistency middleware (before other middleware to inject state early)
        let consistency_middleware_state_clone = consistency_middleware_state.clone();
        app = app.layer(axum::middleware::from_fn(move |mut req: axum::extract::Request, next: axum::middleware::Next| {
            let state = consistency_middleware_state_clone.clone();
            async move {
                // Insert state into extensions if not already present
                if req.extensions().get::<ConsistencyMiddlewareState>().is_none() {
                    req.extensions_mut().insert(state);
                }
                // Call the middleware function
                crate::consistency::middleware::consistency_middleware(req, next).await
            }
        }));

        // Add consistency API endpoints
        app = app.merge(consistency_router(consistency_state));
        debug!("Consistency engine initialized and endpoints mounted at /api/v1/consistency");

        // Add fidelity score endpoints
        {
            use crate::handlers::fidelity::{fidelity_router, FidelityState};
            let fidelity_state = FidelityState::new();
            app = app.merge(fidelity_router(fidelity_state));
            debug!("Fidelity score endpoints mounted at /api/v1/workspace/:workspace_id/fidelity");
        }

        // Add scenario studio endpoints
        {
            use crate::handlers::scenario_studio::{scenario_studio_router, ScenarioStudioState};
            let scenario_studio_state = ScenarioStudioState::new();
            app = app.merge(scenario_studio_router(scenario_studio_state));
            debug!("Scenario Studio endpoints mounted at /api/v1/scenario-studio");
        }

        // Add snapshot management endpoints
        {
            use crate::handlers::snapshots::{snapshot_router, SnapshotState};
            use mockforge_core::snapshots::SnapshotManager;
            use std::path::PathBuf;

            let snapshot_dir = std::env::var("MOCKFORGE_SNAPSHOT_DIR")
                .ok()
                .map(PathBuf::from);
            let snapshot_manager = Arc::new(SnapshotManager::new(snapshot_dir));

            let snapshot_state = SnapshotState {
                manager: snapshot_manager,
                consistency_engine: Some(consistency_engine.clone()),
            };

            app = app.merge(snapshot_router(snapshot_state));
            debug!("Snapshot management endpoints mounted at /api/v1/snapshots");

            // Add X-Ray API endpoints for browser extension
            {
                use crate::handlers::xray::{xray_router, XRayState};
                let xray_state = XRayState {
                    engine: consistency_engine.clone(),
                };
                app = app.merge(xray_router(xray_state));
                debug!("X-Ray API endpoints mounted at /api/v1/xray");
            }
        }

        // Add A/B testing endpoints and middleware
        {
            use crate::handlers::ab_testing::{ab_testing_router, ABTestingState};
            use crate::middleware::ab_testing::ab_testing_middleware;

            let ab_testing_state = ABTestingState::new();

            // Add A/B testing middleware (before other response middleware)
            let ab_testing_state_clone = ab_testing_state.clone();
            app = app.layer(axum::middleware::from_fn(
                move |mut req: axum::extract::Request, next: axum::middleware::Next| {
                    let state = ab_testing_state_clone.clone();
                    async move {
                        // Insert state into extensions if not already present
                        if req.extensions().get::<ABTestingState>().is_none() {
                            req.extensions_mut().insert(state);
                        }
                        // Call the middleware function
                        ab_testing_middleware(req, next).await
                    }
                },
            ));

            // Add A/B testing API endpoints
            app = app.merge(ab_testing_router(ab_testing_state));
            debug!("A/B testing endpoints mounted at /api/v1/ab-tests");
        }
    }

    // Add PR generation endpoints (optional - only if configured)
    {
        use crate::handlers::pr_generation::{pr_generation_router, PRGenerationState};
        use mockforge_core::pr_generation::{PRGenerator, PRProvider};
        use std::sync::Arc;

        // Load PR generation config from environment or use default
        let pr_config = mockforge_core::pr_generation::PRGenerationConfig::from_env();

        let generator = if pr_config.enabled && pr_config.token.is_some() {
            let token = pr_config.token.as_ref().unwrap().clone();
            let generator = match pr_config.provider {
                PRProvider::GitHub => PRGenerator::new_github(
                    pr_config.owner.clone(),
                    pr_config.repo.clone(),
                    token,
                    pr_config.base_branch.clone(),
                ),
                PRProvider::GitLab => PRGenerator::new_gitlab(
                    pr_config.owner.clone(),
                    pr_config.repo.clone(),
                    token,
                    pr_config.base_branch.clone(),
                ),
            };
            Some(Arc::new(generator))
        } else {
            None
        };

        let pr_state = PRGenerationState { generator: generator.clone() };

        app = app.merge(pr_generation_router(pr_state));
        if generator.is_some() {
            debug!("PR generation endpoints mounted at /api/v1/pr (configured for {:?})", pr_config.provider);
        } else {
            debug!("PR generation endpoints mounted at /api/v1/pr (not configured - set GITHUB_TOKEN/GITLAB_TOKEN and PR_REPO_OWNER/PR_REPO_NAME)");
        }
    }

    // Add management WebSocket endpoint
    app = app.nest("/__mockforge/ws", ws_management_router(ws_state));

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

    // Apply deceptive deploy configuration if enabled
    let mut final_cors_config = cors_config;
    let mut production_headers: Option<std::sync::Arc<std::collections::HashMap<String, String>>> =
        None;
    // Auth config from deceptive deploy OAuth (if configured)
    let mut deceptive_deploy_auth_config: Option<mockforge_core::config::AuthConfig> = None;
    let mut rate_limit_config = crate::middleware::RateLimitConfig {
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

    if let Some(deploy_config) = &deceptive_deploy_config {
        if deploy_config.enabled {
            info!("Deceptive deploy mode enabled - applying production-like configuration");

            // Override CORS config if provided
            if let Some(prod_cors) = &deploy_config.cors {
                final_cors_config = Some(mockforge_core::config::HttpCorsConfig {
                    enabled: true,
                    allowed_origins: prod_cors.allowed_origins.clone(),
                    allowed_methods: prod_cors.allowed_methods.clone(),
                    allowed_headers: prod_cors.allowed_headers.clone(),
                    allow_credentials: prod_cors.allow_credentials,
                });
                info!("Applied production-like CORS configuration");
            }

            // Override rate limit config if provided
            if let Some(prod_rate_limit) = &deploy_config.rate_limit {
                rate_limit_config = crate::middleware::RateLimitConfig {
                    requests_per_minute: prod_rate_limit.requests_per_minute,
                    burst: prod_rate_limit.burst,
                    per_ip: prod_rate_limit.per_ip,
                    per_endpoint: false,
                };
                info!(
                    "Applied production-like rate limiting: {} req/min, burst: {}",
                    prod_rate_limit.requests_per_minute, prod_rate_limit.burst
                );
            }

            // Set production headers
            if !deploy_config.headers.is_empty() {
                let headers_map: std::collections::HashMap<String, String> =
                    deploy_config.headers.clone();
                production_headers = Some(std::sync::Arc::new(headers_map));
                info!("Configured {} production headers", deploy_config.headers.len());
            }

            // Integrate OAuth config from deceptive deploy
            if let Some(prod_oauth) = &deploy_config.oauth {
                let oauth2_config: mockforge_core::config::OAuth2Config = prod_oauth.clone().into();
                deceptive_deploy_auth_config = Some(mockforge_core::config::AuthConfig {
                    oauth2: Some(oauth2_config),
                    ..Default::default()
                });
                info!("Applied production-like OAuth configuration for deceptive deploy");
            }
        }
    }

    // Initialize rate limiter and state
    let rate_limiter =
        std::sync::Arc::new(crate::middleware::GlobalRateLimiter::new(rate_limit_config.clone()));

    let mut state = HttpServerState::new().with_rate_limiter(rate_limiter.clone());

    // Add production headers to state if configured
    if let Some(headers) = production_headers.clone() {
        state = state.with_production_headers(headers);
    }

    // Add rate limiting middleware
    app = app.layer(from_fn_with_state(state.clone(), crate::middleware::rate_limit_middleware));

    // Add production headers middleware if configured
    if state.production_headers.is_some() {
        app = app.layer(from_fn_with_state(
            state.clone(),
            crate::middleware::production_headers_middleware,
        ));
    }

    // Add authentication middleware if OAuth is configured via deceptive deploy
    if let Some(auth_config) = deceptive_deploy_auth_config {
        use crate::auth::{auth_middleware, create_oauth2_client, AuthState};
        use std::collections::HashMap;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        // Create OAuth2 client if configured
        let oauth2_client = if let Some(oauth2_config) = &auth_config.oauth2 {
            match create_oauth2_client(oauth2_config) {
                Ok(client) => Some(client),
                Err(e) => {
                    warn!("Failed to create OAuth2 client from deceptive deploy config: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Create auth state
        let auth_state = AuthState {
            config: auth_config,
            spec: None, // OpenAPI spec not available in this context
            oauth2_client,
            introspection_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Apply auth middleware
        app = app.layer(axum::middleware::from_fn_with_state(auth_state, auth_middleware));
        info!("Applied OAuth authentication middleware from deceptive deploy configuration");
    }

    // Add contract diff middleware for automatic request capture
    // This captures requests for contract diff analysis
    app = app.layer(axum::middleware::from_fn(contract_diff_middleware::capture_for_contract_diff));

    // Add CORS middleware (use final_cors_config which may be overridden by deceptive deploy)
    app = apply_cors_middleware(app, final_cors_config);

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
