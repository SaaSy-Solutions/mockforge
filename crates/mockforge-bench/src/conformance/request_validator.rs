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
    let self_test_jsonl_path = output_dir.join("conformance-self-test-requests.jsonl");

    // Round 49 (#79) — Srikanth on 0.3.193: self-test + --targets-file
    // produced no violation logs because validate_emitted_requests
    // only reads `conformance-requests.json` (the bench export
    // shape), and self-test writes `conformance-self-test-
    // requests.jsonl` (the CaseCapture shape). Now read whichever
    // exists, converting the JSONL shape into the same `{check,
    // method, url, request.body}` structure the validator below
    // expects. If both exist, the bench export wins (a deliberate
    // bench run shouldn't be overridden by stale self-test output).
    let entries: Vec<Value> = if requests_path.exists() {
        let bytes = match std::fs::read(&requests_path) {
            Ok(b) => b,
            Err(_) => return Ok(0),
        };
        match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(_) => return Ok(0),
        }
    } else if self_test_jsonl_path.exists() {
        let bytes = match std::fs::read(&self_test_jsonl_path) {
            Ok(b) => b,
            Err(_) => return Ok(0),
        };
        let text = String::from_utf8_lossy(&bytes);
        text.lines()
            .filter(|l| !l.is_empty())
            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
            .map(|case| {
                let label = case.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let method = case.get("method").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let url = case.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let body = case.get("request_body").cloned().unwrap_or(Value::Null);
                let mut req = serde_json::Map::new();
                req.insert("method".into(), Value::String(method));
                req.insert("url".into(), Value::String(url));
                req.insert(
                    "body".into(),
                    match body {
                        Value::String(s) => Value::String(s),
                        Value::Null => Value::String(String::new()),
                        other => other,
                    },
                );
                let mut out = serde_json::Map::new();
                out.insert("check".into(), Value::String(label));
                out.insert("request".into(), Value::Object(req));
                Value::Object(out)
            })
            .collect()
    } else {
        return Ok(0);
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
        // requestBody schema.
        //
        // Round 52 (#79) — Srikanth on 0.3.198: a self-test +
        // `--targets-file` run reported "2700 request-body caught" but
        // the by-request / by-probe violation files came out EMPTY. The
        // old shallow check here only fired when the requestBody media
        // schema was an inline `ReferenceOr::Item`; the Apigee spec (and
        // most real specs) declares
        // `schema.$ref = #/components/schemas/GoogleCloudApigeeV1Organization`,
        // so `schema_ref.as_item()` returned `None` and every body probe
        // was silently skipped. It also only type-checked STRING values,
        // so a `{"analyticsRegion":12345}` (number-where-string) probe
        // never surfaced even when the schema resolved. We now resolve
        // the requestBody + schema `$ref`s and reuse the same full JSON
        // Schema validator `validate_custom_checks` uses (round 18.3's
        // `build_validator`), so nested `$ref`s, root-type mismatches,
        // and non-string type mismatches are all caught.
        let body_str = req.get("body").and_then(|v| v.as_str()).unwrap_or("");
        if !body_str.is_empty() {
            if let Ok(body_json) = serde_json::from_str::<serde_json::Value>(body_str) {
                validate_emitted_body(
                    &check,
                    &method,
                    &url,
                    &body_json,
                    operation,
                    spec,
                    &mut emitted_violations,
                );
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
    // Round 50 (#79) — dedup byte-identical violations. A multi-iteration
    // self-test captures one probe per iteration, so a 22x duration run
    // produced 22 copies of every violation in the flat file (and, before
    // the grouping fixes below, 22 copies inside each grouped row). Keep
    // the first occurrence of each (check_name, method, path,
    // violation_type, message) tuple; re-runs that merged the on-disk file
    // are collapsed too. Preserves first-seen order.
    {
        let mut seen: std::collections::HashSet<(String, String, String, String, String)> =
            std::collections::HashSet::new();
        all.retain(|v| {
            let f = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
            seen.insert((
                f("check_name"),
                f("method"),
                f("path"),
                f("violation_type"),
                f("message"),
            ))
        });
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

    // Round 46 (#79) — Srikanth on 0.3.190: "I see three different
    // messages, is this message for 3 different requests or for 1
    // request. if it is 1 request can we have 1 line item mentioning
    // violation 1 = message1, violation2 = message2 etc". Emit a
    // sibling file grouped by (check_name, method, path) so each
    // wire-level request shows up as a single row carrying every
    // violation it raised. The per-violation file stays as-is for
    // tooling that wants the flat shape.
    let grouped_dst = output_dir.join("conformance-request-violations-by-request.json");
    let grouped_value = group_violations_by_request(&all);
    if let Ok(json) = serde_json::to_string_pretty(&grouped_value) {
        let _ = std::fs::write(&grouped_dst, json);
    }

    // Round 48 (#79) — Srikanth on 0.3.192: "Can I assume all this
    // checks has some violation either in the incoming request or
    // outgoing response if yes then how can I see all this violation
    // individually? Do we have any other Logs pointing each of those
    // so that I can fix in one go?" New per-probe drill-down file
    // emits one row per (check_name, method, path) carrying its full
    // flat violation list. Lets the user see EXACTLY what each probe
    // pattern (body:json, schema:string, constraint:enum, etc.)
    // surfaced rather than just the deduped union the
    // by-request file shows.
    let drill_dst = output_dir.join("conformance-request-violations-by-probe.json");
    let drill_value = group_violations_by_probe(&all);
    if let Ok(json) = serde_json::to_string_pretty(&drill_value) {
        let _ = std::fs::write(&drill_dst, json);
    }
    Ok(emitted_violations.len())
}

/// Round 48 (#79) — emit one entry per (check_name, method, path)
/// with its full violation list. Unlike `group_violations_by_request`,
/// this preserves the per-probe view so the user can see WHICH spec-
/// probing pattern (body:json / schema:string / constraint:enum /
/// method:POST / etc.) surfaced WHICH violation. Sorted by check_name
/// within the same (method, path) so probes group together visually.
fn group_violations_by_probe(flat: &[serde_json::Value]) -> serde_json::Value {
    use serde_json::{Map, Value};

    let mut by_probe_order: Vec<(String, String, String)> = Vec::new();
    let mut by_probe: std::collections::HashMap<(String, String, String), Vec<(String, String)>> =
        std::collections::HashMap::new();

    // Round 50 (#79) — Srikanth on 0.3.194: "I see same violation is
    // getting printed in logs for 22 times" on a multi-iteration run.
    // The self-test capture holds one probe per iteration, so a 22x
    // duration run feeds 22 byte-identical violations per probe into
    // this flat list and we used to append all 22. Dedup identical
    // (violation_type, message) pairs WITHIN a probe so each unique
    // violation shows exactly once regardless of iteration count.
    let mut seen_in_probe: std::collections::HashSet<(String, String, String, String)> =
        std::collections::HashSet::new();
    for v in flat {
        let check = v.get("check_name").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let method = v.get("method").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let path = v.get("path").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let vt = v.get("violation_type").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let msg = v.get("message").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let key = (check.clone(), method.clone(), path.clone());
        if !by_probe.contains_key(&key) {
            by_probe_order.push(key.clone());
        }
        if seen_in_probe.insert((check, method, path, format!("{vt}\u{0}{msg}"))) {
            by_probe.entry(key).or_default().push((vt, msg));
        }
    }

    // Sort within same (method, path) by check_name for visual grouping.
    by_probe_order.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)).then(a.0.cmp(&b.0)));

    let mut rows: Vec<Value> = Vec::with_capacity(by_probe_order.len());
    for key in &by_probe_order {
        let (check, method, path) = key;
        let entries = by_probe.get(key).cloned().unwrap_or_default();
        let mut row = Map::new();
        row.insert("check_name".into(), Value::String(check.clone()));
        row.insert("method".into(), Value::String(method.clone()));
        row.insert("path".into(), Value::String(path.clone()));
        row.insert(
            "violation_count".into(),
            Value::Number(serde_json::Number::from(entries.len())),
        );
        for (i, (vt, msg)) in entries.iter().enumerate() {
            let mut entry = Map::new();
            entry.insert("violation_type".into(), Value::String(vt.clone()));
            entry.insert("message".into(), Value::String(msg.clone()));
            row.insert(format!("violation_{}", i + 1), Value::Object(entry));
        }
        rows.push(Value::Object(row));
    }
    Value::Array(rows)
}

