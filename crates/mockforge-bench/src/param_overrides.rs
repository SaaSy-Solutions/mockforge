//! Parameter overrides for customizing request values in load tests
//!
//! This module allows users to provide custom parameter values instead of
//! the auto-generated placeholder values like "test-value".
//!
//! # Example Configuration File (JSON)
//!
//! ```json
//! {
//!   "defaults": {
//!     "path_params": {
//!       "id": "12345",
//!       "uuid": "550e8400-e29b-41d4-a716-446655440000"
//!     },
//!     "query_params": {
//!       "limit": "100",
//!       "page": "1"
//!     }
//!   },
//!   "operations": {
//!     "createUser": {
//!       "body": {
//!         "name": "Test User",
//!         "email": "test@example.com"
//!       }
//!     },
//!     "getUser": {
//!       "path_params": {
//!         "id": "user-123"
//!       }
//!     }
//!   }
//! }
//! ```

use crate::error::{BenchError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// Parameter overrides configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParameterOverrides {
    /// Default values applied to all operations
    #[serde(default)]
    pub defaults: OperationOverrides,

    /// Per-operation overrides (keyed by operation ID or "METHOD /path")
    #[serde(default)]
    pub operations: HashMap<String, OperationOverrides>,
}

/// Overrides for a specific operation or defaults
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperationOverrides {
    /// Path parameter overrides (e.g., {"id": "123"})
    #[serde(default)]
    pub path_params: HashMap<String, String>,

    /// Query parameter overrides (e.g., {"limit": "50"})
    #[serde(default)]
    pub query_params: HashMap<String, String>,

    /// Header overrides (e.g., {"X-Custom": "value"})
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Request body override (JSON value)
    #[serde(default)]
    pub body: Option<Value>,
}

impl ParameterOverrides {
    /// Load parameter overrides from a JSON or YAML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            BenchError::Other(format!("Failed to read params file '{}': {}", path.display(), e))
        })?;

        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match extension.to_lowercase().as_str() {
            "json" => serde_json::from_str(&content)
                .map_err(|e| BenchError::Other(format!("Failed to parse JSON params file: {}", e))),
            "yaml" | "yml" => serde_yaml::from_str(&content)
                .map_err(|e| BenchError::Other(format!("Failed to parse YAML params file: {}", e))),
            _ => {
                // Try JSON first, then YAML
                serde_json::from_str(&content)
                    .or_else(|_| serde_yaml::from_str(&content))
                    .map_err(|e| {
                        BenchError::Other(format!(
                            "Failed to parse params file (tried JSON and YAML): {}",
                            e
                        ))
                    })
            }
        }
    }

    /// Get the effective overrides for an operation
    ///
    /// Merges default overrides with operation-specific overrides.
    /// Operation-specific values take precedence over defaults.
    pub fn get_for_operation(
        &self,
        operation_id: Option<&str>,
        method: &str,
        path: &str,
    ) -> OperationOverrides {
        let mut result = self.defaults.clone();

        // Try to find operation-specific overrides
        let op_overrides = operation_id
            .and_then(|id| self.operations.get(id))
            .or_else(|| {
                // Try "METHOD /path" format
                let key = format!("{} {}", method.to_uppercase(), path);
                self.operations.get(&key)
            })
            .or_else(|| {
                // Try just the path
                self.operations.get(path)
            });

        if let Some(overrides) = op_overrides {
            // Merge operation overrides into result (operation takes precedence)
            for (k, v) in &overrides.path_params {
                result.path_params.insert(k.clone(), v.clone());
            }
            for (k, v) in &overrides.query_params {
                result.query_params.insert(k.clone(), v.clone());
            }
            for (k, v) in &overrides.headers {
                result.headers.insert(k.clone(), v.clone());
            }
            if overrides.body.is_some() {
                result.body = overrides.body.clone();
            }
        }

        result
    }

    /// Check if this configuration is empty (no overrides defined)
    pub fn is_empty(&self) -> bool {
        self.defaults.is_empty() && self.operations.is_empty()
    }
}

