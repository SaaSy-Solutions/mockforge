//! OpenAPI route registry and management
//!
//! This module provides the main OpenApiRouteRegistry struct and related
//! functionality for managing OpenAPI-based routes.

use super::validation::{ValidationMode, ValidationOptions};
use crate::ai_response::RequestContext;
use crate::openapi::response::AiGenerator;
use crate::openapi::route::OpenApiRoute;
use crate::openapi::spec::OpenApiSpec;
use axum::extract::Json;
use axum::http::HeaderMap;
use openapiv3::{PathItem, ReferenceOr};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use url::Url;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_from_yaml(yaml: &str) -> OpenApiRouteRegistry {
        let spec = OpenApiSpec::from_string(yaml, Some("yaml")).expect("parse spec");
        OpenApiRouteRegistry::new_with_env(spec)
    }

    #[test]
    fn generates_routes_from_components_path_items() {
        let yaml = r#"
openapi: 3.1.0
info:
  title: Test API
  version: "1.0.0"
paths:
  /users:
    $ref: '#/components/pathItems/UserCollection'
components:
  pathItems:
    UserCollection:
      get:
        operationId: listUsers
        responses:
          '200':
            description: ok
            content:
              application/json:
                schema:
                  type: array
                  items:
                    type: string
        "#;

        let registry = registry_from_yaml(yaml);
        let routes = registry.routes();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].method, "GET");
        assert_eq!(routes[0].path, "/users");
    }

    #[test]
    fn generates_routes_from_paths_references() {
        let yaml = r#"
openapi: 3.0.3
info:
  title: PathRef API
  version: "1.0.0"
paths:
  /users:
    get:
      operationId: getUsers
      responses:
        '200':
          description: ok
  /all-users:
    $ref: '#/paths/~1users'
        "#;

        let registry = registry_from_yaml(yaml);
        let routes = registry.routes();
        assert_eq!(routes.len(), 2);

        let mut paths: Vec<(&str, &str)> = routes
            .iter()
            .map(|route| (route.method.as_str(), route.path.as_str()))
            .collect();
        paths.sort();

        assert_eq!(paths, vec![("GET", "/all-users"), ("GET", "/users")]);
    }

    #[test]
    fn generates_routes_with_server_base_path() {
        let yaml = r#"
openapi: 3.0.3
info:
  title: Base Path API
  version: "1.0.0"
servers:
  - url: https://api.example.com/api/v1
paths:
  /users:
    get:
      operationId: getUsers
      responses:
        '200':
          description: ok
        "#;

        let registry = registry_from_yaml(yaml);
        let paths: Vec<String> = registry.routes().iter().map(|route| route.path.clone()).collect();
        assert!(paths.contains(&"/api/v1/users".to_string()));
        assert!(!paths.contains(&"/users".to_string()));
    }

    #[test]
    fn generates_routes_with_relative_server_base_path() {
        let yaml = r#"
openapi: 3.0.3
info:
  title: Relative Base Path API
  version: "1.0.0"
servers:
  - url: /api/v2
paths:
  /orders:
    post:
      operationId: createOrder
      responses:
        '201':
          description: created
        "#;

        let registry = registry_from_yaml(yaml);
        let paths: Vec<String> = registry.routes().iter().map(|route| route.path.clone()).collect();
        assert!(paths.contains(&"/api/v2/orders".to_string()));
        assert!(!paths.contains(&"/orders".to_string()));
    }
}

impl OpenApiRouteRegistry {
    /// Create a new registry from an OpenAPI spec with default options
    pub fn new(spec: OpenApiSpec) -> Self {
        Self::new_with_env(spec)
    }

    /// Create a new registry from an OpenAPI spec with environment-based options
    ///
    /// Options are read from environment variables:
    /// - `MOCKFORGE_REQUEST_VALIDATION`: "off"/"warn"/"enforce" (default: "enforce")
    /// - `MOCKFORGE_AGGREGATE_ERRORS`: "1"/"true" to aggregate errors (default: true)
    /// - `MOCKFORGE_RESPONSE_VALIDATION`: "1"/"true" to validate responses (default: false)
    /// - `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND`: "1"/"true" to expand templates (default: false)
    /// - `MOCKFORGE_VALIDATION_STATUS`: HTTP status code for validation failures (optional)
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