/// Round 46–50 (#79) — collapse the flat list of
/// [`RequestViolation`]-shaped JSON values into exactly ONE entry per
/// `(method, path)`.
///
/// History: Round 46 keyed on `(check_name, method, path)` (too many
/// duplicate rows). Round 47 collapsed by `(method, path)` AND the
/// violation set, listing contributing checks in a `checks: [...]`
/// array. But that re-split a single URL whenever two probe families
/// produced DIFFERENT violation sets for it — Srikanth on 0.3.194:
/// `owasp:ldap-injection` (query violations) landed in a different
/// by-request row than the `request-body:*` checks for the very same
/// URL, so his triage flow ("find the URL with the most violations
/// here, then drill into by-probe") missed half the picture.
///
/// Round 50 makes this file the authoritative per-URL overview: one row
/// per `(method, path)` carrying the DEDUPED UNION of every violation
/// and every contributing `check_name`. The per-probe attribution
/// ("which check surfaced which violation") lives in the sibling
/// `conformance-request-violations-by-probe.json`. First-seen order is
/// preserved for both checks and violations so the output is stable.
fn group_violations_by_request(flat: &[serde_json::Value]) -> serde_json::Value {
    use serde_json::{Map, Value};

    let mut order: Vec<(String, String)> = Vec::new();
    let mut checks_by_key: std::collections::HashMap<(String, String), Vec<String>> =
        std::collections::HashMap::new();
    let mut viols_by_key: std::collections::HashMap<(String, String), Vec<(String, String)>> =
        std::collections::HashMap::new();
    // Per-(method,path) dedup sets so a check fired across 22 iterations,
    // or the same (vt,msg) surfaced by several checks, is counted once.
    let mut seen_check: std::collections::HashSet<(String, String, String)> =
        std::collections::HashSet::new();
    let mut seen_viol: std::collections::HashSet<(String, String, String)> =
        std::collections::HashSet::new();

    for v in flat {
        let check = v.get("check_name").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let method = v.get("method").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let path = v.get("path").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let vt = v.get("violation_type").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let msg = v.get("message").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let key = (method.clone(), path.clone());
        if !checks_by_key.contains_key(&key) && !viols_by_key.contains_key(&key) {
            order.push(key.clone());
        }
        if !check.is_empty() && seen_check.insert((method.clone(), path.clone(), check.clone())) {
            checks_by_key.entry(key.clone()).or_default().push(check);
        }
        if seen_viol.insert((method.clone(), path.clone(), format!("{vt}\u{0}{msg}"))) {
            viols_by_key.entry(key).or_default().push((vt, msg));
        }
    }

    let mut rows: Vec<Value> = Vec::with_capacity(order.len());
    for key in &order {
        let (method, path) = key;
        let checks = checks_by_key.get(key).cloned().unwrap_or_default();
        let viols = viols_by_key.get(key).cloned().unwrap_or_default();
        let mut row = Map::new();
        row.insert(
            "checks".into(),
            Value::Array(checks.iter().map(|s| Value::String(s.clone())).collect()),
        );
        // Round 48 (#79) — keep a single representative `check_name`
        // pointing at the check whose family matches the FIRST violation,
        // so the headline check isn't misleading. The full set is in
        // `checks[]`; per-violation attribution is in the by-probe file.
        let dominant_prefix: &str = viols
            .first()
            .map(|(vt, _)| {
                if vt.starts_with("query_") {
                    "param:query"
                } else if vt.starts_with("body_") {
                    "body:"
                } else if vt.starts_with("path_") {
                    "param:path"
                } else if vt.starts_with("header_") {
                    "param:header"
                } else {
                    ""
                }
            })
            .unwrap_or("");
        let best_check = if !dominant_prefix.is_empty() {
            checks
                .iter()
                .find(|c| c.starts_with(dominant_prefix))
                .cloned()
                .or_else(|| checks.first().cloned())
                .unwrap_or_default()
        } else {
            checks.first().cloned().unwrap_or_default()
        };
        row.insert("check_name".into(), Value::String(best_check));
        row.insert("method".into(), Value::String(method.clone()));
        row.insert("path".into(), Value::String(path.clone()));
        row.insert("violation_count".into(), Value::Number(serde_json::Number::from(viols.len())));
        for (i, (vt, msg)) in viols.iter().enumerate() {
            let mut entry = Map::new();
            entry.insert("violation_type".into(), Value::String(vt.clone()));
            entry.insert("message".into(), Value::String(msg.clone()));
            row.insert(format!("violation_{}", i + 1), Value::Object(entry));
        }
        rows.push(Value::Object(row));
    }
    Value::Array(rows)
}

