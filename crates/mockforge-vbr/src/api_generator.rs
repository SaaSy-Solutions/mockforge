//! CRUD API generator
//!
//! This module generates CRUD endpoints for each entity, including:
//! - GET /api/{entity} - List all (with pagination, filtering, sorting)
//! - GET /api/{entity}/{id} - Get by ID
//! - POST /api/{entity} - Create new
//! - PUT /api/{entity}/{id} - Full update
//! - PATCH /api/{entity}/{id} - Partial update
//! - DELETE /api/{entity}/{id} - Delete
//! - GET /api/{entity}/{id}/{relationship} - Get related entities

use crate::entities::Entity;

/// API endpoint definition
#[derive(Debug, Clone)]
pub struct ApiEndpoint {
    /// HTTP method
    pub method: String,
    /// Path pattern
    pub path: String,
    /// Handler function name
    pub handler_name: String,
    /// Entity name
    pub entity_name: String,
}

/// API generator for creating CRUD endpoints
pub struct ApiGenerator {
    /// API prefix (e.g., "/api")
    pub prefix: String,
}

impl ApiGenerator {
    /// Create a new API generator
    pub fn new(prefix: String) -> Self {
        Self { prefix }
    }

    /// Generate all CRUD endpoints for an entity
    pub fn generate_endpoints(&self, entity: &Entity) -> Vec<ApiEndpoint> {
        let entity_name = entity.name().to_lowercase();
        let mut endpoints = Vec::new();

        // GET /api/{entity} - List all
        endpoints.push(ApiEndpoint {
            method: "GET".to_string(),
            path: format!("{}/{}", self.prefix, entity_name),
            handler_name: format!("list_{}", entity_name),
            entity_name: entity.name().to_string(),
        });

        // GET /api/{entity}/{id} - Get by ID
        endpoints.push(ApiEndpoint {
            method: "GET".to_string(),
            path: format!("{}/{}/{{id}}", self.prefix, entity_name),
            handler_name: format!("get_{}", entity_name),
            entity_name: entity.name().to_string(),
        });

        // POST /api/{entity} - Create
        endpoints.push(ApiEndpoint {
            method: "POST".to_string(),
            path: format!("{}/{}", self.prefix, entity_name),
            handler_name: format!("create_{}", entity_name),
            entity_name: entity.name().to_string(),
        });

        // PUT /api/{entity}/{id} - Full update
        endpoints.push(ApiEndpoint {
            method: "PUT".to_string(),
            path: format!("{}/{}/{{id}}", self.prefix, entity_name),
            handler_name: format!("update_{}", entity_name),
            entity_name: entity.name().to_string(),
        });

        // PATCH /api/{entity}/{id} - Partial update
        endpoints.push(ApiEndpoint {
            method: "PATCH".to_string(),
            path: format!("{}/{}/{{id}}", self.prefix, entity_name),
            handler_name: format!("patch_{}", entity_name),
            entity_name: entity.name().to_string(),
        });

        // DELETE /api/{entity}/{id} - Delete
        endpoints.push(ApiEndpoint {
            method: "DELETE".to_string(),
            path: format!("{}/{}/{{id}}", self.prefix, entity_name),
            handler_name: format!("delete_{}", entity_name),
            entity_name: entity.name().to_string(),
        });

        // Generate relationship endpoints
        for fk in &entity.schema.foreign_keys {
            let relationship_name = fk.field.trim_end_matches("_id");
            endpoints.push(ApiEndpoint {
                method: "GET".to_string(),
                path: format!("{}/{}/{{id}}/{}", self.prefix, entity_name, relationship_name),
                handler_name: format!(
                    "get_{}_by_{}",
                    fk.target_entity.to_lowercase(),
                    relationship_name
                ),
                entity_name: entity.name().to_string(),
            });
        }

        endpoints
    }
}
