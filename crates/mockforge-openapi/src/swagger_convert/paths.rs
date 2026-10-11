//! Expand reusable source objects before converting operation media types.

use super::references;
use serde_json::{Map, Value};

pub(super) fn expand(source: &Value) -> Result<Value, String> {
    let mut expanded = source.clone();
    if let Some(paths) = expanded.get_mut("paths").and_then(Value::as_object_mut) {
        for path in paths.values_mut().filter_map(Value::as_object_mut) {
            expand_path(path, source)?;
        }
    }
    Ok(expanded)
}

fn expand_path(path: &mut Map<String, Value>, source: &Value) -> Result<(), String> {
    let inherited = references::parameters(path.get("parameters"), source)?;
    if path.contains_key("parameters") {
        // Move inherited parameters to each operation, where consumers apply validation.
        path.insert("parameters".into(), Value::Array(Vec::new()));
    }
    for method in ["get", "post", "put", "delete", "patch", "head", "options"] {
        if let Some(operation) = path.get_mut(method).and_then(Value::as_object_mut) {
            expand_operation(operation, &inherited, source)?;
        }
    }
    Ok(())
}

fn expand_operation(
    operation: &mut Map<String, Value>,
    inherited: &[Value],
    source: &Value,
) -> Result<(), String> {
    let mut parameters = references::parameters(operation.get("parameters"), source)?;
    for parameter in inherited {
        let overridden = parameters.iter().any(|own| {
            own.get("name") == parameter.get("name") && own.get("in") == parameter.get("in")
        });
        if !overridden {
            parameters.push(parameter.clone());
        }
    }
    if operation.contains_key("parameters") || !parameters.is_empty() {
        operation.insert("parameters".into(), Value::Array(parameters));
    }
    if let Some(responses) = operation.get_mut("responses").and_then(Value::as_object_mut) {
        for response in responses.values_mut() {
            *response = references::resolve(response, source)?;
        }
    }
    Ok(())
}
