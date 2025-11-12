//! OpenAPI-based route generation for MockForge
//!
//! This module has been refactored into sub-modules for better organization:
//! - registry: OpenAPI route registry and management
//! - validation: Request/response validation logic
//! - generation: Route generation from OpenAPI specs
//! - builder: Axum router building from OpenAPI specs

// Re-export sub-modules for backward compatibility
pub mod builder;
pub mod generation;
pub mod registry;
pub mod validation;

// Re-export commonly used types
pub use builder::*;
pub use generation::*;
pub use validation::*;

// Legacy types and functions for backward compatibility
use crate::ai_response::RequestContext;
use crate::openapi::response::AiGenerator;
use crate::openapi::{OpenApiOperation, OpenApiRoute, OpenApiSchema, OpenApiSpec};
use crate::templating::expand_tokens as core_expand_tokens;
use crate::{latency::LatencyInjector, overrides::Overrides, Error, Result};
use axum::extract::{Path as AxumPath, RawQuery};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::routing::*;
use axum::{Json, Router};
use chrono::Utc;
use once_cell::sync::Lazy;
use openapiv3::ParameterSchemaOrContent;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tracing;

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

/// Validation mode for request/response validation
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub enum ValidationMode {
    /// Validation is disabled (no checks performed)
    Disabled,
    /// Validation warnings are logged but do not fail requests
    #[default]
    Warn,
    /// Validation failures return error responses
    Enforce,
}

/// Options for configuring validation behavior
#[derive(Debug, Clone)]
pub struct ValidationOptions {
    /// Validation mode for incoming requests
    pub request_mode: ValidationMode,
    /// Whether to aggregate multiple validation errors into a single response
    pub aggregate_errors: bool,
    /// Whether to validate outgoing responses against schemas
    pub validate_responses: bool,
    /// Per-operation validation mode overrides (operation ID -> mode)
    pub overrides: std::collections::HashMap<String, ValidationMode>,
    /// Skip validation for request paths starting with any of these prefixes
    pub admin_skip_prefixes: Vec<String>,
    /// Expand templating tokens in responses/examples after generation
    pub response_template_expand: bool,
    /// HTTP status code to return for validation failures (e.g., 400 or 422)
    pub validation_status: Option<u16>,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            request_mode: ValidationMode::Enforce,
            aggregate_errors: true,
            validate_responses: false,
            overrides: std::collections::HashMap::new(),
            admin_skip_prefixes: Vec::new(),
            response_template_expand: false,
            validation_status: None,
        }
    }
}

impl OpenApiRouteRegistry {
    /// Create a new registry from an OpenAPI spec
    pub fn new(spec: OpenApiSpec) -> Self {
        Self::new_with_env(spec)
    }

    /// Create a new registry from an OpenAPI spec with environment-based validation options
    ///
    /// Options are read from environment variables:
    /// - `MOCKFORGE_REQUEST_VALIDATION`: "off"/"warn"/"enforce" (default: "enforce")
    /// - `MOCKFORGE_AGGREGATE_ERRORS`: "1"/"true" to aggregate errors (default: true)
    /// - `MOCKFORGE_RESPONSE_VALIDATION`: "1"/"true" to validate responses (default: false)
    /// - `MOCKFORGE_RESPONSE_TEMPLATE_EXPAND`: "1"/"true" to expand templates (default: false)
    /// - `MOCKFORGE_VALIDATION_STATUS`: HTTP status code for validation failures (optional)
    pub fn new_with_env(spec: OpenApiSpec) -> Self {
        Self::new_with_env_and_persona(spec, None)
    }

    /// Create a new registry from an OpenAPI spec with environment-based validation options and persona
    pub fn new_with_env_and_persona(
        spec: OpenApiSpec,
        persona: Option<Arc<crate::intelligent_behavior::config::Persona>>,
    ) -> Self {
        tracing::debug!("Creating OpenAPI route registry");
        let spec = Arc::new(spec);
        let routes = Self::generate_routes_with_persona(&spec, persona);
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
            overrides: std::collections::HashMap::new(),
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
        Self::new_with_options_and_persona(spec, options, None)
    }

    /// Construct with explicit options and persona
    pub fn new_with_options_and_persona(
        spec: OpenApiSpec,
        options: ValidationOptions,
        persona: Option<Arc<crate::intelligent_behavior::config::Persona>>,
    ) -> Self {
        tracing::debug!("Creating OpenAPI route registry with custom options");
        let spec = Arc::new(spec);
        let routes = Self::generate_routes_with_persona(&spec, persona);
        Self {
            spec,
            routes,
            options,
        }
    }

    /// Clone this registry for validation purposes (creates an independent copy)
    ///
    /// This is useful when you need a separate registry instance for validation
    /// that won't interfere with the main registry's state.
    pub fn clone_for_validation(&self) -> Self {
        OpenApiRouteRegistry {
            spec: self.spec.clone(),
            routes: self.routes.clone(),
            options: self.options.clone(),
        }
    }

    /// Generate routes from the OpenAPI specification
    fn generate_routes(spec: &Arc<OpenApiSpec>) -> Vec<OpenApiRoute> {
        Self::generate_routes_with_persona(spec, None)
    }

