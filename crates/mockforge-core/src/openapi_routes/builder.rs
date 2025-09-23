//! Axum router building from OpenAPI specifications
//!
//! This module handles the creation of Axum routers from OpenAPI specifications,
//! including route registration and middleware integration.

use crate::openapi::OpenApiSpec;
use crate::openapi_routes::OpenApiRouteRegistry;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    routing::{delete, get, head, options, patch, post, put},
    Router,
};
use serde_json::{Map, Value};
use tracing;
use url;

/// Build an Axum router from an OpenAPI specification
pub fn build_router_from_spec(spec: OpenApiSpec) -> Router {
    let registry = OpenApiRouteRegistry::new(spec);
    registry.build_router()
}

/// Build an Axum router from an OpenAPI specification with custom options
pub fn build_router_from_spec_with_options(
    spec: OpenApiSpec,
    options: crate::openapi_routes::ValidationOptions,
) -> Router {
    let registry = OpenApiRouteRegistry::new_with_options(spec, options);
    registry.build_router()
}

/// Router builder for creating complex routing configurations
pub struct RouterBuilder {
    registry: OpenApiRouteRegistry,
    middleware: Option<Box<dyn Fn(Router) -> Router + Send + 'static>>,
    custom_routes: Option<Box<dyn Fn(Router) -> Router + Send + 'static>>,
}

impl RouterBuilder {
    /// Create a new router builder from an OpenAPI spec
    pub fn new(spec: OpenApiSpec) -> Self {
        let registry = OpenApiRouteRegistry::new(spec);
        Self { registry, middleware: None, custom_routes: None }
    }

    /// Create a new router builder with custom validation options
    pub fn with_options(
        spec: OpenApiSpec,
        options: crate::openapi_routes::ValidationOptions,
    ) -> Self {
        let registry = OpenApiRouteRegistry::new_with_options(spec, options);
        Self { registry, middleware: None, custom_routes: None }
    }

    /// Add middleware to all routes
    pub fn with_middleware<F>(mut self, middleware: F) -> Self
    where
        F: Fn(Router) -> Router + Send + 'static,
    {
        self.middleware = Some(Box::new(middleware));
        self
    }

    /// Add custom routes alongside OpenAPI routes
    pub fn with_custom_routes<F>(mut self, route_builder: F) -> Self
    where
        F: Fn(Router) -> Router + Send + 'static,
    {
        self.custom_routes = Some(Box::new(route_builder));
        self
    }

    /// Build the final router
    pub fn build(self) -> Router {
        let mut router = self.registry.build_router();
        if let Some(middleware) = self.middleware {
            router = middleware(router);
        }
        if let Some(custom_routes) = self.custom_routes {
            router = custom_routes(router);
        }
        router
    }
}

/// Helper function to create route handlers
pub fn create_route_handler(
    route: &crate::openapi::route::OpenApiRoute,
    registry: &OpenApiRouteRegistry,
) -> Router {
    let axum_path = route.axum_path();
    let route_clone = route.clone();
    let _validator = registry.clone_for_validation();

    // Create a handler function that matches Axum's expectations
    let handler = move || async move {
        let (status, response) = route_clone.mock_response_with_status();
        (axum::http::StatusCode::from_u16(status).unwrap_or(axum::http::StatusCode::OK), axum::response::Json(response))
    };

    match route.method.as_str() {
        "GET" => Router::new().route(&axum_path, get(handler)),
        "POST" => Router::new().route(&axum_path, post(handler)),
        "PUT" => Router::new().route(&axum_path, put(handler)),
        "DELETE" => Router::new().route(&axum_path, delete(handler)),
        "PATCH" => Router::new().route(&axum_path, patch(handler)),
        "HEAD" => Router::new().route(&axum_path, head(handler)),
        "OPTIONS" => Router::new().route(&axum_path, options(handler)),
        _ => Router::new().route(&axum_path, get(handler)),
    }
}



/// Merge multiple routers into a single router
pub fn merge_routers(routers: Vec<Router>) -> Router {
    let mut merged = Router::new();

    for router in routers {
        merged = merged.merge(router);
    }

    merged
}

