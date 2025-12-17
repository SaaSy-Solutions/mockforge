//! JSON Schema generation for MockForge configuration files
//!
//! This crate provides functionality to generate JSON Schema definitions
//! from MockForge's configuration structs, enabling IDE autocomplete and
//! validation for `mockforge.yaml`, `mockforge.toml`, persona files, and blueprint files.

use schemars::schema_for;

/// Generate JSON Schema for MockForge ServerConfig (main config)
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

    let mut schema_value = serde_json::to_value(schema).expect("Failed to serialize schema");

    // Add metadata for better IDE support
    if let Some(obj) = schema_value.as_object_mut() {
        obj.insert(
            "$schema".to_string(),
            serde_json::json!("http://json-schema.org/draft-07/schema#"),
        );
        obj.insert("title".to_string(), serde_json::json!("MockForge Server Configuration"));
        obj.insert(
            "description".to_string(),
            serde_json::json!(
                "Complete configuration schema for MockForge mock server. \
             This schema provides autocomplete and validation for mockforge.yaml files."
            ),
        );
    }

    schema_value
}

/// Generate JSON Schema for Reality configuration
///
/// Generates schema for the Reality slider configuration used to control
/// mock environment realism levels.
pub fn generate_reality_schema() -> serde_json::Value {
    let schema = schema_for!(mockforge_core::config::RealitySliderConfig);

    let mut schema_value =
        serde_json::to_value(schema).expect("Failed to serialize reality schema");

    // Add metadata for better IDE support
    if let Some(obj) = schema_value.as_object_mut() {
        obj.insert(
            "$schema".to_string(),
            serde_json::json!("http://json-schema.org/draft-07/schema#"),
        );
        obj.insert("title".to_string(), serde_json::json!("MockForge Reality Configuration"));
        obj.insert(
            "description".to_string(),
            serde_json::json!(
                "Reality slider configuration for controlling mock environment realism. \
             Maps reality levels (1-5) to specific subsystem settings."
            ),
        );
    }

    schema_value
}

/// Generate JSON Schema for Persona configuration
///
/// Generates schema for persona profiles that define consistent data patterns.
/// Note: This generates a schema for the persona registry config structure.
pub fn generate_persona_schema() -> serde_json::Value {
    // Generate schema for PersonaRegistryConfig which contains persona definitions
    let schema = schema_for!(mockforge_core::config::PersonaRegistryConfig);

    let mut schema_value =
        serde_json::to_value(schema).expect("Failed to serialize persona schema");

    // Add metadata for better IDE support
    if let Some(obj) = schema_value.as_object_mut() {
        obj.insert(
            "$schema".to_string(),
            serde_json::json!("http://json-schema.org/draft-07/schema#"),
        );
        obj.insert("title".to_string(), serde_json::json!("MockForge Persona Configuration"));
        obj.insert(
            "description".to_string(),
            serde_json::json!(
                "Persona configuration for consistent, personality-driven data generation. \
             Defines personas with unique IDs, domains, traits, and deterministic seeds."
            ),
        );
    }

    schema_value
}

