//! OpenAPI route registry and management
//!
//! This module provides the main OpenApiRouteRegistry struct and related
//! functionality for managing OpenAPI-based routes.

use super::validation::{ValidationMode, ValidationOptions};
use crate::openapi::route::OpenApiRoute;
use crate::openapi::spec::OpenApiSpec;
use std::collections::HashMap;
use std::sync::Arc;

/// OpenAPI route registry that manages generated routes
#[derive(Debug, Clone)]
pub struct OpenApiRouteRegistry {
    /// The OpenAPI specification
    spec: Arc<OpenApiSpec>,
    /// Generated routes
    routes: Vec<OpenApiRoute>,
    /// Validation options
    options: ValidationOptions,
}

impl OpenApiRouteRegistry {
    /// Create a new registry from an OpenAPI spec
    pub fn new(spec: OpenApiSpec) -> Self {
        Self::new_with_env(spec)
    }

    pub fn new_with_env(spec: OpenApiSpec) -> Self {
        let spec = Arc::new(spec);
        let routes = Self::generate_routes(&spec);
        let options = ValidationOptions {
            request_mode: match std::env::var("MOCKFORGE_REQUEST_VALIDATION")
                .unwrap_or_else(|_| "enforce".into())
                .to_ascii_lowercase()
                .as_str()
            {
                "off" | "disable" | "disabled" => ValidationMode::Disabled,
                "warn" | "warning" => ValidationMode::Warn,
                _ => ValidationMode::Enforce,
            },
            aggregate_errors: std::env::var("MOCKFORGE_AGGREGATE_ERRORS")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true),
            validate_responses: std::env::var("MOCKFORGE_RESPONSE_VALIDATION")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            overrides: HashMap::new(),
            admin_skip_prefixes: Vec::new(),
            response_template_expand: std::env::var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            validation_status: std::env::var("MOCKFORGE_VALIDATION_STATUS")
                .ok()
                .and_then(|s| s.parse::<u16>().ok()),
        };
        Self {
            spec,
            routes,
            options,
        }
    }

    /// Construct with explicit options
    pub fn new_with_options(spec: OpenApiSpec, options: ValidationOptions) -> Self {
        let spec = Arc::new(spec);
        let routes = Self::generate_routes(&spec);
        Self {
            spec,
            routes,
            options,
        }
    }

    /// Generate routes from the OpenAPI specification
    fn generate_routes(spec: &Arc<OpenApiSpec>) -> Vec<OpenApiRoute> {
        let mut routes = Vec::new();

        for (path, path_item) in &spec.spec.paths.paths {
            if let Some(item) = path_item.as_item() {
                // Generate route for each HTTP method
                if let Some(op) = &item.get {
                    routes.push(OpenApiRoute::from_operation(
                        "GET",
                        path.clone(),
                        op,
                        spec.clone(),
                    ));
                }
                if let Some(op) = &item.post {
                    routes.push(OpenApiRoute::from_operation(
                        "POST",
                        path.clone(),
                        op,
                        spec.clone(),
                    ));
                }
                if let Some(op) = &item.put {
                    routes.push(OpenApiRoute::from_operation(
                        "PUT",
                        path.clone(),
                        op,
                        spec.clone(),
                    ));
                }
                if let Some(op) = &item.delete {
                    routes.push(OpenApiRoute::from_operation(
                        "DELETE",
                        path.clone(),
                        op,
                        spec.clone(),
                    ));
                }
                if let Some(op) = &item.patch {
                    routes.push(OpenApiRoute::from_operation(
                        "PATCH",
                        path.clone(),
                        op,
                        spec.clone(),
                    ));
                }
                if let Some(op) = &item.head {
                    routes.push(OpenApiRoute::from_operation(
                        "HEAD",
                        path.clone(),
                        op,
                        spec.clone(),
                    ));
                }
                if let Some(op) = &item.options {
                    routes.push(OpenApiRoute::from_operation(
                        "OPTIONS",
                        path.clone(),
                        op,
                        spec.clone(),
                    ));
                }
                if let Some(op) = &item.trace {
                    routes.push(OpenApiRoute::from_operation(
                        "TRACE",
                        path.clone(),
                        op,
                        spec.clone(),
                    ));
                }
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

    /// Get validation options
    pub fn options(&self) -> &ValidationOptions {
        &self.options
    }

    /// Get mutable validation options
    pub fn options_mut(&mut self) -> &mut ValidationOptions {
        &mut self.options
    }

    /// Extract path parameters from a request path by matching against known routes
    pub fn extract_path_parameters(&self, path: &str, method: &str) -> HashMap<String, String> {
        for route in &self.routes {
            if route.method != method {
                continue;
            }

            if let Some(params) = self.match_path_to_route(path, &route.path) {
                return params;
            }
        }
        HashMap::new()
    }

    /// Match a request path against a route pattern and extract parameters
    fn match_path_to_route(
        &self,
        request_path: &str,
        route_pattern: &str,
    ) -> Option<HashMap<String, String>> {
        let mut params = HashMap::new();

        // Split both paths into segments
        let request_segments: Vec<&str> = request_path.trim_start_matches('/').split('/').collect();
        let pattern_segments: Vec<&str> =
            route_pattern.trim_start_matches('/').split('/').collect();

        if request_segments.len() != pattern_segments.len() {
            return None;
        }

        for (req_seg, pat_seg) in request_segments.iter().zip(pattern_segments.iter()) {
            if pat_seg.starts_with('{') && pat_seg.ends_with('}') {
                // This is a parameter
                let param_name = &pat_seg[1..pat_seg.len() - 1];
                params.insert(param_name.to_string(), req_seg.to_string());
            } else if req_seg != pat_seg {
                // Static segment doesn't match
                return None;
            }
        }

        Some(params)
    }
}
