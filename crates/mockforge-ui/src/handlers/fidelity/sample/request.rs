//! Normalize recorder request data and enforce the shared request contract.

use super::*;
use serde_json::Map;
use std::collections::HashMap;

pub(super) fn check(
    registry: &OpenApiRouteRegistry,
    record: &RecordedExchange,
    method: &str,
    template: &str,
    params: HashMap<String, String>,
) -> Result<(Vec<String>, Vec<String>), String> {
    let body = decode(&record.request.body, &record.request.body_encoding)?;
    let parsed = body.as_ref().and_then(|b| serde_json::from_slice::<Value>(b).ok());
    let params = params.into_iter().map(|(k, v)| (k, Value::String(v))).collect();
    let headers: Map<String, Value> = serde_json::from_str(&record.request.headers)
        .map_err(|_| "Recorded headers must be a JSON object")?;
    let cookies = header(&headers, "cookie")
        .unwrap_or("")
        .split(';')
        .filter_map(|pair| {
            let (key, value) = pair.trim().split_once('=')?;
            Some((key.to_string(), Value::String(value.to_string())))
        })
        .collect();
    let query = query_params(record.request.query_params.as_deref())?;
    let mut findings = Vec::new();
    let mut notes = Vec::new();
    if body.as_ref().is_some_and(|b| !b.is_empty()) && parsed.is_none() {
        if header(&headers, "content-type")
            .is_some_and(|v| v.split(';').next().unwrap_or("").trim().ends_with("json"))
        {
            findings.push("Recorded JSON request body is not valid JSON".into());
        } else {
            notes.push("Request body could not be checked by the JSON sample validator".into());
        }
    }
    if let Err(error) = registry.validate_request_with_all_ex(
        template,
        method,
        &params,
        &query,
        &headers,
        &cookies,
        parsed.as_ref(),
        body.as_ref().is_some_and(|b| !b.is_empty()),
    ) {
        findings.push(format!("Request: {error}"));
    }
    Ok((findings, notes))
}

fn header<'a>(headers: &'a Map<String, Value>, name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .and_then(|(_, v)| v.as_str())
}

fn query_params(query: Option<&str>) -> Result<Map<String, Value>, String> {
    let Some(query) = query else {
        return Ok(Map::new());
    };
    if query.trim_start().starts_with('{') {
        return serde_json::from_str(query).map_err(|_| "Invalid recorded query object".into());
    }
    query
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            Ok((
                urlencoding::decode(&key.replace('+', " "))
                    .map_err(|_| "Invalid query encoding")?
                    .into_owned(),
                Value::String(
                    urlencoding::decode(&value.replace('+', " "))
                        .map_err(|_| "Invalid query encoding")?
                        .into_owned(),
                ),
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::query_params;
    use serde_json::json;

    #[test]
    fn raw_queries_decode_form_spaces_without_changing_encoded_or_json_pluses() {
        assert_eq!(
            query_params(Some("full+name=one+two&literal=%2B&encoded=one%20two")).unwrap(),
            json!({"full name":"one two","literal":"+","encoded":"one two"})
                .as_object()
                .unwrap()
                .clone()
        );
        assert_eq!(query_params(Some(r#"{"kind":"one+two"}"#)).unwrap()["kind"], "one+two");
    }
}