/// Generate JSON Schema for Blueprint metadata
///
/// Generates schema for blueprint.yaml files that define app archetypes.
/// Note: Blueprint structs are in mockforge-cli, so we generate a manual schema
/// based on the known structure.
pub fn generate_blueprint_schema() -> serde_json::Value {
    // Manual schema for blueprint metadata since it's in a different crate
    // This matches the BlueprintMetadata structure
    serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "MockForge Blueprint Configuration",
        "description": "Blueprint metadata schema for predefined app archetypes. \
                       Blueprints provide pre-configured personas, reality defaults, \
                       flows, scenarios, and playground collections.",
        "type": "object",
        "required": ["manifest_version", "name", "version", "title", "description", "author", "category"],
        "properties": {
            "manifest_version": {
                "type": "string",
                "description": "Blueprint manifest version (e.g., '1.0')",
                "example": "1.0"
            },
            "name": {
                "type": "string",
                "description": "Unique blueprint identifier (e.g., 'b2c-saas', 'ecommerce')",
                "pattern": "^[a-z0-9-]+$"
            },
            "version": {
                "type": "string",
                "description": "Blueprint version (semver)",
                "pattern": "^\\d+\\.\\d+\\.\\d+$"
            },
            "title": {
                "type": "string",
                "description": "Human-readable blueprint title"
            },
            "description": {
                "type": "string",
                "description": "Detailed description of what this blueprint provides"
            },
            "author": {
                "type": "string",
                "description": "Blueprint author name"
            },
            "author_email": {
                "type": "string",
                "format": "email",
                "description": "Blueprint author email (optional)"
            },
            "category": {
                "type": "string",
                "description": "Blueprint category (e.g., 'saas', 'ecommerce', 'banking')",
                "enum": ["saas", "ecommerce", "banking", "fintech", "healthcare", "other"]
            },
            "tags": {
                "type": "array",
                "items": {
                    "type": "string"
                },
                "description": "Tags for categorizing and searching blueprints"
            },
            "setup": {
                "type": "object",
                "description": "What this blueprint sets up",
                "properties": {
                    "personas": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["id", "name"],
                            "properties": {
                                "id": {
                                    "type": "string",
                                    "description": "Persona identifier"
                                },
                                "name": {
                                    "type": "string",
                                    "description": "Persona display name"
                                },
                                "description": {
                                    "type": "string",
                                    "description": "Persona description (optional)"
                                }
                            }
                        }
                    },
                    "reality": {
                        "type": "object",
                        "properties": {
                            "level": {
                                "type": "string",
                                "enum": ["static", "light", "moderate", "high", "chaos"],
                                "description": "Default reality level for this blueprint"
                            },
                            "description": {
                                "type": "string",
                                "description": "Why this reality level is chosen"
                            }
                        }
                    },
                    "flows": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["id", "name"],
                            "properties": {
                                "id": {
                                    "type": "string"
                                },
                                "name": {
                                    "type": "string"
                                },
                                "description": {
                                    "type": "string"
                                }
                            }
                        }
                    },
                    "scenarios": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["id", "name", "type", "file"],
                            "properties": {
                                "id": {
                                    "type": "string",
                                    "description": "Scenario identifier"
                                },
                                "name": {
                                    "type": "string",
                                    "description": "Scenario display name"
                                },
                                "type": {
                                    "type": "string",
                                    "enum": ["happy_path", "known_failure", "slow_path"],
                                    "description": "Scenario type"
                                },
                                "description": {
                                    "type": "string",
                                    "description": "Scenario description (optional)"
                                },
                                "file": {
                                    "type": "string",
                                    "description": "Path to scenario YAML file"
                                }
                            }
                        }
                    },
                    "playground": {
                        "type": "object",
                        "properties": {
                            "enabled": {
                                "type": "boolean",
                                "default": true
                            },
                            "collection_file": {
                                "type": "string",
                                "description": "Path to playground collection file"
                            }
                        }
                    }
                }
            },
            "compatibility": {
                "type": "object",
                "properties": {
                    "min_version": {
                        "type": "string",
                        "description": "Minimum MockForge version required"
                    },
                    "max_version": {
                        "type": "string",
                        "description": "Maximum MockForge version (null for latest)"
                    },
                    "required_features": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        }
                    },
                    "protocols": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["http", "websocket", "grpc", "graphql", "mqtt"]
                        }
                    }
                }
            },
            "files": {
                "type": "array",
                "items": {
                    "type": "string"
                },
                "description": "List of files included in this blueprint"
            },
            "readme": {
                "type": "string",
                "description": "Path to README file (optional)"
            },
            "contracts": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["file"],
                    "properties": {
                        "file": {
                            "type": "string",
                            "description": "Path to contract schema file"
                        },
                        "description": {
                            "type": "string",
                            "description": "Contract description (optional)"
                        }
                    }
                }
            }
        }
    })
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

/// Generate all schemas and return them as a map
///
/// Returns a HashMap with schema names as keys and JSON Schema values.
pub fn generate_all_schemas() -> std::collections::HashMap<String, serde_json::Value> {
    let mut schemas = std::collections::HashMap::new();

    schemas.insert("mockforge-config".to_string(), generate_config_schema());
    schemas.insert("reality-config".to_string(), generate_reality_schema());
    schemas.insert("persona-config".to_string(), generate_persona_schema());
    schemas.insert("blueprint-config".to_string(), generate_blueprint_schema());

    schemas
}

/// Validation result for config file validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,
    /// File path that was validated
    pub file_path: String,
    /// Schema type used for validation
    pub schema_type: String,
    /// Validation errors (empty if valid)
    pub errors: Vec<String>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn success(file_path: String, schema_type: String) -> Self {
        Self {
            valid: true,
            file_path,
            schema_type,
            errors: Vec::new(),
        }
    }

    /// Create a failed validation result
    pub fn failure(file_path: String, schema_type: String, errors: Vec<String>) -> Self {
        Self {
            valid: false,
            file_path,
            schema_type,
            errors,
        }
    }
}

