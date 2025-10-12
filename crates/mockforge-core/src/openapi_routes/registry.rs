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
        tracing::debug!("Creating OpenAPI route registry");
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
        tracing::debug!("Creating OpenAPI route registry with custom options");
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
        tracing::debug!(
            "Generating routes from OpenAPI spec with {} paths",
            spec.spec.paths.paths.len()
        );

        for (path, path_item) in &spec.spec.paths.paths {
            tracing::debug!("Processing path: {}", path);
            if let Some(item) = path_item.as_item() {
                // Generate route for each HTTP method
                if let Some(op) = &item.get {
                    tracing::debug!("  Adding GET route for path: {}", path);
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

        tracing::debug!("Generated {} total routes from OpenAPI spec", routes.len());
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

    /// Build an Axum router from the generated routes
    pub fn build_router(&self) -> axum::Router {
        use axum::routing::{delete, get, patch, post, put};

        let mut router = axum::Router::new();
        tracing::debug!("Building router from {} routes", self.routes.len());

        for route in &self.routes {
            println!("Adding route: {} {}", route.method, route.path);
            println!(
                "Route operation responses: {:?}",
                route.operation.responses.responses.keys().collect::<Vec<_>>()
            );

            let route_clone = route.clone();
            let handler = move || {
                let route = route_clone.clone();
                async move {
                    println!("Handling request for route: {} {}", route.method, route.path);
                    let (status, response) = route.mock_response_with_status();
                    println!("Generated response with status: {}", status);
                    (
                        axum::http::StatusCode::from_u16(status)
                            .unwrap_or(axum::http::StatusCode::OK),
                        axum::response::Json(response),
                    )
                }
            };

            match route.method.as_str() {
                "GET" => {
                    println!("Registering GET route: {}", route.path);
                    router = router.route(&route.path, get(handler));
                }
                "POST" => {
                    println!("Registering POST route: {}", route.path);
                    router = router.route(&route.path, post(handler));
                }
                "PUT" => {
                    println!("Registering PUT route: {}", route.path);
                    router = router.route(&route.path, put(handler));
                }
                "DELETE" => {
                    println!("Registering DELETE route: {}", route.path);
                    router = router.route(&route.path, delete(handler));
                }
                "PATCH" => {
                    println!("Registering PATCH route: {}", route.path);
                    router = router.route(&route.path, patch(handler));
                }
                _ => println!("Unsupported HTTP method: {}", route.method),
            }
        }

        router
    }

    /// Build router with injectors (latency, failure)
    pub fn build_router_with_injectors(
        &self,
        latency_injector: crate::latency::LatencyInjector,
        failure_injector: Option<crate::failure_injection::FailureInjector>,
    ) -> axum::Router {
        use axum::routing::{delete, get, patch, post, put};

        let mut router = axum::Router::new();
        tracing::debug!("Building router with injectors from {} routes", self.routes.len());

        for route in &self.routes {
            tracing::debug!("Adding route with injectors: {} {}", route.method, route.path);

            let route_clone = route.clone();
            let latency_injector_clone = latency_injector.clone();
            let failure_injector_clone = failure_injector.clone();

            let handler = move || {
                let route = route_clone.clone();
                let latency_injector = latency_injector_clone.clone();
                let failure_injector = failure_injector_clone.clone();

                async move {
                    tracing::debug!(
                        "Handling request with injectors for route: {} {}",
                        route.method,
                        route.path
                    );

                    // Extract tags from the operation
                    let tags = route.operation.tags.clone();

                    // Inject latency if configured
                    if let Err(e) = latency_injector.inject_latency(&tags).await {
                        tracing::warn!("Failed to inject latency: {}", e);
                    }

                    // Check for failure injection
                    if let Some(ref injector) = failure_injector {
                        if injector.should_inject_failure(&tags) {
                            // Return a failure response
                            return (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                axum::response::Json(serde_json::json!({
                                    "error": "Injected failure",
                                    "code": 500
                                })),
                            );
                        }
                    }

                    // Generate normal response
                    let (status, response) = route.mock_response_with_status();
                    (
                        axum::http::StatusCode::from_u16(status)
                            .unwrap_or(axum::http::StatusCode::OK),
                        axum::response::Json(response),
                    )
                }
            };

            match route.method.as_str() {
                "GET" => router = router.route(&route.path, get(handler)),
                "POST" => router = router.route(&route.path, post(handler)),
                "PUT" => router = router.route(&route.path, put(handler)),
                "DELETE" => router = router.route(&route.path, delete(handler)),
                "PATCH" => router = router.route(&route.path, patch(handler)),
                _ => tracing::warn!("Unsupported HTTP method: {}", route.method),
            }
        }

        router
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
