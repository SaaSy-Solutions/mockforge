//! Replace sensitive strings with references, preserving complete body bytes on restore.
use super::classification::{sensitive_name, url_has_credentials};
use super::references::reference;
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn sanitize(
    value: &mut Value,
    parent: &str,
    secrets: &BTreeSet<String>,
    references: &mut Vec<String>,
) {
    match value {
        Value::String(string) => {
            if !string.is_empty()
                && (matches!(parent, "variables" | "credentials" | "keys")
                    || sensitive_name(parent)
                    || url_has_credentials(string)
                    || secrets.iter().any(|secret| string.contains(secret))
                    || (parent == "body"
                        && serde_json::from_str::<Value>(string)
                            .is_ok_and(|body| contains_secret(&body, secrets))))
            {
                *string = reference(references);
            }
        }
        Value::Object(map) => {
            let old = std::mem::take(map);
            for (key, mut value) in old {
                sanitize(&mut value, &key, secrets, references);
                let key = if parent == "credentials" {
                    reference(references)
                } else {
                    key
                };
                map.insert(key, value);
            }
        }
        Value::Array(values) => {
            for value in values {
                sanitize(value, parent, secrets, references);
            }
        }
        _ => {}
    }
}

fn contains_secret(value: &Value, secrets: &BTreeSet<String>) -> bool {
    match value {
        Value::String(string) => secrets.iter().any(|secret| string.contains(secret)),
        Value::Object(map) => map
            .iter()
            .any(|(key, value)| secrets.contains(key) || contains_secret(value, secrets)),
        Value::Array(values) => values.iter().any(|value| contains_secret(value, secrets)),
        _ => false,
    }
}
