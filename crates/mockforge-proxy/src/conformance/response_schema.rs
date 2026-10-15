//! Resolve the declared status and media type before JSON schema validation.

use serde_json::Value;

pub(super) fn declared_json_schema(
    responses: &openapiv3::Responses,
    status: u16,
    spec: &openapiv3::OpenAPI,
    content_type: Option<&str>,
) -> Result<Option<Value>, String> {
    use openapiv3::{ReferenceOr, Response, StatusCode};
    // Exact status overrides a range, which overrides the default response.
    let response_ref: Option<&ReferenceOr<Response>> = responses
        .responses
        .get(&StatusCode::Code(status))
        .or_else(|| responses.responses.get(&StatusCode::Range(status / 100)))
        .or(responses.default.as_ref());
    let mut selected = response_ref.ok_or("response contract is missing")?.clone();
    let mut resolved = None;
    // Resolve only local response references, bounded against cycles.
    for _ in 0..32 {
        match selected {
            ReferenceOr::Item(response) => {
                resolved = Some(response);
                break;
            }
            ReferenceOr::Reference { reference } => {
                let name = reference
                    .strip_prefix("#/components/responses/")
                    .ok_or("external response references are unsupported")?
                    .replace("~1", "/")
                    .replace("~0", "~");
                selected = spec
                    .components
                    .as_ref()
                    .and_then(|c| c.responses.get(&name))
                    .ok_or("response reference cannot be resolved")?
                    .clone();
            }
        }
    }
    let response = resolved.ok_or("response reference cycle exceeds the resolution limit")?;
    if response.content.is_empty() {
        return Ok(None);
    }
    let actual = content_type
        .and_then(|v| v.split(';').next())
        .map(|v| v.trim().to_ascii_lowercase())
        .ok_or("response content-type is missing")?;
    let media = response
        .content
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(&actual))
        .or_else(|| {
            actual
                .split_once('/')
                .and_then(|(kind, _)| response.content.get_key_value(&format!("{kind}/*")))
        })
        .or_else(|| response.content.get_key_value("*/*"))
        .map(|(_, media)| media)
        .ok_or_else(|| format!("response content-type {actual} is not declared"))?;
    // Other declared representations pass through; this validator checks JSON schemas.
    if actual != "application/json" && !actual.ends_with("+json") {
        return Ok(None);
    }
    match &media.schema {
        Some(ReferenceOr::Item(schema)) => {
            serde_json::to_value(schema).map(Some).map_err(|e| e.to_string())
        }
        Some(ReferenceOr::Reference { reference }) => {
            Ok(Some(serde_json::json!({ "$ref": reference })))
        }
        None => Ok(None),
    }
}
