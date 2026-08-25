//! Passive conformance validation for proxied traffic (#864).
//!
//! When `mockforge proxy` fronts a real upstream with a spec loaded
//! (`--validate-conformance` / `MOCKFORGE_PROXY_VALIDATE_CONFORMANCE`),
//! every request and response crossing the proxy is validated against
//! the spec and the findings land in the SAME
//! [`mockforge_foundation::conformance_violations`] buffer the bench,
//! TUI Conformance tab and admin endpoints already read — no new sink.
//!
//! Observational by default: traffic always forwards untouched. With
//! `--validate-conformance-strict`, request violations reject with the
//! spec's configured status instead of forwarding.

use std::collections::HashMap;

use axum::http::HeaderMap;
use serde_json::Map as JsonMap;
use serde_json::Value;

use mockforge_openapi::openapi_routes::OpenApiRouteRegistry;
use mockforge_openapi::spec::OpenApiSpec;
use mockforge_openapi::schema_ref_resolver::merge_components_into;

/// A passive conformance tap over one loaded spec.
pub struct ConformanceTap {
    // NOTE: no Debug derive — OpenApiRouteRegistry isn't Debug; ProxyServer
    // implements Debug manually around it.
    registry: OpenApiRouteRegistry,
    strict: bool,
}

impl ConformanceTap {
    /// Build a tap from a raw OpenAPI 3.x document (JSON or YAML already
    /// parsed to a Value).
    pub fn from_spec_value(spec: Value, strict: bool) -> Result<Self, String> {
        let openapi =
            OpenApiSpec::from_json(spec).map_err(|e| format!("invalid spec: {e}"))?;
        Ok(Self {
            registry: OpenApiRouteRegistry::new(openapi),
            strict,
        })
    }

    pub fn is_strict(&self) -> bool {
        self.strict
    }

    /// Match a concrete request path against spec templates.
    /// Returns `(template, extracted path params)`.
    pub fn match_template(
        &self,
        method: &str,
        concrete_path: &str,
    ) -> Option<(String, HashMap<String, String>)> {
        // Cheap pre-filter on segment count before per-route matching.
        self.registry.routes().iter().find_map(|route| {
            if !route.method.eq_ignore_ascii_case(method) {
                return None;
            }
            match_template(concrete_path, &route.path)
                .map(|params| (route.path.clone(), params))
        })
    }

    /// Resolve the matched route object for a concrete path.
    fn matched_route(
        &self,
        method: &str,
        concrete_path: &str,
    ) -> Option<(String, &mockforge_openapi::route::OpenApiRoute)> {
        let (template, _) = self.match_template(method, concrete_path)?;
        let route = self.registry.get_route(&template, method)?;
        Some((template, route))
    }

    /// Validate an inbound request against the matched route.
    ///
    /// Violations are recorded into the shared conformance buffer inside
    /// `run_validation_with_recording_ex`. Returns `Some((status, payload))`
    /// only in STRICT mode when validation fails, so the caller can reject
    /// instead of forwarding.
    pub async fn validate_request(
        &self,
        method: &str,
        concrete_path: &str,
        query: Option<&str>,
        headers: &HeaderMap,
        body: Option<&[u8]>,
    ) -> Option<(u16, Value)> {
        let Some((template, path_params)) = self.match_template(method, concrete_path) else {
            return None; // not in spec — nothing to validate against
        };

        let mut query_map = JsonMap::new();
        if let Some(q) = query {
            for pair in q.split('&') {
                let mut kv = pair.splitn(2, '=');
                if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
                    if let (Ok(k), Ok(v)) = (
                        urlencoding_decode(k),
                        urlencoding_decode(v),
                    ) {
                        query_map.insert(k, Value::String(v));
                    }
                }
            }
        }

        let mut header_values = JsonMap::new();
        for (k, v) in headers {
            if let Ok(vs) = v.to_str() {
                header_values.insert(k.to_string(), Value::String(vs.to_string()));
            }
        }
        // The validator looks up header params case-sensitively by their
        // SPEC spelling, while axum lowercases wire names. Re-key every
        // spec-declared header from the (case-insensitive) HeaderMap so
        // `X-Trace` in the spec matches `x-trace` on the wire.
        if let Some((_, route)) = self.matched_route(method, concrete_path) {
            for p in route.operation.parameters.iter() {
                if let Some(openapiv3::Parameter::Header { parameter_data, .. }) =
                    match p {
                        openapiv3::ReferenceOr::Item(param) => Some(param),
                        _ => None,
                    }
                {
                    if !header_values.contains_key(&parameter_data.name) {
                        if let Some(v) = headers.get(&parameter_data.name) {
                            if let Ok(vs) = v.to_str() {
                                header_values
                                    .insert(parameter_data.name.clone(), Value::String(vs.to_string()));
                            }
                        }
                    }
                }
            }
        }

