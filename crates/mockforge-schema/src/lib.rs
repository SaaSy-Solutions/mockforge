//! JSON Schema generation for MockForge configuration files
//!
//! This crate provides functionality to generate JSON Schema definitions
//! from MockForge's configuration structs, enabling IDE autocomplete and
//! validation for `mockforge.yaml` and `mockforge.toml` files.

use schemars::schema_for;
use serde_json;

/// Generate JSON Schema for MockForge ServerConfig
///
/// This function generates a complete JSON Schema that can be used by
/// IDEs and editors to provide autocomplete, validation, and documentation
/// for MockForge configuration files.
///
/// # Returns
///
/// A JSON Schema object as a serde_json::Value
///
/// # Example
///
/// ```rust
/// use mockforge_schema::generate_config_schema;
/// use serde_json;
///
/// let schema = generate_config_schema();
/// let schema_json = serde_json::to_string_pretty(&schema).unwrap();
/// println!("{}", schema_json);
/// ```
pub fn generate_config_schema() -> serde_json::Value {
    // Generate schema from ServerConfig
    // ServerConfig needs to have JsonSchema derive (via feature flag)
    let schema = schema_for!(mockforge_core::ServerConfig);
    serde_json::to_value(schema).expect("Failed to serialize schema")
}

/// Generate JSON Schema and return as a formatted JSON string
///
/// # Returns
///
/// A pretty-printed JSON string containing the schema
pub fn generate_config_schema_json() -> String {
    let schema = generate_config_schema();
    serde_json::to_string_pretty(&schema).expect("Failed to format schema as JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_generation() {
        let schema = generate_config_schema();
        assert!(schema.is_object());

        // Verify schema has required fields
        let obj = schema.as_object().unwrap();
        assert!(obj.contains_key("$schema") || obj.contains_key("type"));
    }

    #[test]
    fn test_schema_json_formatting() {
        let json = generate_config_schema_json();
        assert!(!json.is_empty());

        // Verify it's valid JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        assert!(parsed.is_ok());
    }
}
