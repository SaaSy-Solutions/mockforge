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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::Entity;
    use crate::schema::VbrSchemaDefinition;
    use mockforge_data::SchemaDefinition;

    fn create_test_entity(name: &str) -> Entity {
        let base_schema = SchemaDefinition::new(name.to_string());
        let vbr_schema = VbrSchemaDefinition::new(base_schema);
        Entity::new(name.to_string(), vbr_schema)
    }

    fn create_entity_with_foreign_key(name: &str) -> Entity {
        let base_schema = SchemaDefinition::new(name.to_string());
        let mut vbr_schema = VbrSchemaDefinition::new(base_schema);
        vbr_schema.foreign_keys.push(crate::schema::ForeignKeyDefinition {
            field: "user_id".to_string(),
            target_entity: "User".to_string(),
            target_field: "id".to_string(),
            on_delete: crate::schema::CascadeAction::Cascade,
            on_update: crate::schema::CascadeAction::NoAction,
        });
        Entity::new(name.to_string(), vbr_schema)
    }

    #[test]
    fn test_api_generator_new() {
        let generator = ApiGenerator::new("/api".to_string());
        assert_eq!(generator.prefix, "/api");
    }

    #[test]
    fn test_api_generator_custom_prefix() {
        let generator = ApiGenerator::new("/v2/api".to_string());
        assert_eq!(generator.prefix, "/v2/api");
    }

    #[test]
    fn test_generate_endpoints_count() {
        let generator = ApiGenerator::new("/api".to_string());
        let entity = create_test_entity("User");
        let endpoints = generator.generate_endpoints(&entity);

        // Should generate 6 basic CRUD endpoints
        assert_eq!(endpoints.len(), 6);
    }

    #[test]
    fn test_generate_endpoints_methods() {
        let generator = ApiGenerator::new("/api".to_string());
        let entity = create_test_entity("User");
        let endpoints = generator.generate_endpoints(&entity);

        let methods: Vec<&str> = endpoints.iter().map(|e| e.method.as_str()).collect();
        assert!(methods.contains(&"GET"));
        assert!(methods.contains(&"POST"));
        assert!(methods.contains(&"PUT"));
        assert!(methods.contains(&"PATCH"));
        assert!(methods.contains(&"DELETE"));
    }

    #[test]
    fn test_generate_endpoints_paths() {
        let generator = ApiGenerator::new("/api".to_string());
        let entity = create_test_entity("User");
        let endpoints = generator.generate_endpoints(&entity);

        let paths: Vec<&str> = endpoints.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"/api/user"));
        assert!(paths.contains(&"/api/user/{id}"));
    }

    #[test]
    fn test_generate_endpoints_handler_names() {
        let generator = ApiGenerator::new("/api".to_string());
        let entity = create_test_entity("User");
        let endpoints = generator.generate_endpoints(&entity);

        let handlers: Vec<&str> = endpoints.iter().map(|e| e.handler_name.as_str()).collect();
        assert!(handlers.contains(&"list_user"));
        assert!(handlers.contains(&"get_user"));
        assert!(handlers.contains(&"create_user"));
        assert!(handlers.contains(&"update_user"));
        assert!(handlers.contains(&"patch_user"));
        assert!(handlers.contains(&"delete_user"));
    }

    #[test]
    fn test_generate_endpoints_entity_name() {
        let generator = ApiGenerator::new("/api".to_string());
        let entity = create_test_entity("Product");
        let endpoints = generator.generate_endpoints(&entity);

        for endpoint in endpoints {
            assert_eq!(endpoint.entity_name, "Product");
        }
    }

    #[test]
    fn test_generate_endpoints_with_foreign_key() {
        let generator = ApiGenerator::new("/api".to_string());
        let entity = create_entity_with_foreign_key("Order");
        let endpoints = generator.generate_endpoints(&entity);

        // Should have 6 basic + 1 relationship endpoint
        assert_eq!(endpoints.len(), 7);

        // Check for the relationship endpoint
        let relationship_endpoint = endpoints.iter().find(|e| e.path.contains("/user"));
        assert!(relationship_endpoint.is_some());
        assert_eq!(relationship_endpoint.unwrap().method, "GET");
    }

    #[test]
    fn test_api_endpoint_debug() {
        let endpoint = ApiEndpoint {
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            handler_name: "test_handler".to_string(),
            entity_name: "Test".to_string(),
        };

        let debug = format!("{:?}", endpoint);
        assert!(debug.contains("ApiEndpoint"));
        assert!(debug.contains("GET"));
        assert!(debug.contains("/api/test"));
    }

    #[test]
    fn test_api_endpoint_clone() {
        let endpoint = ApiEndpoint {
            method: "POST".to_string(),
            path: "/api/test".to_string(),
            handler_name: "create_test".to_string(),
            entity_name: "Test".to_string(),
        };

        let cloned = endpoint.clone();
        assert_eq!(endpoint.method, cloned.method);
        assert_eq!(endpoint.path, cloned.path);
        assert_eq!(endpoint.handler_name, cloned.handler_name);
        assert_eq!(endpoint.entity_name, cloned.entity_name);
    }

    #[test]
    fn test_lowercase_entity_in_path() {
        let generator = ApiGenerator::new("/api".to_string());
        let entity = create_test_entity("UserProfile");
        let endpoints = generator.generate_endpoints(&entity);

        // Path should use lowercase entity name
        let list_endpoint = endpoints.iter().find(|e| e.handler_name == "list_userprofile");
        assert!(list_endpoint.is_some());
        assert_eq!(list_endpoint.unwrap().path, "/api/userprofile");
    }
}
