//! Decode wire parameters while preserving spec header spelling.

use axum::http::HeaderMap;
use serde_json::{Map as JsonMap, Value};

pub(super) fn query_parameters(query: Option<&str>) -> JsonMap<String, Value> {
    let mut query_map = JsonMap::new();
    if let Some(q) = query {
        for pair in q.split('&') {
            let mut kv = pair.splitn(2, '=');
            if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
                if let (Ok(k), Ok(v)) = (
                    urlencoding::decode(k).map(|v| v.into_owned()),
                    urlencoding::decode(v).map(|v| v.into_owned()),
                ) {
                    query_map.insert(k, Value::String(v));
                }
            }
        }
    }
    query_map
}

pub(super) fn header_parameters(
    headers: &HeaderMap,
    route: Option<&mockforge_openapi::route::OpenApiRoute>,
) -> JsonMap<String, Value> {
    let mut header_values: JsonMap<String, Value> = headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|text| (name.to_string(), Value::String(text.to_string())))
        })
        .collect();
    // Validators use the spec spelling; HTTP wire names are case-insensitive.
    let Some(route) = route else {
        return header_values;
    };
    for parameter in &route.operation.parameters {
        let openapiv3::ReferenceOr::Item(openapiv3::Parameter::Header { parameter_data, .. }) =
            parameter
        else {
            continue;
        };
        if header_values.contains_key(&parameter_data.name) {
            continue;
        }
        let Some(value) = headers.get(&parameter_data.name).and_then(|v| v.to_str().ok()) else {
            continue;
        };
        header_values.insert(parameter_data.name.clone(), Value::String(value.to_string()));
    }
    header_values
}

pub(super) fn cookie_parameters(headers: &HeaderMap) -> JsonMap<String, Value> {
    let mut cookies = JsonMap::new();
    for header in headers.get_all(http::header::COOKIE) {
        if let Ok(header) = header.to_str() {
            for pair in header.split(';') {
                if let Some((name, value)) = pair.trim().split_once('=') {
                    let value =
                        value.strip_prefix('"').and_then(|v| v.strip_suffix('"')).unwrap_or(value);
                    cookies.insert(name.trim().to_string(), Value::String(value.to_string()));
                }
            }
        }
    }
    cookies
}