/// Middleware function to handle errors and panics
pub async fn error_handler(
    request: Request<Body>,
    next: Next,
) -> Response {
    // Extract request details before moving the request
    let method = request.method().clone();
    let uri = request.uri().clone();

    let response = next.run(request).await;

    // Enhanced error handling with more detailed logging and response transformation
    if response.status().is_server_error() {
        tracing::error!(
            "Server error response: {} for request: {} {}",
            response.status(),
            method,
            uri
        );
        // Could transform the response here if needed
    } else if response.status().is_client_error() {
        tracing::warn!(
            "Client error response: {} for request: {} {}",
            response.status(),
            method,
            uri
        );
    }

    // In a production system, you might want to:
    // - Add timeout handling
    // - Convert custom errors to proper HTTP responses
    // - Add error logging and monitoring
    // - Implement circuit breaker patterns
    // - Add panic recovery with tower::catch_panic

    response
}

/// Create a router with error handling middleware
pub fn create_router_with_error_handling(router: Router) -> Router {
    router.layer(axum::middleware::from_fn(error_handler))
}

/// Create a router with logging middleware
pub fn create_router_with_logging(router: Router) -> Router {
    router.layer(axum::middleware::from_fn(request_logger))
}

/// Middleware function to validate requests against OpenAPI spec
pub async fn validate_request(
    State(validator): State<OpenApiRouteRegistry>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract request components for validation
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path().to_string();

    // Parse query parameters
    let query_string = uri.query().unwrap_or("");
    let mut query_map = Map::new();
    for (k, v) in url::form_urlencoded::parse(query_string.as_bytes()) {
        query_map.insert(k.to_string(), Value::String(v.to_string()));
    }

    // Parse headers
    let headers = request.headers();
    let mut header_map = Map::new();
    for (name, value) in headers {
        if let Ok(value_str) = value.to_str() {
            header_map.insert(name.as_str().to_string(), Value::String(value_str.to_string()));
        }
    }

    // Parse cookies from Cookie header
    let mut cookie_map = Map::new();
    if let Some(cookie_header) = headers.get("cookie") {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for part in cookie_str.split(';') {
                let part = part.trim();
                if let Some((k, v)) = part.split_once('=') {
                    cookie_map.insert(k.to_string(), Value::String(v.to_string()));
                }
            }
        }
    }

    // Extract path parameters from the matched route
    let path_params = validator.extract_path_parameters(&path, method.as_str());
    let mut path_map = Map::new();
    for (key, value) in path_params {
        path_map.insert(key, Value::String(value));
    }

    // Parse body if present
    let body = std::mem::take(request.body_mut());
    let body_bytes = axum::body::to_bytes(body, usize::MAX).await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let body_json = if !body_bytes.is_empty() {
        serde_json::from_slice(&body_bytes).ok()
    } else {
        None
    };

    // Validate the request
    if validator.validate_request_with_all(
        &path,
        method.as_str(),
        &path_map,
        &query_map,
        &header_map,
        &cookie_map,
        body_json.as_ref(),
    ).is_err() {
        // Return validation error status
        let status_code = validator.options.validation_status
            .unwrap_or_else(|| {
                std::env::var("MOCKFORGE_VALIDATION_STATUS")
                    .ok()
                    .and_then(|s| s.parse::<u16>().ok())
                    .unwrap_or(400)
            });
        return Err(StatusCode::from_u16(status_code).unwrap_or(StatusCode::BAD_REQUEST));
    }

    // If validation passes, continue to next middleware
    Ok(next.run(request).await)
}

/// Middleware function to log incoming requests
pub async fn request_logger(
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let version = request.version();

    // Log the incoming request
    tracing::info!(
        "Request: {} {} {:?}",
        method,
        uri,
        version
    );

    // Log headers if debug logging is enabled
    if tracing::level_enabled!(tracing::Level::DEBUG) {
        for (name, value) in request.headers() {
            if let Ok(value_str) = value.to_str() {
                tracing::debug!("Header: {}: {}", name, value_str);
            }
        }
    }

    let start = std::time::Instant::now();

    // Call the next middleware
    let response = next.run(request).await;

    let duration = start.elapsed();

    // Log the response
    tracing::info!(
        "Response: {} {} - {} in {:?}",
        method,
        uri,
        response.status(),
        duration
    );

    Ok(response)
}

/// Create a router with validation middleware
pub fn create_router_with_validation(router: Router, validator: OpenApiRouteRegistry) -> Router {
    router.layer(axum::middleware::from_fn_with_state(validator, validate_request))
}