    /// Create a new registry from an OpenAPI spec with explicit validation options
    ///
    /// # Arguments
    /// * `spec` - OpenAPI specification
    /// * `options` - Validation options to use
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
        tracing::debug!(
            "Generating routes from OpenAPI spec with {} paths",
            spec.spec.paths.paths.len()
        );
        let base_paths = Self::collect_base_paths(spec);

        // Optimize: Use parallel iteration for route generation when beneficial
        #[cfg(feature = "rayon")]
        {
            use rayon::prelude::*;
            let path_items: Vec<_> = spec.spec.paths.paths.iter().collect();

            // Use parallel processing for large specs (100+ paths)
            if path_items.len() > 100 {
                tracing::debug!("Using parallel route generation for {} paths", path_items.len());
                let routes: Vec<Vec<OpenApiRoute>> = path_items
                    .par_iter()
                    .map(|(path, path_item)| {
                        let mut routes = Vec::new();
                        let mut visited = HashSet::new();
                        if let Some(item) = Self::resolve_path_item(path_item, spec, &mut visited) {
                            Self::collect_routes_for_path(&mut routes, path, &item, spec, &base_paths);
                        } else {
                            tracing::warn!(
                                "Skipping path {} because the referenced PathItem could not be resolved",
                                path
                            );
                        }
                        routes
                    })
                    .collect();

                let mut all_routes = Vec::new();
                for route_batch in routes {
                    all_routes.extend(route_batch);
                }
                tracing::debug!(
                    "Generated {} total routes from OpenAPI spec (parallel)",
                    all_routes.len()
                );
                return all_routes;
            }
        }

        // Sequential processing for smaller specs or when rayon is not available
        let mut routes = Vec::new();
        for (path, path_item) in &spec.spec.paths.paths {
            tracing::debug!("Processing path: {}", path);
            let mut visited = HashSet::new();
            if let Some(item) = Self::resolve_path_item(path_item, spec, &mut visited) {
                Self::collect_routes_for_path(&mut routes, path, &item, spec, &base_paths);
            } else {
                tracing::warn!(
                    "Skipping path {} because the referenced PathItem could not be resolved",
                    path
                );
            }
        }

