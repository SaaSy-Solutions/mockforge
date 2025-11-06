//! HTTP integration layer
//!
//! This module provides integration with the existing HTTP server (mockforge-http)
//! for route registration and middleware integration.

use crate::Result;
use axum::{
    extract::Extension,
    routing::{delete, get, patch, post, put},
    Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

/// Create router for VBR endpoints
///
/// This creates a router with generic entity routes that can be merged into
/// the main mockforge-http router. The HandlerContext must be provided via
/// Extension when the router is used.
///
/// # Example
/// ```no_run
/// use mockforge_vbr::integration::create_vbr_router;
/// use axum::Router;
///
/// let vbr_router = create_vbr_router("/vbr-api").unwrap();
/// let app = Router::new().merge(vbr_router);
/// ```
pub fn create_vbr_router(api_prefix: &str) -> Result<Router> {
    let router = Router::new()
        // Health check endpoint
        .route(
            &format!("{}/health", api_prefix),
            get(|| async { "OK" }),
        )
        // Entity list endpoint (will be registered per entity)
        // GET /api/{entity}
        .route(
            &format!("{}/:entity", api_prefix),
            get(crate::handlers::list_handler),
        )
        // Entity create endpoint
        // POST /api/{entity}
        .route(
            &format!("{}/:entity", api_prefix),
            post(crate::handlers::create_handler),
        )
        // Entity get by ID endpoint
        // GET /api/{entity}/{id}
        .route(
            &format!("{}/:entity/:id", api_prefix),
            get(crate::handlers::get_handler),
        )
        // Entity update endpoint (PUT)
        // PUT /api/{entity}/{id}
        .route(
            &format!("{}/:entity/:id", api_prefix),
            put(crate::handlers::update_handler),
        )
        // Entity partial update endpoint (PATCH)
        // PATCH /api/{entity}/{id}
        .route(
            &format!("{}/:entity/:id", api_prefix),
            patch(crate::handlers::patch_handler),
        )
        // Entity delete endpoint
        // DELETE /api/{entity}/{id}
        .route(
            &format!("{}/:entity/:id", api_prefix),
            delete(crate::handlers::delete_handler),
        )
        // Relationship endpoint
        // GET /api/{entity}/{id}/{relationship}
        .route(
            &format!("{}/:entity/:id/:relationship", api_prefix),
            get(crate::handlers::get_relationship_handler),
        )
        .layer(CorsLayer::permissive());

    Ok(router)
}

/// Create a VBR router with handler context
///
/// This is a convenience function that creates a router with the HandlerContext
/// already provided via Extension. Use this when you have a VbrEngine ready.
pub fn create_vbr_router_with_context(
    api_prefix: &str,
    context: crate::handlers::HandlerContext,
) -> Result<Router> {
    let router = create_vbr_router(api_prefix)?;
    Ok(router.layer(ServiceBuilder::new().layer(Extension(context)).into_inner()))
}

/// Register VBR routes dynamically for each entity
///
/// Adds entity-specific routes to an existing router. This allows you to
/// register routes for individual entities as they are added to the registry.
pub fn register_entity_routes(router: Router, entity_name: &str, api_prefix: &str) -> Router {
    router
        // List all entities
        .route(
            &format!("{}/{}", api_prefix, entity_name.to_lowercase()),
            get(crate::handlers::list_handler),
        )
        // Create entity
        .route(
            &format!("{}/{}", api_prefix, entity_name.to_lowercase()),
            post(crate::handlers::create_handler),
        )
        // Get entity by ID
        .route(
            &format!("{}/{}/:id", api_prefix, entity_name.to_lowercase()),
            get(crate::handlers::get_handler),
        )
        // Update entity (PUT)
        .route(
            &format!("{}/{}/:id", api_prefix, entity_name.to_lowercase()),
            put(crate::handlers::update_handler),
        )
        // Partial update entity (PATCH)
        .route(
            &format!("{}/{}/:id", api_prefix, entity_name.to_lowercase()),
            patch(crate::handlers::patch_handler),
        )
        // Delete entity
        .route(
            &format!("{}/{}/:id", api_prefix, entity_name.to_lowercase()),
            delete(crate::handlers::delete_handler),
        )
        // Get relationship endpoint
        // GET /api/{entity}/{id}/{relationship}
        .route(
            &format!("{}/{}/:id/:relationship", api_prefix, entity_name.to_lowercase()),
            get(crate::handlers::get_relationship_handler),
        )
}

/// Integration helper for mockforge-http
///
/// This function can be called from mockforge-http to integrate VBR routes
/// into the main application router. It takes an existing router and merges
/// VBR routes into it.
pub fn integrate_vbr_routes(
    app: Router,
    api_prefix: &str,
    context: crate::handlers::HandlerContext,
) -> Result<Router> {
    let vbr_router = create_vbr_router_with_context(api_prefix, context)?;
    Ok(app.merge(vbr_router))
}
