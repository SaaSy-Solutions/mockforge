//! Request validation against OpenAPI spec.
//!
//! Validates that conformance test requests (especially from HAR custom checks)
//! conform to the OpenAPI specification: correct paths, required parameters,
//! valid request body schemas, and matching content types.

use crate::error::Result;
use crate::spec_parser::SpecParser;
use openapiv3::{OpenAPI, ReferenceOr};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

use super::custom::CustomConformanceConfig;

/// A single request validation violation
#[derive(Debug, Serialize)]
pub struct RequestViolation {
    /// Check name from the custom YAML
    pub check_name: String,
    /// Request method
    pub method: String,
    /// Request path
    pub path: String,
    /// Type of violation
    pub violation_type: String,
    /// Human-readable description
    pub message: String,
}

/// Validate custom conformance checks against an OpenAPI spec.
///
/// Returns a list of violations (empty if all checks are valid).
pub fn validate_custom_checks(
    spec: &OpenAPI,
    custom_checks_file: &Path,
    base_path: Option<&str>,
) -> Result<Vec<RequestViolation>> {
    let config = CustomConformanceConfig::from_file(custom_checks_file)?;
    let mut violations = Vec::new();

    // Build a map of spec paths -> operations for matching
    let spec_ops = build_spec_operation_map(spec);

    for check in &config.custom_checks {
        // Strip query string from path for matching
        let check_path = check.path.split('?').next().unwrap_or(&check.path);

        // Try to match the check's path to a spec operation
        let spec_path = match find_matching_spec_path(check_path, &spec_ops, base_path) {
            Some(p) => p,
            None => {
                violations.push(RequestViolation {
                    check_name: check.name.clone(),
                    method: check.method.clone(),
                    path: check.path.clone(),
                    violation_type: "unknown_path".to_string(),
                    message: format!(
                        "Path '{}' not found in OpenAPI spec (checked with base_path={:?})",
                        check_path, base_path
                    ),
                });
                continue;
            }
        };

        // Check if the method is defined for this path
        let path_item = match spec.paths.paths.get(&spec_path) {
            Some(ReferenceOr::Item(item)) => item,
            _ => continue,
        };

        let method_lower = check.method.to_lowercase();
        let operation = match method_lower.as_str() {
            "get" => path_item.get.as_ref(),
            "post" => path_item.post.as_ref(),
            "put" => path_item.put.as_ref(),
            "delete" => path_item.delete.as_ref(),
            "patch" => path_item.patch.as_ref(),
            "head" => path_item.head.as_ref(),
            "options" => path_item.options.as_ref(),
            _ => None,
        };

        let operation = match operation {
            Some(op) => op,
            None => {
                violations.push(RequestViolation {
                    check_name: check.name.clone(),
                    method: check.method.clone(),
                    path: check.path.clone(),
                    violation_type: "method_not_allowed".to_string(),
                    message: format!(
                        "Method '{}' not defined for path '{}' in the spec",
                        check.method, spec_path
                    ),
                });
                continue;
            }
        };

        // Validate request body for POST/PUT/PATCH
        if matches!(method_lower.as_str(), "post" | "put" | "patch") {
            validate_request_body(
                &check.name,
                &check.method,
                &check.path,
                check.body.as_deref(),
                operation,
                spec,
                &mut violations,
            );
        }

        // Check required parameters
        validate_parameters(
            &check.name,
            &check.method,
            &check.path,
            check_path,
            &check.headers,
            operation,
            path_item,
            spec,
            &mut violations,
        );
    }

    Ok(violations)
}

/// Collected spec operations indexed by path
type SpecOperationMap = HashMap<String, Vec<String>>; // path -> [methods]

fn build_spec_operation_map(spec: &OpenAPI) -> SpecOperationMap {
    let mut map = HashMap::new();
    for (path, item_ref) in &spec.paths.paths {
        if let ReferenceOr::Item(item) = item_ref {
            let mut methods = Vec::new();
            if item.get.is_some() {
                methods.push("GET".to_string());
            }
            if item.post.is_some() {
                methods.push("POST".to_string());
            }
            if item.put.is_some() {
                methods.push("PUT".to_string());
            }
            if item.delete.is_some() {
                methods.push("DELETE".to_string());
            }
            if item.patch.is_some() {
                methods.push("PATCH".to_string());
            }
            if item.head.is_some() {
                methods.push("HEAD".to_string());
            }
            if item.options.is_some() {
                methods.push("OPTIONS".to_string());
            }
            map.insert(path.clone(), methods);
        }
    }
    map
}

