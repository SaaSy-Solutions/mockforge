//! JSON Schema generation for MockForge configuration files
//!
//! This crate provides functionality to generate JSON Schema definitions
//! from MockForge's configuration structs, enabling IDE autocomplete and
//! validation for `mockforge.yaml`, `mockforge.toml`, persona files, and blueprint files.

use schemars::schema_for;
use serde_json;

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
        obj.insert("$schema".to_string(), serde_json::json!("http://json-schema.org/draft-07/schema#"));
        obj.insert("title".to_string(), serde_json::json!("MockForge Server Configuration"));
        obj.insert("description".to_string(), serde_json::json!(
            "Complete configuration schema for MockForge mock server. \
             This schema provides autocomplete and validation for mockforge.yaml files."
        ));
    }

    schema_value
}

/// Generate JSON Schema for Reality configuration
///
/// Generates schema for the Reality slider configuration used to control
/// mock environment realism levels.
pub fn generate_reality_schema() -> serde_json::Value {
    let schema = schema_for!(mockforge_core::config::RealitySliderConfig);

    let mut schema_value = serde_json::to_value(schema).expect("Failed to serialize reality schema");

    // Add metadata for better IDE support
    if let Some(obj) = schema_value.as_object_mut() {
        obj.insert("$schema".to_string(), serde_json::json!("http://json-schema.org/draft-07/schema#"));
        obj.insert("title".to_string(), serde_json::json!("MockForge Reality Configuration"));
        obj.insert("description".to_string(), serde_json::json!(
            "Reality slider configuration for controlling mock environment realism. \
             Maps reality levels (1-5) to specific subsystem settings."
        ));
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

    let mut schema_value = serde_json::to_value(schema).expect("Failed to serialize persona schema");

    // Add metadata for better IDE support
    if let Some(obj) = schema_value.as_object_mut() {
        obj.insert("$schema".to_string(), serde_json::json!("http://json-schema.org/draft-07/schema#"));
        obj.insert("title".to_string(), serde_json::json!("MockForge Persona Configuration"));
        obj.insert("description".to_string(), serde_json::json!(
            "Persona configuration for consistent, personality-driven data generation. \
             Defines personas with unique IDs, domains, traits, and deterministic seeds."
        ));
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
    use std::fs;
    use jsonschema::{Draft, Validator as SchemaValidator};

    // Read and parse the config file
    let content = fs::read_to_string(file_path)?;
    let config_value: serde_json::Value = if file_path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"))
        .unwrap_or(false)
    {
        // Parse YAML
        serde_yaml::from_str(&content)
            .map_err(|e| format!("Failed to parse YAML: {}", e))?
    } else {
        // Parse JSON
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?
    };

    // Compile the schema
    let compiled_schema = SchemaValidator::options()
        .with_draft(Draft::Draft7)
        .build(schema)
        .map_err(|e| format!("Failed to compile schema: {}", e))?;

    // Validate
    let mut errors = Vec::new();
    for error in compiled_schema.iter_errors(&config_value) {
        errors.push(format!(
            "{}: {}",
            error.instance_path.to_string(),
            error
        ));
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
    if file_name == "mockforge.yaml" || file_name == "mockforge.yml" || file_name == "mockforge.json" {
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
