//! Versioned live workspace snapshots with deterministic output and secret references.

use crate::workspace::{Folder, MockRequest, Workspace};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

const REFERENCE_PREFIX: &str = "MOCKFORGE_WORKSPACE_EXPORT_VALUE_";

/// Reviewable local workspace configuration, excluding execution history and host sync settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceExportDocument {
    /// Format version; importers reject unknown versions.
    pub format_version: u32,
    /// Workspace configuration with sensitive strings replaced by environment references.
    pub workspace: Value,
    /// Environment variable names needed to restore referenced values during import.
    pub secret_references: Vec<String>,
}

impl WorkspaceExportDocument {
    /// Snapshot authored workspace state without reading process environment or writing files.
    pub fn from_workspace(workspace: &Workspace) -> Result<Self> {
        let mut copy = workspace.clone();
        let epoch = chrono::DateTime::UNIX_EPOCH;
        copy.created_at = epoch;
        copy.updated_at = epoch;
        copy.config.global_environment.created_at = epoch;
        copy.config.global_environment.updated_at = epoch;
        for environment in &mut copy.config.environments {
            environment.created_at = epoch;
            environment.updated_at = epoch;
        }
        copy.config.sync = Default::default();
        copy.config.auto_encryption = Default::default();
        copy.config.fidelity_score = None;
        normalize_requests(&mut copy.requests);
        normalize_folders(&mut copy.folders);
        let mut value = serde_json::to_value(copy)
            .map_err(|_| Error::internal("Workspace snapshot serialization failed"))?;
        canonicalize(&mut value);
        let mut secrets = BTreeSet::new();
        collect_secrets(&value, "", &mut secrets);
        let mut references = Vec::new();
        sanitize(&mut value, "", &secrets, &mut references);
        Ok(Self {
            format_version: 1,
            workspace: value,
            secret_references: references,
        })
    }

    /// Restore a typed workspace using only values explicitly supplied by the caller.
    ///
    /// The caller controls environment access. This function never reads platform credentials.
    pub fn restore(&self, lookup: impl Fn(&str) -> Option<String>) -> Result<Workspace> {
        if self.format_version != 1 {
            return Err(Error::validation("Unsupported workspace export version"));
        }
        let mut workspace = self.workspace.clone();
        resolve(&mut workspace, &self.secret_references, &lookup)?;
        serde_json::from_value(workspace)
            .map_err(|_| Error::validation("Invalid workspace export configuration"))
    }
}

fn normalize_requests(requests: &mut [MockRequest]) {
    for request in requests {
        request.created_at = chrono::DateTime::UNIX_EPOCH;
        request.updated_at = chrono::DateTime::UNIX_EPOCH;
        request.response_history.clear();
    }
}

fn normalize_folders(folders: &mut [Folder]) {
    for folder in folders {
        folder.created_at = chrono::DateTime::UNIX_EPOCH;
        folder.updated_at = chrono::DateTime::UNIX_EPOCH;
        normalize_requests(&mut folder.requests);
        normalize_folders(&mut folder.folders);
    }
}

fn sensitive_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase().replace('-', "_");
    if matches!(name.as_str(), "rsa_public_key" | "ecdsa_public_key" | "token_type_hint") {
        return false;
    }
    if matches!(name.as_str(), "key" | "keys" | "auth")
        || name.ends_with("_key")
        || name.ends_with("_auth")
    {
        return true;
    }
    [
        "password",
        "secret",
        "token",
        "credential",
        "authorization",
        "cookie",
        "api_key",
        "apikey",
        "private_key",
        "privatekey",
        "passphrase",
    ]
    .iter()
    .any(|part| name.contains(part))
}

fn sensitive_field(parent: &str, key: &str) -> bool {
    sensitive_name(key) || matches!(parent, "variables" | "credentials" | "keys")
}