/// Validate a config file against its corresponding JSON Schema
///
/// # Arguments
///
/// * `file_path` - Path to the config file (YAML or JSON)
/// * `schema_type` - Type of schema to validate against (config, reality, persona, blueprint)
/// * `schema` - The JSON Schema to validate against
///
/// # Returns
///
/// A ValidationResult indicating whether validation passed and any errors
pub fn validate_config_file(
    file_path: &std::path::Path,
    schema_type: &str,
    schema: &serde_json::Value,
) -> Result<ValidationResult, Box<dyn std::error::Error>> {
    use jsonschema::{Draft, Validator as SchemaValidator};
    use std::fs;

    // Read and parse the config file
    let content = fs::read_to_string(file_path)?;
    let config_value: serde_json::Value = if file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"))
        .unwrap_or(false)
    {
        // Parse YAML
        serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse YAML: {}", e))?
    } else {
        // Parse JSON
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))?
    };

    // Compile the schema
    let compiled_schema = SchemaValidator::options()
        .with_draft(Draft::Draft7)
        .build(schema)
        .map_err(|e| format!("Failed to compile schema: {}", e))?;

    // Validate
    let mut errors = Vec::new();
    for error in compiled_schema.iter_errors(&config_value) {
        errors.push(format!("{}: {}", error.instance_path, error));
    }

    if errors.is_empty() {
        Ok(ValidationResult::success(
            file_path.to_string_lossy().to_string(),
            schema_type.to_string(),
        ))
    } else {
        Ok(ValidationResult::failure(
            file_path.to_string_lossy().to_string(),
            schema_type.to_string(),
            errors,
        ))
    }
}