    /// Generate routes from the OpenAPI specification with optional persona
    fn generate_routes_with_persona(spec: &Arc<OpenApiSpec>, persona: Option<Arc<crate::intelligent_behavior::config::Persona>>) -> Vec<OpenApiRoute> {
        let mut routes = Vec::new();

        let all_paths_ops = spec.all_paths_and_operations();
        tracing::debug!("Generating routes from OpenAPI spec with {} paths", all_paths_ops.len());

        for (path, operations) in all_paths_ops {
            tracing::debug!("Processing path: {}", path);
            for (method, operation) in operations {
                routes.push(OpenApiRoute::from_operation_with_persona(
                    &method,
                    path.clone(),
                    &operation,
                    spec.clone(),
                    persona.clone(),
                ));
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

    /// Build an Axum router from the OpenAPI spec (simplified)
    pub fn build_router(self) -> Router {
        let mut router = Router::new();
        tracing::debug!("Building router from {} routes", self.routes.len());

        // Create individual routes for each operation
        for route in &self.routes {
            tracing::debug!("Adding route: {} {}", route.method, route.path);
            let axum_path = route.axum_path();
            let operation = route.operation.clone();
            let method = route.method.clone();
            let path_template = route.path.clone();
            let validator = self.clone_for_validation();
            let route_clone = route.clone();

            // Handler: validate path/query/header/cookie/body, then return mock
            let handler = move |AxumPath(path_params): AxumPath<
                std::collections::HashMap<String, String>,
            >,
                                RawQuery(raw_query): RawQuery,
                                headers: HeaderMap,
                                body: axum::body::Bytes| async move {
                tracing::debug!("Handling OpenAPI request: {} {}", method, path_template);

                // Determine scenario from header or environment variable
                // Header takes precedence over environment variable
                let scenario = headers
                    .get("X-Mockforge-Scenario")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
                    .or_else(|| std::env::var("MOCKFORGE_HTTP_SCENARIO").ok());

                // Generate mock response for this request with scenario support
                let (selected_status, mock_response) =
                    route_clone.mock_response_with_status_and_scenario(scenario.as_deref());
                // Admin routes are mounted separately; no validation skip needed here.
                // Build params maps
                let mut path_map = serde_json::Map::new();
                for (k, v) in path_params {
                    path_map.insert(k, Value::String(v));
                }

                // Query
                let mut query_map = Map::new();
                if let Some(q) = raw_query {
                    for (k, v) in url::form_urlencoded::parse(q.as_bytes()) {
                        query_map.insert(k.to_string(), Value::String(v.to_string()));
                    }
                }

                // Headers: only capture those declared on this operation
                let mut header_map = Map::new();
                for p_ref in &operation.parameters {
                    if let Some(openapiv3::Parameter::Header { parameter_data, .. }) =
                        p_ref.as_item()
                    {
                        let name_lc = parameter_data.name.to_ascii_lowercase();
                        if let Ok(hn) = axum::http::HeaderName::from_bytes(name_lc.as_bytes()) {
                            if let Some(val) = headers.get(hn) {
                                if let Ok(s) = val.to_str() {
                                    header_map.insert(
                                        parameter_data.name.clone(),
                                        Value::String(s.to_string()),
                                    );
                                }
                            }
                        }
                    }
                }

                // Cookies: parse Cookie header
                let mut cookie_map = Map::new();
                if let Some(val) = headers.get(axum::http::header::COOKIE) {
                    if let Ok(s) = val.to_str() {
                        for part in s.split(';') {
                            let part = part.trim();
                            if let Some((k, v)) = part.split_once('=') {
                                cookie_map.insert(k.to_string(), Value::String(v.to_string()));
                            }
                        }
                    }
                }

                // Check if this is a multipart request
                let is_multipart = headers
                    .get(axum::http::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .map(|ct| ct.starts_with("multipart/form-data"))
                    .unwrap_or(false);

                // Extract multipart data if applicable
                let mut multipart_fields = std::collections::HashMap::new();
                let mut multipart_files = std::collections::HashMap::new();
                let mut body_json: Option<Value> = None;

                if is_multipart {
                    // For multipart requests, extract fields and files
                    match extract_multipart_from_bytes(&body, &headers).await {
                        Ok((fields, files)) => {
                            multipart_fields = fields;
                            multipart_files = files;
                            // Also create a JSON representation for validation
                            let mut body_obj = serde_json::Map::new();
                            for (k, v) in &multipart_fields {
                                body_obj.insert(k.clone(), v.clone());
                            }
                            if !body_obj.is_empty() {
                                body_json = Some(Value::Object(body_obj));
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse multipart data: {}", e);
                        }
                    }
                } else {
                    // Body: try JSON when present
                    body_json = if !body.is_empty() {
                        serde_json::from_slice(&body).ok()
                    } else {
                        None
                    };
                }

                if let Err(e) = validator.validate_request_with_all(
                    &path_template,
                    &method,
                    &path_map,
                    &query_map,
                    &header_map,
                    &cookie_map,
                    body_json.as_ref(),
                ) {
                    // Choose status: prefer options.validation_status, fallback to env, else 400
                    let status_code = validator.options.validation_status.unwrap_or_else(|| {
                        std::env::var("MOCKFORGE_VALIDATION_STATUS")
                            .ok()
                            .and_then(|s| s.parse::<u16>().ok())
                            .unwrap_or(400)
                    });

                    let payload = if status_code == 422 {
                        // For 422 responses, use enhanced schema validation with detailed errors
                        // Note: We need to extract parameters from the request context
                        // For now, using empty maps as placeholders
                        let empty_params = serde_json::Map::new();
                        generate_enhanced_422_response(
                            &validator,
                            &path_template,
                            &method,
                            body_json.as_ref(),
                            &empty_params, // path_params
                            &empty_params, // query_params
                            &empty_params, // header_params
                            &empty_params, // cookie_params
                        )
                    } else {
                        // For other status codes, use generic error format
                        let msg = format!("{}", e);
                        let detail_val = serde_json::from_str::<serde_json::Value>(&msg)
                            .unwrap_or(serde_json::json!(msg));
                        json!({
                            "error": "request validation failed",
                            "detail": detail_val,
                            "method": method,
                            "path": path_template,
                            "timestamp": Utc::now().to_rfc3339(),
                        })
                    };

                    record_validation_error(&payload);
                    let status = axum::http::StatusCode::from_u16(status_code)
                        .unwrap_or(axum::http::StatusCode::BAD_REQUEST);

                    // Serialize payload with fallback for serialization errors
                    let body_bytes = serde_json::to_vec(&payload)
                        .unwrap_or_else(|_| br#"{"error":"Serialization failed"}"#.to_vec());

                    return axum::http::Response::builder()
                        .status(status)
                        .header(axum::http::header::CONTENT_TYPE, "application/json")
                        .body(axum::body::Body::from(body_bytes))
                        .expect("Response builder should create valid response with valid headers and body");
                }

                // Expand tokens in the response if enabled (options or env)
                let mut final_response = mock_response.clone();
                let env_expand = std::env::var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND")
                    .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false);
                let expand = validator.options.response_template_expand || env_expand;
                if expand {
                    final_response = core_expand_tokens(&final_response);
                }

                // Optional response validation
                if validator.options.validate_responses {
                    // Find the first 2xx response in the operation
                    if let Some((status_code, _response)) = operation
                        .responses
                        .responses
                        .iter()
                        .filter_map(|(status, resp)| match status {
                            openapiv3::StatusCode::Code(code) if *code >= 200 && *code < 300 => {
                                resp.as_item().map(|r| ((*code), r))
                            }
                            openapiv3::StatusCode::Range(range)
                                if *range >= 200 && *range < 300 =>
                            {
                                resp.as_item().map(|r| (200, r))
                            }
                            _ => None,
                        })
                        .next()
                    {
                        // Basic response validation - check if response is valid JSON
                        if serde_json::from_value::<serde_json::Value>(final_response.clone())
                            .is_err()
                        {
                            tracing::warn!(
                                "Response validation failed: invalid JSON for status {}",
                                status_code
                            );
                        }
                    }
                }

                // Return the mock response with the correct status code
                let mut response = Json(final_response).into_response();
                *response.status_mut() = axum::http::StatusCode::from_u16(selected_status)
                    .unwrap_or(axum::http::StatusCode::OK);
                response
            };

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

    /// Build an Axum router from the OpenAPI spec with latency injection support
    pub fn build_router_with_latency(self, latency_injector: LatencyInjector) -> Router {
        self.build_router_with_injectors(latency_injector, None)
    }

    /// Build an Axum router from the OpenAPI spec with both latency and failure injection support
    pub fn build_router_with_injectors(
        self,
        latency_injector: LatencyInjector,
        failure_injector: Option<crate::FailureInjector>,
    ) -> Router {
        self.build_router_with_injectors_and_overrides(
            latency_injector,
            failure_injector,
            None,
            false,
        )
    }

    /// Build an Axum router from the OpenAPI spec with latency, failure injection, and overrides support
    pub fn build_router_with_injectors_and_overrides(
        self,
        latency_injector: LatencyInjector,
        failure_injector: Option<crate::FailureInjector>,
        overrides: Option<Overrides>,
        overrides_enabled: bool,
    ) -> Router {
        let mut router = Router::new();

        // Create individual routes for each operation
        for route in &self.routes {
            let axum_path = route.axum_path();
            let operation = route.operation.clone();
            let method = route.method.clone();
            let method_str = method.clone();
            let method_for_router = method_str.clone();
            let path_template = route.path.clone();
            let validator = self.clone_for_validation();
            let route_clone = route.clone();
            let injector = latency_injector.clone();
            let failure_injector = failure_injector.clone();
            let route_overrides = overrides.clone();

            // Extract tags from operation for latency and failure injection
            let mut operation_tags = operation.tags.clone();
            if let Some(operation_id) = &operation.operation_id {
                operation_tags.push(operation_id.clone());
            }

            // Handler: inject latency, validate path/query/header/cookie/body, then return mock
            let handler = move |AxumPath(path_params): AxumPath<
                std::collections::HashMap<String, String>,
            >,
                                RawQuery(raw_query): RawQuery,
                                headers: HeaderMap,
                                body: axum::body::Bytes| async move {
                // Check for failure injection first
                if let Some(ref failure_injector) = failure_injector {
                    if let Some((status_code, error_message)) =
                        failure_injector.process_request(&operation_tags)
                    {
                        return (
                            axum::http::StatusCode::from_u16(status_code)
                                .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                            axum::Json(serde_json::json!({
                                "error": error_message,
                                "injected_failure": true
                            })),
                        );
                    }
                }

                // Inject latency before processing the request
                if let Err(e) = injector.inject_latency(&operation_tags).await {
                    tracing::warn!("Failed to inject latency: {}", e);
                }

                // Determine scenario from header or environment variable
                // Header takes precedence over environment variable
                let scenario = headers
                    .get("X-Mockforge-Scenario")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
                    .or_else(|| std::env::var("MOCKFORGE_HTTP_SCENARIO").ok());

                // Admin routes are mounted separately; no validation skip needed here.
                // Build params maps
                let mut path_map = Map::new();
                for (k, v) in path_params {
                    path_map.insert(k, Value::String(v));
                }

                // Query
                let mut query_map = Map::new();
                if let Some(q) = raw_query {
                    for (k, v) in url::form_urlencoded::parse(q.as_bytes()) {
                        query_map.insert(k.to_string(), Value::String(v.to_string()));
                    }
                }

                // Headers: only capture those declared on this operation
                let mut header_map = Map::new();
                for p_ref in &operation.parameters {
                    if let Some(openapiv3::Parameter::Header { parameter_data, .. }) =
                        p_ref.as_item()
                    {
                        let name_lc = parameter_data.name.to_ascii_lowercase();
                        if let Ok(hn) = axum::http::HeaderName::from_bytes(name_lc.as_bytes()) {
                            if let Some(val) = headers.get(hn) {
                                if let Ok(s) = val.to_str() {
                                    header_map.insert(
                                        parameter_data.name.clone(),
                                        Value::String(s.to_string()),
                                    );
                                }
                            }
                        }
                    }
                }

                // Cookies: parse Cookie header
                let mut cookie_map = Map::new();
                if let Some(val) = headers.get(axum::http::header::COOKIE) {
                    if let Ok(s) = val.to_str() {
                        for part in s.split(';') {
                            let part = part.trim();
                            if let Some((k, v)) = part.split_once('=') {
                                cookie_map.insert(k.to_string(), Value::String(v.to_string()));
                            }
                        }
                    }
                }

                // Check if this is a multipart request
                let is_multipart = headers
                    .get(axum::http::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .map(|ct| ct.starts_with("multipart/form-data"))
                    .unwrap_or(false);

                // Extract multipart data if applicable
                let mut multipart_fields = std::collections::HashMap::new();
                let mut multipart_files = std::collections::HashMap::new();
                let mut body_json: Option<Value> = None;

                if is_multipart {
                    // For multipart requests, extract fields and files
                    match extract_multipart_from_bytes(&body, &headers).await {
                        Ok((fields, files)) => {
                            multipart_fields = fields;
                            multipart_files = files;
                            // Also create a JSON representation for validation
                            let mut body_obj = serde_json::Map::new();
                            for (k, v) in &multipart_fields {
                                body_obj.insert(k.clone(), v.clone());
                            }
                            if !body_obj.is_empty() {
                                body_json = Some(Value::Object(body_obj));
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse multipart data: {}", e);
                        }
                    }
                } else {
                    // Body: try JSON when present
                    body_json = if !body.is_empty() {
                        serde_json::from_slice(&body).ok()
                    } else {
                        None
                    };
                }

                if let Err(e) = validator.validate_request_with_all(
                    &path_template,
                    &method_str,
                    &path_map,
                    &query_map,
                    &header_map,
                    &cookie_map,
                    body_json.as_ref(),
                ) {
                    let msg = format!("{}", e);
                    let detail_val = serde_json::from_str::<serde_json::Value>(&msg)
                        .unwrap_or(serde_json::json!(msg));
                    let payload = serde_json::json!({
                        "error": "request validation failed",
                        "detail": detail_val,
                        "method": method_str,
                        "path": path_template,
                        "timestamp": Utc::now().to_rfc3339(),
                    });
                    record_validation_error(&payload);
                    // Choose status: prefer options.validation_status, fallback to env, else 400
                    let status_code = validator.options.validation_status.unwrap_or_else(|| {
                        std::env::var("MOCKFORGE_VALIDATION_STATUS")
                            .ok()
                            .and_then(|s| s.parse::<u16>().ok())
                            .unwrap_or(400)
                    });
                    return (
                        axum::http::StatusCode::from_u16(status_code)
                            .unwrap_or(axum::http::StatusCode::BAD_REQUEST),
                        Json(payload),
                    );
                }

                // Generate mock response with scenario support
                let (selected_status, mock_response) =
                    route_clone.mock_response_with_status_and_scenario(scenario.as_deref());

                // Expand templating tokens in response if enabled (options or env)
                let mut response = mock_response.clone();
                let env_expand = std::env::var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND")
                    .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false);
                let expand = validator.options.response_template_expand || env_expand;
                if expand {
                    response = core_expand_tokens(&response);
                }

                // Apply overrides if provided and enabled
                if let Some(ref overrides) = route_overrides {
                    if overrides_enabled {
                        // Extract tags from operation for override matching
                        let operation_tags =
                            operation.operation_id.clone().map(|id| vec![id]).unwrap_or_default();
                        overrides.apply(
                            &operation.operation_id.unwrap_or_default(),
                            &operation_tags,
                            &path_template,
                            &mut response,
                        );
                    }
                }

                // Return the mock response
                (
                    axum::http::StatusCode::from_u16(selected_status)
                        .unwrap_or(axum::http::StatusCode::OK),
                    Json(response),
                )
            };

            // Add route to router based on HTTP method
            router = match method_for_router.as_str() {
                "GET" => router.route(&axum_path, get(handler)),
                "POST" => router.route(&axum_path, post(handler)),
                "PUT" => router.route(&axum_path, put(handler)),
                "PATCH" => router.route(&axum_path, patch(handler)),
                "DELETE" => router.route(&axum_path, delete(handler)),
                "HEAD" => router.route(&axum_path, head(handler)),
                "OPTIONS" => router.route(&axum_path, options(handler)),
                _ => router.route(&axum_path, get(handler)), // Default to GET for unknown methods
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
        self.validate_request_with_all(
            path,
            method,
            path_params,
            query_params,
            &Map::new(),
            &Map::new(),
            body,
        )
    }

    /// Validate request against OpenAPI spec with path/query/header/cookie params
    #[allow(clippy::too_many_arguments)]
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
        // Skip validation for any configured admin prefixes
        for pref in &self.options.admin_skip_prefixes {
            if !pref.is_empty() && path.starts_with(pref) {
                return Ok(());
            }
        }
        // Runtime env overrides
        let env_mode = std::env::var("MOCKFORGE_REQUEST_VALIDATION").ok().map(|v| {
            match v.to_ascii_lowercase().as_str() {
                "off" | "disable" | "disabled" => ValidationMode::Disabled,
                "warn" | "warning" => ValidationMode::Warn,
                _ => ValidationMode::Enforce,
            }
        });
        let aggregate = std::env::var("MOCKFORGE_AGGREGATE_ERRORS")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(self.options.aggregate_errors);
        // Per-route runtime overrides via JSON env var
        let env_overrides: Option<serde_json::Map<String, serde_json::Value>> =
            std::env::var("MOCKFORGE_VALIDATION_OVERRIDES_JSON")
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .and_then(|v| v.as_object().cloned());
        // Response validation is handled in HTTP layer now
        let mut effective_mode = env_mode.unwrap_or(self.options.request_mode.clone());
        // Apply runtime overrides first if present
        if let Some(map) = &env_overrides {
            if let Some(v) = map.get(&format!("{} {}", method, path)) {
                if let Some(m) = v.as_str() {
                    effective_mode = match m {
                        "off" => ValidationMode::Disabled,
                        "warn" => ValidationMode::Warn,
                        _ => ValidationMode::Enforce,
                    };
                }
            }
        }
        // Then static options overrides
        if let Some(override_mode) = self.options.overrides.get(&format!("{} {}", method, path)) {
            effective_mode = override_mode.clone();
        }
        if matches!(effective_mode, ValidationMode::Disabled) {
            return Ok(());
        }
        if let Some(route) = self.get_route(path, method) {
            if matches!(effective_mode, ValidationMode::Disabled) {
                return Ok(());
            }
            let mut errors: Vec<String> = Vec::new();
            let mut details: Vec<serde_json::Value> = Vec::new();
            // Validate request body if required
            if let Some(schema) = &route.operation.request_body {
                if let Some(value) = body {
                    // First resolve the request body reference if it's a reference
                    let request_body = match schema {
                        openapiv3::ReferenceOr::Item(rb) => Some(rb),
                        openapiv3::ReferenceOr::Reference { reference } => {
                            // Try to resolve request body reference through spec
                            self.spec
                                .spec
                                .components
                                .as_ref()
                                .and_then(|components| {
                                    components.request_bodies.get(
                                        reference.trim_start_matches("#/components/requestBodies/"),
                                    )
                                })
                                .and_then(|rb_ref| rb_ref.as_item())
                        }
                    };

                    if let Some(rb) = request_body {
                        if let Some(content) = rb.content.get("application/json") {
                            if let Some(schema_ref) = &content.schema {
                                // Resolve schema reference and validate
                                match schema_ref {
                                    openapiv3::ReferenceOr::Item(schema) => {
                                        // Direct schema - validate immediately
                                        if let Err(validation_error) =
                                            OpenApiSchema::new(schema.clone()).validate(value)
                                        {
                                            let error_msg = validation_error.to_string();
                                            errors.push(format!(
                                                "body validation failed: {}",
                                                error_msg
                                            ));
                                            if aggregate {
                                                details.push(serde_json::json!({"path":"body","code":"schema_validation","message":error_msg}));
                                            }
                                        }
                                    }
                                    openapiv3::ReferenceOr::Reference { reference } => {
                                        // Referenced schema - resolve and validate
                                        if let Some(resolved_schema_ref) =
                                            self.spec.get_schema(reference)
                                        {
                                            if let Err(validation_error) = OpenApiSchema::new(
                                                resolved_schema_ref.schema.clone(),
                                            )
                                            .validate(value)
                                            {
                                                let error_msg = validation_error.to_string();
                                                errors.push(format!(
                                                    "body validation failed: {}",
                                                    error_msg
                                                ));
                                                if aggregate {
                                                    details.push(serde_json::json!({"path":"body","code":"schema_validation","message":error_msg}));
                                                }
                                            }
                                        } else {
                                            // Schema reference couldn't be resolved
                                            errors.push(format!("body validation failed: could not resolve schema reference {}", reference));
                                            if aggregate {
                                                details.push(serde_json::json!({"path":"body","code":"reference_error","message":"Could not resolve schema reference"}));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // Request body reference couldn't be resolved or no application/json content
                        errors.push("body validation failed: could not resolve request body or no application/json content".to_string());
                        if aggregate {
                            details.push(serde_json::json!({"path":"body","code":"reference_error","message":"Could not resolve request body reference"}));
                        }
                    }
                } else {
                    errors.push("body: Request body is required but not provided".to_string());
                    details.push(serde_json::json!({"path":"body","code":"required","message":"Request body is required"}));
                }
            } else if body.is_some() {
                // No body expected but provided — not an error by default, but log it
                tracing::debug!("Body provided for operation without requestBody; accepting");
            }

            // Validate path/query parameters
            for p_ref in &route.operation.parameters {
                if let Some(p) = p_ref.as_item() {
                    match p {
                        openapiv3::Parameter::Path { parameter_data, .. } => {
                            validate_parameter(
                                parameter_data,
                                path_params,
                                "path",
                                aggregate,
                                &mut errors,
                                &mut details,
                            );
                        }
                        openapiv3::Parameter::Query {
                            parameter_data,
                            style,
                            ..
                        } => {
                            // For query deepObject, reconstruct value from key-likes: name[prop]
                            let deep_value = None; // Simplified for now
                            let style_str = match style {
                                openapiv3::QueryStyle::Form => Some("form"),
                                openapiv3::QueryStyle::SpaceDelimited => Some("spaceDelimited"),
                                openapiv3::QueryStyle::PipeDelimited => Some("pipeDelimited"),
                                openapiv3::QueryStyle::DeepObject => Some("deepObject"),
                            };
                            validate_parameter_with_deep_object(
                                parameter_data,
                                query_params,
                                "query",
                                deep_value,
                                style_str,
                                aggregate,
                                &mut errors,
                                &mut details,
                            );
                        }
                        openapiv3::Parameter::Header { parameter_data, .. } => {
                            validate_parameter(
                                parameter_data,
                                header_params,
                                "header",
                                aggregate,
                                &mut errors,
                                &mut details,
                            );
                        }
                        openapiv3::Parameter::Cookie { parameter_data, .. } => {
                            validate_parameter(
                                parameter_data,
                                cookie_params,
                                "cookie",
                                aggregate,
                                &mut errors,
                                &mut details,
                            );
                        }
                    }
                }
            }
            if errors.is_empty() {
                return Ok(());
            }
            match effective_mode {
                ValidationMode::Disabled => Ok(()),
                ValidationMode::Warn => {
                    tracing::warn!("Request validation warnings: {:?}", errors);
                    Ok(())
                }
                ValidationMode::Enforce => Err(Error::validation(
                    serde_json::json!({"errors": errors, "details": details}).to_string(),
                )),
            }
        } else {
            Err(Error::generic(format!("Route {} {} not found in OpenAPI spec", method, path)))
        }
    }

    // Legacy helper removed (mock + status selection happens in handler via route.mock_response_with_status)

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
    pub fn get_operation(&self, path: &str, method: &str) -> Option<OpenApiOperation> {
        self.get_route(path, method).map(|route| {
            OpenApiOperation::from_operation(
                &route.method,
                route.path.clone(),
                &route.operation,
                &self.spec,
            )
        })
    }

    /// Extract path parameters from a request path by matching against known routes
    pub fn extract_path_parameters(
        &self,
        path: &str,
        method: &str,
    ) -> std::collections::HashMap<String, String> {
        for route in &self.routes {
            if route.method != method {
                continue;
            }

            if let Some(params) = self.match_path_to_route(path, &route.path) {
                return params;
            }
        }
        std::collections::HashMap::new()
    }

    /// Match a request path against a route pattern and extract parameters
    fn match_path_to_route(
        &self,
        request_path: &str,
        route_pattern: &str,
    ) -> Option<std::collections::HashMap<String, String>> {
        let mut params = std::collections::HashMap::new();

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

    /// Convert OpenAPI path to Axum-compatible path
    /// This is a utility function for converting path parameters from {param} to :param format
    pub fn convert_path_to_axum(openapi_path: &str) -> String {
        // Axum v0.7+ uses {param} format, same as OpenAPI
        openapi_path.to_string()
    }

    /// Build router with AI generator support
    pub fn build_router_with_ai(
        &self,
        ai_generator: Option<std::sync::Arc<dyn AiGenerator + Send + Sync>>,
    ) -> Router {
        use axum::routing::{delete, get, patch, post, put};

        let mut router = Router::new();
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
    ) -> Router {
        use crate::intelligent_behavior::{Request as MockAIRequest, Response as MockAIResponse};
        use axum::extract::Query;
        use axum::routing::{delete, get, patch, post, put};

        let mut router = Router::new();
        tracing::debug!("Building router with MockAI support from {} routes", self.routes.len());

        for route in &self.routes {
            tracing::debug!("Adding MockAI-enabled route: {} {}", route.method, route.path);

            let route_clone = route.clone();
            let mockai_clone = mockai.clone();

            // Create async handler that processes requests through MockAI
            // Query params are extracted via Query extractor with HashMap
            // Note: Using Query<HashMap<String, String>> wrapped in Option to handle missing query params
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

// Note: templating helpers are now in core::templating (shared across modules)

/// Extract multipart form data from request body bytes
/// Returns (form_fields, file_paths) where file_paths maps field names to stored file paths
async fn extract_multipart_from_bytes(
    body: &axum::body::Bytes,
    headers: &HeaderMap,
) -> Result<(
    std::collections::HashMap<String, Value>,
    std::collections::HashMap<String, String>,
)> {
    // Get boundary from Content-Type header
    let boundary = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|ct| {
            ct.split(';').find_map(|part| {
                let part = part.trim();
                if part.starts_with("boundary=") {
                    Some(part.strip_prefix("boundary=").unwrap_or("").trim_matches('"'))
                } else {
                    None
                }
            })
        })
        .ok_or_else(|| Error::generic("Missing boundary in Content-Type header"))?;

    let mut fields = std::collections::HashMap::new();
    let mut files = std::collections::HashMap::new();

    // Parse multipart data using bytes directly (not string conversion)
    // Multipart format: --boundary\r\n...\r\n--boundary\r\n...\r\n--boundary--\r\n
    let boundary_prefix = format!("--{}", boundary).into_bytes();
    let boundary_line = format!("\r\n--{}\r\n", boundary).into_bytes();
    let end_boundary = format!("\r\n--{}--\r\n", boundary).into_bytes();

    // Find all boundary positions
    let mut pos = 0;
    let mut parts = Vec::new();

    // Skip initial boundary if present
    if body.starts_with(&boundary_prefix) {
        if let Some(first_crlf) = body.iter().position(|&b| b == b'\r') {
            pos = first_crlf + 2; // Skip --boundary\r\n
        }
    }

    // Find all middle boundaries
    while let Some(boundary_pos) = body[pos..]
        .windows(boundary_line.len())
        .position(|window| window == boundary_line.as_slice())
    {
        let actual_pos = pos + boundary_pos;
        if actual_pos > pos {
            parts.push((pos, actual_pos));
        }
        pos = actual_pos + boundary_line.len();
    }

    // Find final boundary
    if let Some(end_pos) = body[pos..]
        .windows(end_boundary.len())
        .position(|window| window == end_boundary.as_slice())
    {
        let actual_end = pos + end_pos;
        if actual_end > pos {
            parts.push((pos, actual_end));
        }
    } else if pos < body.len() {
        // No final boundary found, treat rest as last part
        parts.push((pos, body.len()));
    }

    // Process each part
    for (start, end) in parts {
        let part_data = &body[start..end];

        // Find header/body separator (CRLF CRLF)
        let separator = b"\r\n\r\n";
        if let Some(sep_pos) =
            part_data.windows(separator.len()).position(|window| window == separator)
        {
            let header_bytes = &part_data[..sep_pos];
            let body_start = sep_pos + separator.len();
            let body_data = &part_data[body_start..];

            // Parse headers (assuming UTF-8)
            let header_str = String::from_utf8_lossy(header_bytes);
            let mut field_name = None;
            let mut filename = None;

            for header_line in header_str.lines() {
                if header_line.starts_with("Content-Disposition:") {
                    // Extract field name
                    if let Some(name_start) = header_line.find("name=\"") {
                        let name_start = name_start + 6;
                        if let Some(name_end) = header_line[name_start..].find('"') {
                            field_name =
                                Some(header_line[name_start..name_start + name_end].to_string());
                        }
                    }

                    // Extract filename if present
                    if let Some(file_start) = header_line.find("filename=\"") {
                        let file_start = file_start + 10;
                        if let Some(file_end) = header_line[file_start..].find('"') {
                            filename =
                                Some(header_line[file_start..file_start + file_end].to_string());
                        }
                    }
                }
            }

            if let Some(name) = field_name {
                if let Some(file) = filename {
                    // This is a file upload - store to temp directory
                    let temp_dir = std::env::temp_dir().join("mockforge-uploads");
                    std::fs::create_dir_all(&temp_dir).map_err(|e| {
                        Error::generic(format!("Failed to create temp directory: {}", e))
                    })?;

                    let file_path = temp_dir.join(format!("{}_{}", uuid::Uuid::new_v4(), file));
                    std::fs::write(&file_path, body_data)
                        .map_err(|e| Error::generic(format!("Failed to write file: {}", e)))?;

                    let file_path_str = file_path.to_string_lossy().to_string();
                    files.insert(name.clone(), file_path_str.clone());
                    fields.insert(name, Value::String(file_path_str));
                } else {
                    // This is a regular form field - try to parse as UTF-8 string
                    // Trim trailing CRLF
                    let body_str = body_data
                        .strip_suffix(b"\r\n")
                        .or_else(|| body_data.strip_suffix(b"\n"))
                        .unwrap_or(body_data);

                    if let Ok(field_value) = String::from_utf8(body_str.to_vec()) {
                        fields.insert(name, Value::String(field_value.trim().to_string()));
                    } else {
                        // Non-UTF-8 field value - store as base64 encoded string
                        use base64::{engine::general_purpose, Engine as _};
                        fields.insert(
                            name,
                            Value::String(general_purpose::STANDARD.encode(body_str)),
                        );
                    }
                }
            }
        }
    }

    Ok((fields, files))
}

static LAST_ERRORS: Lazy<Mutex<VecDeque<serde_json::Value>>> =
    Lazy::new(|| Mutex::new(VecDeque::with_capacity(20)));

/// Record last validation error for Admin UI inspection
pub fn record_validation_error(v: &serde_json::Value) {
    if let Ok(mut q) = LAST_ERRORS.lock() {
        if q.len() >= 20 {
            q.pop_front();
        }
        q.push_back(v.clone());
    }
    // If mutex is poisoned, we silently fail - validation errors are informational only
}

/// Get most recent validation error
pub fn get_last_validation_error() -> Option<serde_json::Value> {
    LAST_ERRORS.lock().ok()?.back().cloned()
}

/// Get recent validation errors (most recent last)
pub fn get_validation_errors() -> Vec<serde_json::Value> {
    LAST_ERRORS.lock().map(|q| q.iter().cloned().collect()).unwrap_or_default()
}

/// Coerce a parameter `value` into the expected JSON type per `schema` where reasonable.
/// Applies only to param contexts (not request bodies). Conservative conversions:
/// - integer/number: parse from string; arrays: split comma-separated strings and coerce items
/// - boolean: parse true/false (case-insensitive) from string
fn coerce_value_for_schema(value: &Value, schema: &openapiv3::Schema) -> Value {
    // Basic coercion: try to parse strings as appropriate types
    match value {
        Value::String(s) => {
            // Check if schema expects an array and we have a comma-separated string
            if let openapiv3::SchemaKind::Type(openapiv3::Type::Array(array_type)) =
                &schema.schema_kind
            {
                if s.contains(',') {
                    // Split comma-separated string into array
                    let parts: Vec<&str> = s.split(',').map(|s| s.trim()).collect();
                    let mut array_values = Vec::new();

                    for part in parts {
                        // Coerce each part based on array item type
                        if let Some(items_schema) = &array_type.items {
                            if let Some(items_schema_obj) = items_schema.as_item() {
                                let part_value = Value::String(part.to_string());
                                let coerced_part =
                                    coerce_value_for_schema(&part_value, items_schema_obj);
                                array_values.push(coerced_part);
                            } else {
                                // If items schema is a reference or not available, keep as string
                                array_values.push(Value::String(part.to_string()));
                            }
                        } else {
                            // No items schema defined, keep as string
                            array_values.push(Value::String(part.to_string()));
                        }
                    }
                    return Value::Array(array_values);
                }
            }

            // Only coerce if the schema expects a different type
            match &schema.schema_kind {
                openapiv3::SchemaKind::Type(openapiv3::Type::String(_)) => {
                    // Schema expects string, keep as string
                    value.clone()
                }
                openapiv3::SchemaKind::Type(openapiv3::Type::Number(_)) => {
                    // Schema expects number, try to parse
                    if let Ok(n) = s.parse::<f64>() {
                        if let Some(num) = serde_json::Number::from_f64(n) {
                            return Value::Number(num);
                        }
                    }
                    value.clone()
                }
                openapiv3::SchemaKind::Type(openapiv3::Type::Integer(_)) => {
                    // Schema expects integer, try to parse
                    if let Ok(n) = s.parse::<i64>() {
                        if let Some(num) = serde_json::Number::from_f64(n as f64) {
                            return Value::Number(num);
                        }
                    }
                    value.clone()
                }
                openapiv3::SchemaKind::Type(openapiv3::Type::Boolean(_)) => {
                    // Schema expects boolean, try to parse
                    match s.to_lowercase().as_str() {
                        "true" | "1" | "yes" | "on" => Value::Bool(true),
                        "false" | "0" | "no" | "off" => Value::Bool(false),
                        _ => value.clone(),
                    }
                }
                _ => {
                    // Unknown schema type, keep as string
                    value.clone()
                }
            }
        }
        _ => value.clone(),
    }
}

/// Apply style-aware coercion for query params
fn coerce_by_style(value: &Value, schema: &openapiv3::Schema, style: Option<&str>) -> Value {
    // Style-aware coercion for query parameters
    match value {
        Value::String(s) => {
            // Check if schema expects an array and we have a delimited string
            if let openapiv3::SchemaKind::Type(openapiv3::Type::Array(array_type)) =
                &schema.schema_kind
            {
                let delimiter = match style {
                    Some("spaceDelimited") => " ",
                    Some("pipeDelimited") => "|",
                    Some("form") | None => ",", // Default to form style (comma-separated)
                    _ => ",",                   // Fallback to comma
                };

                if s.contains(delimiter) {
                    // Split delimited string into array
                    let parts: Vec<&str> = s.split(delimiter).map(|s| s.trim()).collect();
                    let mut array_values = Vec::new();

                    for part in parts {
                        // Coerce each part based on array item type
                        if let Some(items_schema) = &array_type.items {
                            if let Some(items_schema_obj) = items_schema.as_item() {
                                let part_value = Value::String(part.to_string());
                                let coerced_part =
                                    coerce_by_style(&part_value, items_schema_obj, style);
                                array_values.push(coerced_part);
                            } else {
                                // If items schema is a reference or not available, keep as string
                                array_values.push(Value::String(part.to_string()));
                            }
                        } else {
                            // No items schema defined, keep as string
                            array_values.push(Value::String(part.to_string()));
                        }
                    }
                    return Value::Array(array_values);
                }
            }

            // Try to parse as number first
            if let Ok(n) = s.parse::<f64>() {
                if let Some(num) = serde_json::Number::from_f64(n) {
                    return Value::Number(num);
                }
            }
            // Try to parse as boolean
            match s.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => return Value::Bool(true),
                "false" | "0" | "no" | "off" => return Value::Bool(false),
                _ => {}
            }
            // Keep as string
            value.clone()
        }
        _ => value.clone(),
    }
}

/// Build a deepObject from query params like `name[prop]=val`
fn build_deep_object(name: &str, params: &Map<String, Value>) -> Option<Value> {
    let prefix = format!("{}[", name);
    let mut obj = Map::new();
    for (k, v) in params.iter() {
        if let Some(rest) = k.strip_prefix(&prefix) {
            if let Some(key) = rest.strip_suffix(']') {
                obj.insert(key.to_string(), v.clone());
            }
        }
    }
    if obj.is_empty() {
        None
    } else {
        Some(Value::Object(obj))
    }
}

// Import the enhanced schema diff functionality
// use crate::schema_diff::{validation_diff, to_enhanced_422_json, ValidationError}; // Not currently used

/// Generate an enhanced 422 response with detailed schema validation errors
/// This function provides comprehensive error information using the new schema diff utility
#[allow(clippy::too_many_arguments)]
fn generate_enhanced_422_response(
    validator: &OpenApiRouteRegistry,
    path_template: &str,
    method: &str,
    body: Option<&Value>,
    path_params: &serde_json::Map<String, Value>,
    query_params: &serde_json::Map<String, Value>,
    header_params: &serde_json::Map<String, Value>,
    cookie_params: &serde_json::Map<String, Value>,
) -> Value {
    let mut field_errors = Vec::new();

    // Extract schema validation details if we have a route
    if let Some(route) = validator.get_route(path_template, method) {
        // Validate request body with detailed error collection
        if let Some(schema) = &route.operation.request_body {
            if let Some(value) = body {
                if let Some(content) =
                    schema.as_item().and_then(|rb| rb.content.get("application/json"))
                {
                    if let Some(_schema_ref) = &content.schema {
                        // Basic JSON validation - schema validation deferred
                        if serde_json::from_value::<serde_json::Value>(value.clone()).is_err() {
                            field_errors.push(json!({
                                "path": "body",
                                "message": "invalid JSON"
                            }));
                        }
                    }
                }
            } else {
                field_errors.push(json!({
                    "path": "body",
                    "expected": "object",
                    "found": "missing",
                    "message": "Request body is required but not provided"
                }));
            }
        }

        // Validate parameters with detailed error collection
        for param_ref in &route.operation.parameters {
            if let Some(param) = param_ref.as_item() {
                match param {
                    openapiv3::Parameter::Path { parameter_data, .. } => {
                        validate_parameter_detailed(
                            parameter_data,
                            path_params,
                            "path",
                            "path parameter",
                            &mut field_errors,
                        );
                    }
                    openapiv3::Parameter::Query { parameter_data, .. } => {
                        let deep_value = if Some("form") == Some("deepObject") {
                            build_deep_object(&parameter_data.name, query_params)
                        } else {
                            None
                        };
                        validate_parameter_detailed_with_deep(
                            parameter_data,
                            query_params,
                            "query",
                            "query parameter",
                            deep_value,
                            &mut field_errors,
                        );
                    }
                    openapiv3::Parameter::Header { parameter_data, .. } => {
                        validate_parameter_detailed(
                            parameter_data,
                            header_params,
                            "header",
                            "header parameter",
                            &mut field_errors,
                        );
                    }
                    openapiv3::Parameter::Cookie { parameter_data, .. } => {
                        validate_parameter_detailed(
                            parameter_data,
                            cookie_params,
                            "cookie",
                            "cookie parameter",
                            &mut field_errors,
                        );
                    }
                }
            }
        }
    }

    // Return the detailed 422 error format
    json!({
        "error": "Schema validation failed",
        "details": field_errors,
        "method": method,
        "path": path_template,
        "timestamp": Utc::now().to_rfc3339(),
        "validation_type": "openapi_schema"
    })
}

/// Helper function to validate a parameter
fn validate_parameter(
    parameter_data: &openapiv3::ParameterData,
    params_map: &Map<String, Value>,
    prefix: &str,
    aggregate: bool,
    errors: &mut Vec<String>,
    details: &mut Vec<serde_json::Value>,
) {
    match params_map.get(&parameter_data.name) {
        Some(v) => {
            if let ParameterSchemaOrContent::Schema(s) = &parameter_data.format {
                if let Some(schema) = s.as_item() {
                    let coerced = coerce_value_for_schema(v, schema);
                    // Validate the coerced value against the schema
                    if let Err(validation_error) =
                        OpenApiSchema::new(schema.clone()).validate(&coerced)
                    {
                        let error_msg = validation_error.to_string();
                        errors.push(format!(
                            "{} parameter '{}' validation failed: {}",
                            prefix, parameter_data.name, error_msg
                        ));
                        if aggregate {
                            details.push(serde_json::json!({"path":format!("{}.{}", prefix, parameter_data.name),"code":"schema_validation","message":error_msg}));
                        }
                    }
                }
            }
        }
        None => {
            if parameter_data.required {
                errors.push(format!(
                    "missing required {} parameter '{}'",
                    prefix, parameter_data.name
                ));
                details.push(serde_json::json!({"path":format!("{}.{}", prefix, parameter_data.name),"code":"required","message":"Missing required parameter"}));
            }
        }
    }
}

/// Helper function to validate a parameter with deep object support
#[allow(clippy::too_many_arguments)]
fn validate_parameter_with_deep_object(
    parameter_data: &openapiv3::ParameterData,
    params_map: &Map<String, Value>,
    prefix: &str,
    deep_value: Option<Value>,
    style: Option<&str>,
    aggregate: bool,
    errors: &mut Vec<String>,
    details: &mut Vec<serde_json::Value>,
) {
    match deep_value.as_ref().or_else(|| params_map.get(&parameter_data.name)) {
        Some(v) => {
            if let ParameterSchemaOrContent::Schema(s) = &parameter_data.format {
                if let Some(schema) = s.as_item() {
                    let coerced = coerce_by_style(v, schema, style); // Use the actual style
                                                                     // Validate the coerced value against the schema
                    if let Err(validation_error) =
                        OpenApiSchema::new(schema.clone()).validate(&coerced)
                    {
                        let error_msg = validation_error.to_string();
                        errors.push(format!(
                            "{} parameter '{}' validation failed: {}",
                            prefix, parameter_data.name, error_msg
                        ));
                        if aggregate {
                            details.push(serde_json::json!({"path":format!("{}.{}", prefix, parameter_data.name),"code":"schema_validation","message":error_msg}));
                        }
                    }
                }
            }
        }
        None => {
            if parameter_data.required {
                errors.push(format!(
                    "missing required {} parameter '{}'",
                    prefix, parameter_data.name
                ));
                details.push(serde_json::json!({"path":format!("{}.{}", prefix, parameter_data.name),"code":"required","message":"Missing required parameter"}));
            }
        }
    }
}

/// Helper function to validate a parameter with detailed error collection
fn validate_parameter_detailed(
    parameter_data: &openapiv3::ParameterData,
    params_map: &Map<String, Value>,
    location: &str,
    value_type: &str,
    field_errors: &mut Vec<Value>,
) {
    match params_map.get(&parameter_data.name) {
        Some(value) => {
            if let ParameterSchemaOrContent::Schema(schema) = &parameter_data.format {
                // Collect detailed validation errors for this parameter
                let details: Vec<serde_json::Value> = Vec::new();
                let param_path = format!("{}.{}", location, parameter_data.name);

                // Apply coercion before validation
                if let Some(schema_ref) = schema.as_item() {
                    let coerced_value = coerce_value_for_schema(value, schema_ref);
                    // Validate the coerced value against the schema
                    if let Err(validation_error) =
                        OpenApiSchema::new(schema_ref.clone()).validate(&coerced_value)
                    {
                        field_errors.push(json!({
                            "path": param_path,
                            "expected": "valid according to schema",
                            "found": coerced_value,
                            "message": validation_error.to_string()
                        }));
                    }
                }

                for detail in details {
                    field_errors.push(json!({
                        "path": detail["path"],
                        "expected": detail["expected_type"],
                        "found": detail["value"],
                        "message": detail["message"]
                    }));
                }
            }
        }
        None => {
            if parameter_data.required {
                field_errors.push(json!({
                    "path": format!("{}.{}", location, parameter_data.name),
                    "expected": "value",
                    "found": "missing",
                    "message": format!("Missing required {} '{}'", value_type, parameter_data.name)
                }));
            }
        }
    }
}

/// Helper function to validate a parameter with deep object support and detailed errors
fn validate_parameter_detailed_with_deep(
    parameter_data: &openapiv3::ParameterData,
    params_map: &Map<String, Value>,
    location: &str,
    value_type: &str,
    deep_value: Option<Value>,
    field_errors: &mut Vec<Value>,
) {
    match deep_value.as_ref().or_else(|| params_map.get(&parameter_data.name)) {
        Some(value) => {
            if let ParameterSchemaOrContent::Schema(schema) = &parameter_data.format {
                // Collect detailed validation errors for this parameter
                let details: Vec<serde_json::Value> = Vec::new();
                let param_path = format!("{}.{}", location, parameter_data.name);

                // Apply coercion before validation
                if let Some(schema_ref) = schema.as_item() {
                    let coerced_value = coerce_by_style(value, schema_ref, Some("form")); // Default to form style for now
                                                                                          // Validate the coerced value against the schema
                    if let Err(validation_error) =
                        OpenApiSchema::new(schema_ref.clone()).validate(&coerced_value)
                    {
                        field_errors.push(json!({
                            "path": param_path,
                            "expected": "valid according to schema",
                            "found": coerced_value,
                            "message": validation_error.to_string()
                        }));
                    }
                }

                for detail in details {
                    field_errors.push(json!({
                        "path": detail["path"],
                        "expected": detail["expected_type"],
                        "found": detail["value"],
                        "message": detail["message"]
                    }));
                }
            }
        }
        None => {
            if parameter_data.required {
                field_errors.push(json!({
                    "path": format!("{}.{}", location, parameter_data.name),
                    "expected": "value",
                    "found": "missing",
                    "message": format!("Missing required {} '{}'", value_type, parameter_data.name)
                }));
            }
        }
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
        assert_eq!(user_by_id_route.axum_path(), "/users/{id}");
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
        assert!(registry
            .validate_request_with("/users/{id}", "POST", &path_params, &query_params, Some(&body))
            .is_ok());

        // invalid email
        let bad_email = json!({"email":"not-an-email","website":"https://example.com"});
        assert!(registry
            .validate_request_with(
                "/users/{id}",
                "POST",
                &path_params,
                &query_params,
                Some(&bad_email)
            )
            .is_err());

        // missing required path param
        let empty_path_params = serde_json::Map::new();
        assert!(registry
            .validate_request_with(
                "/users/{id}",
                "POST",
                &empty_path_params,
                &query_params,
                Some(&body)
            )
            .is_err());
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
        assert!(registry
            .validate_request_with("/users/{id}", "POST", &path_params, &query_params, Some(&body))
            .is_ok());

        let bad = json!({"email":"nope","website":"https://example.com"});
        assert!(registry
            .validate_request_with("/users/{id}", "POST", &path_params, &query_params, Some(&bad))
            .is_err());
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

        assert!(registry
            .validate_request_with_all(
                "/items",
                "GET",
                &path_params,
                &query_params,
                &header_params,
                &cookie_params,
                None
            )
            .is_ok());

        // Missing required cookie
        let empty_cookie = serde_json::Map::new();
        assert!(registry
            .validate_request_with_all(
                "/items",
                "GET",
                &path_params,
                &query_params,
                &header_params,
                &empty_cookie,
                None
            )
            .is_err());

        // Bad boolean header value (cannot coerce)
        let mut bad_header = serde_json::Map::new();
        bad_header.insert("X-Flag".to_string(), json!("notabool"));
        assert!(registry
            .validate_request_with_all(
                "/items",
                "GET",
                &path_params,
                &query_params,
                &bad_header,
                &cookie_params,
                None
            )
            .is_err());
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

        assert!(registry
            .validate_request_with("/search", "GET", &path_params, &query, None)
            .is_ok());
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
                                            {"type": "object", "properties": {"a": {"type": "integer"}}, "required": ["a"], "not": {"required": ["b"]}},
                                            {"type": "object", "properties": {"b": {"type": "integer"}}, "required": ["b"], "not": {"required": ["a"]}}
                                        ],
                                        "anyOf": [
                                            {"type": "object", "properties": {"flag": {"type": "boolean"}}, "required": ["flag"]},
                                            {"type": "object", "properties": {"extra": {"type": "string"}}, "required": ["extra"]}
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
        assert!(registry
            .validate_request_with(
                "/composite",
                "POST",
                &serde_json::Map::new(),
                &serde_json::Map::new(),
                Some(&ok)
            )
            .is_ok());

        // invalid oneOf: both a and b present
        let bad_oneof = json!({"base": "x", "a": 1, "b": 2, "flag": false});
        assert!(registry
            .validate_request_with(
                "/composite",
                "POST",
                &serde_json::Map::new(),
                &serde_json::Map::new(),
                Some(&bad_oneof)
            )
            .is_err());

        // invalid anyOf: none of flag/extra present
        let bad_anyof = json!({"base": "x", "a": 1});
        assert!(registry
            .validate_request_with(
                "/composite",
                "POST",
                &serde_json::Map::new(),
                &serde_json::Map::new(),
                Some(&bad_anyof)
            )
            .is_err());

        // invalid allOf: missing base
        let bad_allof = json!({"a": 1, "flag": true});
        assert!(registry
            .validate_request_with(
                "/composite",
                "POST",
                &serde_json::Map::new(),
                &serde_json::Map::new(),
                Some(&bad_allof)
            )
            .is_err());
    }

    #[tokio::test]
    async fn test_overrides_warn_mode_allows_invalid() {
        // Spec with a POST route expecting an integer query param
        let spec_json = json!({
            "openapi": "3.0.0",
            "info": { "title": "Overrides API", "version": "1.0.0" },
            "paths": {"/things": {"post": {
                "parameters": [{"name":"q","in":"query","required":true,"schema":{"type":"integer"}}],
                "responses": {"200": {"description":"ok"}}
            }}}
        });

        let spec = OpenApiSpec::from_json(spec_json).unwrap();
        let mut overrides = std::collections::HashMap::new();
        overrides.insert("POST /things".to_string(), ValidationMode::Warn);
        let registry = OpenApiRouteRegistry::new_with_options(
            spec,
            ValidationOptions {
                request_mode: ValidationMode::Enforce,
                aggregate_errors: true,
                validate_responses: false,
                overrides,
                admin_skip_prefixes: vec![],
                response_template_expand: false,
                validation_status: None,
            },
        );

        // Invalid q (missing) should warn, not error
        let ok = registry.validate_request_with("/things", "POST", &Map::new(), &Map::new(), None);
        assert!(ok.is_ok());
    }

    #[tokio::test]
    async fn test_admin_skip_prefix_short_circuit() {
        let spec_json = json!({
            "openapi": "3.0.0",
            "info": { "title": "Skip API", "version": "1.0.0" },
            "paths": {}
        });
        let spec = OpenApiSpec::from_json(spec_json).unwrap();
        let registry = OpenApiRouteRegistry::new_with_options(
            spec,
            ValidationOptions {
                request_mode: ValidationMode::Enforce,
                aggregate_errors: true,
                validate_responses: false,
                overrides: std::collections::HashMap::new(),
                admin_skip_prefixes: vec!["/admin".into()],
                response_template_expand: false,
                validation_status: None,
            },
        );

        // No route exists for this, but skip prefix means it is accepted
        let res = registry.validate_request_with_all(
            "/admin/__mockforge/health",
            "GET",
            &Map::new(),
            &Map::new(),
            &Map::new(),
            &Map::new(),
            None,
        );
        assert!(res.is_ok());
    }

    #[test]
    fn test_path_conversion() {
        assert_eq!(OpenApiRouteRegistry::convert_path_to_axum("/users"), "/users");
        assert_eq!(OpenApiRouteRegistry::convert_path_to_axum("/users/{id}"), "/users/{id}");
        assert_eq!(
            OpenApiRouteRegistry::convert_path_to_axum("/users/{id}/posts/{postId}"),
            "/users/{id}/posts/{postId}"
        );
    }
}
