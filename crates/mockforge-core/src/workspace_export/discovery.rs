//! Collect configured sensitive values so copies elsewhere are also protected.
use super::classification::{sensitive_field, sensitive_name};
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn collect_secrets(value: &Value, parent: &str, secrets: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                if parent == "credentials" && !key.is_empty() {
                    secrets.insert(key.clone());
                }
                if sensitive_field(parent, key) {
                    if key == "credentials" {
                        if let Value::Object(credentials) = value {
                            secrets
                                .extend(credentials.keys().filter(|key| !key.is_empty()).cloned());
                        }
                    }
                    if value.is_object() {
                        collect_secrets(value, key, secrets);
                    } else {
                        collect_strings(value, secrets);
                    }
                } else {
                    collect_secrets(value, key, secrets);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_secrets(value, parent, secrets);
            }
        }
        Value::String(string) => {
            if let Ok(url) = url::Url::parse(string) {
                for credential in [Some(url.username()), url.password()]
                    .into_iter()
                    .flatten()
                    .filter(|s| !s.is_empty())
                {
                    secrets.insert(credential.to_owned());
                    if let Ok(decoded) = urlencoding::decode(credential) {
                        secrets.insert(decoded.into_owned());
                    }
                }
                for (key, value) in url.query_pairs() {
                    if sensitive_name(&key) && !value.is_empty() {
                        secrets.insert(value.into_owned());
                    }
                }
            }
            if parent == "body" {
                if let Ok(parsed) = serde_json::from_str::<Value>(string) {
                    collect_secrets(&parsed, "", secrets);
                }
            }
        }
        _ => {}
    }
}

fn collect_strings(value: &Value, secrets: &mut BTreeSet<String>) {
    match value {
        Value::String(string) if !string.is_empty() => {
            secrets.insert(string.clone());
        }
        Value::Object(map) => {
            for value in map.values() {
                collect_strings(value, secrets);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_strings(value, secrets);
            }
        }
        _ => {}
    }
}
