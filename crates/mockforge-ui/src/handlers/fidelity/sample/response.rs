//! Validate JSON responses and distinguish unsupported media from verified data.

use super::*;
use mockforge_contracts::response_validation::{check_response, ResponseAssessment};

pub(super) fn check(
    registry: &OpenApiRouteRegistry,
    spec: &Value,
    record: &RecordedExchange,
    method: &str,
    template: &str,
) -> Result<(Vec<String>, Vec<String>), String> {
    let Some(response) = &record.response else {
        return Ok((vec!["Recorded request has no response".into()], vec![]));
    };
    let body = decode(&response.body, &response.body_encoding)?;
    let status =
        u16::try_from(response.status_code).map_err(|_| "Invalid recorded response status")?;
    let headers: serde_json::Map<String, Value> = serde_json::from_str(&response.headers)
        .map_err(|_| "Recorded response headers must be a JSON object")?;
    let media = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .and_then(|(_, v)| v.as_str())
        .map(|v| v.split(';').next().unwrap_or("").trim());
    Ok(
        match check_response(registry, spec, method, template, status, media, body.as_deref()) {
            ResponseAssessment::Verified => (vec![], vec![]),
            ResponseAssessment::Drift(drift) => {
                (vec![format!("Response: {}", drift.detail)], vec![])
            }
            ResponseAssessment::Unverified(note) => (vec![], vec![note]),
        },
    )
}
