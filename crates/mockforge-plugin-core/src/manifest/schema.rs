//! Plugin configuration schema definitions
//!
//! This module defines the schema for plugin configuration, including
//! property types, validation rules, and schema validation.

use crate::{PluginError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plugin configuration schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSchema {
    /// Schema format version
    pub version: String,
    /// Configuration properties
    pub properties: HashMap<String, ConfigProperty>,
    /// Required properties
    pub required: Vec<String>,
}

impl Default for ConfigSchema {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigSchema {
    /// Create new config schema
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            properties: HashMap::new(),
            required: Vec::new(),
        }
    }

    /// Add a property to the schema
    pub fn add_property(&mut self, name: String, property: ConfigProperty) {
        self.properties.insert(name, property);
    }

    /// Mark a property as required
    pub fn require(&mut self, name: &str) {
        if !self.required.contains(&name.to_string()) {
            self.required.push(name.to_string());
        }
    }

    /// Validate the schema
    pub fn validate(&self) -> Result<()> {
        if self.version != "1.0" {
            return Err(PluginError::config_error(&format!(
                "Unsupported schema version: {}",
                self.version
            )));
        }

        // Validate all properties
        for (name, property) in &self.properties {
            property.validate(name)?;
        }

        // Validate required properties exist
        for required_name in &self.required {
            if !self.properties.contains_key(required_name) {
                return Err(PluginError::config_error(&format!(
                    "Required property '{}' not defined in schema",
                    required_name
                )));
            }
        }

        Ok(())
    }

    /// Validate configuration against schema
    pub fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
        if let serde_json::Value::Object(config_obj) = config {
            // Check required properties
            for required_name in &self.required {
                if !config_obj.contains_key(required_name) {
                    return Err(PluginError::config_error(&format!(
                        "Missing required configuration property: {}",
                        required_name
                    )));
                }
            }

            // Validate each provided property
            for (key, value) in config_obj {
                if let Some(property) = self.properties.get(key) {
                    property.validate_value(value)?;
                } else {
                    return Err(PluginError::config_error(&format!(
                        "Unknown configuration property: {}",
                        key
                    )));
                }
            }

            Ok(())
        } else {
            Err(PluginError::config_error("Configuration must be an object"))
        }
    }

    /// Get property by name
    pub fn get_property(&self, name: &str) -> Option<&ConfigProperty> {
        self.properties.get(name)
    }

    /// Check if property is required
    pub fn is_required(&self, name: &str) -> bool {
        self.required.contains(&name.to_string())
    }
}

/// Configuration property definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigProperty {
    /// Property type
    #[serde(rename = "type")]
    pub property_type: PropertyType,
    /// Property description
    pub description: Option<String>,
    /// Default value
    pub default: Option<serde_json::Value>,
    /// Validation rules
    pub validation: Option<PropertyValidation>,
    /// Whether the property is deprecated
    pub deprecated: Option<bool>,
}