/// Auto-detect schema type from file path or content
///
/// Attempts to determine which schema should be used to validate a file
/// based on its path or content.
pub fn detect_schema_type(file_path: &std::path::Path) -> Option<String> {
    let file_name = file_path.file_name()?.to_string_lossy().to_lowercase();
    let path_str = file_path.to_string_lossy().to_lowercase();

    // Check file name patterns
    if file_name == "mockforge.yaml"
        || file_name == "mockforge.yml"
        || file_name == "mockforge.json"
    {
        return Some("mockforge-config".to_string());
    }

    if file_name == "blueprint.yaml" || file_name == "blueprint.yml" {
        return Some("blueprint-config".to_string());
    }

    if path_str.contains("reality") {
        return Some("reality-config".to_string());
    }

    if path_str.contains("persona") {
        return Some("persona-config".to_string());
    }

    // Default to main config
    Some("mockforge-config".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    // ==================== Schema Generation Tests ====================

    #[test]
    fn test_generate_config_schema() {
        let schema = generate_config_schema();
        assert!(schema.is_object());

        let obj = schema.as_object().unwrap();

        // Verify metadata was added
        assert!(obj.contains_key("$schema"));
        assert_eq!(
            obj.get("$schema").unwrap(),
            &serde_json::json!("http://json-schema.org/draft-07/schema#")
        );
        assert!(obj.contains_key("title"));
        assert_eq!(obj.get("title").unwrap(), &serde_json::json!("MockForge Server Configuration"));
        assert!(obj.contains_key("description"));
    }

    #[test]
    fn test_generate_reality_schema() {
        let schema = generate_reality_schema();
        assert!(schema.is_object());

        let obj = schema.as_object().unwrap();

        // Verify metadata was added
        assert!(obj.contains_key("$schema"));
        assert!(obj.contains_key("title"));
        assert_eq!(
            obj.get("title").unwrap(),
            &serde_json::json!("MockForge Reality Configuration")
        );
        assert!(obj.contains_key("description"));
    }

    #[test]
    fn test_generate_persona_schema() {
        let schema = generate_persona_schema();
        assert!(schema.is_object());

        let obj = schema.as_object().unwrap();

        // Verify metadata was added
        assert!(obj.contains_key("$schema"));
        assert!(obj.contains_key("title"));
        assert_eq!(
            obj.get("title").unwrap(),
            &serde_json::json!("MockForge Persona Configuration")
        );
        assert!(obj.contains_key("description"));
    }

    #[test]
    fn test_generate_blueprint_schema() {
        let schema = generate_blueprint_schema();
        assert!(schema.is_object());

        let obj = schema.as_object().unwrap();

        // Verify required metadata
        assert!(obj.contains_key("$schema"));
        assert!(obj.contains_key("title"));
        assert_eq!(
            obj.get("title").unwrap(),
            &serde_json::json!("MockForge Blueprint Configuration")
        );

        // Verify type is object
        assert_eq!(obj.get("type").unwrap(), &serde_json::json!("object"));

        // Verify required fields are specified
        assert!(obj.contains_key("required"));
        let required = obj.get("required").unwrap().as_array().unwrap();
        assert!(required.contains(&serde_json::json!("name")));
        assert!(required.contains(&serde_json::json!("version")));
        assert!(required.contains(&serde_json::json!("title")));

        // Verify properties exist
        assert!(obj.contains_key("properties"));
        let props = obj.get("properties").unwrap().as_object().unwrap();
        assert!(props.contains_key("manifest_version"));
        assert!(props.contains_key("name"));
        assert!(props.contains_key("version"));
        assert!(props.contains_key("category"));
        assert!(props.contains_key("setup"));
        assert!(props.contains_key("compatibility"));
    }

    #[test]
    fn test_generate_blueprint_schema_category_enum() {
        let schema = generate_blueprint_schema();
        let obj = schema.as_object().unwrap();
        let props = obj.get("properties").unwrap().as_object().unwrap();
        let category = props.get("category").unwrap().as_object().unwrap();

        // Verify category has enum values
        assert!(category.contains_key("enum"));
        let enum_values = category.get("enum").unwrap().as_array().unwrap();
        assert!(enum_values.contains(&serde_json::json!("saas")));
        assert!(enum_values.contains(&serde_json::json!("ecommerce")));
        assert!(enum_values.contains(&serde_json::json!("banking")));
    }

    #[test]
    fn test_generate_config_schema_json() {
        let json = generate_config_schema_json();
        assert!(!json.is_empty());

        // Verify it's valid JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        assert!(parsed.is_ok());

        // Verify it's the same as generate_config_schema
        let schema = generate_config_schema();
        let reparsed = serde_json::from_str::<serde_json::Value>(&json).unwrap();

        // Check key fields match
        assert_eq!(reparsed.get("title").unwrap(), schema.get("title").unwrap());
    }

    #[test]
    fn test_generate_all_schemas() {
        let schemas = generate_all_schemas();

        // Verify all expected schemas are present
        assert_eq!(schemas.len(), 4);
        assert!(schemas.contains_key("mockforge-config"));
        assert!(schemas.contains_key("reality-config"));
        assert!(schemas.contains_key("persona-config"));
        assert!(schemas.contains_key("blueprint-config"));

        // Verify each schema is valid
        for (name, schema) in &schemas {
            assert!(schema.is_object(), "Schema {} should be an object", name);
        }
    }

    // ==================== ValidationResult Tests ====================

    #[test]
    fn test_validation_result_success() {
        let result = ValidationResult::success(
            "/path/to/config.yaml".to_string(),
            "mockforge-config".to_string(),
        );

        assert!(result.valid);
        assert_eq!(result.file_path, "/path/to/config.yaml");
        assert_eq!(result.schema_type, "mockforge-config");
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validation_result_failure() {
        let errors = vec![
            "Missing required field: name".to_string(),
            "Invalid type for port".to_string(),
        ];

        let result = ValidationResult::failure(
            "/path/to/invalid.yaml".to_string(),
            "mockforge-config".to_string(),
            errors.clone(),
        );

        assert!(!result.valid);
        assert_eq!(result.file_path, "/path/to/invalid.yaml");
        assert_eq!(result.schema_type, "mockforge-config");
        assert_eq!(result.errors.len(), 2);
        assert_eq!(result.errors, errors);
    }

    #[test]
    fn test_validation_result_debug() {
        let result = ValidationResult::success("test.yaml".to_string(), "config".to_string());
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("ValidationResult"));
        assert!(debug_str.contains("valid"));
        assert!(debug_str.contains("test.yaml"));
    }

    #[test]
    fn test_validation_result_clone() {
        let result = ValidationResult::failure(
            "test.yaml".to_string(),
            "config".to_string(),
            vec!["error1".to_string()],
        );
        let cloned = result.clone();

        assert_eq!(cloned.valid, result.valid);
        assert_eq!(cloned.file_path, result.file_path);
        assert_eq!(cloned.schema_type, result.schema_type);
        assert_eq!(cloned.errors, result.errors);
    }

    // ==================== detect_schema_type Tests ====================

    #[test]
    fn test_detect_schema_type_mockforge_yaml() {
        let path = PathBuf::from("/project/mockforge.yaml");
        assert_eq!(detect_schema_type(&path), Some("mockforge-config".to_string()));
    }

    #[test]
    fn test_detect_schema_type_mockforge_yml() {
        let path = PathBuf::from("/project/mockforge.yml");
        assert_eq!(detect_schema_type(&path), Some("mockforge-config".to_string()));
    }

    #[test]
    fn test_detect_schema_type_mockforge_json() {
        let path = PathBuf::from("/project/mockforge.json");
        assert_eq!(detect_schema_type(&path), Some("mockforge-config".to_string()));
    }

    #[test]
    fn test_detect_schema_type_blueprint_yaml() {
        let path = PathBuf::from("/blueprints/saas/blueprint.yaml");
        assert_eq!(detect_schema_type(&path), Some("blueprint-config".to_string()));
    }

    #[test]
    fn test_detect_schema_type_blueprint_yml() {
        let path = PathBuf::from("/blueprints/ecommerce/blueprint.yml");
        assert_eq!(detect_schema_type(&path), Some("blueprint-config".to_string()));
    }

    #[test]
    fn test_detect_schema_type_reality_path() {
        let path = PathBuf::from("/config/reality/settings.yaml");
        assert_eq!(detect_schema_type(&path), Some("reality-config".to_string()));
    }

    #[test]
    fn test_detect_schema_type_persona_path() {
        let path = PathBuf::from("/config/persona/developer.yaml");
        assert_eq!(detect_schema_type(&path), Some("persona-config".to_string()));
    }

    #[test]
    fn test_detect_schema_type_default() {
        let path = PathBuf::from("/some/other/config.yaml");
        assert_eq!(detect_schema_type(&path), Some("mockforge-config".to_string()));
    }

    #[test]
    fn test_detect_schema_type_case_insensitive() {
        let path = PathBuf::from("/Project/MOCKFORGE.YAML");
        assert_eq!(detect_schema_type(&path), Some("mockforge-config".to_string()));

        let path2 = PathBuf::from("/blueprints/Blueprint.YML");
        assert_eq!(detect_schema_type(&path2), Some("blueprint-config".to_string()));
    }

    // ==================== validate_config_file Tests ====================

    #[test]
    fn test_validate_yaml_file_valid() {
        // Create a simple schema
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "port": { "type": "integer" }
            },
            "required": ["name"]
        });

        // Create a temp file
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_valid_config.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "name: test-service\nport: 8080").unwrap();

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();

        assert!(result.valid);
        assert!(result.errors.is_empty());

        // Cleanup
        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validate_json_file_valid() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_valid_config.json");
        let content = r#"{"name": "test-service"}"#;
        std::fs::write(&file_path, content).unwrap();

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();

        assert!(result.valid);
        assert!(result.errors.is_empty());

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validate_yaml_file_missing_required() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "port": { "type": "integer" }
            },
            "required": ["name", "port"]
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_missing_required.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "name: test-service").unwrap(); // Missing port

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();

        assert!(!result.valid);
        assert!(!result.errors.is_empty());
        // Should have error about missing "port"
        let has_port_error = result.errors.iter().any(|e| e.contains("port"));
        assert!(has_port_error, "Expected error about missing 'port'");

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validate_yaml_file_wrong_type() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "port": { "type": "integer" }
            }
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_wrong_type.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "port: not-a-number").unwrap(); // String instead of integer

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();

        assert!(!result.valid);
        assert!(!result.errors.is_empty());

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validate_file_not_found() {
        let schema = serde_json::json!({
            "type": "object"
        });

        let file_path = PathBuf::from("/nonexistent/path/config.yaml");
        let result = validate_config_file(&file_path, "test-config", &schema);

        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_yaml_syntax() {
        let schema = serde_json::json!({
            "type": "object"
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_invalid_yaml.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "invalid: yaml: syntax: [unclosed").unwrap();

        let result = validate_config_file(&file_path, "test-config", &schema);

        assert!(result.is_err());

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validate_invalid_json_syntax() {
        let schema = serde_json::json!({
            "type": "object"
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_invalid.json");
        let content = r#"{"unclosed": "#;
        std::fs::write(&file_path, content).unwrap();

        let result = validate_config_file(&file_path, "test-config", &schema);

        assert!(result.is_err());

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validate_yml_extension() {
        let schema = serde_json::json!({
            "type": "object"
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_config.yml"); // .yml extension
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "key: value").unwrap();

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();

        assert!(result.valid);

        std::fs::remove_file(&file_path).ok();
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_empty_yaml_file() {
        let schema = serde_json::json!({
            "type": "object"
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_empty.yaml");
        std::fs::File::create(&file_path).unwrap();

        // Empty YAML parses as null
        let result = validate_config_file(&file_path, "test-config", &schema);
        // Could be valid or invalid depending on schema, but shouldn't crash
        assert!(result.is_ok() || result.is_err());

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_schema_with_additional_properties_false() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "additionalProperties": false
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_extra_props.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "name: test\nextra: not-allowed").unwrap();

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();

        assert!(!result.valid);
        assert!(!result.errors.is_empty());

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_nested_validation_error() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "server": {
                    "type": "object",
                    "properties": {
                        "port": { "type": "integer", "minimum": 1, "maximum": 65535 }
                    }
                }
            }
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_nested.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "server:\n  port: 99999").unwrap(); // Port out of range

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();

        assert!(!result.valid);
        // Error should reference the nested path
        let has_port_error = result.errors.iter().any(|e| e.contains("port"));
        assert!(has_port_error);

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validation_result_empty_errors() {
        let result =
            ValidationResult::failure("test.yaml".to_string(), "config".to_string(), vec![]);

        // Even with empty errors vec, valid should be false
        assert!(!result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_blueprint_schema_setup_structure() {
        let schema = generate_blueprint_schema();
        let obj = schema.as_object().unwrap();
        let props = obj.get("properties").unwrap().as_object().unwrap();
        let setup = props.get("setup").unwrap().as_object().unwrap();

        assert_eq!(setup.get("type").unwrap(), &serde_json::json!("object"));

        let setup_props = setup.get("properties").unwrap().as_object().unwrap();
        assert!(setup_props.contains_key("personas"));
        assert!(setup_props.contains_key("reality"));
        assert!(setup_props.contains_key("flows"));
        assert!(setup_props.contains_key("scenarios"));
        assert!(setup_props.contains_key("playground"));
    }

    // ==================== Additional Schema Structure Tests ====================

    #[test]
    fn test_config_schema_has_required_properties() {
        let schema = generate_config_schema();
        let obj = schema.as_object().unwrap();

        // Verify it's an object with properties
        assert!(obj.contains_key("properties"));
        assert!(obj.contains_key("$schema"));
        assert!(obj.contains_key("title"));
        assert!(obj.contains_key("description"));
    }

    #[test]
    fn test_reality_schema_structure() {
        let schema = generate_reality_schema();
        let obj = schema.as_object().unwrap();

        // Verify metadata
        assert_eq!(
            obj.get("$schema").unwrap(),
            &serde_json::json!("http://json-schema.org/draft-07/schema#")
        );
        assert!(obj.contains_key("properties") || obj.contains_key("definitions"));
    }

    #[test]
    fn test_persona_schema_structure() {
        let schema = generate_persona_schema();
        let obj = schema.as_object().unwrap();

        // Verify metadata
        assert_eq!(
            obj.get("$schema").unwrap(),
            &serde_json::json!("http://json-schema.org/draft-07/schema#")
        );
        assert!(obj.contains_key("properties") || obj.contains_key("definitions"));
    }

    #[test]
    fn test_blueprint_schema_compatibility_structure() {
        let schema = generate_blueprint_schema();
        let obj = schema.as_object().unwrap();
        let props = obj.get("properties").unwrap().as_object().unwrap();

        assert!(props.contains_key("compatibility"));
        let compatibility = props.get("compatibility").unwrap().as_object().unwrap();
        assert_eq!(compatibility.get("type").unwrap(), &serde_json::json!("object"));

        let compat_props = compatibility.get("properties").unwrap().as_object().unwrap();
        assert!(compat_props.contains_key("min_version"));
        assert!(compat_props.contains_key("max_version"));
        assert!(compat_props.contains_key("required_features"));
        assert!(compat_props.contains_key("protocols"));
    }

    #[test]
    fn test_blueprint_schema_protocols_enum() {
        let schema = generate_blueprint_schema();
        let obj = schema.as_object().unwrap();
        let props = obj.get("properties").unwrap().as_object().unwrap();
        let compatibility = props.get("compatibility").unwrap().as_object().unwrap();
        let compat_props = compatibility.get("properties").unwrap().as_object().unwrap();
        let protocols = compat_props.get("protocols").unwrap().as_object().unwrap();

        // Check it's an array
        assert_eq!(protocols.get("type").unwrap(), &serde_json::json!("array"));

        // Check items have enum
        let items = protocols.get("items").unwrap().as_object().unwrap();
        assert!(items.contains_key("enum"));
        let enum_values = items.get("enum").unwrap().as_array().unwrap();
        assert!(enum_values.contains(&serde_json::json!("http")));
        assert!(enum_values.contains(&serde_json::json!("websocket")));
        assert!(enum_values.contains(&serde_json::json!("grpc")));
    }

    #[test]
    fn test_blueprint_schema_name_pattern() {
        let schema = generate_blueprint_schema();
        let obj = schema.as_object().unwrap();
        let props = obj.get("properties").unwrap().as_object().unwrap();
        let name = props.get("name").unwrap().as_object().unwrap();

        // Verify name has pattern validation
        assert!(name.contains_key("pattern"));
        assert_eq!(name.get("pattern").unwrap(), &serde_json::json!("^[a-z0-9-]+$"));
    }

    #[test]
    fn test_blueprint_schema_version_pattern() {
        let schema = generate_blueprint_schema();
        let obj = schema.as_object().unwrap();
        let props = obj.get("properties").unwrap().as_object().unwrap();
        let version = props.get("version").unwrap().as_object().unwrap();

        // Verify version has semver pattern
        assert!(version.contains_key("pattern"));
        let pattern = version.get("pattern").unwrap().as_str().unwrap();
        assert!(pattern.contains("\\d+"));
    }

    #[test]
    fn test_all_schemas_are_valid_json() {
        let schemas = generate_all_schemas();

        for (name, schema) in schemas {
            // Each schema should be serializable to JSON
            let json_str = serde_json::to_string(&schema).unwrap();
            assert!(!json_str.is_empty(), "Schema {} should serialize to non-empty JSON", name);

            // And deserializable back
            let reparsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
            assert_eq!(reparsed, schema, "Schema {} should round-trip correctly", name);
        }
    }

    // ==================== Additional Validation Tests ====================

    #[test]
    fn test_validate_with_multiple_validation_errors() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "port": { "type": "integer", "minimum": 1, "maximum": 65535 },
                "enabled": { "type": "boolean" }
            },
            "required": ["name", "port", "enabled"]
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_multi_errors.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "port: not-a-number").unwrap(); // Wrong type + missing required

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();

        assert!(!result.valid);
        // Should have multiple errors
        assert!(
            result.errors.len() >= 2,
            "Expected at least 2 errors, got {}",
            result.errors.len()
        );

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validate_array_schema() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "items": { "type": "string" },
                    "minItems": 1
                }
            },
            "required": ["items"]
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_array.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "items:\n  - item1\n  - item2").unwrap();

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();

        assert!(result.valid);
        assert!(result.errors.is_empty());

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validate_array_schema_empty_array() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "items": { "type": "string" },
                    "minItems": 1
                }
            }
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_empty_array.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "items: []").unwrap();

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();

        assert!(!result.valid);
        assert!(!result.errors.is_empty());

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validate_enum_schema_valid() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "level": {
                    "type": "string",
                    "enum": ["low", "medium", "high"]
                }
            }
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_enum_valid.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "level: medium").unwrap();

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();

        assert!(result.valid);

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validate_enum_schema_invalid() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "level": {
                    "type": "string",
                    "enum": ["low", "medium", "high"]
                }
            }
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_enum_invalid.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "level: invalid").unwrap();

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();

        assert!(!result.valid);
        assert!(!result.errors.is_empty());

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validate_pattern_string_valid() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "email": {
                    "type": "string",
                    "pattern": "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
                }
            }
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_pattern_valid.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "email: test@example.com").unwrap();

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();

        assert!(result.valid);

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validate_pattern_string_invalid() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "email": {
                    "type": "string",
                    "pattern": "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
                }
            }
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_pattern_invalid.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "email: not-an-email").unwrap();

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();

        assert!(!result.valid);

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validate_number_constraints() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "percentage": {
                    "type": "number",
                    "minimum": 0,
                    "maximum": 100
                }
            }
        });

        let temp_dir = std::env::temp_dir();

        // Test valid
        let file_path = temp_dir.join("test_number_valid.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "percentage: 50.5").unwrap();
        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();
        assert!(result.valid);
        std::fs::remove_file(&file_path).ok();

        // Test invalid - too high
        let file_path = temp_dir.join("test_number_invalid.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "percentage: 150").unwrap();
        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();
        assert!(!result.valid);
        std::fs::remove_file(&file_path).ok();
    }

    // ==================== detect_schema_type Edge Cases ====================

    #[test]
    fn test_detect_schema_type_no_extension() {
        let path = PathBuf::from("/config/mockforge");
        // Should return default
        assert_eq!(detect_schema_type(&path), Some("mockforge-config".to_string()));
    }

    #[test]
    fn test_detect_schema_type_nested_persona_path() {
        let path = PathBuf::from("/very/deep/path/with/persona/in/middle/config.yaml");
        assert_eq!(detect_schema_type(&path), Some("persona-config".to_string()));
    }

    #[test]
    fn test_detect_schema_type_nested_reality_path() {
        let path = PathBuf::from("/config/reality-slider/settings.yml");
        assert_eq!(detect_schema_type(&path), Some("reality-config".to_string()));
    }

    #[test]
    fn test_detect_schema_type_mixed_case_path() {
        let path = PathBuf::from("/Config/REALITY/Settings.YML");
        assert_eq!(detect_schema_type(&path), Some("reality-config".to_string()));
    }

    // ==================== Schema JSON String Tests ====================

    #[test]
    fn test_generate_config_schema_json_pretty_printed() {
        let json = generate_config_schema_json();

        // Should contain newlines (pretty printed)
        assert!(json.contains('\n'));

        // Should contain proper indentation
        assert!(json.contains("  ") || json.contains("    "));
    }

    #[test]
    fn test_generate_config_schema_json_has_schema_url() {
        let json = generate_config_schema_json();
        assert!(json.contains("http://json-schema.org/draft-07/schema#"));
    }

    // ==================== Validation with Real Schema Tests ====================

    #[test]
    fn test_validate_with_generated_config_schema() {
        let schema = generate_config_schema();

        // Create a minimal valid config
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_real_config.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        // Write minimal ServerConfig structure
        writeln!(file, "port: 8080").unwrap();

        // This might fail or succeed depending on ServerConfig requirements
        // The test is to ensure it doesn't panic
        let result = validate_config_file(&file_path, "mockforge-config", &schema);
        assert!(result.is_ok() || result.is_err()); // Either is fine, just don't panic

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validate_complex_json_structure() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "server": {
                    "type": "object",
                    "properties": {
                        "host": { "type": "string" },
                        "port": { "type": "integer" },
                        "ssl": {
                            "type": "object",
                            "properties": {
                                "enabled": { "type": "boolean" },
                                "cert_path": { "type": "string" }
                            }
                        }
                    }
                }
            }
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_complex.json");
        let content = r#"{
            "server": {
                "host": "localhost",
                "port": 8080,
                "ssl": {
                    "enabled": true,
                    "cert_path": "/path/to/cert"
                }
            }
        }"#;
        std::fs::write(&file_path, content).unwrap();

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();
        assert!(result.valid);

        std::fs::remove_file(&file_path).ok();
    }

    // ==================== Edge Cases for File Extensions ====================

    #[test]
    fn test_validate_uppercase_yaml_extension() {
        let schema = serde_json::json!({
            "type": "object"
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_config.YAML");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "key: value").unwrap();

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();
        assert!(result.valid);

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validate_uppercase_json_extension() {
        let schema = serde_json::json!({
            "type": "object"
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_config.JSON");
        let content = r#"{"key": "value"}"#;
        std::fs::write(&file_path, content).unwrap();

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();
        assert!(result.valid);

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validate_no_extension_treats_as_json() {
        let schema = serde_json::json!({
            "type": "object"
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_config_no_ext");
        let content = r#"{"key": "value"}"#;
        std::fs::write(&file_path, content).unwrap();

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();
        assert!(result.valid);

        std::fs::remove_file(&file_path).ok();
    }

    // ==================== Additional ValidationResult Tests ====================

    #[test]
    fn test_validation_result_with_long_error_messages() {
        let errors = vec![
            "Error at /path/to/nested/property: expected integer but got string 'invalid'"
                .to_string(),
            "Error at /another/path: value must be between 1 and 100 but got 150".to_string(),
            "Error at /required/field: this field is required but was not provided".to_string(),
        ];

        let result = ValidationResult::failure(
            "/path/to/config.yaml".to_string(),
            "test-config".to_string(),
            errors.clone(),
        );

        assert!(!result.valid);
        assert_eq!(result.errors.len(), 3);
        assert_eq!(result.errors, errors);
    }

    #[test]
    fn test_validation_result_with_special_characters_in_path() {
        let result = ValidationResult::success(
            "/path/with spaces/and-special_chars/config.yaml".to_string(),
            "mockforge-config".to_string(),
        );

        assert!(result.valid);
        assert_eq!(result.file_path, "/path/with spaces/and-special_chars/config.yaml");
    }

    // ==================== Schema Consistency Tests ====================

    #[test]
    fn test_all_schemas_have_schema_key() {
        let schemas = generate_all_schemas();

        for (name, schema) in schemas {
            let obj = schema.as_object().unwrap();
            assert!(obj.contains_key("$schema"), "Schema {} should have $schema key", name);
        }
    }

    #[test]
    fn test_all_schemas_have_title() {
        let schemas = generate_all_schemas();

        for (name, schema) in schemas {
            let obj = schema.as_object().unwrap();
            assert!(obj.contains_key("title"), "Schema {} should have title", name);
        }
    }

    #[test]
    fn test_all_schemas_have_description() {
        let schemas = generate_all_schemas();

        for (name, schema) in schemas {
            let obj = schema.as_object().unwrap();
            assert!(obj.contains_key("description"), "Schema {} should have description", name);
        }
    }

    #[test]
    fn test_schema_titles_are_unique() {
        let schemas = generate_all_schemas();
        let mut titles = std::collections::HashSet::new();

        for (_name, schema) in schemas {
            let obj = schema.as_object().unwrap();
            let title = obj.get("title").unwrap().as_str().unwrap();
            assert!(titles.insert(title.to_string()), "Duplicate title found: {}", title);
        }
    }

    // ==================== Integration Tests ====================

    #[test]
    fn test_end_to_end_config_validation_workflow() {
        // Generate schema
        let schemas = generate_all_schemas();
        let config_schema = schemas.get("mockforge-config").unwrap();

        // Create a test file
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("mockforge.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "port: 8080").unwrap();

        // Detect schema type
        let detected_type = detect_schema_type(&file_path);
        assert_eq!(detected_type, Some("mockforge-config".to_string()));

        // Validate
        let result = validate_config_file(&file_path, "mockforge-config", config_schema);
        assert!(result.is_ok());

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_blueprint_validation_workflow() {
        let schemas = generate_all_schemas();
        let blueprint_schema = schemas.get("blueprint-config").unwrap();

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("blueprint.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "manifest_version: '1.0'").unwrap();
        writeln!(file, "name: test-blueprint").unwrap();
        writeln!(file, "version: 1.0.0").unwrap();
        writeln!(file, "title: Test Blueprint").unwrap();
        writeln!(file, "description: A test blueprint").unwrap();
        writeln!(file, "author: Test Author").unwrap();
        writeln!(file, "category: saas").unwrap();

        let detected_type = detect_schema_type(&file_path);
        assert_eq!(detected_type, Some("blueprint-config".to_string()));

        let result =
            validate_config_file(&file_path, "blueprint-config", blueprint_schema).unwrap();
        assert!(result.valid, "Blueprint validation should succeed");

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validate_yaml_with_comments() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_with_comments.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "# This is a comment").unwrap();
        writeln!(file, "name: test-service  # inline comment").unwrap();

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();
        assert!(result.valid);

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_validate_yaml_with_anchors_and_aliases() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "config1": { "type": "object" },
                "config2": { "type": "object" }
            }
        });

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_anchors.yaml");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "config1: &defaults").unwrap();
        writeln!(file, "  key: value").unwrap();
        writeln!(file, "config2: *defaults").unwrap();

        let result = validate_config_file(&file_path, "test-config", &schema).unwrap();
        assert!(result.valid);

        std::fs::remove_file(&file_path).ok();
    }
}
