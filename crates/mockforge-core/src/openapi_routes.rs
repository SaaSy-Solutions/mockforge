//! OpenAPI-based route generation for MockForge
//!
//! This module provides functionality to automatically generate Axum routes
//! from OpenAPI specifications, including mock response generation and validation.

use crate::{Error, OpenApiOperation, OpenApiRoute, OpenApiSpec, Result};
use axum::{
    routing::{delete, get, head, options, patch, post, put},
    Json, Router,
};
use serde_json::Value;
use std::sync::Arc;

/// OpenAPI route registry that manages generated routes
#[derive(Debug)]
pub struct OpenApiRouteRegistry {
    /// The OpenAPI specification
    spec: Arc<OpenApiSpec>,
    /// Generated routes
    routes: Vec<OpenApiRoute>,
}

impl OpenApiRouteRegistry {
    /// Create a new registry from an OpenAPI spec
    pub fn new(spec: OpenApiSpec) -> Self {
        let spec = Arc::new(spec);
        let routes = Self::generate_routes(&spec);

        Self { spec, routes }
    }

    /// Generate routes from the OpenAPI specification
    fn generate_routes(spec: &Arc<OpenApiSpec>) -> Vec<OpenApiRoute> {
        let mut routes = Vec::new();

        for (path, operations) in spec.all_paths_and_operations() {
            for (method, operation) in operations {
                routes.push(OpenApiRoute::from_operation(method, path.clone(), operation));
            }
        }

        routes
    }

    /// Get all routes
    pub fn routes(&self) -> &[OpenApiRoute] {
        &self.routes
    }

    /// Get the OpenAPI specification
    pub fn spec(&self) -> &OpenApiSpec {
        &self.spec
    }

    /// Build an Axum router from the OpenAPI spec (simplified)
    pub fn build_router(self) -> Router {
        let mut router = Router::new();

        // Create individual routes for each operation
        for route in self.routes {
            let axum_path = route.axum_path();
            let response = route.mock_response();

            // Create a simple handler that returns the mock response
            let handler = move || async move { Json(response.clone()) };

            // Register the handler based on HTTP method
            router = match route.method.as_str() {
                "GET" => router.route(&axum_path, get(handler)),
                "POST" => router.route(&axum_path, post(handler)),
                "PUT" => router.route(&axum_path, put(handler)),
                "DELETE" => router.route(&axum_path, delete(handler)),
                "PATCH" => router.route(&axum_path, patch(handler)),
                "HEAD" => router.route(&axum_path, head(handler)),
                "OPTIONS" => router.route(&axum_path, options(handler)),
                _ => router, // Skip unknown methods
            };
        }

        // Add OpenAPI documentation endpoint
        let spec_json = serde_json::to_value(&self.spec.spec).unwrap_or(Value::Null);
        router = router.route("/openapi.json", get(move || async move { Json(spec_json) }));

        router
    }

    /// Get route by path and method
    pub fn get_route(&self, path: &str, method: &str) -> Option<&OpenApiRoute> {
        self.routes.iter().find(|route| route.path == path && route.method == method)
    }

    /// Get all routes for a specific path
    pub fn get_routes_for_path(&self, path: &str) -> Vec<&OpenApiRoute> {
        self.routes.iter().filter(|route| route.path == path).collect()
    }

    /// Validate request against OpenAPI spec
    pub fn validate_request(&self, path: &str, method: &str, body: Option<&Value>) -> Result<()> {
        if let Some(route) = self.get_route(path, method) {
            // Validate request body if required
            if let Some(schema) = &route.operation.request_body {
                let value = body
                    .ok_or_else(|| Error::generic("Request body is required but not provided"))?;
                schema
                    .validate_value(value, "body")
                    .map_err(|e| Error::validation(format!("{}", e)))?;
            } else if body.is_some() {
                // No body expected but provided — not an error by default, but log it
                tracing::debug!("Body provided for operation without requestBody; accepting");
            }

            // Validate path/query parameters if schema is available
            // Note: this API doesn’t receive actual request params; we validate only required flags here.
            for p in &route.operation.parameters {
                if p.required && p.location == "path" {
                    // For now, assume path templating enforces presence; leave as informational
                    // Future: validate against extracted params map.
                }
            }

            Ok(())
        } else {
            Err(Error::generic(format!("Route {} {} not found in OpenAPI spec", method, path)))
        }
    }

