//! OpenAPI-based route generation for MockForge
//!
//! This module provides functionality to automatically generate Axum routes
//! from OpenAPI specifications, including mock response generation and validation.

use crate::{Error, OpenApiOperation, OpenApiRoute, OpenApiSpec, Result};
use axum::{
    routing::{delete, get, head, options, patch, post, put},
    Json, Router,
};
use serde_json::{Map, Value};
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
                routes.push(OpenApiRoute::from_operation(method, path.clone(), operation, spec.as_ref()));
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

    /// Validate request against OpenAPI spec (legacy body-only)
    pub fn validate_request(&self, path: &str, method: &str, body: Option<&Value>) -> Result<()> {
        self.validate_request_with(path, method, &Map::new(), &Map::new(), body)
    }

    /// Validate request against OpenAPI spec with path/query params
    pub fn validate_request_with(
        &self,
        path: &str,
        method: &str,
        path_params: &Map<String, Value>,
        query_params: &Map<String, Value>,
        body: Option<&Value>,
    ) -> Result<()> {
        self.validate_request_with_all(path, method, path_params, query_params, &Map::new(), &Map::new(), body)
    }

    /// Validate request against OpenAPI spec with path/query/header/cookie params
    pub fn validate_request_with_all(
        &self,
        path: &str,
        method: &str,
        path_params: &Map<String, Value>,
        query_params: &Map<String, Value>,
        header_params: &Map<String, Value>,
        cookie_params: &Map<String, Value>,
        body: Option<&Value>,
    ) -> Result<()> {
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

            // Validate path/query parameters
            for p in &route.operation.parameters {
                let (params_map, prefix) = match p.location.as_str() {
                    "path" => (path_params, "path"),
                    "query" => (query_params, "query"),
                    "header" => (header_params, "header"),
                    "cookie" => (cookie_params, "cookie"),
                    _ => continue,
                };

                // For query deepObject, reconstruct value from key-likes: name[prop]
                let deep_value = if p.location == "query" && p.style.as_deref() == Some("deepObject") {
                    build_deep_object(&p.name, params_map)
                } else { None };

                match deep_value.as_ref().or_else(|| params_map.get(&p.name)) {
                    Some(v) => {
                        if let Some(s) = &p.schema {
                            let coerced = if p.location == "query" { coerce_by_style(v, s, p.style.as_deref()) } else { coerce_value_for_schema(v, s) };
                            s.validate_value(&coerced, &format!("{}.{}", prefix, p.name))
                                .map_err(|e| Error::validation(format!("{}", e)))?;
                        }
                    }
                    None => {
                        if p.required {
                            return Err(Error::validation(format!(
                                "missing required {} parameter '{}'",
                                prefix, p.name
                            )));
                        }
                    }
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

/// Coerce a parameter `value` into the expected JSON type per `schema` where reasonable.
/// Applies only to param contexts (not request bodies). Conservative conversions:
/// - integer/number: parse from string; arrays: split comma-separated strings and coerce items
/// - boolean: parse true/false (case-insensitive) from string
fn coerce_value_for_schema(value: &Value, schema: &crate::OpenApiSchema) -> Value {
    match schema.schema_type.as_deref() {
        Some("integer") => match value {
            Value::String(s) => s.parse::<i64>().map(Value::from).unwrap_or(value.clone()),
            _ => value.clone(),
        },
        Some("number") => match value {
            Value::String(s) => s.parse::<f64>().ok().and_then(|n| serde_json::Number::from_f64(n)).map(Value::Number).unwrap_or(value.clone()),
            _ => value.clone(),
        },
        Some("boolean") => match value {
            Value::String(s) => {
                let ls = s.to_ascii_lowercase();
                match ls.as_str() { "true" => Value::Bool(true), "false" => Value::Bool(false), _ => value.clone() }
            }
            _ => value.clone(),
        },
        Some("array") => {
            if let Some(items) = &schema.items {
                match value {
                    Value::String(s) => {
                        // Split comma-separated values: "1,2,3"
                        let parts = s.split(',').map(|p| Value::String(p.trim().to_string())).collect::<Vec<_>>();
                        let coerced = parts.into_iter().map(|v| coerce_value_for_schema(&v, items)).collect::<Vec<_>>();
                        Value::Array(coerced)
                    }
                    Value::Array(arr) => {
                        Value::Array(arr.iter().map(|v| coerce_value_for_schema(v, items)).collect())
                    }
                    _ => value.clone(),
                }
            } else {
                value.clone()
            }
        }
        _ => value.clone(),
    }
}

/// Apply style-aware coercion for query params
fn coerce_by_style(value: &Value, schema: &crate::OpenApiSchema, style: Option<&str>) -> Value {
    match (schema.schema_type.as_deref(), value, style) {
        (Some("array"), Value::String(s), Some("spaceDelimited")) => {
            let items = s.split(' ').filter(|p| !p.is_empty()).map(|p| Value::String(p.to_string())).collect::<Vec<_>>();
            let item_schema = schema.items.as_deref().unwrap_or(schema);
            Value::Array(items.into_iter().map(|v| coerce_value_for_schema(&v, item_schema)).collect())
        }
        (Some("array"), Value::String(s), Some("pipeDelimited")) => {
            let items = s.split('|').map(|p| Value::String(p.to_string())).collect::<Vec<_>>();
            let item_schema = schema.items.as_deref().unwrap_or(schema);
            Value::Array(items.into_iter().map(|v| coerce_value_for_schema(&v, item_schema)).collect())
        }
        _ => coerce_value_for_schema(value, schema),
    }
}

/// Build a deepObject from query params like `name[prop]=val`
fn build_deep_object(name: &str, params: &Map<String, Value>) -> Option<Value> {
    let prefix = format!("{}[", name);
    let mut obj = Map::new();
    for (k, v) in params.iter() {
        if let Some(rest) = k.strip_prefix(&prefix) {
            if rest.ends_with(']') {
                let key = &rest[..rest.len()-1];
                obj.insert(key.to_string(), v.clone());
            }
        }
    }
    if obj.is_empty() { None } else { Some(Value::Object(obj)) }
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

    #[tokio::test]
    async fn test_validate_request_with_params_and_formats() {
        let spec_json = json!({
            "openapi": "3.0.0",
            "info": { "title": "Test API", "version": "1.0.0" },
            "paths": {
                "/users/{id}": {
                    "post": {
                        "parameters": [
                            { "name": "id", "in": "path", "required": true, "schema": {"type": "string"} },
                            { "name": "q",  "in": "query", "required": false, "schema": {"type": "integer"} }
                        ],
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "required": ["email", "website"],
                                        "properties": {
                                            "email":   {"type": "string", "format": "email"},
                                            "website": {"type": "string", "format": "uri"}
                                        }
                                    }
                                }
                            }
                        },
                        "responses": {"200": {"description": "ok"}}
                    }
                }
            }
        });

        let registry = create_registry_from_json(spec_json).unwrap();
        let mut path_params = serde_json::Map::new();
        path_params.insert("id".to_string(), json!("abc"));
        let mut query_params = serde_json::Map::new();
        query_params.insert("q".to_string(), json!(123));

        // valid body
        let body = json!({"email":"a@b.co","website":"https://example.com"});
        assert!(registry.validate_request_with("/users/{id}", "POST", &path_params, &query_params, Some(&body)).is_ok());

        // invalid email
        let bad_email = json!({"email":"not-an-email","website":"https://example.com"});
        assert!(registry.validate_request_with("/users/{id}", "POST", &path_params, &query_params, Some(&bad_email)).is_err());

        // missing required path param
        let empty_path_params = serde_json::Map::new();
        assert!(registry.validate_request_with("/users/{id}", "POST", &empty_path_params, &query_params, Some(&body)).is_err());
    }

    #[tokio::test]
    async fn test_ref_resolution_for_params_and_body() {
        let spec_json = json!({
            "openapi": "3.0.0",
            "info": { "title": "Ref API", "version": "1.0.0" },
            "components": {
                "schemas": {
                    "EmailWebsite": {
                        "type": "object",
                        "required": ["email", "website"],
                        "properties": {
                            "email":   {"type": "string", "format": "email"},
                            "website": {"type": "string", "format": "uri"}
                        }
                    }
                },
                "parameters": {
                    "PathId": {"name": "id", "in": "path", "required": true, "schema": {"type": "string"}},
                    "QueryQ": {"name": "q",  "in": "query", "required": false, "schema": {"type": "integer"}}
                },
                "requestBodies": {
                    "CreateUser": {
                        "content": {
                            "application/json": {
                                "schema": {"$ref": "#/components/schemas/EmailWebsite"}
                            }
                        }
                    }
                }
            },
            "paths": {
                "/users/{id}": {
                    "post": {
                        "parameters": [
                            {"$ref": "#/components/parameters/PathId"},
                            {"$ref": "#/components/parameters/QueryQ"}
                        ],
                        "requestBody": {"$ref": "#/components/requestBodies/CreateUser"},
                        "responses": {"200": {"description": "ok"}}
                    }
                }
            }
        });

        let registry = create_registry_from_json(spec_json).unwrap();
        let mut path_params = serde_json::Map::new();
        path_params.insert("id".to_string(), json!("abc"));
        let mut query_params = serde_json::Map::new();
        query_params.insert("q".to_string(), json!(7));

        let body = json!({"email":"user@example.com","website":"https://example.com"});
        assert!(registry.validate_request_with("/users/{id}", "POST", &path_params, &query_params, Some(&body)).is_ok());

        let bad = json!({"email":"nope","website":"https://example.com"});
        assert!(registry.validate_request_with("/users/{id}", "POST", &path_params, &query_params, Some(&bad)).is_err());
    }

    #[tokio::test]
    async fn test_header_cookie_and_query_coercion() {
        let spec_json = json!({
            "openapi": "3.0.0",
            "info": { "title": "Params API", "version": "1.0.0" },
            "paths": {
                "/items": {
                    "get": {
                        "parameters": [
                            {"name": "X-Flag", "in": "header", "required": true, "schema": {"type": "boolean"}},
                            {"name": "session", "in": "cookie", "required": true, "schema": {"type": "string"}},
                            {"name": "ids", "in": "query", "required": false, "schema": {"type": "array", "items": {"type": "integer"}}}
                        ],
                        "responses": {"200": {"description": "ok"}}
                    }
                }
            }
        });

        let registry = create_registry_from_json(spec_json).unwrap();

        let path_params = serde_json::Map::new();
        let mut query_params = serde_json::Map::new();
        // comma-separated string for array should coerce
        query_params.insert("ids".to_string(), json!("1,2,3"));
        let mut header_params = serde_json::Map::new();
        header_params.insert("X-Flag".to_string(), json!("true"));
        let mut cookie_params = serde_json::Map::new();
        cookie_params.insert("session".to_string(), json!("abc123"));

        assert!(registry.validate_request_with_all(
            "/items", "GET", &path_params, &query_params, &header_params, &cookie_params, None
        ).is_ok());

        // Missing required cookie
        let empty_cookie = serde_json::Map::new();
        assert!(registry.validate_request_with_all(
            "/items", "GET", &path_params, &query_params, &header_params, &empty_cookie, None
        ).is_err());

        // Bad boolean header value (cannot coerce)
        let mut bad_header = serde_json::Map::new();
        bad_header.insert("X-Flag".to_string(), json!("notabool"));
        assert!(registry.validate_request_with_all(
            "/items", "GET", &path_params, &query_params, &bad_header, &cookie_params, None
        ).is_err());
    }

    #[tokio::test]
    async fn test_query_styles_space_pipe_deepobject() {
        let spec_json = json!({
            "openapi": "3.0.0",
            "info": { "title": "Query Styles API", "version": "1.0.0" },
            "paths": {"/search": {"get": {
                "parameters": [
                    {"name":"tags","in":"query","style":"spaceDelimited","schema":{"type":"array","items":{"type":"string"}}},
                    {"name":"ids","in":"query","style":"pipeDelimited","schema":{"type":"array","items":{"type":"integer"}}},
                    {"name":"filter","in":"query","style":"deepObject","schema":{"type":"object","properties":{"color":{"type":"string"}},"required":["color"]}}
                ],
                "responses": {"200": {"description":"ok"}}
            }} }
        });

        let registry = create_registry_from_json(spec_json).unwrap();

        let path_params = Map::new();
        let mut query = Map::new();
        query.insert("tags".into(), json!("alpha beta gamma"));
        query.insert("ids".into(), json!("1|2|3"));
        query.insert("filter[color]".into(), json!("red"));

        assert!(registry.validate_request_with(
            "/search", "GET", &path_params, &query, None
        ).is_ok());
    }

    #[tokio::test]
    async fn test_oneof_anyof_allof_validation() {
        let spec_json = json!({
            "openapi": "3.0.0",
            "info": { "title": "Composite API", "version": "1.0.0" },
            "paths": {
                "/composite": {
                    "post": {
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "allOf": [
                                            {"type": "object", "required": ["base"], "properties": {"base": {"type": "string"}}}
                                        ],
                                        "oneOf": [
                                            {"type": "object", "properties": {"a": {"type": "integer"}}},
                                            {"type": "object", "properties": {"b": {"type": "integer"}}}
                                        ],
                                        "anyOf": [
                                            {"type": "object", "properties": {"flag": {"type": "boolean"}}},
                                            {"type": "object", "properties": {"extra": {"type": "string"}}}
                                        ]
                                    }
                                }
                            }
                        },
                        "responses": {"200": {"description": "ok"}}
                    }
                }
            }
        });

        let registry = create_registry_from_json(spec_json).unwrap();
        // valid: satisfies base via allOf, exactly one of a/b, and at least one of flag/extra
        let ok = json!({"base": "x", "a": 1, "flag": true});
        assert!(registry.validate_request_with("/composite", "POST", &serde_json::Map::new(), &serde_json::Map::new(), Some(&ok)).is_ok());

        // invalid oneOf: both a and b present
        let bad_oneof = json!({"base": "x", "a": 1, "b": 2, "flag": false});
        assert!(registry.validate_request_with("/composite", "POST", &serde_json::Map::new(), &serde_json::Map::new(), Some(&bad_oneof)).is_err());

        // invalid anyOf: none of flag/extra present
        let bad_anyof = json!({"base": "x", "a": 1});
        assert!(registry.validate_request_with("/composite", "POST", &serde_json::Map::new(), &serde_json::Map::new(), Some(&bad_anyof)).is_err());

        // invalid allOf: missing base
        let bad_allof = json!({"a": 1, "flag": true});
        assert!(registry.validate_request_with("/composite", "POST", &serde_json::Map::new(), &serde_json::Map::new(), Some(&bad_allof)).is_err());
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
