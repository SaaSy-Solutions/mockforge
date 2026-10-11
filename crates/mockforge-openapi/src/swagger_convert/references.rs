//! Resolution of reusable parameters and responses in the source document.

use serde_json::Value;
use std::collections::HashSet;

pub(super) fn resolve(value: &Value, source: &Value) -> Result<Value, String> {
    let mut current = value;
    let mut seen = HashSet::new();
    while let Some(reference) = current.get("$ref").and_then(Value::as_str) {
        if !reference.starts_with("#/parameters/") && !reference.starts_with("#/responses/") {
            break;
        }
        if !seen.insert(reference) {
            return Err(format!("Cyclic Swagger reference: {reference}"));
        }
        current = source
            .pointer(&reference[1..])
            .ok_or_else(|| format!("Missing Swagger reference: {reference}"))?;
    }
    Ok(current.clone())
}

pub(super) fn parameters(value: Option<&Value>, source: &Value) -> Result<Vec<Value>, String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|parameter| resolve(parameter, source))
        .collect()
}
