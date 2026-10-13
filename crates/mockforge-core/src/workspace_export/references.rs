//! Create and resolve only declared, caller-supplied workspace bindings.
use super::REFERENCE_PREFIX;
use crate::{Error, Result};
use serde_json::Value;

pub(super) fn reference(references: &mut Vec<String>) -> String {
    let name = format!("{REFERENCE_PREFIX}{:06}", references.len());
    references.push(name.clone());
    format!("${{{name}}}")
}

pub(super) fn resolve(
    value: &mut Value,
    names: &[String],
    lookup: &impl Fn(&str) -> Option<String>,
) -> Result<()> {
    match value {
        Value::String(string) => {
            if let Some(name) = string.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
                if name.starts_with(REFERENCE_PREFIX) {
                    if !names.iter().any(|allowed| allowed == name) {
                        return Err(Error::validation("Undeclared workspace export reference"));
                    }
                    *string = lookup(name).ok_or_else(|| {
                        Error::validation("Missing workspace export environment binding")
                    })?;
                }
            }
        }
        Value::Object(map) => {
            let old = std::mem::take(map);
            for (key, mut value) in old {
                let mut resolved_key = Value::String(key);
                resolve(&mut resolved_key, names, lookup)?;
                resolve(&mut value, names, lookup)?;
                let key = resolved_key
                    .as_str()
                    .ok_or_else(|| Error::validation("Invalid workspace export key"))?
                    .to_owned();
                if map.insert(key, value).is_some() {
                    return Err(Error::validation("Duplicate resolved workspace export key"));
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                resolve(value, names, lookup)?;
            }
        }
        _ => {}
    }
    Ok(())
}
