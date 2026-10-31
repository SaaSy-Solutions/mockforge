//! Build effective per-route definitions, including inheritance and authentication.

use super::{context::Context, projection::Projection};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

const METHODS: [&str; 8] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];

pub(super) fn operations(
    document: &Value,
    max_bytes: usize,
) -> Result<BTreeMap<String, Value>, String> {
    let paths = document
        .get("paths")
        .and_then(Value::as_object)
        .ok_or_else(|| "definition must contain an OpenAPI paths object".to_string())?;
    let mut projection = Projection::new(document, max_bytes);
    let mut out = BTreeMap::new();
    for (path, item) in paths {
        if !path.starts_with('/') {
            continue;
        }
        let item = projection.value(item, Context::Contract, 0)?;
        for method in METHODS {
            let Some(operation) = item.get(method) else {
                continue;
            };
            let mut contract = operation
                .as_object()
                .cloned()
                .ok_or_else(|| "HTTP operation must be an object".to_string())?;
            let mut parameters = BTreeMap::new();
            for parameter in
                item.get("parameters").and_then(Value::as_array).into_iter().flatten().chain(
                    operation.get("parameters").and_then(Value::as_array).into_iter().flatten(),
                )
            {
                let location = parameter
                    .get("in")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "parameter is missing its location".to_string())?;
                let name = parameter
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "parameter is missing its name".to_string())?;
                parameters.insert(format!("{location}:{name}"), parameter.clone());
            }
            contract.insert(
                "parameters".into(),
                serde_json::to_value(parameters).map_err(|e| e.to_string())?,
            );
            for inherited in ["security", "servers"] {
                if !contract.contains_key(inherited) {
                    if let Some(value) = inherited_value(&item, document, inherited) {
                        contract.insert(
                            inherited.into(),
                            projection.value(
                                value,
                                if inherited == "security" {
                                    Context::Literal
                                } else {
                                    Context::Contract
                                },
                                0,
                            )?,
                        );
                    }
                }
            }
            // Canonical absence represents the root URL default. Keeping it absent
            // also preserves existing whole-operation added/removed report values.
            if contract.get("servers") == Some(&serde_json::json!([{"url":"/"}])) {
                contract.remove("servers");
            }
            if let Some(requirements) = contract.get_mut("security").and_then(Value::as_array_mut) {
                let mut schemes = Map::new();
                for requirement in requirements.iter_mut() {
                    let entries = requirement
                        .as_object_mut()
                        .ok_or_else(|| "security requirement must be an object".to_string())?;
                    for (name, scopes) in entries {
                        if let Some(scopes) = scopes.as_array_mut() {
                            scopes.sort_by_key(Value::to_string);
                        }
                        let scheme = document
                            .get("components")
                            .and_then(|v| v.get("securitySchemes"))
                            .and_then(|v| v.get(name))
                            .ok_or_else(|| {
                                "security requirement names an undefined scheme".to_string()
                            })?;
                        schemes
                            .insert(name.clone(), projection.value(scheme, Context::Contract, 0)?);
                    }
                }
                requirements.sort_by_key(Value::to_string);
                contract.insert("securitySchemes".into(), Value::Object(schemes));
            }
            out.insert(format!("{} {path}", method.to_uppercase()), Value::Object(contract));
        }
    }
    Ok(out)
}

/// Ignore an empty root server list, whose OpenAPI meaning is the default `/`.
/// Keep path/operation overrides intact, including explicit empty arrays.
fn inherited_value<'a>(item: &'a Value, document: &'a Value, key: &str) -> Option<&'a Value> {
    item.get(key).or_else(|| {
        document
            .get(key)
            .filter(|value| key != "servers" || !value.as_array().is_some_and(Vec::is_empty))
    })
}
