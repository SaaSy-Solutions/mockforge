//! Route generation for CRUD operations
//!
//! Automatically generates REST API routes with CRUD operations based on entity definitions.

use crate::generative_schema::naming_rules::NamingRules;
use serde::{Deserialize, Serialize};

/// CRUD operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CrudOperation {
    /// Create - POST /entities
    Create,
    /// Read - GET /entities/{id}
    Read,
    /// Update - PUT /entities/{id}
    Update,
    /// Delete - DELETE /entities/{id}
    Delete,
    /// List - GET /entities
    List,
}

/// Route definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDefinition {
    /// HTTP method
    pub method: String,
    /// Route path
    pub path: String,
    /// Operation type
    pub operation: CrudOperation,
    /// Entity name this route belongs to
    pub entity: String,
    /// Route description
    pub description: String,
}

/// Route generator
pub struct RouteGenerator {
    naming_rules: NamingRules,
}

impl RouteGenerator {
    /// Create a new route generator
    pub fn new(naming_rules: NamingRules) -> Self {
        Self { naming_rules }
    }

    /// Generate CRUD routes for an entity
    pub fn generate_crud_routes(&self, entity_name: &str) -> Vec<RouteDefinition> {
        let base_path = self.naming_rules.entity_to_route(entity_name);
        let entity_id_path = format!("{}/{{id}}", base_path);

        vec![
            RouteDefinition {
                method: "POST".to_string(),
                path: base_path.clone(),
                operation: CrudOperation::Create,
                entity: entity_name.to_string(),
                description: format!("Create a new {}", entity_name),
            },
            RouteDefinition {
                method: "GET".to_string(),
                path: base_path.clone(),
                operation: CrudOperation::List,
                entity: entity_name.to_string(),
                description: format!("List all {}", self.naming_rules.pluralize(entity_name)),
            },
            RouteDefinition {
                method: "GET".to_string(),
                path: entity_id_path.clone(),
                operation: CrudOperation::Read,
                entity: entity_name.to_string(),
                description: format!("Get a {} by ID", entity_name),
            },
            RouteDefinition {
                method: "PUT".to_string(),
                path: entity_id_path.clone(),
                operation: CrudOperation::Update,
                entity: entity_name.to_string(),
                description: format!("Update a {} by ID", entity_name),
            },
            RouteDefinition {
                method: "DELETE".to_string(),
                path: entity_id_path,
                operation: CrudOperation::Delete,
                entity: entity_name.to_string(),
                description: format!("Delete a {} by ID", entity_name),
            },
        ]
    }
}
