pub mod latency_profiles;
pub mod op_middleware;
pub mod overrides;
pub mod replay_listing;
pub mod schema_diff;

use axum::Router;
use mockforge_core::{OpenApiRouteRegistry, OpenApiSpec};
use mockforge_core::openapi_routes::{ValidationMode, ValidationOptions};
use tracing::*;

/// Build the base HTTP router, optionally from an OpenAPI spec.
pub async fn build_router(spec_path: Option<String>, options: Option<ValidationOptions>) -> Router {
    // Set up the basic router
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
                // Fall back to basic router
            }
        }
    }

    // Add basic health check endpoint if not already provided by OpenAPI spec
    app.route(
        "/health",
        axum::routing::get(|| async {
            use mockforge_core::server_utils::health::HealthStatus;
            axum::Json(serde_json::to_value(HealthStatus::healthy(0, "mockforge-http")).unwrap())
        }),
    )
}

/// Serve a provided router on the given port.
pub async fn serve_router(
    port: u16,
    app: Router,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = mockforge_core::wildcard_socket_addr(port);
    info!("HTTP listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

/// Backwards-compatible start that builds + serves the base router.
pub async fn start(
    port: u16,
    spec_path: Option<String>,
    options: Option<ValidationOptions>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = build_router(spec_path, options).await;
    serve_router(port, app).await
}
