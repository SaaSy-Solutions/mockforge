//! Shared response-contract validation for captured traffic and replay fixtures.

use mockforge_openapi::openapi_routes::OpenApiRouteRegistry;
use mockforge_openapi::schema_ref_resolver::merge_components_into;
use serde::Serialize;
use serde_json::Value;

/// A response schema error with its original field and value details.
#[derive(Debug, Serialize)]
pub struct FieldViolation {
    /// Field path supplied by the shared OpenAPI validator.
    pub field: String,
    /// Human-readable schema error.
    pub message: String,
    /// Expected value or type, when available.
    pub expected: Option<Value>,
    /// Observed value, when available.
    pub actual: Option<Value>,
}

/// Response-contract drift, including structured schema errors when available.
#[derive(Debug, Serialize)]
pub struct ResponseDrift {
    /// Summary of the response-contract failure.
    pub detail: String,
    /// Individual field errors; empty for status, reference and decoding errors.
    pub fields: Vec<FieldViolation>,
}

impl ResponseDrift {
    /// Create a non-field response-contract failure.
    pub fn message(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
            fields: Vec::new(),
        }
    }
}

/// Result of checking a recorded response against the supported contract.
pub enum ResponseAssessment {
    /// The recorded response satisfies the supported checks.
    Verified,
    /// The recorded response violates its declared contract.
    Drift(ResponseDrift),
    /// The contract or recorded media cannot be checked by this validator.
    Unverified(String),
}

/// Validate status and JSON response bodies without treating unsupported media
/// as verified. Status selection is exact, range, then default.
pub fn check_response(
    registry: &OpenApiRouteRegistry,
    spec: &Value,
    method: &str,
    template: &str,
    status: u16,
    content_type: Option<&str>,
    body: Option<&[u8]>,
) -> ResponseAssessment {
    let response = match response_definition(registry, spec, method, template, status) {
        Ok(response) => response,
        Err(drift) => return ResponseAssessment::Drift(drift),
    };
    let media = content_type
        .map(|value| value.split(';').next().unwrap_or(value).trim().to_ascii_lowercase());
    if let Some(content) = response.get("content").and_then(Value::as_object) {
        if !content.is_empty()
            && (!content.contains_key("application/json")
                || media.as_deref().is_some_and(|value| value != "application/json")
                || (media.is_none() && content.len() > 1))
        {
            return ResponseAssessment::Unverified(
                "Response media could not be checked by the JSON sample validator".into(),
            );
        }
        if content.contains_key("application/json")
            && body.is_some_and(|bytes| {
                !bytes.is_empty() && serde_json::from_slice::<Value>(bytes).is_err()
            })
        {
            return ResponseAssessment::Drift(ResponseDrift::message(
                "Recorded JSON response body is not valid JSON",
            ));
        }
    }
    match check_json_schema(registry, &response, status, body) {
        Some(drift) => ResponseAssessment::Drift(drift),
        None => ResponseAssessment::Verified,
    }
}

fn check_json_schema(
    registry: &OpenApiRouteRegistry,
    response: &Value,
    status: u16,
    body: Option<&[u8]>,
) -> Option<ResponseDrift> {
    let schema = response.pointer("/content/application~1json/schema")?;
    let Some(bytes) = body else {
        return Some(ResponseDrift::message(format!(
            "declared {status} response schema but the recorded response had no body"
        )));
    };
    let Ok(body_json) = serde_json::from_slice::<Value>(bytes) else {
        return Some(ResponseDrift::message(format!(
            "declared JSON schema for {status} but the recorded response is not JSON"
        )));
    };
    let merged = merge_components_into(schema.clone(), &registry.spec().spec);
    let result =
        mockforge_openapi::openapi_routes::validation::validate_json_value(&body_json, &merged);
    if result.errors.is_empty() {
        return None;
    }
    Some(ResponseDrift {
        detail: result.errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>().join("; "),
        fields: result
            .errors
            .into_iter()
            .map(|e| FieldViolation {
                field: e.field,
                message: e.message,
                expected: e.expected,
                actual: e.actual,
            })
            .collect(),
    })
}

