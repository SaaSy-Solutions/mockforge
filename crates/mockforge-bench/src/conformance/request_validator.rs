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
                // Resolve the immediate $ref (one level) to get the
                // root schema, then hand both schema + spec to the
                // ref-resolver helper so nested `$ref` strings (e.g.
                // `#/components/schemas/Vcenter.VM.DiskCloneSpec`)
                // resolve against the full document context.
                //
                // Round 18.3 — pre-fix this called
                // `jsonschema::validator_for(&schema_json)` directly,
                // which used the inner schema as the validator's
                // document. Nested $refs to `#/components/schemas/X`
                // then failed with "Pointer '...' does not exist"
                // because the validator's document had no
                // `components` key (Srikanth's vCenter run: 157
                // violations).
                let root_schema = match schema_ref {
                    ReferenceOr::Item(s) => s.clone(),
                    ReferenceOr::Reference { reference } => {
                        let name =
                            reference.strip_prefix("#/components/schemas/").unwrap_or(reference);
                        match spec.components.as_ref().and_then(|c| c.schemas.get(name)) {
                            Some(ReferenceOr::Item(s)) => s.clone(),
                            _ => return,
                        }
                    }
                };

                // Parse body as JSON and validate against schema
                match serde_json::from_str::<serde_json::Value>(body_str) {
                    Ok(body_value) => {
                        match mockforge_openapi::schema_ref_resolver::build_validator(
                            &root_schema,
                            spec,
                        ) {
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
    check_headers: &HashMap<String, String>,
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

/// Resolve a schema reference to a serde_json::Value for validation.
/// Reserved for round 21.3 (response-body shape validation against the
/// spec's response schema). Not yet wired into a call site.
#[allow(dead_code)]
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

/// Round 44 (#79) — validate each emitted request retrospectively
/// against the OpenAPI spec, after the bench run completes. Reads
/// `conformance-requests.json` (which `--export-requests` writes) and
/// emits one [`RequestViolation`] entry per actual wire-level
/// rule break (enum, type, required field, etc.), so a user can see
/// the client's own view of what it sent that violated the contract
/// without having to query the server's `/__mockforge/api/conformance/violations`.
///
/// Srikanth on 0.3.188: "Any reason why validate-requests in mockforge
/// client are not catching all this query param or body params or path
/// params violation issues and record in conformance-request-failure
/// logs?" The existing `validate_custom_checks` only looks at the YAML
/// shape at config time (missing required params, unknown path);
/// auto-generated self-test probes ARE intentionally invalid but were
/// never recorded client-side because they don't come from the YAML.
/// This function complements the YAML-shape pass by checking each
/// emitted request against the spec's actual rule set.
///
/// Appends to (not overwrites) `conformance-request-violations.json`
/// when YAML-shape violations were already written above, so a single
/// file holds both views.
pub async fn validate_emitted_requests(
    spec_files: &[std::path::PathBuf],
    output_dir: &Path,
) -> Result<usize> {
    validate_emitted_requests_with_base_path(spec_files, output_dir, None).await
}

/// Round 45 (#79) — same as `validate_emitted_requests` but accepts an
/// explicit `base_path` (e.g. Srikanth's `--base-path /api` for the
/// Apigee spec where every operation lives under `/api/v1/...` on the
/// wire but `/v1/...` in the spec). Without it the emitted URL doesn't
/// match the spec path and every request silently skips validation.
///
/// Also broadened in r45 to:
/// - extract path params from the URL and validate their values
///   against the spec's path-parameter schemas (enum / type)
/// - parse the request body when content-type is JSON and walk it
///   against the requestBody schema's `required: [...]` and enum
///   constraints on top-level properties
///
/// Body and path-param coverage is INTENTIONALLY shallow (top-level
/// `required` + `enum`/`type` on direct properties only) — the
/// authoritative validator is the OpenAPI server's; this is the
/// client-side cross-check that mirrors the server's view on the
/// wire-level requests the bench actually sent.
pub async fn validate_emitted_requests_with_base_path(
    spec_files: &[std::path::PathBuf],
    output_dir: &Path,
    base_path: Option<&str>,
) -> Result<usize> {
    use serde_json::Value;

    if spec_files.is_empty() {
        return Ok(0);
    }
    let requests_path = output_dir.join("conformance-requests.json");
    if !requests_path.exists() {
        return Ok(0);
    }
    let bytes = match std::fs::read(&requests_path) {
        Ok(b) => b,
        Err(_) => return Ok(0),
    };
    let entries: Vec<Value> = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => return Ok(0),
    };
    if entries.is_empty() {
        return Ok(0);
    }

    let parser = SpecParser::from_file(&spec_files[0]).await?;
    let spec = parser.spec();
    let spec_ops = build_spec_operation_map(spec);

    let mut emitted_violations: Vec<RequestViolation> = Vec::new();

    for entry in &entries {
        let check = entry.get("check").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let req = match entry.get("request") {
            Some(r) => r,
            None => continue,
        };
        let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("").to_uppercase();
        let url = req.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if method.is_empty() || url.is_empty() {
            continue;
        }
        let (path_only, query_string) = match url.find('?') {
            Some(i) => (url[..i].to_string(), url[i + 1..].to_string()),
            None => (url.clone(), String::new()),
        };
        // Trim scheme + host from path so we match spec paths cleanly.
        // "http://host:port/api/x" → "/api/x".
        let path_only = if let Some(stripped) = path_only.split_once("://") {
            match stripped.1.find('/') {
                Some(i) => stripped.1[i..].to_string(),
                None => "/".to_string(),
            }
        } else {
            path_only
        };

        // Round 45 — strip base_path BEFORE matching so an Apigee-style
        // `/api/v1/organizations` on the wire matches `/v1/organizations`
        // in the spec when `--base-path /api` was passed.
        let lookup_path = if let Some(bp) = base_path {
            let bp = bp.trim_end_matches('/');
            if !bp.is_empty() && path_only.starts_with(bp) {
                let stripped = &path_only[bp.len()..];
                if stripped.is_empty() {
                    "/".to_string()
                } else {
                    stripped.to_string()
                }
            } else {
                path_only.clone()
            }
        } else {
            path_only.clone()
        };

        let spec_path = match find_matching_spec_path(&lookup_path, &spec_ops, None) {
            Some(p) => p,
            None => continue,
        };
        let path_item = match spec.paths.paths.get(&spec_path) {
            Some(ReferenceOr::Item(item)) => item,
            _ => continue,
        };
        let operation = match method.as_str() {
            "GET" => path_item.get.as_ref(),
            "POST" => path_item.post.as_ref(),
            "PUT" => path_item.put.as_ref(),
            "DELETE" => path_item.delete.as_ref(),
            "PATCH" => path_item.patch.as_ref(),
            "HEAD" => path_item.head.as_ref(),
            "OPTIONS" => path_item.options.as_ref(),
            _ => None,
        };
        let Some(operation) = operation else { continue };

        // Inspect query parameters declared on this operation; for each
        // sent query field, check it against the parameter's schema enum
        // and type. This is what catches Srikanth's `?$.xgafv=test-value`
        // case where the value isn't `"1"` or `"2"`.
        let sent_query: HashMap<String, String> = query_string
            .split('&')
            .filter_map(|kv| {
                let mut it = kv.splitn(2, '=');
                let k = it.next()?.to_string();
                let v = it.next().unwrap_or("").to_string();
                if k.is_empty() {
                    None
                } else {
                    Some((k, v))
                }
            })
            .collect();

        // Round 45 — bind path parameters by zipping the concrete URL
        // path against the spec's template path. `/v1/{name}` ←
        // `/v1/projects/abc` produces `{ "name": "projects/abc" }`.
        // Used below to value-check each path-param against its
        // declared schema (enum / type).
        let path_params: HashMap<String, String> = {
            let mut out = HashMap::new();
            let concrete_parts: Vec<&str> = lookup_path.split('/').collect();
            let template_parts: Vec<&str> = spec_path.split('/').collect();
            if concrete_parts.len() == template_parts.len() {
                for (c, t) in concrete_parts.iter().zip(template_parts.iter()) {
                    if t.starts_with('{') && t.ends_with('}') {
                        let name = &t[1..t.len() - 1];
                        out.insert(name.to_string(), (*c).to_string());
                    }
                }
            }
            out
        };

        let mut all_params: Vec<&openapiv3::Parameter> = Vec::new();
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
            let (loc_str, name, schema_ref) = match param {
                openapiv3::Parameter::Query { parameter_data, .. } => {
                    let openapiv3::ParameterSchemaOrContent::Schema(sref) = &parameter_data.format
                    else {
                        continue;
                    };
                    let Some(v) = sent_query.get(&parameter_data.name) else {
                        continue;
                    };
                    ("query", &parameter_data.name, (sref, v.clone()))
                }
                openapiv3::Parameter::Path { parameter_data, .. } => {
                    let openapiv3::ParameterSchemaOrContent::Schema(sref) = &parameter_data.format
                    else {
                        continue;
                    };
                    let Some(v) = path_params.get(&parameter_data.name) else {
                        continue;
                    };
                    ("path", &parameter_data.name, (sref, v.clone()))
                }
                _ => continue,
            };
            let (schema_ref, value) = schema_ref;
            let Some(schema) = schema_ref.as_item() else {
                continue;
            };
            if let Some(msg) = check_value_against_schema(&value, schema) {
                emitted_violations.push(RequestViolation {
                    check_name: check.clone(),
                    method: method.clone(),
                    path: url.clone(),
                    violation_type: format!("{}_value_mismatch", loc_str),
                    message: format!("{}.{}: {}", loc_str, name, msg),
                });
            }
        }

        // Round 45 — request-body cross-check. Only kicks in when the
        // sent body parses as JSON and the operation declares a JSON
        // requestBody schema. Shallow: missing required top-level
        // fields + enum/type mismatches on direct properties. Deeper
        // schema walks (nested objects, oneOf/anyOf) are the server-
        // side validator's job; we just want to surface the obvious
        // wire-level breaks the bench actually fired.
        let body_str = req.get("body").and_then(|v| v.as_str()).unwrap_or("");
        if !body_str.is_empty() {
            if let Ok(body_json) = serde_json::from_str::<serde_json::Value>(body_str) {
                if let Some(req_body) = operation.request_body.as_ref().and_then(|r| r.as_item()) {
                    for (ct, media) in &req_body.content {
                        if !ct.contains("json") {
                            continue;
                        }
                        let Some(schema_ref) = &media.schema else {
                            continue;
                        };
                        let Some(schema) = schema_ref.as_item() else {
                            continue;
                        };
                        check_body_against_schema(
                            &check,
                            &method,
                            &url,
                            &body_json,
                            schema,
                            &mut emitted_violations,
                        );
                    }
                }
            }
        }
    }

    // Merge with any pre-existing custom-YAML violations on disk.
    let dst = output_dir.join("conformance-request-violations.json");
    let mut all: Vec<Value> = if dst.exists() {
        match std::fs::read(&dst) {
            Ok(b) => serde_json::from_slice(&b).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };
    for v in &emitted_violations {
        if let Ok(val) = serde_json::to_value(v) {
            all.push(val);
        }
    }
    if !all.is_empty() {
        if let Ok(json) = serde_json::to_string_pretty(&all) {
            let _ = std::fs::write(&dst, json);
            tracing::info!(
                "validate-requests: wrote {} entries to {} ({} from emitted requests)",
                all.len(),
                dst.display(),
                emitted_violations.len()
            );
        }
    }
    Ok(emitted_violations.len())
}

/// Round 45 (#79) — shallow body-vs-schema check for the retroactive
/// emitted-request validator. Pushes a [`RequestViolation`] for each
/// missing top-level `required` field and for each direct property
/// that fails an `enum` / type check. Intentionally does NOT recurse
/// into nested objects or follow `$ref` — the server-side validator is
/// authoritative there; this client-side pass only mirrors the obvious
/// wire-level breaks the bench actually fired.
fn check_body_against_schema(
    check: &str,
    method: &str,
    url: &str,
    body: &serde_json::Value,
    schema: &openapiv3::Schema,
    violations: &mut Vec<RequestViolation>,
) {
    use openapiv3::{SchemaKind, Type};

    let SchemaKind::Type(Type::Object(obj_type)) = &schema.schema_kind else {
        return;
    };
    let Some(body_obj) = body.as_object() else {
        return;
    };

    for required in &obj_type.required {
        if !body_obj.contains_key(required) {
            violations.push(RequestViolation {
                check_name: check.to_string(),
                method: method.to_string(),
                path: url.to_string(),
                violation_type: "body_missing_required".to_string(),
                message: format!("body.{}: required field missing", required),
            });
        }
    }

    for (prop_name, prop_ref) in &obj_type.properties {
        let Some(value) = body_obj.get(prop_name) else {
            continue;
        };
        let Some(prop_schema) = prop_ref.as_item() else {
            continue;
        };
        if let Some(value_str) = value.as_str() {
            if let Some(msg) = check_value_against_schema(value_str, prop_schema) {
                violations.push(RequestViolation {
                    check_name: check.to_string(),
                    method: method.to_string(),
                    path: url.to_string(),
                    violation_type: "body_value_mismatch".to_string(),
                    message: format!("body.{}: {}", prop_name, msg),
                });
            }
        }
    }
}

/// Round 44 (#79) — minimal value-vs-schema check for the retroactive
/// emitted-request validator. Returns a human-readable error message
/// when the value doesn't satisfy the schema, or `None` when it does.
/// Only handles the rules Srikanth's Apigee spec uses (enum, type:
/// integer, type: boolean); falls through silently for any other
/// rule rather than producing a false positive.
fn check_value_against_schema(value: &str, schema: &openapiv3::Schema) -> Option<String> {
    use openapiv3::{SchemaKind, Type};

    let SchemaKind::Type(t) = &schema.schema_kind else {
        return None;
    };
    match t {
        Type::String(s) => {
            if !s.enumeration.is_empty() {
                let allowed: Vec<String> = s.enumeration.iter().filter_map(|e| e.clone()).collect();
                if !allowed.iter().any(|a| a == value) {
                    let quoted: Vec<String> =
                        allowed.iter().map(|a| format!("\"{}\"", a)).collect();
                    return Some(format!(
                        "value \"{}\" is not one of {}",
                        value,
                        quoted.join(" or ")
                    ));
                }
            }
            None
        }
        Type::Integer(_) => {
            if value.parse::<i64>().is_err() {
                Some(format!("value \"{}\" is not of type \"integer\"", value))
            } else {
                None
            }
        }
        Type::Number(_) => {
            if value.parse::<f64>().is_err() {
                Some(format!("value \"{}\" is not of type \"number\"", value))
            } else {
                None
            }
        }
        Type::Boolean(_) => match value {
            "true" | "false" => None,
            _ => Some(format!("value \"{}\" is not of type \"boolean\"", value)),
        },
        _ => None,
    }
}
