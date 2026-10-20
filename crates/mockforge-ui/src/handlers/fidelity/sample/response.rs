//! Validate JSON responses and distinguish unsupported media from verified data.

use super::*;
use mockforge_contracts::response_validation::{check_response_drift, response_definition};

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
    let definition = match response_definition(registry, spec, method, template, status) {
        Ok(value) => value,
        Err(drift) => return Ok((vec![format!("Response: {}", drift.detail)], vec![])),
    };
    let headers: serde_json::Map<String, Value> = serde_json::from_str(&response.headers)
        .map_err(|_| "Recorded response headers must be a JSON object")?;
    let media = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .and_then(|(_, v)| v.as_str())
        .map(|v| v.split(';').next().unwrap_or("").trim());
    if let Some(content) = definition.get("content").and_then(Value::as_object) {
        if !content.is_empty()
            && (!content.contains_key("application/json")
                || media.is_some_and(|v| v != "application/json")
                || (media.is_none() && content.len() > 1))
        {
            return Ok((
                vec![],
                vec!["Response media could not be checked by the JSON sample validator".into()],
            ));
        }
        if content.contains_key("application/json")
            && body
                .as_ref()
                .is_some_and(|b| !b.is_empty() && serde_json::from_slice::<Value>(b).is_err())
        {
            return Ok((vec!["Recorded JSON response body is not valid JSON".into()], vec![]));
        }
    }
    Ok((
        check_response_drift(registry, spec, method, template, status, body.as_deref())
            .map(|d| vec![format!("Response: {}", d.detail)])
            .unwrap_or_default(),
        vec![],
    ))
}