        tracing::debug!("Generated {} total routes from OpenAPI spec", routes.len());
        routes
    }

    fn collect_routes_for_path(
        routes: &mut Vec<OpenApiRoute>,
        path: &str,
        item: &PathItem,
        spec: &Arc<OpenApiSpec>,
        base_paths: &[String],
    ) {
        if let Some(op) = &item.get {
            tracing::debug!("  Adding GET route for path: {}", path);
            Self::push_routes_for_method(routes, "GET", path, op, spec, base_paths);
        }
        if let Some(op) = &item.post {
            Self::push_routes_for_method(routes, "POST", path, op, spec, base_paths);
        }
        if let Some(op) = &item.put {
            Self::push_routes_for_method(routes, "PUT", path, op, spec, base_paths);
        }
        if let Some(op) = &item.delete {
            Self::push_routes_for_method(routes, "DELETE", path, op, spec, base_paths);
        }
        if let Some(op) = &item.patch {
            Self::push_routes_for_method(routes, "PATCH", path, op, spec, base_paths);
        }
        if let Some(op) = &item.head {
            Self::push_routes_for_method(routes, "HEAD", path, op, spec, base_paths);
        }
        if let Some(op) = &item.options {
            Self::push_routes_for_method(routes, "OPTIONS", path, op, spec, base_paths);
        }
        if let Some(op) = &item.trace {
            Self::push_routes_for_method(routes, "TRACE", path, op, spec, base_paths);
        }
    }

    fn push_routes_for_method(
        routes: &mut Vec<OpenApiRoute>,
        method: &str,
        path: &str,
        operation: &openapiv3::Operation,
        spec: &Arc<OpenApiSpec>,
        base_paths: &[String],
    ) {
        for base in base_paths {
            let full_path = Self::join_base_path(base, path);
            routes.push(OpenApiRoute::from_operation(method, full_path, operation, spec.clone()));
        }
    }

    fn collect_base_paths(spec: &Arc<OpenApiSpec>) -> Vec<String> {
        let mut base_paths = Vec::new();

        for server in spec.servers() {
            if let Some(base_path) = Self::extract_base_path(server.url.as_str()) {
                if !base_paths.contains(&base_path) {
                    base_paths.push(base_path);
                }
            }
        }

        if base_paths.is_empty() {
            base_paths.push(String::new());
        }

        base_paths
    }

    fn extract_base_path(raw_url: &str) -> Option<String> {
        let trimmed = raw_url.trim();
        if trimmed.is_empty() {
            return None;
        }

        if trimmed.starts_with('/') {
            return Some(Self::normalize_base_path(trimmed));
        }

        if let Ok(parsed) = Url::parse(trimmed) {
            return Some(Self::normalize_base_path(parsed.path()));
        }

        None
    }

    fn normalize_base_path(path: &str) -> String {
        let trimmed = path.trim();
        if trimmed.is_empty() || trimmed == "/" {
            String::new()
        } else {
            let mut normalized = trimmed.trim_end_matches('/').to_string();
            if !normalized.starts_with('/') {
                normalized.insert(0, '/');
            }
            normalized
        }
    }

    fn join_base_path(base: &str, path: &str) -> String {
        let trimmed_path = path.trim_start_matches('/');

        if base.is_empty() {
            if trimmed_path.is_empty() {
                "/".to_string()
            } else {
                format!("/{}", trimmed_path)
            }
        } else if trimmed_path.is_empty() {
            base.to_string()
        } else {
            format!("{}/{}", base, trimmed_path)
        }
    }

    fn resolve_path_item(
        value: &ReferenceOr<PathItem>,
        spec: &Arc<OpenApiSpec>,
        visited: &mut HashSet<String>,
    ) -> Option<PathItem> {
        match value {
            ReferenceOr::Item(item) => Some(item.clone()),
            ReferenceOr::Reference { reference } => {
                Self::resolve_path_item_reference(reference, spec, visited)
            }
        }
    }

    fn resolve_path_item_reference(
        reference: &str,
        spec: &Arc<OpenApiSpec>,
        visited: &mut HashSet<String>,
    ) -> Option<PathItem> {
        if !visited.insert(reference.to_string()) {
            tracing::warn!("Detected recursive path item reference: {}", reference);
            return None;
        }

        if let Some(name) = reference.strip_prefix("#/components/pathItems/") {
            return Self::resolve_component_path_item(name, spec, visited);
        }

        if let Some(pointer) = reference.strip_prefix("#/paths/") {
            let decoded_path = Self::decode_json_pointer(pointer);
            if let Some(next) = spec.spec.paths.paths.get(&decoded_path) {
                return Self::resolve_path_item(next, spec, visited);
            }
            tracing::warn!(
                "Path reference {} resolved to missing path '{}'",
                reference,
                decoded_path
            );
            return None;
        }

        tracing::warn!("Unsupported path item reference: {}", reference);
        None
    }

    fn resolve_component_path_item(
        name: &str,
        spec: &Arc<OpenApiSpec>,
        visited: &mut HashSet<String>,
    ) -> Option<PathItem> {
        let raw = spec.raw_document.as_ref()?;
        let components = raw.get("components")?.as_object()?;
        let path_items = components.get("pathItems")?.as_object()?;
        let item_value = path_items.get(name)?;

        if let Some(reference) = item_value
            .as_object()
            .and_then(|obj| obj.get("$ref"))
            .and_then(|value| value.as_str())
        {
            tracing::debug!(
                "Resolving components.pathItems entry '{}' via reference {}",
                name,
                reference
            );
            return Self::resolve_path_item_reference(reference, spec, visited);
        }

        match serde_json::from_value(item_value.clone()) {
            Ok(item) => Some(item),
            Err(err) => {
                tracing::warn!(
                    "Failed to deserialize components.pathItems entry '{}' as a PathItem: {}",
                    name,
                    err
                );
                None
            }
        }
    }

    fn decode_json_pointer(pointer: &str) -> String {
        let segments: Vec<String> = pointer
            .split('/')
            .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
            .collect();
        segments.join("/")
    }

    /// Get all generated routes
    pub fn routes(&self) -> &[OpenApiRoute] {
        &self.routes
    }

    /// Get the OpenAPI specification used to generate routes
    pub fn spec(&self) -> &OpenApiSpec {
        &self.spec
    }

    /// Get immutable reference to validation options
    pub fn options(&self) -> &ValidationOptions {
        &self.options
    }

    /// Get mutable reference to validation options for runtime configuration changes
    pub fn options_mut(&mut self) -> &mut ValidationOptions {
        &mut self.options
    }

    /// Build an Axum router from the generated routes
    pub fn build_router(&self) -> axum::Router {
        use axum::routing::{delete, get, patch, post, put};

        let mut router = axum::Router::new();
        tracing::debug!("Building router from {} routes", self.routes.len());

        for route in &self.routes {
            tracing::debug!("Adding route: {} {}", route.method, route.path);
            tracing::debug!(
                "Route operation responses: {:?}",
                route.operation.responses.responses.keys().collect::<Vec<_>>()
            );

            let route_clone = route.clone();
            let handler = move || {
                let route = route_clone.clone();
                async move {
                    tracing::debug!("Handling request for route: {} {}", route.method, route.path);
                    let (status, response, trace) =
                        route.mock_response_with_status_and_scenario_and_trace(None);
                    tracing::debug!("Generated response with status: {}", status);

                    // Create response with trace attached to extensions
                    use axum::response::IntoResponse;
                    let mut axum_response = (
                        axum::http::StatusCode::from_u16(status)
                            .unwrap_or(axum::http::StatusCode::OK),
                        axum::response::Json(response),
                    )
                        .into_response();

                    // Attach trace to response extensions so logging middleware can pick it up
                    axum_response.extensions_mut().insert(trace);

                    axum_response
                }
            };

            match route.method.as_str() {
                "GET" => {
                    tracing::debug!("Registering GET route: {}", route.path);
                    router = router.route(&route.path, get(handler));
                }
                "POST" => {
                    tracing::debug!("Registering POST route: {}", route.path);
                    router = router.route(&route.path, post(handler));
                }
                "PUT" => {
                    tracing::debug!("Registering PUT route: {}", route.path);
                    router = router.route(&route.path, put(handler));
                }
                "DELETE" => {
                    tracing::debug!("Registering DELETE route: {}", route.path);
                    router = router.route(&route.path, delete(handler));
                }
                "PATCH" => {
                    tracing::debug!("Registering PATCH route: {}", route.path);
                    router = router.route(&route.path, patch(handler));
                }
                _ => tracing::warn!("Unsupported HTTP method: {}", route.method),
            }
        }

        router
    }

    /// Build router with latency and failure injection support
    ///
    /// # Arguments
    /// * `latency_injector` - Latency injector for simulating network delays
    /// * `failure_injector` - Optional failure injector for simulating errors
    ///
    /// # Returns
    /// Axum router with chaos engineering capabilities
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
    ///
    /// # Arguments
    /// * `path` - Request path (e.g., "/users/123")
    /// * `method` - HTTP method (e.g., "GET")
    ///
    /// # Returns
    /// Map of parameter names to values extracted from the path
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

    /// Build router with AI generator support for dynamic response generation
    ///
    /// # Arguments
    /// * `ai_generator` - Optional AI generator for creating dynamic responses based on request context
    ///
    /// # Returns
    /// Axum router with AI-powered response generation
    pub fn build_router_with_ai(
        &self,
        ai_generator: Option<std::sync::Arc<dyn AiGenerator + Send + Sync>>,
    ) -> axum::Router {
        use axum::routing::{delete, get, patch, post, put};

        let mut router = axum::Router::new();
        tracing::debug!("Building router with AI support from {} routes", self.routes.len());

        for route in &self.routes {
            tracing::debug!("Adding AI-enabled route: {} {}", route.method, route.path);

            let route_clone = route.clone();
            let ai_generator_clone = ai_generator.clone();

            // Create async handler that extracts request data and builds context
            let handler = move |headers: HeaderMap, body: Option<Json<Value>>| {
                let route = route_clone.clone();
                let ai_generator = ai_generator_clone.clone();

                async move {
                    tracing::debug!(
                        "Handling AI request for route: {} {}",
                        route.method,
                        route.path
                    );

                    // Build request context
                    let mut context = RequestContext::new(route.method.clone(), route.path.clone());

                    // Extract headers
                    context.headers = headers
                        .iter()
                        .map(|(k, v)| {
                            (k.to_string(), Value::String(v.to_str().unwrap_or("").to_string()))
                        })
                        .collect();

                    // Extract body if present
                    context.body = body.map(|Json(b)| b);

                    // Generate AI response if AI generator is available and route has AI config
                    let (status, response) = if let (Some(generator), Some(_ai_config)) =
                        (ai_generator, &route.ai_config)
                    {
                        route
                            .mock_response_with_status_async(&context, Some(generator.as_ref()))
                            .await
                    } else {
                        // No AI support, use static response
                        route.mock_response_with_status()
                    };

                    (
                        axum::http::StatusCode::from_u16(status)
                            .unwrap_or(axum::http::StatusCode::OK),
                        axum::response::Json(response),
                    )
                }
            };

            match route.method.as_str() {
                "GET" => {
                    router = router.route(&route.path, get(handler));
                }
                "POST" => {
                    router = router.route(&route.path, post(handler));
                }
                "PUT" => {
                    router = router.route(&route.path, put(handler));
                }
                "DELETE" => {
                    router = router.route(&route.path, delete(handler));
                }
                "PATCH" => {
                    router = router.route(&route.path, patch(handler));
                }
                _ => tracing::warn!("Unsupported HTTP method for AI: {}", route.method),
            }
        }

        router
    }

    /// Build router with MockAI (Behavioral Mock Intelligence) support
    ///
    /// This method integrates MockAI for intelligent, context-aware response generation,
    /// mutation detection, validation error generation, and pagination intelligence.
    ///
    /// # Arguments
    /// * `mockai` - Optional MockAI instance for intelligent behavior
    ///
    /// # Returns
    /// Axum router with MockAI-powered response generation
    pub fn build_router_with_mockai(
        &self,
        mockai: Option<std::sync::Arc<tokio::sync::RwLock<crate::intelligent_behavior::MockAI>>>,
    ) -> axum::Router {
        use crate::intelligent_behavior::Request as MockAIRequest;

        use axum::routing::{delete, get, patch, post, put};

        let mut router = axum::Router::new();
        tracing::debug!("Building router with MockAI support from {} routes", self.routes.len());

        for route in &self.routes {
            tracing::debug!("Adding MockAI-enabled route: {} {}", route.method, route.path);

            let route_clone = route.clone();
            let mockai_clone = mockai.clone();

            // Create async handler that processes requests through MockAI
            // Query params are extracted via Query extractor with HashMap
            // Note: Using Query<HashMap<String, String>> to handle query params
            let handler = move |query: axum::extract::Query<HashMap<String, String>>,
                                headers: HeaderMap,
                                body: Option<Json<Value>>| {
                let route = route_clone.clone();
                let mockai = mockai_clone.clone();

                async move {
                    tracing::debug!(
                        "Handling MockAI request for route: {} {}",
                        route.method,
                        route.path
                    );

                    // Query parameters are already parsed by Query extractor
                    let mockai_query = query.0;

                    // If MockAI is enabled, use it to process the request
                    if let Some(mockai_arc) = mockai {
                        let mockai_guard = mockai_arc.read().await;

                        // Build MockAI request
                        let mut mockai_headers = HashMap::new();
                        for (k, v) in headers.iter() {
                            mockai_headers
                                .insert(k.to_string(), v.to_str().unwrap_or("").to_string());
                        }

                        let mockai_request = MockAIRequest {
                            method: route.method.clone(),
                            path: route.path.clone(),
                            body: body.as_ref().map(|Json(b)| b.clone()),
                            query_params: mockai_query,
                            headers: mockai_headers,
                        };

                        // Process request through MockAI
                        match mockai_guard.process_request(&mockai_request).await {
                            Ok(mockai_response) => {
                                tracing::debug!(
                                    "MockAI generated response with status: {}",
                                    mockai_response.status_code
                                );
                                return (
                                    axum::http::StatusCode::from_u16(mockai_response.status_code)
                                        .unwrap_or(axum::http::StatusCode::OK),
                                    axum::response::Json(mockai_response.body),
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "MockAI processing failed for {} {}: {}, falling back to standard response",
                                    route.method,
                                    route.path,
                                    e
                                );
                                // Fall through to standard response generation
                            }
                        }
                    }

                    // Fallback to standard response generation
                    let (status, response) = route.mock_response_with_status();
                    (
                        axum::http::StatusCode::from_u16(status)
                            .unwrap_or(axum::http::StatusCode::OK),
                        axum::response::Json(response),
                    )
                }
            };

            match route.method.as_str() {
                "GET" => {
                    router = router.route(&route.path, get(handler));
                }
                "POST" => {
                    router = router.route(&route.path, post(handler));
                }
                "PUT" => {
                    router = router.route(&route.path, put(handler));
                }
                "DELETE" => {
                    router = router.route(&route.path, delete(handler));
                }
                "PATCH" => {
                    router = router.route(&route.path, patch(handler));
                }
                _ => tracing::warn!("Unsupported HTTP method for MockAI: {}", route.method),
            }
        }

        router
    }
}
