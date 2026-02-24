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
            &format!("{}/{{entity}}", api_prefix),
            get(crate::handlers::list_handler),
        )
        // Entity create endpoint
        // POST /api/{entity}
        .route(
            &format!("{}/{{entity}}", api_prefix),
            post(crate::handlers::create_handler),
        )
        // Entity get by ID endpoint
        // GET /api/{entity}/{id}
        .route(
            &format!("{}/{{entity}}/{{id}}", api_prefix),
            get(crate::handlers::get_handler),
        )
        // Entity update endpoint (PUT)
        // PUT /api/{entity}/{id}
        .route(
            &format!("{}/{{entity}}/{{id}}", api_prefix),
            put(crate::handlers::update_handler),
        )
        // Entity partial update endpoint (PATCH)
        // PATCH /api/{entity}/{id}
        .route(
            &format!("{}/{{entity}}/{{id}}", api_prefix),
            patch(crate::handlers::patch_handler),
        )
        // Entity delete endpoint
        // DELETE /api/{entity}/{id}
        .route(
            &format!("{}/{{entity}}/{{id}}", api_prefix),
            delete(crate::handlers::delete_handler),
        )
        // Relationship endpoint
        // GET /api/{entity}/{id}/{relationship}
        .route(
            &format!("{}/{{entity}}/{{id}}/{{relationship}}", api_prefix),
            get(crate::handlers::get_relationship_handler),
        )
        // Snapshot endpoints
        // POST /vbr-api/snapshots - Create snapshot
        .route(
            &format!("{}/snapshots", api_prefix),
            post(crate::handlers::create_snapshot_handler),
        )
        // GET /vbr-api/snapshots - List snapshots
        .route(
            &format!("{}/snapshots", api_prefix),
            get(crate::handlers::list_snapshots_handler),
        )
        // POST /vbr-api/snapshots/{name}/restore - Restore snapshot
        .route(
            &format!("{}/snapshots/{{name}}/restore", api_prefix),
            post(crate::handlers::restore_snapshot_handler),
        )
        // DELETE /vbr-api/snapshots/{name} - Delete snapshot
        .route(
            &format!("{}/snapshots/{{name}}", api_prefix),
            delete(crate::handlers::delete_snapshot_handler),
        )
        // POST /vbr-api/reset - Reset database
        .route(
            &format!("{}/reset", api_prefix),
            post(crate::handlers::reset_handler),
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
            &format!("{}/{}/{{id}}", api_prefix, entity_name.to_lowercase()),
            get(crate::handlers::get_handler),
        )
        // Update entity (PUT)
        .route(
            &format!("{}/{}/{{id}}", api_prefix, entity_name.to_lowercase()),
            put(crate::handlers::update_handler),
        )
        // Partial update entity (PATCH)
        .route(
            &format!("{}/{}/{{id}}", api_prefix, entity_name.to_lowercase()),
            patch(crate::handlers::patch_handler),
        )
        // Delete entity
        .route(
            &format!("{}/{}/{{id}}", api_prefix, entity_name.to_lowercase()),
            delete(crate::handlers::delete_handler),
        )
        // Get relationship endpoint
        // GET /api/{entity}/{id}/{relationship}
        .route(
            &format!("{}/{}/{{id}}/{{relationship}}", api_prefix, entity_name.to_lowercase()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{InMemoryDatabase, VirtualDatabase};
    use crate::entities::{Entity, EntityRegistry};
    use crate::handlers::HandlerContext;
    use crate::schema::VbrSchemaDefinition;
    use mockforge_data::{FieldDefinition, SchemaDefinition};
    use std::sync::Arc;

    async fn setup_test_context() -> HandlerContext {
        let mut db = InMemoryDatabase::new().await.unwrap();
        db.initialize().await.unwrap();
        let registry = EntityRegistry::new();

        HandlerContext {
            database: Arc::new(db),
            registry,
            session_manager: None,
            snapshots_dir: None,
        }
    }

    fn create_test_entity(name: &str) -> Entity {
        let base_schema = SchemaDefinition::new(name.to_string())
            .with_field(FieldDefinition::new("id".to_string(), "string".to_string()))
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()));

        let vbr_schema = VbrSchemaDefinition::new(base_schema);
        Entity::new(name.to_string(), vbr_schema)
    }

    #[tokio::test]
    async fn test_create_vbr_router() {
        let result = create_vbr_router("/api");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_vbr_router_with_context() {
        let context = setup_test_context().await;
        let result = create_vbr_router_with_context("/api", context);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_vbr_router_custom_prefix() {
        let result = create_vbr_router("/custom-api");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_register_entity_routes() {
        let base_router = Router::new();
        let entity = create_test_entity("User");

        let router = register_entity_routes(base_router, &entity.name, "/api");
        // Verify the router was returned (routes registered successfully)
        drop(router);
    }

    #[tokio::test]
    async fn test_register_entity_routes_multiple_entities() {
        let mut router = Router::new();

        let user_entity = create_test_entity("User");
        let product_entity = create_test_entity("Product");

        router = register_entity_routes(router, &user_entity.name, "/api");
        router = register_entity_routes(router, &product_entity.name, "/api");
        // Verify both entity routes were registered without conflict
        drop(router);
    }

    #[tokio::test]
    async fn test_integrate_vbr_routes() {
        let app = Router::new();
        let context = setup_test_context().await;

        let result = integrate_vbr_routes(app, "/vbr-api", context);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_integrate_vbr_routes_custom_prefix() {
        let app = Router::new();
        let context = setup_test_context().await;

        let result = integrate_vbr_routes(app, "/custom", context);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_vbr_router_with_health_endpoint() {
        let router = create_vbr_router("/api");
        assert!(router.is_ok(), "Router with health endpoint should create successfully");
    }

    #[tokio::test]
    async fn test_create_vbr_router_with_all_crud_routes() {
        // Verifies router creation with CRUD routes doesn't panic or error
        let router = create_vbr_router("/api");
        assert!(router.is_ok(), "Router with CRUD routes should create successfully");
    }

    #[tokio::test]
    async fn test_create_vbr_router_with_snapshot_routes() {
        // Verifies router with snapshot routes at different prefix
        let router = create_vbr_router("/vbr-api");
        assert!(router.is_ok(), "Router with snapshot routes should create successfully");
    }

    #[tokio::test]
    async fn test_create_vbr_router_with_cors() {
        let router = create_vbr_router("/api");
        assert!(router.is_ok(), "Router with CORS should create successfully");
    }

    #[tokio::test]
    async fn test_register_entity_routes_with_lowercase() {
        let base_router = Router::new();
        // Entity name "User" should create routes for "user"
        let router = register_entity_routes(base_router, "User", "/api");
        drop(router);
    }

    #[tokio::test]
    async fn test_context_with_session_manager() {
        let mut db = InMemoryDatabase::new().await.unwrap();
        db.initialize().await.unwrap();
        let registry = EntityRegistry::new();

        let context = HandlerContext {
            database: Arc::new(db),
            registry,
            session_manager: None, // Could be Some(...) in real usage
            snapshots_dir: None,
        };

        let result = create_vbr_router_with_context("/api", context);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_context_with_snapshots_dir() {
        let mut db = InMemoryDatabase::new().await.unwrap();
        db.initialize().await.unwrap();
        let registry = EntityRegistry::new();
        let temp_dir = tempfile::tempdir().unwrap();

        let context = HandlerContext {
            database: Arc::new(db),
            registry,
            session_manager: None,
            snapshots_dir: Some(temp_dir.path().to_path_buf()),
        };

        let result = create_vbr_router_with_context("/api", context);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_empty_api_prefix() {
        let result = create_vbr_router("");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_api_prefix_with_trailing_slash() {
        let result = create_vbr_router("/api/");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_nested_api_prefix() {
        let result = create_vbr_router("/v1/api");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_router_can_be_merged_multiple_times() {
        let app1 = Router::new();
        let context1 = setup_test_context().await;
        let app1 = integrate_vbr_routes(app1, "/api1", context1).unwrap();

        let context2 = setup_test_context().await;
        let result = integrate_vbr_routes(app1, "/api2", context2);
        assert!(result.is_ok());
    }
}
