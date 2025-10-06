//! OpenAPI route generation from specifications
//!
//! This module provides functionality for generating Axum routes
//! from OpenAPI path definitions.

use crate::{openapi::spec::OpenApiSpec, Result};
use openapiv3::{Operation, PathItem, ReferenceOr};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Extract path parameters from an OpenAPI path template
fn extract_path_parameters(path_template: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut in_param = false;
    let mut current_param = String::new();

    for ch in path_template.chars() {
        match ch {
            '{' => {
                in_param = true;
                current_param.clear();
            }
            '}' => {
                if in_param {
                    params.push(current_param.clone());
                    in_param = false;
                }
            }
            ch if in_param => {
                current_param.push(ch);
            }
            _ => {}
        }
    }

    params
}

/// OpenAPI route wrapper with additional metadata
#[derive(Debug, Clone)]
pub struct OpenApiRoute {
    /// The HTTP method
    pub method: String,
    /// The path pattern
    pub path: String,
    /// The OpenAPI operation
    pub operation: Operation,
    /// Route-specific metadata
    pub metadata: BTreeMap<String, String>,
    /// Path parameters extracted from the path
    pub parameters: Vec<String>,
    /// Reference to the OpenAPI spec for response generation
    pub spec: Arc<OpenApiSpec>,
}

impl OpenApiRoute {
    /// Create a new OpenApiRoute
    pub fn new(method: String, path: String, operation: Operation, spec: Arc<OpenApiSpec>) -> Self {
        let parameters = extract_path_parameters(&path);
        Self {
            method,
            path,
            operation,
            metadata: BTreeMap::new(),
            parameters,
            spec,
        }
    }

    /// Create an OpenApiRoute from an operation
    pub fn from_operation(
        method: &str,
        path: String,
        operation: &Operation,
        spec: Arc<OpenApiSpec>,
    ) -> Self {
        Self::new(method.to_string(), path, operation.clone(), spec)
    }

    /// Convert OpenAPI path to Axum-compatible path format
    pub fn axum_path(&self) -> String {
        // Axum v0.7+ uses {param} format, same as OpenAPI
        self.path.clone()
    }

    /// Add metadata to the route
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Generate a mock response with status code for this route
    pub fn mock_response_with_status(&self) -> (u16, serde_json::Value) {
        use crate::openapi::response::ResponseGenerator;

        // Find the first available status code from the OpenAPI spec
        let status_code = self.find_first_available_status_code();

        // Try to generate a response based on the OpenAPI schema
        // Check if token expansion should be enabled
        let expand_tokens = std::env::var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        match ResponseGenerator::generate_response_with_expansion(
            &self.spec,
            &self.operation,
            status_code,
            Some("application/json"),
            expand_tokens,
        ) {
            Ok(response_body) => {
                tracing::debug!("ResponseGenerator succeeded for {} {} with status {}: {:?}", self.method, self.path, status_code, response_body);
                (status_code, response_body)
            },
            Err(e) => {
                tracing::debug!("ResponseGenerator failed for {} {}: {}, using fallback", self.method, self.path, e);
                // Fallback to simple mock response if schema-based generation fails
                let response_body = serde_json::json!({
                    "message": format!("Mock response for {} {}", self.method, self.path),
                    "operation_id": self.operation.operation_id,
                    "status": status_code
                });
                (status_code, response_body)
            }
        }
    }

    /// Find the first available status code from the OpenAPI operation responses
    fn find_first_available_status_code(&self) -> u16 {
        // Look for the first available status code in the responses
        for (status, _) in &self.operation.responses.responses {
            match status {
                openapiv3::StatusCode::Code(code) => {
                    return *code;
                },
                openapiv3::StatusCode::Range(range) => {
                    // For ranges, use the appropriate status code
                    match range {
                        2 => return 200, // 2XX range
                        3 => return 300, // 3XX range
                        4 => return 400, // 4XX range
                        5 => return 500, // 5XX range
                        _ => continue,   // Skip unknown ranges
                    }
                }
            }
        }

        // If no specific status codes found, check for default
        if self.operation.responses.default.is_some() {
            return 200; // Default to 200 for default responses
        }

        // Fallback to 200 if nothing else is available
        200
    }
}

/// OpenAPI operation wrapper with path context
#[derive(Debug, Clone)]
pub struct OpenApiOperation {
    /// The HTTP method
    pub method: String,
    /// The path this operation belongs to
    pub path: String,
    /// The OpenAPI operation
    pub operation: Operation,
}

impl OpenApiOperation {
    /// Create a new OpenApiOperation
    pub fn new(method: String, path: String, operation: Operation) -> Self {
        Self {
            method,
            path,
            operation,
        }
    }
}

/// Route generation utilities
pub struct RouteGenerator;

impl RouteGenerator {
    /// Generate routes from an OpenAPI path item
    pub fn generate_routes_from_path(
        path: &str,
        path_item: &ReferenceOr<PathItem>,
        spec: &Arc<OpenApiSpec>,
    ) -> Result<Vec<OpenApiRoute>> {
        let mut routes = Vec::new();

        if let Some(item) = path_item.as_item() {
            // Generate route for each HTTP method
            if let Some(op) = &item.get {
                routes.push(OpenApiRoute::new(
                    "GET".to_string(),
                    path.to_string(),
                    op.clone(),
                    spec.clone(),
                ));
            }
            if let Some(op) = &item.post {
                routes.push(OpenApiRoute::new(
                    "POST".to_string(),
                    path.to_string(),
                    op.clone(),
                    spec.clone(),
                ));
            }
            if let Some(op) = &item.put {
                routes.push(OpenApiRoute::new(
                    "PUT".to_string(),
                    path.to_string(),
                    op.clone(),
                    spec.clone(),
                ));
            }
            if let Some(op) = &item.delete {
                routes.push(OpenApiRoute::new(
                    "DELETE".to_string(),
                    path.to_string(),
                    op.clone(),
                    spec.clone(),
                ));
            }
            if let Some(op) = &item.patch {
                routes.push(OpenApiRoute::new(
                    "PATCH".to_string(),
                    path.to_string(),
                    op.clone(),
                    spec.clone(),
                ));
            }
            if let Some(op) = &item.head {
                routes.push(OpenApiRoute::new(
                    "HEAD".to_string(),
                    path.to_string(),
                    op.clone(),
                    spec.clone(),
                ));
            }
            if let Some(op) = &item.options {
                routes.push(OpenApiRoute::new(
                    "OPTIONS".to_string(),
                    path.to_string(),
                    op.clone(),
                    spec.clone(),
                ));
            }
            if let Some(op) = &item.trace {
                routes.push(OpenApiRoute::new(
                    "TRACE".to_string(),
                    path.to_string(),
                    op.clone(),
                    spec.clone(),
                ));
            }
        }

        Ok(routes)
    }
}