impl OperationOverrides {
    /// Check if this override set is empty
    pub fn is_empty(&self) -> bool {
        self.path_params.is_empty()
            && self.query_params.is_empty()
            && self.headers.is_empty()
            && self.body.is_none()
    }

    /// Get a path parameter value if overridden
    pub fn get_path_param(&self, name: &str) -> Option<&String> {
        self.path_params.get(name)
    }

    /// Get a query parameter value if overridden
    pub fn get_query_param(&self, name: &str) -> Option<&String> {
        self.query_params.get(name)
    }

    /// Get a header value if overridden
    pub fn get_header(&self, name: &str) -> Option<&String> {
        self.headers.get(name)
    }

    /// Get the body override if present
    pub fn get_body(&self) -> Option<&Value> {
        self.body.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_json_overrides() {
        let json = r#"{
            "defaults": {
                "path_params": {
                    "id": "default-id"
                },
                "query_params": {
                    "limit": "50"
                }
            },
            "operations": {
                "getUser": {
                    "path_params": {
                        "id": "user-123"
                    }
                },
                "POST /users": {
                    "body": {
                        "name": "Test User"
                    }
                }
            }
        }"#;

        let overrides: ParameterOverrides = serde_json::from_str(json).unwrap();

        // Check defaults
        assert_eq!(overrides.defaults.path_params.get("id"), Some(&"default-id".to_string()));
        assert_eq!(overrides.defaults.query_params.get("limit"), Some(&"50".to_string()));

        // Check operation-specific
        let get_user = overrides.operations.get("getUser").unwrap();
        assert_eq!(get_user.path_params.get("id"), Some(&"user-123".to_string()));

        let post_users = overrides.operations.get("POST /users").unwrap();
        assert!(post_users.body.is_some());
    }

    #[test]
    fn test_get_for_operation_with_defaults() {
        let overrides = ParameterOverrides {
            defaults: OperationOverrides {
                path_params: [("id".to_string(), "default-id".to_string())].into_iter().collect(),
                query_params: [("limit".to_string(), "10".to_string())].into_iter().collect(),
                ..Default::default()
            },
            operations: HashMap::new(),
        };

        let result = overrides.get_for_operation(Some("unknownOp"), "GET", "/users");
        assert_eq!(result.path_params.get("id"), Some(&"default-id".to_string()));
        assert_eq!(result.query_params.get("limit"), Some(&"10".to_string()));
    }

    #[test]
    fn test_get_for_operation_with_override() {
        let mut operations = HashMap::new();
        operations.insert(
            "getUser".to_string(),
            OperationOverrides {
                path_params: [("id".to_string(), "user-456".to_string())].into_iter().collect(),
                ..Default::default()
            },
        );

        let overrides = ParameterOverrides {
            defaults: OperationOverrides {
                path_params: [("id".to_string(), "default-id".to_string())].into_iter().collect(),
                ..Default::default()
            },
            operations,
        };

        // Operation-specific should override default
        let result = overrides.get_for_operation(Some("getUser"), "GET", "/users/{id}");
        assert_eq!(result.path_params.get("id"), Some(&"user-456".to_string()));
    }

    #[test]
    fn test_get_for_operation_by_method_path() {
        let mut operations = HashMap::new();
        operations.insert(
            "POST /virtualservice".to_string(),
            OperationOverrides {
                body: Some(json!({"name": "my-service"})),
                ..Default::default()
            },
        );

        let overrides = ParameterOverrides {
            defaults: OperationOverrides::default(),
            operations,
        };

        let result = overrides.get_for_operation(None, "POST", "/virtualservice");
        assert!(result.body.is_some());
        assert_eq!(result.body.unwrap()["name"], "my-service");
    }

    #[test]
    fn test_empty_overrides() {
        let overrides = ParameterOverrides::default();
        assert!(overrides.is_empty());

        let result = overrides.get_for_operation(Some("anyOp"), "GET", "/any");
        assert!(result.is_empty());
    }
}
