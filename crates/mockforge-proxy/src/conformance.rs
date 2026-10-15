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

mod encoding;
mod parameters;

use parameters::{cookie_parameters, header_parameters, query_parameters};
mod response_schema;

use response_schema::declared_json_schema;

use encoding::decode_body;
use std::collections::HashMap;

use axum::http::HeaderMap;
use serde_json::Map as JsonMap;
use serde_json::Value;

use mockforge_openapi::openapi_routes::{OpenApiRouteRegistry, ValidationMode, ValidationOptions};
use mockforge_openapi::schema_ref_resolver::merge_components_into;
use mockforge_openapi::spec::OpenApiSpec;

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
        let openapi = OpenApiSpec::from_json(spec).map_err(|e| format!("invalid spec: {e}"))?;
        let routes = OpenApiRouteRegistry::new(openapi.clone());
        // Always detect violations here; this tap decides whether they reject
        // traffic. Generic mock-server environment overrides must not disable it.
        let options = ValidationOptions {
            overrides: routes
                .routes()
                .iter()
                .map(|route| {
                    (
                        format!("{} {}", route.method.to_uppercase(), route.path),
                        ValidationMode::Enforce,
                    )
                })
                .collect(),
            ..Default::default()
        };
        Ok(Self {
            registry: OpenApiRouteRegistry::new_with_options(openapi, options),
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
            match_template(concrete_path, &route.path).map(|params| (route.path.clone(), params))
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

        let query_map = query_parameters(query);
        let route = self.matched_route(method, concrete_path).map(|(_, route)| route);
        let header_values = header_parameters(headers, route);

        // Path params feed the validator as their own bucket.
        let path_param_map: JsonMap<String, Value> =
            path_params.iter().map(|(k, v)| (k.clone(), Value::String(v.clone()))).collect();

        let decoded;
        let body = if let Some(encoding) = headers.get(http::header::CONTENT_ENCODING) {
            let result = match encoding.to_str() {
                Ok(encoding) => decode_body(body.unwrap_or_default(), Some(encoding)).await,
                Err(_) => Err("request content-encoding is invalid".to_string()),
            };
            match result {
                Ok(bytes) => {
                    decoded = bytes;
                    Some(decoded.as_slice())
                }
                Err(error) => {
                    let reason = format!("request body cannot be validated: {error}");
                    mockforge_foundation::conformance_violations::record(
                        mockforge_foundation::conformance_violations::ServerConformanceViolation {
                            timestamp: chrono::Utc::now(),
                            method: method.to_string(),
                            path: concrete_path.to_string(),
                            client_ip: "unknown".to_string(),
                            status: 400,
                            reason: reason.clone(),
                            category: "request-body".to_string(),
                            occurrences: 1,
                            client_mockforge_version: None,
                            client_sent_at: None,
                            summary: String::new(),
                            categories: vec!["request-body".to_string()],
                        },
                    );
                    return self.strict.then(|| (400, serde_json::json!({"error": reason})));
                }
            }
        } else {
            body
        };
        let (body_json, body_present) = parse_body(body);
        let json_content_type = headers
            .get(http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .map(|value| value.trim().to_ascii_lowercase())
            .is_some_and(|value| value == "application/json" || value.ends_with("+json"));
        if body_present && body_json.is_none() && json_content_type {
            let reason = "request body is not valid JSON";
            mockforge_foundation::conformance_violations::record(
                mockforge_foundation::conformance_violations::ServerConformanceViolation {
                    timestamp: chrono::Utc::now(),
                    method: method.to_string(),
                    path: concrete_path.to_string(),
                    client_ip: "unknown".to_string(),
                    status: 400,
                    reason: reason.to_string(),
                    category: "request-body".to_string(),
                    occurrences: 1,
                    client_mockforge_version: None,
                    client_sent_at: None,
                    summary: String::new(),
                    categories: vec!["request-body".to_string()],
                },
            );
            return self.strict.then(|| (400, serde_json::json!({"error": reason})));
        }

        let cookies = cookie_parameters(headers);

        match self.registry.run_validation_with_recording_ex(
            &template,
            method,
            &path_param_map,
            &query_map,
            &header_values,
            &cookies,
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
        self.check_response(method, concrete_path, status, body).await;
    }

    /// Record response violations and return whether the declared contract holds.
    /// This permits the caller to enforce an explicitly requested strict policy.
    pub async fn check_response(
        &self,
        method: &str,
        concrete_path: &str,
        status: u16,
        body: Option<&[u8]>,
    ) -> bool {
        self.check_encoded_response(method, concrete_path, status, body, None).await
    }

    /// Validate a decoded copy while leaving the encoded upstream bytes untouched.
    /// Decoded JSON is bounded to 16 MiB and at most four encoding layers.
    pub async fn check_encoded_response(
        &self,
        method: &str,
        concrete_path: &str,
        status: u16,
        body: Option<&[u8]>,
        content_encoding: Option<&str>,
    ) -> bool {
        let mut headers = HeaderMap::new();
        headers
            .insert(http::header::CONTENT_TYPE, http::HeaderValue::from_static("application/json"));
        if let Some(encoding) = content_encoding {
            let Ok(value) = http::HeaderValue::from_str(encoding) else {
                return false;
            };
            headers.insert(http::header::CONTENT_ENCODING, value);
        }
        self.check_response_headers(method, concrete_path, status, body, &headers).await
    }

    /// Select the contract for the upstream media type and validate a body copy.
    pub async fn check_response_headers(
        &self,
        method: &str,
        concrete_path: &str,
        status: u16,
        body: Option<&[u8]>,
        headers: &HeaderMap,
    ) -> bool {
        let Some((template, _)) = self.match_template(method, concrete_path) else {
            return true;
        };
        let Some(route) = self.registry.get_route(&template, method) else {
            return true;
        };

        // 1. Is this status even declared?
        let responses = &route.operation.responses;
        let status_declared = responses.responses.keys().any(|code| match code {
            openapiv3::StatusCode::Code(c) => *c == status,
            openapiv3::StatusCode::Range(class) => status / 100 == *class,
        });
        let has_default = responses.default.is_some();
        if !status_declared && !has_default {
            record_response_shape(
                method,
                concrete_path,
                status,
                &format!("response status {status} is not declared for {method} {template}"),
            );
            return false;
        }

        // HEAD and bodyless HTTP statuses carry no representation to validate.
        if method.eq_ignore_ascii_case("HEAD") || status == 204 || status == 304 {
            return true;
        }
        let content_type = headers.get(http::header::CONTENT_TYPE).and_then(|v| v.to_str().ok());
        let schema_value =
            match declared_json_schema(responses, status, &self.registry.spec().spec, content_type)
            {
                Ok(Some(schema)) => schema,
                Ok(None) => return true,
                Err(error) => {
                    record_response_shape(method, concrete_path, status, &error);
                    return false;
                }
            };
        let content_encoding =
            headers.get(http::header::CONTENT_ENCODING).and_then(|v| v.to_str().ok());
        let decoded = match decode_body(body.unwrap_or_default(), content_encoding).await {
            Ok(decoded) => decoded,
            Err(error) => {
                record_response_shape(method, concrete_path, status, &error);
                return false;
            }
        };
        let Ok(body_json) = serde_json::from_slice::<Value>(&decoded) else {
            record_response_shape(
                method,
                concrete_path,
                status,
                "response body is not valid JSON for the declared JSON schema",
            );
            return false;
        };

        let merged = merge_components_into(schema_value.clone(), &self.registry.spec().spec);
        let result =
            mockforge_openapi::openapi_routes::validation::validate_json_value(&body_json, &merged);
        if !result.errors.is_empty() {
            let detail =
                result.errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>().join("; ");
            record_response_shape(
                method,
                concrete_path,
                status,
                &format!("response body violates {method} {template} {status} schema: {detail}"),
            );
            return false;
        }
        true
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

fn parse_body(body: Option<&[u8]>) -> (Option<Value>, bool) {
    match body {
        None => (None, false),
        Some([]) => (None, false),
        Some(b) => (serde_json::from_slice::<Value>(b).ok(), true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_foundation::conformance_violations;

    #[tokio::test]
    async fn response_uses_actual_media_type_and_http_body_semantics() {
        let spec = serde_json::json!({
            "openapi":"3.0.0", "info":{"title":"Media", "version":"1"},
            "paths":{"/media":{"get":{"responses":{
                "200":{"description":"ok", "content":{
                    "application/json":{"schema":{"type":"object", "required":["id"]}},
                    "application/problem+json":{"schema":{"type":"object", "required":["title"]}},
                    "application/xml":{"schema":{"type":"string"}}
                }},
                "204":{"description":"no content", "content":{"application/json":{"schema":{"type":"object"}}}},
                "304":{"description":"not modified", "content":{"application/json":{"schema":{"type":"object"}}}}
            }}, "head":{"responses":{"200":{"description":"headers", "content":{"application/json":{"schema":{"type":"object"}}}}}}}}
        });
        let tap = ConformanceTap::from_spec_value(spec, true).unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("application/xml; charset=utf-8"),
        );
        assert!(
            tap.check_response_headers("GET", "/media", 200, Some(b"<id>1</id>"), &headers)
                .await
        );
        headers.insert(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("application/problem+json"),
        );
        assert!(
            tap.check_response_headers(
                "GET",
                "/media",
                200,
                Some(br#"{"title":"Problem"}"#),
                &headers
            )
            .await
        );
        assert!(
            !tap.check_response_headers("GET", "/media", 200, Some(br#"{"id":1}"#), &headers)
                .await
        );
        headers.insert(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("Application/JSON; charset=utf-8"),
        );
        assert!(
            tap.check_response_headers("GET", "/media", 200, Some(br#"{"id":1}"#), &headers)
                .await
        );
        headers.insert(http::header::CONTENT_TYPE, http::HeaderValue::from_static("text/plain"));
        assert!(
            !tap.check_response_headers("GET", "/media", 200, Some(b"unexpected"), &headers)
                .await
        );
        headers.clear();
        assert!(
            !tap.check_response_headers("GET", "/media", 200, Some(br#"{"id":1}"#), &headers)
                .await
        );
        for status in [204, 304] {
            assert!(tap.check_response_headers("GET", "/media", status, None, &headers).await);
        }
        assert!(tap.check_response_headers("HEAD", "/media", 200, None, &headers).await);
        assert!(!tap.check_response_headers("GET", "/media", 418, None, &headers).await);
    }

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
        let rejected = t.validate_request("GET", "/users/abc", None, &HeaderMap::new(), None).await;
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
        let rejected = t.validate_request("GET", "/users/ok", None, &headers, None).await;
        // /users/ok matches no route? It DOES match /users/{id}; body absent
        // and all params satisfied -> no rejection.
        assert!(rejected.is_none());
    }

    #[tokio::test]
    async fn unmatched_paths_are_skipped_silently() {
        conformance_violations::clear();
        let t = tap(false).await;
        assert!(t.match_template("GET", "/unknown/path/deep").is_none());
        assert!(t
            .validate_request("GET", "/unknown/path/deep", None, &HeaderMap::new(), None)
            .await
            .is_none());
        assert!(
            !conformance_violations::snapshot()
                .iter()
                .any(|v| v.path.starts_with("/unknown")),
            "unmatched paths must not produce violations"
        );
    }

    #[test]
    fn template_matcher_extracts_params() {
        let params =
            match_template("/users/42/orders/7", "/users/{u}/orders/{o}").expect("matches");
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