/// Round 52 (#79) — validate an emitted request body against the
/// operation's requestBody schema, resolving `$ref` at both the
/// requestBody and schema level and delegating to the same full JSON
/// Schema validator (`build_validator`) the custom-checks path uses.
///
/// This replaced a shallow, `$ref`-unaware check that only fired for
/// inline schemas and only type-checked string values — which is why a
/// self-test run against the Apigee spec (whose request bodies are all
/// `$ref`s to component schemas) produced empty violation logs even
/// though the summary reported thousands of caught request-body
/// negatives. We cap at 5 errors per body so a deeply-broken probe
/// can't flood the log; the by-probe file still gets one row per probe.
fn validate_emitted_body(
    check: &str,
    method: &str,
    url: &str,
    body: &serde_json::Value,
    operation: &openapiv3::Operation,
    spec: &OpenAPI,
    violations: &mut Vec<RequestViolation>,
) {
    // Resolve the requestBody (may itself be a $ref into components).
    let Some(request_body_ref) = &operation.request_body else {
        return;
    };
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

    // Only cross-check JSON bodies — other media types (multipart,
    // urlencoded) are handled elsewhere / by the server validator.
    let json_media = request_body
        .content
        .get("application/json")
        .or_else(|| request_body.content.iter().find(|(k, _)| k.contains("json")).map(|(_, v)| v));
    let Some(media) = json_media else {
        return;
    };
    let Some(schema_ref) = &media.schema else {
        return;
    };

    // Resolve the immediate schema $ref (one level) to the root schema,
    // then hand it to the resolver so nested $refs resolve against the
    // full document (round 18.3's fix for vCenter's nested components).
    let root_schema = match schema_ref {
        ReferenceOr::Item(s) => s.clone(),
        ReferenceOr::Reference { reference } => {
            let name = reference.strip_prefix("#/components/schemas/").unwrap_or(reference);
            match spec.components.as_ref().and_then(|c| c.schemas.get(name)) {
                Some(ReferenceOr::Item(s)) => s.clone(),
                _ => return,
            }
        }
    };

    let Ok(validator) = mockforge_openapi::schema_ref_resolver::build_validator(&root_schema, spec)
    else {
        // Schema itself is unbuildable — skip rather than false-positive.
        return;
    };
    for err in validator.iter_errors(body).take(5) {
        let loc = err.instance_path.to_string();
        let loc = if loc.is_empty() { "$".to_string() } else { loc };
        violations.push(RequestViolation {
            check_name: check.to_string(),
            method: method.to_string(),
            path: url.to_string(),
            violation_type: "body_schema_violation".to_string(),
            message: format!("body{}: {}", loc, err),
        });
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

#[cfg(test)]
mod grouping_tests {
    use super::{group_violations_by_probe, group_violations_by_request};
    use serde_json::json;

    /// Build a flat violation value the way `validate_emitted_requests` does.
    fn viol(check: &str, method: &str, path: &str, vt: &str, msg: &str) -> serde_json::Value {
        json!({
            "check_name": check,
            "method": method,
            "path": path,
            "violation_type": vt,
            "message": msg,
        })
    }

    /// Round 50 (#79) — reproduces Srikanth's 0.3.194 report: a single URL
    /// whose query violations come from `owasp:ldap-injection` while its
    /// body violations come from `request-body:*` checks must collapse into
    /// ONE by-request row that lists BOTH check families and the UNION of
    /// every violation. Previously these split into two separate rows, so
    /// the owasp check was invisible from the body row he was reading.
    #[test]
    fn by_request_unions_all_checks_for_a_url() {
        let path = "https://host/v1/organizations?alt=test-value&prettyPrint=test-value";
        let flat = vec![
            viol(
                "request-body:type-mismatch:billingType",
                "POST",
                path,
                "body_type_mismatch",
                "body.billingType: expected string",
            ),
            viol(
                "owasp:ldap-injection",
                "POST",
                path,
                "query_value_mismatch",
                "query.alt: value \"test-value\" is not one of \"json\" or \"media\"",
            ),
            viol(
                "owasp:ldap-injection",
                "POST",
                path,
                "query_value_mismatch",
                "query.prettyPrint: value \"test-value\" is not of type \"boolean\"",
            ),
        ];

        let out = group_violations_by_request(&flat);
        let rows = out.as_array().expect("array");
        // Exactly one row for the URL — no fragmentation.
        assert_eq!(rows.len(), 1, "expected a single by-request row per URL");
        let row = &rows[0];
        assert_eq!(row["violation_count"], 3);
        let checks: Vec<&str> =
            row["checks"].as_array().unwrap().iter().map(|c| c.as_str().unwrap()).collect();
        assert!(checks.contains(&"owasp:ldap-injection"), "owasp check must appear: {checks:?}");
        assert!(
            checks.iter().any(|c| c.starts_with("request-body:")),
            "body check must appear: {checks:?}"
        );
    }

    /// Round 50 (#79) — "I see same violation is getting printed in logs for
    /// 22 times." A multi-iteration run feeds N identical violations per
    /// probe; the by-probe drill-down must show each unique violation once.
    #[test]
    fn by_probe_dedups_repeated_iterations() {
        let path = "https://host/v1/organizations?alt=test-value";
        let mut flat = Vec::new();
        for _ in 0..22 {
            flat.push(viol(
                "owasp:ldap-injection",
                "POST",
                path,
                "query_value_mismatch",
                "query.alt: value \"test-value\" is not one of \"json\" or \"media\"",
            ));
        }

        let out = group_violations_by_probe(&flat);
        let rows = out.as_array().expect("array");
        assert_eq!(rows.len(), 1, "one probe row");
        assert_eq!(rows[0]["violation_count"], 1, "22 identical iterations collapse to 1");
        assert!(rows[0].get("violation_1").is_some());
        assert!(rows[0].get("violation_2").is_none(), "no duplicate violation_2");
    }

    /// The by-request union must also collapse the 22x duplicates, not just
    /// dedup across checks.
    #[test]
    fn by_request_dedups_repeated_iterations() {
        let path = "https://host/v1/widgets";
        let mut flat = Vec::new();
        for _ in 0..22 {
            flat.push(viol(
                "request-body:type-mismatch:name",
                "POST",
                path,
                "body_type_mismatch",
                "body.name: expected string",
            ));
        }
        let out = group_violations_by_request(&flat);
        let rows = out.as_array().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["violation_count"], 1, "duplicate iterations collapse");
        let checks = rows[0]["checks"].as_array().unwrap();
        assert_eq!(checks.len(), 1, "the same check listed once");
    }

    /// Distinct URLs stay distinct.
    #[test]
    fn by_request_keeps_distinct_urls_separate() {
        let flat = vec![
            viol("c1", "POST", "https://host/a", "body_type_mismatch", "a"),
            viol("c2", "GET", "https://host/b", "query_value_mismatch", "b"),
        ];
        let out = group_violations_by_request(&flat);
        assert_eq!(out.as_array().unwrap().len(), 2);
    }
}

#[cfg(test)]
mod emitted_body_tests {
    use super::validate_emitted_requests_with_base_path;
    use std::io::Write;

    /// Round 52 (#79) — Srikanth on 0.3.198: a `--conformance-self-test
    /// --targets-file` run reported "2700 request-body caught" in the
    /// summary but wrote EMPTY `conformance-request-violations-by-request.json`
    /// and `-by-probe.json`. Root cause: the emitted-request validator's
    /// body check (`check_body_against_schema`) only fired when the
    /// requestBody media schema was an inline `ReferenceOr::Item`. The
    /// Apigee spec (like most real specs) declares
    /// `requestBody.content.application/json.schema.$ref =
    /// #/components/schemas/GoogleCloudApigeeV1Organization`, so
    /// `schema_ref.as_item()` returned `None` and every body probe was
    /// skipped. It also only type-checked STRING property values, so a
    /// `{"analyticsRegion":12345}` (number where string expected) probe
    /// produced no violation even when the schema resolved.
    ///
    /// This reproduces the multi-target self-test shape: a JSONL of
    /// captured probes, a spec whose requestBody is a `$ref`, and the
    /// exact negative labels the self-test generator emits.
    #[tokio::test]
    async fn emitted_requests_validate_ref_bodied_negatives() {
        let dir = tempfile::tempdir().expect("tempdir");

        // Spec: /v1/organizations POST, requestBody is a $ref to a
        // component schema (the real-world shape). No `required` fields
        // so the positive `{}` probe stays clean (no false positive).
        let spec_json = serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "apigee-min", "version": "1.0.0" },
            "paths": {
                "/v1/organizations": {
                    "post": {
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/Organization" }
                                }
                            }
                        },
                        "responses": { "200": { "description": "ok" } }
                    }
                }
            },
            "components": {
                "schemas": {
                    "Organization": {
                        "type": "object",
                        "properties": {
                            "analyticsRegion": { "type": "string" },
                            "displayName": { "type": "string" }
                        }
                    }
                }
            }
        });
        let spec_path = dir.path().join("apigee-min.json");
        std::fs::write(&spec_path, serde_json::to_vec_pretty(&spec_json).unwrap()).unwrap();

        // JSONL of captured probes, mirroring the self-test capture shape
        // (label / method / url / request_body). One positive, two
        // negatives (a type-mismatch on a $ref'd property, and a
        // wrong-root-type body).
        let jsonl_path = dir.path().join("conformance-self-test-requests.jsonl");
        let mut f = std::fs::File::create(&jsonl_path).unwrap();
        let base = "https://172.22.232.2:443/v1/organizations?alt=json";
        for line in [
            serde_json::json!({
                "label": "positive", "method": "POST", "url": base, "request_body": "{}"
            }),
            serde_json::json!({
                "label": "request-body:type-mismatch:analyticsRegion",
                "method": "POST", "url": base,
                "request_body": "{\"analyticsRegion\":12345}"
            }),
            serde_json::json!({
                "label": "request-body:wrong-type",
                "method": "POST", "url": base, "request_body": "[]"
            }),
        ] {
            writeln!(f, "{}", serde_json::to_string(&line).unwrap()).unwrap();
        }
        drop(f);

        let n = validate_emitted_requests_with_base_path(
            std::slice::from_ref(&spec_path),
            dir.path(),
            None,
        )
        .await
        .expect("validation runs");

        assert!(n >= 2, "expected the two request-body negatives to be flagged, got {n}");

        // The grouped files the user actually reads must be non-empty.
        let by_request = std::fs::read_to_string(
            dir.path().join("conformance-request-violations-by-request.json"),
        )
        .unwrap();
        let by_request: serde_json::Value = serde_json::from_str(&by_request).unwrap();
        assert!(
            !by_request.as_array().unwrap().is_empty(),
            "by-request file must not be empty for a spec with $ref request bodies"
        );

        let by_probe = std::fs::read_to_string(
            dir.path().join("conformance-request-violations-by-probe.json"),
        )
        .unwrap();
        let by_probe: serde_json::Value = serde_json::from_str(&by_probe).unwrap();
        assert!(!by_probe.as_array().unwrap().is_empty(), "by-probe file must not be empty");

        // The type-mismatch probe must surface as a violation naming the field.
        let flat = std::fs::read_to_string(dir.path().join("conformance-request-violations.json"))
            .unwrap();
        assert!(
            flat.contains("analyticsRegion"),
            "the number-where-string probe must be reported: {flat}"
        );
    }
}
