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

/// Response-side drift: validate the recorded body against the declared
/// JSON schema for the actual status code, returning structured drift details.
/// Status selection is exact, range, then default; local response references
/// are resolved before schema validation. No network access occurs.
pub fn check_response_drift(
    registry: &OpenApiRouteRegistry,
    spec: &Value,
    method: &str,
    template: &str,
    status: u16,
    body: Option<&[u8]>,
) -> Option<ResponseDrift> {
    let Some(route) = registry.get_route(template, method) else {
        return Some(ResponseDrift::message(
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
        return Some(ResponseDrift::message(format!(
            "recorded response status {status} is not declared by the spec"
        )));
    };
    let Ok(mut response) = serde_json::to_value(selected) else {
        return Some(ResponseDrift::message("could not inspect declared response"));
    };
    let mut references = std::collections::HashSet::new();
    while let Some(reference) = response.get("$ref").and_then(Value::as_str) {
        if references.len() >= 64 || !references.insert(reference.to_string()) {
            return Some(ResponseDrift::message("cyclic or excessively nested response reference"));
        }
        let Some(value) = reference.strip_prefix('#').and_then(|pointer| spec.pointer(pointer))
        else {
            return Some(ResponseDrift::message(
                "declared response reference cannot be resolved locally",
            ));
        };
        response = value.clone();
    }
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
        assert!(check_response_drift(&registry, &spec, "GET", "/items", 200, None).is_none());
        let drift = check_response_drift(&registry, &spec, "GET", "/absent", 200, None).unwrap();
        assert!(drift.detail.contains("absent"));
    }
}
