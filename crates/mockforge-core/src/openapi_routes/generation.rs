//! Route generation from OpenAPI specifications
//!
//! This module handles the generation of routes from OpenAPI specifications,
//! including parameter extraction, path matching, and route creation.

use crate::openapi::spec::OpenApiSpec;
use crate::openapi::route::OpenApiRoute;
use std::sync::Arc;

/// Generate routes from an OpenAPI specification
pub fn generate_routes_from_spec(spec: &Arc<OpenApiSpec>) -> Vec<OpenApiRoute> {
    let mut routes = Vec::new();

    for (path, path_item) in &spec.spec.paths.paths {
        if let Some(item) = path_item.as_item() {
            // Generate route for each HTTP method
            if let Some(op) = &item.get {
                routes.push(OpenApiRoute::from_operation("GET", path.clone(), op, spec.clone()));
            }
            if let Some(op) = &item.post {
                routes.push(OpenApiRoute::from_operation("POST", path.clone(), op, spec.clone()));
            }
            if let Some(op) = &item.put {
                routes.push(OpenApiRoute::from_operation("PUT", path.clone(), op, spec.clone()));
            }
            if let Some(op) = &item.delete {
                routes.push(OpenApiRoute::from_operation("DELETE", path.clone(), op, spec.clone()));
            }
            if let Some(op) = &item.patch {
                routes.push(OpenApiRoute::from_operation("PATCH", path.clone(), op, spec.clone()));
            }
            if let Some(op) = &item.head {
                routes.push(OpenApiRoute::from_operation("HEAD", path.clone(), op, spec.clone()));
            }
            if let Some(op) = &item.options {
                routes.push(OpenApiRoute::from_operation("OPTIONS", path.clone(), op, spec.clone()));
            }
            if let Some(op) = &item.trace {
                routes.push(OpenApiRoute::from_operation("TRACE", path.clone(), op, spec.clone()));
            }
        }
    }

    routes
}

/// Extract path parameters from an OpenAPI path template
pub fn extract_path_parameters(path_template: &str) -> Vec<String> {
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

/// Convert OpenAPI path parameters to Axum path format
pub fn convert_path_to_axum_format(path: &str) -> String {
    // Simple conversion - replace {param} with :param
    path.replace('{', ":").replace('}', "")
}

/// Validate that path parameters match between template and actual path
pub fn validate_path_parameters(template_path: &str, actual_path: &str) -> bool {
    // Convert OpenAPI-style parameters {param} to routing format :param
    let routing_template = convert_path_to_axum_format(template_path);

    // Use proper pattern matching to validate path structure compatibility
    // This ensures parameters, wildcards, and exact segments are properly validated
    route_matches_pattern(&routing_template, actual_path)
}

/// Generate a unique route key for caching/routing purposes
pub fn generate_route_key(method: &str, path: &str) -> String {
    format!("{}:{}", method.to_uppercase(), path)
}

/// Check if a route path matches a pattern (for routing purposes)
pub fn route_matches_pattern(route_path: &str, request_path: &str) -> bool {
    let route_parts: Vec<&str> = route_path.split('/').filter(|s| !s.is_empty()).collect();
    let request_parts: Vec<&str> = request_path.split('/').filter(|s| !s.is_empty()).collect();

    match_segments(&route_parts, &request_parts, 0, 0)
}

/// Recursive function to match path segments with wildcards and parameters
fn match_segments(route_parts: &[&str], request_parts: &[&str], route_idx: usize, request_idx: usize) -> bool {
    // If we've consumed both patterns and paths, it's a match
    if route_idx == route_parts.len() && request_idx == request_parts.len() {
        return true;
    }

    // If we've consumed the route pattern but not the request path, no match
    if route_idx == route_parts.len() {
        return false;
    }

    let current_route = route_parts[route_idx];

    match current_route {
        "*" => {
            // Single wildcard: matches any single segment
            if request_idx < request_parts.len() {
                // Try consuming one segment
                if match_segments(route_parts, request_parts, route_idx + 1, request_idx + 1) {
                    return true;
                }
            }
            false
        }
        "**" => {
            // Double wildcard: can match zero or more segments
            // Try matching zero segments (skip this pattern)
            if match_segments(route_parts, request_parts, route_idx + 1, request_idx) {
                return true;
            }
            // Try matching one or more segments
            if request_idx < request_parts.len()
                && match_segments(route_parts, request_parts, route_idx, request_idx + 1) {
                return true;
            }
            false
        }
        route_seg if route_seg.starts_with(':') => {
            // Parameter placeholder: matches any single segment
            if request_idx < request_parts.len() {
                return match_segments(route_parts, request_parts, route_idx + 1, request_idx + 1);
            }
            false
        }
        _ => {
            // Exact match required
            if request_idx < request_parts.len() && current_route == request_parts[request_idx] {
                return match_segments(route_parts, request_parts, route_idx + 1, request_idx + 1);
            }
            false
        }
    }
}

/// Generate parameter extraction code for a route
pub fn generate_parameter_extraction_code(route: &OpenApiRoute) -> String {
    let mut code = String::new();

    // Add path parameter extraction
    for param_name in &route.parameters {
        if param_name.starts_with(':') {
            code.push_str(&format!(
                "let {} = path_params.get(\"{}\").cloned().unwrap_or_default();\n",
                param_name.trim_start_matches(':'),
                param_name.trim_start_matches(':')
            ));
        }
    }

    code
}

/// Generate validation code for route parameters
pub fn generate_parameter_validation_code(route: &OpenApiRoute) -> String {
    let mut code = String::new();

    // Add parameter validation
    for param in &route.parameters {
        if param.starts_with(':') {
            code.push_str(&format!(
                "if {}.is_empty() {{ return Err(Error::generic(\"Missing parameter: {}\")); }}\n",
                param.trim_start_matches(':'),
                param.trim_start_matches(':')
            ));
        }
    }

    code
}

/// Generate mock response generation code
pub fn generate_mock_response_code(route: &OpenApiRoute) -> String {
    let mut code = String::new();

    code.push_str("let mut response = json!({});\n");

    // Add response generation logic based on the route's operation
    let _operation = &route.operation;
    code.push_str("// Generate response based on OpenAPI operation\n");
    code.push_str(&format!(
        "// Operation: {} {}\n",
        route.method,
        route.path
    ));

    code
}
