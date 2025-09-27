pub mod auth;
pub mod chain_handlers;
pub mod latency_profiles;
pub mod op_middleware;
pub mod replay_listing;
pub mod request_logging;
pub mod sse;

use axum::middleware::from_fn_with_state;
use axum::Router;
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

/// Build the base HTTP router, optionally from an OpenAPI spec.
pub async fn build_router(
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
    failure_config: Option<FailureConfig>,
) -> Router {
    // Set up the basic router
    let mut app = Router::new();

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

                app = if let Some(failure_config) = &failure_config {
                    let failure_injector = FailureInjector::new(Some(failure_config.clone()), true);
                    registry.build_router_with_injectors(
                        LatencyInjector::default(),
                        Some(failure_injector),
                    )
                } else {
                    registry.build_router()
                };
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
            .route("/:id", get(chain_handlers::get_chain))
            .route("/:id", put(chain_handlers::update_chain))
            .route("/:id", delete(chain_handlers::delete_chain))
            .route("/:id/execute", post(chain_handlers::execute_chain))
            .route("/:id/validate", post(chain_handlers::validate_chain))
            .route("/:id/history", get(chain_handlers::get_chain_history))
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