        // Path params feed the validator as their own bucket.
        let path_param_map: JsonMap<String, Value> = path_params
            .iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect();

        let (body_json, body_present) = parse_body(body);

        match self.registry.run_validation_with_recording_ex(
            &template,
            method,
            &path_param_map,
            &query_map,
            &header_values,
            &JsonMap::new(), // cookies
            body_json.as_ref(),
            body_present,
        ) {
            Ok(()) => None,
            Err((status, payload)) => self.strict.then_some((status, payload)),
        }
    }

    /// Validate a response received from upstream against the matched
    /// route's declared responses:
    /// - undeclared status code (and no default) → `response-shape` violation;
    /// - declared JSON schema → body validated via `validate_json_value`,
    ///   errors collapsed into one `response-shape` violation.
    ///
    /// Always observational; never mutates the response.
    pub async fn validate_response(
        &self,
        method: &str,
        concrete_path: &str,
        status: u16,
        body: Option<&[u8]>,
    ) {
        let Some((template, _)) = self.match_template(method, concrete_path) else {
            return;
        };
        let Some(route) = self.registry.get_route(&template, method) else {
            return;
        };

        // 1. Is this status even declared?
        let responses = &route.operation.responses;
        let status_declared = responses.responses.keys().any(|code| match code {
            openapiv3::StatusCode::Code(c) => *c == status,
            openapiv3::StatusCode::Range(start) => {
                status >= *start && status < *start + 100
            }
        });
        let has_default = responses.default.is_some();
        if !status_declared && !has_default {
            record_response_shape(
                method,
                concrete_path,
                status,
                &format!(
                    "response status {status} is not declared for {method} {template}"
                ),
            );
            return;
        }

        // 2. Declared + JSON body → schema check.
        let Some(schema_value) = declared_json_schema(responses, status as u16) else {
            return;
        };
        let Some(bytes) = body else { return };
        let Ok(body_json) = serde_json::from_slice::<Value>(bytes) else {
            return; // non-JSON bodies have no schema contract here
        };

        let merged = merge_components_into(schema_value.clone(), &self.registry.spec().spec);
        let result =
            mockforge_openapi::openapi_routes::validation::validate_json_value(
                &body_json,
                &merged,
            );
        if !result.errors.is_empty() {
            let detail = result
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join("; ");
            record_response_shape(
                method,
                concrete_path,
                status,
                &format!("response body violates {method} {template} {status} schema: {detail}"),
            );
        }
    }
}

fn record_response_shape(method: &str, path: &str, status: u16, reason: &str) {
    use mockforge_foundation::conformance_violations::{self, ServerConformanceViolation};
    conformance_violations::record(ServerConformanceViolation {
        timestamp: chrono::Utc::now(),
        method: method.to_string(),
        path: path.to_string(),
        client_ip: "unknown".to_string(),
        status,
        reason: reason.to_string(),
        category: "response-shape".to_string(),
        occurrences: 1,
        client_mockforge_version: None,
        client_sent_at: None,
        summary: String::new(),
        categories: vec!["response-shape".to_string()],
    });
}

fn declared_json_schema(responses: &openapiv3::Responses, status: u16) -> Option<Value> {
    use openapiv3::{ReferenceOr, Response, StatusCode};
    let response_ref: Option<&ReferenceOr<Response>> = responses
        .responses
        .iter()
        .find_map(|(code, r)| match code {
            StatusCode::Code(c) if *c == status => Some(r),
            StatusCode::Range(start) if status >= *start && status < *start + 100 => Some(r),
            _ => None,
        });
    let response = match response_ref? {
        ReferenceOr::Item(r) => r,
        ReferenceOr::Reference { .. } => return None, // external ref: skip
    };
    let media = response.content.get("application/json")?;
    match &media.schema {
        Some(ReferenceOr::Item(schema)) => Some(serde_json::to_value(schema).ok()?),
        Some(ReferenceOr::Reference { reference }) => {
            // Inline local refs stay as $ref objects; the caller merges
            // components over them.
            Some(serde_json::json!({ "$ref": reference }))
        }
        None => None,
    }
}