/// Resolve the source response declaration for a method, template and status.
///
/// Returns drift for an absent route/status or an unresolved/cyclic reference.
/// Local references are resolved; this function performs no network requests.
pub fn response_definition(
    registry: &OpenApiRouteRegistry,
    spec: &Value,
    method: &str,
    template: &str,
    status: u16,
) -> Result<Value, ResponseDrift> {
    let Some(route) = registry.get_route(template, method) else {
        return Err(ResponseDrift::message(
            "response route is absent from the source specification",
        ));
    };
    let responses = &route.operation.responses;
    let selected = responses
        .responses
        .get(&openapiv3::StatusCode::Code(status))
        .or_else(|| responses.responses.get(&openapiv3::StatusCode::Range(status / 100)))
        .or(responses.default.as_ref());
    let Some(selected) = selected else {
        return Err(ResponseDrift::message(format!(
            "recorded response status {status} is not declared by the spec"
        )));
    };
    let Ok(mut response) = serde_json::to_value(selected) else {
        return Err(ResponseDrift::message("could not inspect declared response"));
    };
    let mut references = std::collections::HashSet::new();
    while let Some(reference) = response.get("$ref").and_then(Value::as_str) {
        if references.len() >= 64 || !references.insert(reference.to_string()) {
            return Err(ResponseDrift::message("cyclic or excessively nested response reference"));
        }
        let Some(value) = reference.strip_prefix('#').and_then(|pointer| spec.pointer(pointer))
        else {
            return Err(ResponseDrift::message(
                "declared response reference cannot be resolved locally",
            ));
        };
        response = value.clone();
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_routes_never_pass_the_public_response_validator() {
        let spec = serde_json::json!({"openapi":"3.0.3","info":{"title":"response","version":"1"},
            "paths":{"/items":{"get":{"responses":{"200":{"description":"ok"}}}}}});
        let registry = OpenApiRouteRegistry::new(
            mockforge_openapi::spec::OpenApiSpec::from_json(spec.clone()).unwrap(),
        );
        assert!(matches!(
            check_response(&registry, &spec, "GET", "/items", 200, None, None),
            ResponseAssessment::Verified
        ));
        let ResponseAssessment::Drift(drift) =
            check_response(&registry, &spec, "GET", "/absent", 200, None, None)
        else {
            panic!("missing route must be drift")
        };
        assert!(drift.detail.contains("absent"));
    }
    #[test]
    fn response_media_is_verified_only_when_supported_and_unambiguous() {
        for (content, media, expected) in [
            (
                serde_json::json!({"text/plain":{"schema":{"type":"string","enum":["ok"]}}}),
                Some("text/plain"),
                "unverified",
            ),
            (
                serde_json::json!({"application/problem+json":{"schema":{"type":"integer"}}}),
                Some("application/problem+json"),
                "unverified",
            ),
            (serde_json::json!({"application/json":{},"text/plain":{}}), None, "unverified"),
            (serde_json::json!({"application/json":{}}), Some("text/plain"), "unverified"),
            (
                serde_json::json!({"application/json":{"schema":{"type":"integer"}}}),
                Some("Application/JSON; charset=utf-8"),
                "verified",
            ),
            (
                serde_json::json!({"application/json":{"schema":{"type":"integer"}}}),
                None,
                "verified",
            ),
        ] {
            let spec = serde_json::json!({"openapi":"3.0.3","info":{"title":"media","version":"1"},
                "paths":{"/items":{"get":{"responses":{"200":{"description":"ok","content":content}}}}}});
            let registry = OpenApiRouteRegistry::new(
                mockforge_openapi::spec::OpenApiSpec::from_json(spec.clone()).unwrap(),
            );
            let actual =
                match check_response(&registry, &spec, "GET", "/items", 200, media, Some(b"1")) {
                    ResponseAssessment::Verified => "verified",
                    ResponseAssessment::Unverified(_) => "unverified",
                    ResponseAssessment::Drift(_) => "drift",
                };
            assert_eq!(actual, expected, "media {media:?}, contract {content}");
            assert!(
                matches!(
                    check_response(&registry, &spec, "GET", "/items", 418, media, Some(b"1")),
                    ResponseAssessment::Drift(_)
                ),
                "unsupported media must not hide undeclared status"
            );
        }
    }

    #[test]
    fn declared_json_without_a_schema_still_rejects_malformed_json() {
        let spec = serde_json::json!({"openapi":"3.0.3","info":{"title":"media","version":"1"},
            "paths":{"/items":{"get":{"responses":{"200":{"description":"ok","content":{"application/json":{}}}}}}}});
        let registry = OpenApiRouteRegistry::new(
            mockforge_openapi::spec::OpenApiSpec::from_json(spec.clone()).unwrap(),
        );
        assert!(matches!(
            check_response(
                &registry,
                &spec,
                "GET",
                "/items",
                200,
                Some("application/json"),
                Some(b"not json")
            ),
            ResponseAssessment::Drift(_)
        ));
    }
}