/// Try to match a concrete path (e.g., "/users/123") to a spec path template
/// (e.g., "/users/{id}"). Handles base_path stripping.
fn find_matching_spec_path(
    check_path: &str,
    spec_ops: &SpecOperationMap,
    base_path: Option<&str>,
) -> Option<String> {
    // Try exact match first
    if spec_ops.contains_key(check_path) {
        return Some(check_path.to_string());
    }

    // Try with base_path prepended
    if let Some(bp) = base_path {
        let with_base = format!("{}{}", bp.trim_end_matches('/'), check_path);
        if spec_ops.contains_key(&with_base) {
            return Some(with_base);
        }
    }

    // Try template matching (e.g., /users/123 matches /users/{id})
    for spec_path in spec_ops.keys() {
        if path_matches_template(check_path, spec_path)
            || base_path
                .map(|bp| {
                    let with_base = format!("{}{}", bp.trim_end_matches('/'), check_path);
                    path_matches_template(&with_base, spec_path)
                })
                .unwrap_or(false)
        {
            return Some(spec_path.clone());
        }
    }

    None
}

/// Check if a concrete path matches a path template with {param} segments
fn path_matches_template(concrete: &str, template: &str) -> bool {
    let concrete_parts: Vec<&str> = concrete.split('/').collect();
    let template_parts: Vec<&str> = template.split('/').collect();

    if concrete_parts.len() != template_parts.len() {
        return false;
    }

    concrete_parts
        .iter()
        .zip(template_parts.iter())
        .all(|(c, t)| t.starts_with('{') && t.ends_with('}') || c == t)
}

/// Validate request body against the spec's requestBody schema
#[allow(clippy::too_many_arguments)]
fn validate_request_body(
    check_name: &str,
    method: &str,
    path: &str,
    body: Option<&str>,
    operation: &openapiv3::Operation,
    spec: &OpenAPI,
    violations: &mut Vec<RequestViolation>,
) {
    let request_body_ref = match &operation.request_body {
        Some(rb) => rb,
        None => {
            // Spec doesn't define a requestBody — body is optional
            return;
        }
    };

    // Resolve $ref if needed
    let request_body = match request_body_ref {
        ReferenceOr::Item(rb) => rb,
        ReferenceOr::Reference { reference } => {
            let name = reference.strip_prefix("#/components/requestBodies/").unwrap_or(reference);
            match spec.components.as_ref().and_then(|c| c.request_bodies.get(name)) {
                Some(ReferenceOr::Item(rb)) => rb,
                _ => return,
            }
        }
    };

    // Check if body is required but missing
    if request_body.required && body.is_none() {
        violations.push(RequestViolation {
            check_name: check_name.to_string(),
            method: method.to_string(),
            path: path.to_string(),
            violation_type: "missing_required_body".to_string(),
            message: "Spec requires a request body but none is provided in the check".to_string(),
        });
        return;
    }

    // If body is provided, validate against schema
    if let Some(body_str) = body {
        // Find JSON content type
        let json_media = request_body.content.get("application/json").or_else(|| {
            request_body.content.iter().find(|(k, _)| k.contains("json")).map(|(_, v)| v)
        });

        if let Some(media) = json_media {
            if let Some(schema_ref) = &media.schema {
                // Resolve schema $ref
                let schema_json = match resolve_schema_to_json(schema_ref, spec) {
                    Some(s) => s,
                    None => return,
                };

                // Parse body as JSON and validate against schema
                match serde_json::from_str::<serde_json::Value>(body_str) {
                    Ok(body_value) => {
                        match jsonschema::validator_for(&schema_json) {
                            Ok(validator) => {
                                let errors: Vec<_> = validator.iter_errors(&body_value).collect();
                                for err in errors.iter().take(5) {
                                    violations.push(RequestViolation {
                                        check_name: check_name.to_string(),
                                        method: method.to_string(),
                                        path: path.to_string(),
                                        violation_type: "body_schema_violation".to_string(),
                                        message: format!(
                                            "Request body schema violation at {}: {}",
                                            err.instance_path, err
                                        ),
                                    });
                                }
                            }
                            Err(_) => {
                                // Schema itself is invalid — skip validation
                            }
                        }
                    }
                    Err(e) => {
                        violations.push(RequestViolation {
                            check_name: check_name.to_string(),
                            method: method.to_string(),
                            path: path.to_string(),
                            violation_type: "body_not_json".to_string(),
                            message: format!("Request body is not valid JSON: {}", e),
                        });
                    }
                }
            }
        }
    }
}