fn collect_secrets(value: &Value, parent: &str, secrets: &mut BTreeSet<String>) {
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

fn reference(references: &mut Vec<String>) -> String {
    let name = format!("{REFERENCE_PREFIX}{:06}", references.len());
    references.push(name.clone());
    format!("${{{name}}}")
}

fn sanitize(
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

fn url_has_credentials(string: &str) -> bool {
    url::Url::parse(string).is_ok_and(|url| {
        !url.username().is_empty()
            || url.password().is_some()
            || url.query_pairs().any(|(key, _)| sensitive_name(&key))
    })
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

fn resolve(
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

fn canonicalize(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = std::mem::take(map).into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let mut sorted = Map::new();
            for (key, mut value) in entries {
                canonicalize(&mut value);
                sorted.insert(key, value);
            }
            *map = sorted;
        }
        Value::Array(values) => {
            for value in values {
                canonicalize(value);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AuthConfig, BasicAuthConfig};
    use crate::routing::HttpMethod;
    use serde_json::json;
    use std::collections::{BTreeMap, HashMap};

    fn workspace() -> Workspace {
        let mut workspace = Workspace::new("Export fixture".into());
        let mut request = MockRequest::new(HttpMethod::GET, "/fixture".into(), "Fixture".into());
        request.response.body = Some("public fixture response".into());
        request.response.status_code = 201;
        workspace.requests.push(request);
        workspace
    }

    fn binding_name(value: &str) -> String {
        value.strip_prefix("${").unwrap().strip_suffix('}').unwrap().to_owned()
    }

    #[test]
    fn sensitive_values_and_basic_usernames_are_absent_from_json_and_yaml() {
        let mut workspace = workspace();
        workspace.config.auth = Some(AuthConfig {
            basic_auth: Some(BasicAuthConfig {
                credentials: HashMap::from([(
                    "sentinel-private-user".into(),
                    "sentinel-private-password".into(),
                )]),
            }),
            ..Default::default()
        });
        workspace
            .config
            .global_environment
            .variables
            .insert("fixture_value".into(), "sentinel-env-value".into());
        workspace
            .config
            .default_headers
            .insert("Authorization".into(), "Bearer sentinel-private-password".into());
        workspace
            .config
            .default_headers
            .insert("X-Auth".into(), "sentinel-custom-auth".into());
        workspace
            .config
            .default_headers
            .insert("X-Key".into(), "sentinel-custom-key".into());
        workspace.requests[0].description = Some("Copied sentinel-private-user".into());
        let export = WorkspaceExportDocument::from_workspace(&workspace).unwrap();
        for serialized in [
            serde_json::to_string(&export).unwrap(),
            serde_yaml::to_string(&export).unwrap(),
        ] {
            for secret in [
                "sentinel-private-user",
                "sentinel-private-password",
                "sentinel-env-value",
                "sentinel-custom-auth",
                "sentinel-custom-key",
            ] {
                assert!(!serialized.contains(secret));
            }
        }
        assert!(!export.secret_references.is_empty());
        assert_eq!(export.workspace["requests"][0]["response"]["body"], "public fixture response");
        assert_eq!(export.workspace["requests"][0]["response_history"], json!([]));
    }

    #[test]
    fn metadata_changes_and_map_insertion_order_do_not_change_export_bytes() {
        let mut first = workspace();
        first.requests[0].headers.insert("Z-Public".into(), "z".into());
        first.requests[0].headers.insert("A-Public".into(), "a".into());
        let mut second = first.clone();
        second.requests[0].headers.clear();
        second.requests[0].headers.insert("A-Public".into(), "a".into());
        second.requests[0].headers.insert("Z-Public".into(), "z".into());
        second.created_at = chrono::DateTime::UNIX_EPOCH;
        second.updated_at = chrono::DateTime::UNIX_EPOCH;
        second.requests[0].updated_at = chrono::DateTime::UNIX_EPOCH;
        assert_eq!(
            serde_json::to_string_pretty(&WorkspaceExportDocument::from_workspace(&first).unwrap())
                .unwrap(),
            serde_json::to_string_pretty(
                &WorkspaceExportDocument::from_workspace(&second).unwrap()
            )
            .unwrap()
        );
        assert_eq!(
            serde_yaml::to_string(&WorkspaceExportDocument::from_workspace(&first).unwrap())
                .unwrap(),
            serde_yaml::to_string(&WorkspaceExportDocument::from_workspace(&second).unwrap())
                .unwrap()
        );
    }

    #[test]
    fn url_credentials_and_json_escaped_body_credentials_are_referenced() {
        let mut workspace = workspace();
        workspace.config.base_url =
            Some("https://sentinel%2Durl%2Duser:sentinel%2Durl%2Dpassword@fixture.invalid/path?api_key=sentinel%2Durl%2Dkey".into());
        workspace.description = Some("Copied sentinel-url-user and sentinel-url-password".into());
        workspace.requests[0].response.body =
            Some(r#"{"password":"\u0073entinel-body-password"}"#.into());
        let export = WorkspaceExportDocument::from_workspace(&workspace).unwrap();
        assert!(export.workspace["config"]["base_url"].as_str().unwrap().starts_with("${"));
        assert!(export.workspace["requests"][0]["response"]["body"]
            .as_str()
            .unwrap()
            .starts_with("${"));
        assert!(!serde_json::to_string(&export).unwrap().contains("sentinel"));
    }

    #[test]
    fn restore_requires_declared_bindings_and_preserves_supported_configuration() {
        let mut original = workspace();
        original.config.auth = Some(AuthConfig {
            basic_auth: Some(BasicAuthConfig {
                credentials: HashMap::from([(
                    "fixture-username".into(),
                    "fixture-password".into(),
                )]),
            }),
            ..Default::default()
        });
        let export = WorkspaceExportDocument::from_workspace(&original).unwrap();
        let credentials = export.workspace["config"]["auth"]["basic_auth"]["credentials"]
            .as_object()
            .unwrap();
        let (username, password) = credentials.iter().next().unwrap();
        let bindings = BTreeMap::from([
            (binding_name(username), "fixture-username".to_owned()),
            (binding_name(password.as_str().unwrap()), "fixture-password".to_owned()),
        ]);
        assert!(export.restore(|_| None).is_err());
        let restored = export.restore(|name| bindings.get(name).cloned()).unwrap();
        assert_eq!(
            restored.config.auth.unwrap().basic_auth.unwrap().credentials,
            original.config.auth.unwrap().basic_auth.unwrap().credentials
        );
        assert_eq!(restored.requests[0].response.body, original.requests[0].response.body);
        assert_eq!(restored.requests[0].path, original.requests[0].path);
        let mut unknown = export.clone();
        unknown.format_version = 2;
        assert!(unknown.restore(|name| bindings.get(name).cloned()).is_err());
        let mut undeclared = export;
        undeclared.secret_references.clear();
        assert!(undeclared.restore(|name| bindings.get(name).cloned()).is_err());
    }
}