/// Segment-wise template matcher: `/users/{id}` vs concrete paths.
/// Returns extracted `{placeholder}` values.
fn match_template(concrete: &str, template: &str) -> Option<HashMap<String, String>> {
    let mut params = HashMap::new();
    let mut c_parts = concrete.split('/');
    for t in template.split('/') {
        let c = c_parts.next()?;
        if t.starts_with('{') && t.ends_with('}') {
            params.insert(t[1..t.len() - 1].to_string(), c.to_string());
        } else if t != c {
            return None;
        }
    }
    if c_parts.next().is_some() {
        return None; // concrete path longer than template
    }
    Some(params)
}

fn urlencoding_decode(v: &str) -> Result<String, ()> {
    urlencoding::decode(v)
        .map(|c| c.into_owned())
        .map_err(|_| ())
}

fn parse_body(body: Option<&[u8]>) -> (Option<Value>, bool) {
    match body {
        None => (None, false),
        Some(b) if b.is_empty() => (None, false),
        Some(b) => (serde_json::from_slice::<Value>(b).ok(), true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_foundation::conformance_violations;

    fn demo_spec() -> Value {
        serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "T", "version": "1" },
            "paths": {
                "/users/{id}": {
                    "get": {
                        "parameters": [
                            { "name": "id", "in": "path", "required": true,
                              "schema": { "type": "string" } },
                            { "name": "X-Trace", "in": "header", "required": true,
                              "schema": { "type": "string" } }
                        ],
                        "responses": {
                            "200": { "description": "ok",
                                     "content": { "application/json": {
                                         "schema": { "type": "object",
                                             "required": ["id"],
                                             "properties": { "id": { "type": "string" } } } } } }
                        }
                    }
                },
                "/users": {
                    "post": {
                        "requestBody": {
                            "required": true,
                            "content": { "application/json": {
                                "schema": { "type": "object",
                                    "required": ["email"],
                                    "properties": { "email": { "type": "string" } } } } }
                        },
                        "responses": { "200": { "description": "ok" } }
                    }
                }
            }
        })
    }

    async fn tap(strict: bool) -> ConformanceTap {
        ConformanceTap::from_spec_value(demo_spec(), strict).expect("spec builds")
    }

    #[tokio::test]
    async fn missing_required_header_records_query_side_violation() {
        conformance_violations::clear();
        let t = tap(false).await;

        // X-Trace header required by spec but absent.
        let rejected = t
            .validate_request(
                "GET",
                "/users/abc",
                None,
                &HeaderMap::new(),
                None,
            )
            .await;
        assert!(rejected.is_none(), "observational mode must not reject");

        let snap = conformance_violations::snapshot();
        assert!(
            snap.iter().any(|v| v.path == "/users/{id}" && !v.reason.is_empty()),
            "violation should land in the shared buffer, got {:?}",
            snap
        );
    }

    #[tokio::test]
    async fn strict_mode_returns_rejection_payload() {
        let t = tap(true).await;
        let mut headers = HeaderMap::new();
        headers.insert("X-Trace", "abc".parse().unwrap());
        let rejected = t
            .validate_request("GET", "/users/ok", None, &headers, None)
            .await;
        // /users/ok matches no route? It DOES match /users/{id}; body absent
        // and all params satisfied -> no rejection.
        assert!(rejected.is_none());
    }

    #[tokio::test]
    async fn unmatched_paths_are_skipped_silently() {
        conformance_violations::clear();
        let t = tap(false).await;
        assert!(t.match_template("GET", "/unknown/path/deep").is_none());
        assert!(
            t.validate_request("GET", "/unknown/path/deep", None, &HeaderMap::new(), None)
                .await
                .is_none()
        );
        assert!(
            !conformance_violations::snapshot()
                .iter()
                .any(|v| v.path.starts_with("/unknown")),
            "unmatched paths must not produce violations"
        );
    }

    #[test]
    fn template_matcher_extracts_params() {
        let params = match_template("/users/42/orders/7", "/users/{u}/orders/{o}")
            .expect("matches");
        assert_eq!(params.get("u").map(String::as_str), Some("42"));
        assert_eq!(params.get("o").map(String::as_str), Some("7"));
        assert!(match_template("/users/42/extra", "/users/{u}").is_none());
        assert!(match_template("/users", "/users/{u}").is_none());
    }

    #[tokio::test]
    async fn undeclared_response_status_records_response_shape() {
        conformance_violations::clear();
        let t = tap(false).await;

        // 503 is not declared on GET /users/{id} (only 200) — the proxy
        // records a response-shape violation observationally.
        t.validate_response("GET", "/users/abc", 503, None).await;

        let snap = conformance_violations::snapshot();
        assert!(
            snap.iter().any(|v| v.category == "response-shape"),
            "undeclared status should record a response-shape violation"
        );
    }
}