/// Validate required parameters from the spec
#[allow(clippy::too_many_arguments)]
fn validate_parameters(
    check_name: &str,
    method: &str,
    path: &str,
    check_path_no_query: &str,
    check_headers: &std::collections::HashMap<String, String>,
    operation: &openapiv3::Operation,
    path_item: &openapiv3::PathItem,
    spec: &OpenAPI,
    violations: &mut Vec<RequestViolation>,
) {
    // Collect all parameters (path-level + operation-level)
    let mut all_params = Vec::new();
    for p in &path_item.parameters {
        if let Some(param) = resolve_parameter(p, spec) {
            all_params.push(param);
        }
    }
    for p in &operation.parameters {
        if let Some(param) = resolve_parameter(p, spec) {
            all_params.push(param);
        }
    }

    for param in &all_params {
        let param_data = match param {
            openapiv3::Parameter::Query { parameter_data, .. } => {
                if !parameter_data.required {
                    continue;
                }
                // Check if query param is in the path's query string
                let has_param = check_path_no_query != path
                    && path.contains(&format!("{}=", parameter_data.name));
                if !has_param {
                    violations.push(RequestViolation {
                        check_name: check_name.to_string(),
                        method: method.to_string(),
                        path: path.to_string(),
                        violation_type: "missing_required_query_param".to_string(),
                        message: format!(
                            "Required query parameter '{}' is missing",
                            parameter_data.name
                        ),
                    });
                }
                continue;
            }
            openapiv3::Parameter::Header { parameter_data, .. } => parameter_data,
            openapiv3::Parameter::Path { parameter_data, .. } => {
                // Path params are always required — but they're embedded in the URL
                // so we can't easily validate them here (they're already resolved)
                let _ = parameter_data;
                continue;
            }
            openapiv3::Parameter::Cookie { .. } => continue,
        };

        if param_data.required {
            let has_header = check_headers.keys().any(|k| k.eq_ignore_ascii_case(&param_data.name));
            if !has_header {
                violations.push(RequestViolation {
                    check_name: check_name.to_string(),
                    method: method.to_string(),
                    path: path.to_string(),
                    violation_type: "missing_required_header".to_string(),
                    message: format!("Required header parameter '{}' is missing", param_data.name),
                });
            }
        }
    }
}

/// Resolve a parameter reference
fn resolve_parameter<'a>(
    param_ref: &'a ReferenceOr<openapiv3::Parameter>,
    spec: &'a OpenAPI,
) -> Option<&'a openapiv3::Parameter> {
    match param_ref {
        ReferenceOr::Item(p) => Some(p),
        ReferenceOr::Reference { reference } => {
            let name = reference.strip_prefix("#/components/parameters/")?;
            match spec.components.as_ref()?.parameters.get(name)? {
                ReferenceOr::Item(p) => Some(p),
                _ => None,
            }
        }
    }
}

/// Resolve a schema reference to a serde_json::Value for validation
fn resolve_schema_to_json(
    schema_ref: &ReferenceOr<openapiv3::Schema>,
    spec: &OpenAPI,
) -> Option<serde_json::Value> {
    let schema = match schema_ref {
        ReferenceOr::Item(s) => s,
        ReferenceOr::Reference { reference } => {
            let name = reference.strip_prefix("#/components/schemas/")?;
            match spec.components.as_ref()?.schemas.get(name)? {
                ReferenceOr::Item(s) => s,
                _ => return None,
            }
        }
    };
    serde_json::to_value(schema).ok()
}

/// Run request validation and write results to a file.
/// Called from the conformance execution path.
pub async fn run_request_validation(
    spec_files: &[std::path::PathBuf],
    custom_checks_file: Option<&Path>,
    base_path: Option<&str>,
    output_dir: &Path,
) -> Result<usize> {
    let custom_file = match custom_checks_file {
        Some(f) => f,
        None => return Ok(0),
    };

    if spec_files.is_empty() {
        return Ok(0);
    }

    let parser = SpecParser::from_file(&spec_files[0]).await?;
    let spec = parser.spec();

    let violations = validate_custom_checks(spec, custom_file, base_path)?;

    if !violations.is_empty() {
        let path = output_dir.join("conformance-request-violations.json");
        if let Ok(json) = serde_json::to_string_pretty(&violations) {
            let _ = std::fs::write(&path, json);
            tracing::info!(
                "Found {} request validation violation(s), saved to {}",
                violations.len(),
                path.display()
            );
        }
    }

    Ok(violations.len())
}