impl ConfigProperty {
    /// Create new property
    pub fn new(property_type: PropertyType) -> Self {
        Self {
            property_type,
            description: None,
            default: None,
            validation: None,
            deprecated: None,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set default value
    pub fn with_default(mut self, default: serde_json::Value) -> Self {
        self.default = Some(default);
        self
    }

    /// Set validation
    pub fn with_validation(mut self, validation: PropertyValidation) -> Self {
        self.validation = Some(validation);
        self
    }

    /// Mark as deprecated
    pub fn deprecated(mut self) -> Self {
        self.deprecated = Some(true);
        self
    }

    /// Validate property definition
    pub fn validate(&self, name: &str) -> Result<()> {
        // Validate default value matches type
        if let Some(default) = &self.default {
            self.validate_value(default).map_err(|e| {
                PluginError::config_error(&format!(
                    "Invalid default value for property '{}': {}",
                    name, e
                ))
            })?;
        }

        Ok(())
    }

    /// Validate a value against this property
    pub fn validate_value(&self, value: &serde_json::Value) -> Result<()> {
        // Type validation
        match &self.property_type {
            PropertyType::String => {
                if !value.is_string() {
                    return Err(PluginError::config_error("Expected string value"));
                }
            }
            PropertyType::Number => {
                if !value.is_number() {
                    return Err(PluginError::config_error("Expected number value"));
                }
            }
            PropertyType::Boolean => {
                if !value.is_boolean() {
                    return Err(PluginError::config_error("Expected boolean value"));
                }
            }
            PropertyType::Array => {
                if !value.is_array() {
                    return Err(PluginError::config_error("Expected array value"));
                }
            }
            PropertyType::Object => {
                if !value.is_object() {
                    return Err(PluginError::config_error("Expected object value"));
                }
            }
        }

        // Custom validation rules
        if let Some(validation) = &self.validation {
            validation.validate_value(value)?;
        }

        Ok(())
    }
}

/// Property type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
}

/// Property validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyValidation {
    /// Minimum value (for numbers)
    pub min: Option<f64>,
    /// Maximum value (for numbers)
    pub max: Option<f64>,
    /// Minimum length (for strings/arrays)
    pub min_length: Option<usize>,
    /// Maximum length (for strings/arrays)
    pub max_length: Option<usize>,
    /// Regular expression pattern (for strings)
    pub pattern: Option<String>,
    /// Allowed values (enum)
    pub enum_values: Option<Vec<serde_json::Value>>,
}

impl Default for PropertyValidation {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyValidation {
    /// Create new validation
    pub fn new() -> Self {
        Self {
            min: None,
            max: None,
            min_length: None,
            max_length: None,
            pattern: None,
            enum_values: None,
        }
    }

    /// Validate a value against these rules
    pub fn validate_value(&self, value: &serde_json::Value) -> Result<()> {
        // Number validations
        if let Some(num) = value.as_f64() {
            if let Some(min) = self.min {
                if num < min {
                    return Err(PluginError::config_error(&format!(
                        "Value {} is less than minimum {}",
                        num, min
                    )));
                }
            }
            if let Some(max) = self.max {
                if num > max {
                    return Err(PluginError::config_error(&format!(
                        "Value {} is greater than maximum {}",
                        num, max
                    )));
                }
            }
        }

        // String validations
        if let Some(s) = value.as_str() {
            if let Some(min_len) = self.min_length {
                if s.len() < min_len {
                    return Err(PluginError::config_error(&format!(
                        "String length {} is less than minimum {}",
                        s.len(),
                        min_len
                    )));
                }
            }
            if let Some(max_len) = self.max_length {
                if s.len() > max_len {
                    return Err(PluginError::config_error(&format!(
                        "String length {} is greater than maximum {}",
                        s.len(),
                        max_len
                    )));
                }
            }
            if let Some(pattern) = &self.pattern {
                let regex = regex::Regex::new(pattern).map_err(|e| {
                    PluginError::config_error(&format!("Invalid regex pattern: {}", e))
                })?;
                if !regex.is_match(s) {
                    return Err(PluginError::config_error(&format!(
                        "String '{}' does not match pattern '{}'",
                        s, pattern
                    )));
                }
            }
        }

        // Array validations
        if let Some(arr) = value.as_array() {
            if let Some(min_len) = self.min_length {
                if arr.len() < min_len {
                    return Err(PluginError::config_error(&format!(
                        "Array length {} is less than minimum {}",
                        arr.len(),
                        min_len
                    )));
                }
            }
            if let Some(max_len) = self.max_length {
                if arr.len() > max_len {
                    return Err(PluginError::config_error(&format!(
                        "Array length {} is greater than maximum {}",
                        arr.len(),
                        max_len
                    )));
                }
            }
        }

        // Enum validations
        if let Some(enum_values) = &self.enum_values {
            let mut found = false;
            for allowed_value in enum_values {
                if value == allowed_value {
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(PluginError::config_error(&format!(
                    "Value {} is not in allowed values",
                    value
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_compiles() {
        // Basic compilation test
        assert!(true);
    }
}
