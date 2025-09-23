//! Postman Environment import functionality
//!
//! This module handles parsing Postman environment files and extracting
//! variables that can be used in MockForge templates and request chaining.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Postman Environment structure
#[derive(Debug, Deserialize)]
pub struct PostmanEnvironment {
    pub id: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub values: Vec<EnvironmentValue>,
}

/// Environment variable value
#[derive(Debug, Deserialize)]
pub struct EnvironmentValue {
    pub key: String,
    pub value: Option<String>,
    pub description: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// Environment import result
#[derive(Debug)]
pub struct EnvironmentImportResult {
    pub name: String,
    pub variables: HashMap<String, EnvironmentVariable>,
    pub enabled_count: usize,
    pub total_count: usize,
}

/// Environment variable with metadata
#[derive(Debug, Serialize, Clone)]
pub struct EnvironmentVariable {
    pub value: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub source: VariableSource,
}

/// Source of the environment variable
#[derive(Debug, Serialize, Clone)]
pub enum VariableSource {
    Environment(String), // Environment name
    Collection,          // From collection-level variables
}

fn default_enabled() -> bool {
    true
}

/// Import a Postman Environment JSON
pub fn import_postman_environment(content: &str) -> Result<EnvironmentImportResult, String> {
    let environment: PostmanEnvironment = serde_json::from_str(content)
        .map_err(|e| format!("Failed to parse Postman environment: {}", e))?;

    let mut variables = HashMap::new();
    let mut enabled_count = 0;
    let mut total_count = 0;

    let env_name = environment.name.unwrap_or_else(|| "Unnamed Environment".to_string());

    for env_value in environment.values {
        total_count += 1;

        if env_value.enabled && env_value.value.is_some() {
            enabled_count += 1;
            let variable = EnvironmentVariable {
                value: env_value.value.unwrap(),
                description: env_value.description,
                enabled: env_value.enabled,
                source: VariableSource::Environment(env_name.clone()),
            };
            variables.insert(env_value.key, variable);
        }
    }

    Ok(EnvironmentImportResult {
        name: env_name,
        variables,
        enabled_count,
        total_count,
    })
}

/// Check if content is a Postman environment JSON
pub fn is_postman_environment_json(content: &str) -> bool {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
        if let Some(obj) = json.as_object() {
            // Check for typical environment fields
            let has_values = obj.contains_key("values");
            let has_name_or_id = obj.contains_key("name") || obj.contains_key("id");

            // values should be an array
            let values_is_array = if let Some(values) = obj.get("values") {
                values.is_array()
            } else {
                false
            };

            if has_values && has_name_or_id && values_is_array {
                // Check if values array contains environment variable structure
                if let Some(values_array) = obj.get("values") {
                    if let Some(arr) = values_array.as_array() {
                        if !arr.is_empty() {
                            // Check if first item has typical environment variable fields
                            if let Some(first_item) = arr.first() {
                                if let Some(item_obj) = first_item.as_object() {
                                    let has_key = item_obj.contains_key("key");
                                    let has_value = item_obj.contains_key("value") || item_obj.contains_key("enabled");
                                    return has_key && has_value;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_postman_environment() {
        let env_json = r#"{
            "id": "env-123",
            "name": "Development Environment",
            "values": [
                {
                    "key": "base_url",
                    "value": "https://api.dev.example.com",
                    "description": "API base URL",
                    "enabled": true
                },
                {
                    "key": "api_key",
                    "value": "dev-key-123",
                    "enabled": true
                },
                {
                    "key": "disabled_var",
                    "value": "should_not_import",
                    "enabled": false
                }
            ]
        }"#;

        let result = import_postman_environment(env_json).unwrap();

        assert_eq!(result.name, "Development Environment");
        assert_eq!(result.total_count, 3);
        assert_eq!(result.enabled_count, 2);
        assert_eq!(result.variables.len(), 2);

        // Check base_url variable
        let base_url_var = result.variables.get("base_url").unwrap();
        assert_eq!(base_url_var.value, "https://api.dev.example.com");
        assert_eq!(base_url_var.description.as_ref().unwrap(), "API base URL");
        assert_eq!(base_url_var.enabled, true);

        // Check api_key variable
        let api_key_var = result.variables.get("api_key").unwrap();
        assert_eq!(api_key_var.value, "dev-key-123");
        assert_eq!(api_key_var.enabled, true);
    }

    #[test]
    fn test_detect_postman_environment() {
        let env_json = r#"{
            "id": "env-123",
            "name": "Test Environment",
            "values": [
                {
                    "key": "test_var",
                    "value": "test_value",
                    "enabled": true
                }
            ]
        }"#;

        assert!(is_postman_environment_json(env_json));

        let not_env_json = r#"{
            "info": {
                "name": "Test Collection"
            },
            "item": []
        }"#;

        assert!(!is_postman_environment_json(not_env_json));
    }

    #[test]
    fn test_parse_minimal_environment() {
        let minimal_env_json = r#"{
            "values": [
                {
                    "key": "minimal_var",
                    "value": "minimal_value",
                    "enabled": true
                }
            ]
        }"#;

        let result = import_postman_environment(minimal_env_json).unwrap();

        assert_eq!(result.name, "Unnamed Environment");
        assert_eq!(result.total_count, 1);
        assert_eq!(result.enabled_count, 1);
        assert_eq!(result.variables.len(), 1);
    }
}
