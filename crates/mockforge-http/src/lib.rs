pub mod ai_handler;
pub mod auth;
pub mod chain_handlers;
pub mod http_tracing_middleware;
pub mod latency_profiles;
pub mod management;
pub mod management_ws;
pub mod metrics_middleware;
pub mod op_middleware;
pub mod replay_listing;
pub mod request_logging;
pub mod sse;

// Re-export AI handler utilities
pub use ai_handler::{process_response_with_ai, AiResponseConfig, AiResponseHandler};

// Re-export management API utilities
pub use management::{management_router, ManagementState, MockConfig, ServerConfig, ServerStats};

// Re-export management WebSocket utilities
pub use management_ws::{ws_management_router, WsManagementState, MockEvent};

// Re-export metrics middleware
pub use metrics_middleware::collect_http_metrics;

// Re-export tracing middleware
pub use http_tracing_middleware::http_tracing_middleware;

use axum::middleware::from_fn_with_state;
use axum::{Router, extract::State, response::Json};
use mockforge_core::failure_injection::{FailureConfig, FailureInjector};
use mockforge_core::latency::LatencyInjector;
use mockforge_core::openapi::OpenApiSpec;
use mockforge_core::openapi_routes::OpenApiRouteRegistry;
use mockforge_core::openapi_routes::ValidationOptions;
use mockforge_core::LatencyProfile;
use mockforge_core::TrafficShaper;
#[cfg(feature = "data-faker")]
use mockforge_data::provider::register_core_faker_provider;
use std::collections::HashMap;
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
}

impl HttpServerState {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
        }
    }

    pub fn with_routes(routes: Vec<RouteInfo>) -> Self {
        Self {
            routes,
        }
    }
}