    /// Generate mock response for a route
    pub fn generate_mock_response(&self, path: &str, method: &str) -> Option<Value> {
        self.get_route(path, method).map(|route| route.mock_response())
    }

    /// Get all paths defined in the spec
    pub fn paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self.routes.iter().map(|route| route.path.clone()).collect();
        paths.sort();
        paths.dedup();
        paths
    }

    /// Get all HTTP methods supported
    pub fn methods(&self) -> Vec<String> {
        let mut methods: Vec<String> =
            self.routes.iter().map(|route| route.method.clone()).collect();
        methods.sort();
        methods.dedup();
        methods
    }

    /// Get operation details for a route
    pub fn get_operation(&self, path: &str, method: &str) -> Option<&OpenApiOperation> {
        self.get_route(path, method).map(|route| &route.operation)
    }

    /// Convert OpenAPI path to Axum-compatible path
    /// This is a utility function for converting path parameters from {param} to :param format
    pub fn convert_path_to_axum(openapi_path: &str) -> String {
        openapi_path.replace("{", ":").replace("}", "")
    }
}

/// Helper function to create an OpenAPI route registry from a file
pub async fn create_registry_from_file<P: AsRef<std::path::Path>>(
    path: P,
) -> Result<OpenApiRouteRegistry> {
    let spec = OpenApiSpec::from_file(path).await?;
    spec.validate()?;
    Ok(OpenApiRouteRegistry::new(spec))
}

/// Helper function to create an OpenAPI route registry from JSON
pub fn create_registry_from_json(json: Value) -> Result<OpenApiRouteRegistry> {
    let spec = OpenApiSpec::from_json(json)?;
    spec.validate()?;
    Ok(OpenApiRouteRegistry::new(spec))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_registry_creation() {
        let spec_json = json!({
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/users": {
                    "get": {
                        "summary": "Get users",
                        "responses": {
                            "200": {
                                "description": "Success",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "array",
                                            "items": {
                                                "type": "object",
                                                "properties": {
                                                    "id": {"type": "integer"},
                                                    "name": {"type": "string"}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "post": {
                        "summary": "Create user",
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "name": {"type": "string"}
                                        },
                                        "required": ["name"]
                                    }
                                }
                            }
                        },
                        "responses": {
                            "201": {
                                "description": "Created",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "object",
                                            "properties": {
                                                "id": {"type": "integer"},
                                                "name": {"type": "string"}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                "/users/{id}": {
                    "get": {
                        "summary": "Get user by ID",
                        "parameters": [
                            {
                                "name": "id",
                                "in": "path",
                                "required": true,
                                "schema": {"type": "integer"}
                            }
                        ],
                        "responses": {
                            "200": {
                                "description": "Success",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "object",
                                            "properties": {
                                                "id": {"type": "integer"},
                                                "name": {"type": "string"}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        let registry = create_registry_from_json(spec_json).unwrap();

        // Test basic properties
        assert_eq!(registry.paths().len(), 2);
        assert!(registry.paths().contains(&"/users".to_string()));
        assert!(registry.paths().contains(&"/users/{id}".to_string()));

        assert_eq!(registry.methods().len(), 2);
        assert!(registry.methods().contains(&"GET".to_string()));
        assert!(registry.methods().contains(&"POST".to_string()));

        // Test route lookup
        let get_users_route = registry.get_route("/users", "GET").unwrap();
        assert_eq!(get_users_route.method, "GET");
        assert_eq!(get_users_route.path, "/users");

        let post_users_route = registry.get_route("/users", "POST").unwrap();
        assert_eq!(post_users_route.method, "POST");
        assert!(post_users_route.operation.request_body.is_some());

        // Test path parameter conversion
        let user_by_id_route = registry.get_route("/users/{id}", "GET").unwrap();
        assert_eq!(user_by_id_route.axum_path(), "/users/:id");
    }

    #[test]
    fn test_path_conversion() {
        assert_eq!(OpenApiRouteRegistry::convert_path_to_axum("/users"), "/users");
        assert_eq!(OpenApiRouteRegistry::convert_path_to_axum("/users/{id}"), "/users/:id");
        assert_eq!(
            OpenApiRouteRegistry::convert_path_to_axum("/users/{id}/posts/{postId}"),
            "/users/:id/posts/:postId"
        );
    }
}