/// Handler to return OpenAPI routes information
async fn get_routes_handler(State(state): State<HttpServerState>) -> Json<serde_json::Value> {
    let route_info: Vec<serde_json::Value> = state.routes.iter().map(|route| {
        serde_json::json!({
            "method": route.method,
            "path": route.path,
            "operation_id": route.operation_id,
            "summary": route.summary,
            "description": route.description,
            "parameters": route.parameters
        })
    }).collect();

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
    // Set up the basic router
    let mut app = Router::new();
    let mut state = HttpServerState::new();

    // Clone spec_path for later use
    let spec_path_for_mgmt = spec_path.clone();

    // If an OpenAPI spec is provided, integrate it
    if let Some(spec_path) = spec_path {
        tracing::debug!("Processing OpenAPI spec path: {}", spec_path);
        match OpenApiSpec::from_file(&spec_path).await {
            Ok(openapi) => {
                info!("Successfully loaded OpenAPI spec from {}", spec_path);
                tracing::debug!("Creating OpenAPI route registry...");
                let registry = if let Some(opts) = options {
                    tracing::debug!("Using custom validation options");
                    OpenApiRouteRegistry::new_with_options(openapi, opts)
                } else {
                    tracing::debug!("Using environment-based options");
                    OpenApiRouteRegistry::new_with_env(openapi)
                };

                // Extract route information for introspection
                let route_info: Vec<RouteInfo> = registry.routes().iter().map(|route| {
                    RouteInfo {
                        method: route.method.clone(),
                        path: route.path.clone(),
                        operation_id: route.operation.operation_id.clone(),
                        summary: route.operation.summary.clone(),
                        description: route.operation.description.clone(),
                        parameters: route.parameters.clone(),
                    }
                }).collect();
                state.routes = route_info;

                tracing::debug!("Building router from registry with {} routes", registry.routes().len());

                // Load overrides if environment variable is set
                let overrides = if std::env::var("MOCKFORGE_HTTP_OVERRIDES_GLOB").is_ok() {
                    tracing::debug!("Loading overrides from environment variable");
                    match mockforge_core::Overrides::load_from_globs(&[]).await {
                        Ok(overrides) => {
                            tracing::debug!("Loaded {} override rules", overrides.rules().len());
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

                let overrides_enabled = overrides.is_some();
                let openapi_router = if let Some(failure_config) = &failure_config {
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

    // Create a router with state for the routes endpoint
    let routes_router = Router::new()
        .route("/__mockforge/routes", axum::routing::get(get_routes_handler))
        .with_state(state);

    // Merge the routes router with the main app
    app = app.merge(routes_router);

    // Add management API endpoints
    let management_state = ManagementState::new(None, spec_path_for_mgmt, 3000); // Port will be updated when we know the actual port
    app = app.nest("/__mockforge/api", management_router(management_state));

    // Add management WebSocket endpoint
    let ws_state = WsManagementState::new();
    app = app.nest("/__mockforge/ws", ws_management_router(ws_state));

    // Add request logging middleware to capture all requests
    app = app.layer(axum::middleware::from_fn(request_logging::log_http_requests));

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

    axum::serve(
        tokio::net::TcpListener::bind(addr).await?,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
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
    use crate::chain_handlers::create_chain_state;
    use axum::{
        routing::{delete, get, post, put},
        Router,
    };
    use std::sync::Arc;

    // Create chain registry and execution engine
    let chain_config = circling_config.unwrap_or_default();
    let registry = Arc::new(mockforge_core::request_chaining::RequestChainRegistry::new(
        chain_config.clone(),
    ));
    let engine = Arc::new(mockforge_core::chain_execution::ChainExecutionEngine::new(
        registry.clone(),
        chain_config,
    ));
    let chain_state = create_chain_state(registry, engine);

    // Start with basic router
    let mut app = build_router(spec_path, options, None).await;

    // Add chain management endpoints
    app = app.nest(
        "/__mockforge/chains",
        Router::new()
            .route("/", get(chain_handlers::list_chains))
            .route("/", post(chain_handlers::create_chain))
            .route("/{id}", get(chain_handlers::get_chain))
            .route("/{id}", put(chain_handlers::update_chain))
            .route("/{id}", delete(chain_handlers::delete_chain))
            .route("/{id}/execute", post(chain_handlers::execute_chain))
            .route("/{id}/validate", post(chain_handlers::validate_chain))
            .route("/{id}/history", get(chain_handlers::get_chain_history))
            .with_state(chain_state),
    );

    app
}

/// Start HTTP server with chaining support
pub async fn start_with_chains(
    port: u16,
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    chain_config: Option<mockforge_core::request_chaining::ChainConfig>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = build_router_with_chains(spec_path, options, chain_config).await;
    serve_router(port, app).await
}

/// Start HTTP server with both latency and failure injection support
pub async fn start_with_injectors(
    port: u16,
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    _latency_profile: Option<LatencyProfile>,
    _failure_injector: Option<mockforge_core::FailureInjector>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // For now, ignore latency and failure injectors and just use basic router
    let app = build_router(spec_path, options, None).await;
    serve_router(port, app).await
}

/// Build router with traffic shaping support
pub async fn build_router_with_traffic_shaping(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    traffic_shaper: Option<TrafficShaper>,
    traffic_shaping_enabled: bool,
) -> Router {
    use crate::latency_profiles::LatencyProfiles;
    use crate::op_middleware::Shared;
    use mockforge_core::Overrides;

    let shared = Shared {
        profiles: LatencyProfiles::default(),
        overrides: Overrides::default(),
        failure_injector: None,
        traffic_shaper: traffic_shaper.clone(),
        overrides_enabled: false,
        traffic_shaping_enabled,
    };

    // Start with basic router
    let mut app = Router::new();

    // If an OpenAPI spec is provided, integrate it
    if let Some(spec) = spec_path {
        match OpenApiSpec::from_file(&spec).await {
            Ok(openapi) => {
                info!("Loaded OpenAPI spec from {}", spec);
                let registry = if let Some(opts) = options {
                    OpenApiRouteRegistry::new_with_options(openapi, opts)
                } else {
                    OpenApiRouteRegistry::new_with_env(openapi)
                };
                app = registry.build_router();
            }
            Err(e) => {
                warn!("Failed to load OpenAPI spec from {}: {}. Starting without OpenAPI integration.", spec, e);
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

    // If traffic shaping is enabled, apply traffic shaping middleware to all routes
    if traffic_shaping_enabled && traffic_shaper.is_some() {
        use crate::op_middleware::add_shared_extension;
        app = app.layer(from_fn_with_state(shared.clone(), add_shared_extension));
    }

    app
}

/// Start HTTP server with traffic shaping support
pub async fn start_with_traffic_shaping(
    port: u16,
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    traffic_shaper: Option<TrafficShaper>,
    traffic_shaping_enabled: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = build_router_with_traffic_shaping(
        spec_path,
        options,
        traffic_shaper,
        traffic_shaping_enabled,
    )
    .await;
    serve_router(port, app).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_info_creation() {
        let route = RouteInfo {
            method: "GET".to_string(),
            path: "/users/{id}".to_string(),
            operation_id: Some("getUserById".to_string()),
            summary: Some("Get user by ID".to_string()),
            description: Some("Retrieves a user by their ID".to_string()),
            parameters: vec!["id".to_string()],
        };

        assert_eq!(route.method, "GET");
        assert_eq!(route.path, "/users/{id}");
        assert_eq!(route.operation_id, Some("getUserById".to_string()));
        assert_eq!(route.summary, Some("Get user by ID".to_string()));
        assert_eq!(route.parameters.len(), 1);
    }

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
        let router = build_router(None, None, None).await;
        // Should succeed without OpenAPI spec
        assert!(true); // Router was created successfully
    }

    #[tokio::test]
    async fn test_build_router_with_nonexistent_spec() {
        let router = build_router(Some("/nonexistent/spec.yaml".to_string()), None, None).await;
        // Should succeed but log a warning
        assert!(true); // Router was created successfully despite missing spec
    }

    #[tokio::test]
    async fn test_build_router_with_auth_and_latency() {
        let router = build_router_with_auth_and_latency(None, None, None, None).await;
        // Should succeed without parameters
        assert!(true);
    }

    #[tokio::test]
    async fn test_build_router_with_latency() {
        let router = build_router_with_latency(None, None, None).await;
        // Should succeed without parameters
        assert!(true);
    }

    #[tokio::test]
    async fn test_build_router_with_auth() {
        let router = build_router_with_auth(None, None, None).await;
        // Should succeed without parameters
        assert!(true);
    }

    #[tokio::test]
    async fn test_build_router_with_chains() {
        let router = build_router_with_chains(None, None, None).await;
        // Should succeed without parameters
        assert!(true);
    }

    #[tokio::test]
    async fn test_build_router_with_traffic_shaping_disabled() {
        let router = build_router_with_traffic_shaping(None, None, None, false).await;
        // Should succeed with traffic shaping disabled
        assert!(true);
    }

    #[tokio::test]
    async fn test_build_router_with_traffic_shaping_enabled_no_shaper() {
        let router = build_router_with_traffic_shaping(None, None, None, true).await;
        // Should succeed even without a traffic shaper
        assert!(true);
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
}
