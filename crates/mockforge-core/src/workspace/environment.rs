//! Environment configuration and management
//!
//! This module provides functionality for managing environments, variable substitution,
//! and environment-specific configurations.

use crate::workspace::core::{EntityId, Environment, EnvironmentColor};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Environment manager for handling multiple environments
#[derive(Debug, Clone)]
pub struct EnvironmentManager {
    /// All environments indexed by ID
    environments: HashMap<EntityId, Environment>,
    /// Active environment ID
    active_environment_id: Option<EntityId>,
}

/// Environment variable substitution result
#[derive(Debug, Clone)]
pub struct VariableSubstitution {
    /// The substituted value
    pub value: String,
    /// Whether substitution was successful
    pub success: bool,
    /// Any errors that occurred during substitution
    pub errors: Vec<String>,
}

/// Environment validation result
#[derive(Debug, Clone)]
pub struct EnvironmentValidationResult {
    /// Whether the environment is valid
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
}

/// Environment export format
#[derive(Debug, Clone)]
pub enum EnvironmentExportFormat {
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// Environment variables file format (.env)
    DotEnv,
    /// Custom format with template
    Custom(String),
}

impl EnvironmentManager {
    /// Create a new empty environment manager
    pub fn new() -> Self {
        Self {
            environments: HashMap::new(),
            active_environment_id: None,
        }
    }

    /// Add an environment
    pub fn add_environment(&mut self, environment: Environment) -> EntityId {
        let id = environment.id.clone();
        self.environments.insert(id.clone(), environment);

        // Set as active if it's the first environment
        if self.environments.len() == 1 {
            self.active_environment_id = Some(id.clone());
            if let Some(env) = self.environments.get_mut(&id) {
                env.active = true;
            }
        }

        id
    }

    /// Get an environment by ID
    pub fn get_environment(&self, id: &EntityId) -> Option<&Environment> {
        self.environments.get(id)
    }

    /// Get a mutable environment by ID
    pub fn get_environment_mut(&mut self, id: &EntityId) -> Option<&mut Environment> {
        self.environments.get_mut(id)
    }

    /// Remove an environment
    pub fn remove_environment(&mut self, id: &EntityId) -> Result<Environment, String> {
        if let Some(environment) = self.environments.remove(id) {
            // Update active environment if necessary
            if self.active_environment_id.as_ref() == Some(id) {
                self.active_environment_id = self.environments.keys().next().cloned();
                if let Some(active_id) = &self.active_environment_id {
                    if let Some(env) = self.environments.get_mut(active_id) {
                        env.active = true;
                    }
                }
            }

            Ok(environment)
        } else {
            Err(format!("Environment with ID {} not found", id))
        }
    }

    /// Get all environments
    pub fn get_all_environments(&self) -> Vec<&Environment> {
        self.environments.values().collect()
    }

    /// Get the active environment
    pub fn get_active_environment(&self) -> Option<&Environment> {
        self.active_environment_id.as_ref().and_then(|id| self.environments.get(id))
    }

    /// Set the active environment
    pub fn set_active_environment(&mut self, id: EntityId) -> Result<(), String> {
        if self.environments.contains_key(&id) {
            // Deactivate all environments
            for environment in self.environments.values_mut() {
                environment.active = false;
            }

            // Activate the selected environment
            if let Some(env) = self.environments.get_mut(&id) {
                env.active = true;
                self.active_environment_id = Some(id);
            }

            Ok(())
        } else {
            Err(format!("Environment with ID {} not found", id))
        }
    }

    /// Substitute variables in a template string
    pub fn substitute_variables(&self, template: &str) -> VariableSubstitution {
        let mut result = String::new();
        let mut success = true;
        let mut errors = Vec::new();

        // Get active environment variables
        let variables = if let Some(active_env) = self.get_active_environment() {
            &active_env.variables
        } else {
            // No active environment, so return empty variables (will fail on any variable reference)
            &std::collections::HashMap::new()
        };

        let mut chars = template.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '{' && chars.peek() == Some(&'{') {
                // Found {{
                chars.next(); // consume second {
                if let Some(var_name) = self.parse_variable_name(&mut chars) {
                    if let Some(value) = variables.get(&var_name) {
                        result.push_str(value);
                    } else {
                        success = false;
                        errors.push(format!("Variable '{}' not found", var_name));
                        result.push_str(&format!("{{{{{}}}}}", var_name));
                    }
                } else {
                    result.push_str("{{");
                }
            } else {
                result.push(ch);
            }
        }

        VariableSubstitution {
            value: result,
            success,
            errors,
        }
    }

    /// Parse variable name from template
    fn parse_variable_name(
        &self,
        chars: &mut std::iter::Peekable<std::str::Chars>,
    ) -> Option<String> {
        let mut var_name = String::new();

        while let Some(ch) = chars.peek() {
            if *ch == '}' {
                if let Some(next_ch) = chars.clone().nth(1) {
                    if next_ch == '}' {
                        // Found }} - end of variable
                        chars.next(); // consume first }
                        chars.next(); // consume second }
                        break;
                    }
                }
            } else if ch.is_alphanumeric() || *ch == '_' || *ch == '-' || *ch == '.' {
                var_name.push(*ch);
                chars.next();
            } else {
                return None; // Invalid character in variable name
            }
        }

        if var_name.is_empty() {
            None
        } else {
            Some(var_name)
        }
    }

    /// Validate an environment
    pub fn validate_environment(&self, environment: &Environment) -> EnvironmentValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check for empty name
        if environment.name.trim().is_empty() {
            errors.push("Environment name cannot be empty".to_string());
        }

        // Check for duplicate variable names
        let mut seen_variables = std::collections::HashSet::new();
        for (key, value) in &environment.variables {
            if !seen_variables.insert(key.clone()) {
                errors.push(format!("Duplicate variable name: {}", key));
            }

            // Check for empty keys
            if key.trim().is_empty() {
                errors.push("Variable key cannot be empty".to_string());
            }

            // Check for empty values (warning)
            if value.trim().is_empty() {
                warnings.push(format!("Variable '{}' has empty value", key));
            }
        }

        // Color values are u8, so always valid (0-255)

        EnvironmentValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Export environment to specified format
    pub fn export_environment(
        &self,
        environment_id: &EntityId,
        format: EnvironmentExportFormat,
    ) -> Result<String, String> {
        let environment = self
            .environments
            .get(environment_id)
            .ok_or_else(|| format!("Environment with ID {} not found", environment_id))?;

        match format {
            EnvironmentExportFormat::Json => serde_json::to_string_pretty(environment)
                .map_err(|e| format!("Failed to serialize environment: {}", e)),
            EnvironmentExportFormat::Yaml => serde_yaml::to_string(environment)
                .map_err(|e| format!("Failed to serialize environment: {}", e)),
            EnvironmentExportFormat::DotEnv => {
                let mut result = String::new();
                for (key, value) in &environment.variables {
                    result.push_str(&format!("{}={}\n", key, value));
                }
                Ok(result)
            }
            EnvironmentExportFormat::Custom(template) => {
                let mut result = template.clone();
                for (key, value) in &environment.variables {
                    let placeholder = format!("{{{{{}}}}}", key);
                    result = result.replace(&placeholder, value);
                }
                Ok(result)
            }
        }
    }

    /// Import environment from JSON
    pub fn import_environment(&mut self, json_data: &str) -> Result<EntityId, String> {
        let environment: Environment = serde_json::from_str(json_data)
            .map_err(|e| format!("Failed to deserialize environment: {}", e))?;

        // Validate the imported environment
        let validation = self.validate_environment(&environment);
        if !validation.is_valid {
            return Err(format!("Environment validation failed: {:?}", validation.errors));
        }

        Ok(self.add_environment(environment))
    }

    /// Get environment statistics
    pub fn get_stats(&self) -> EnvironmentStats {
        let total_variables =
            self.environments.values().map(|env| env.variables.len()).sum::<usize>();

        let active_count = self.environments.values().filter(|env| env.active).count();

        EnvironmentStats {
            total_environments: self.environments.len(),
            total_variables,
            active_environments: active_count,
        }
    }

    /// Find environments by name
    pub fn find_environments_by_name(&self, name_query: &str) -> Vec<&Environment> {
        let query_lower = name_query.to_lowercase();
        self.environments
            .values()
            .filter(|env| env.name.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Get all variables across all environments
    pub fn get_all_variables(&self) -> HashMap<String, String> {
        let mut all_vars = HashMap::new();

        for environment in self.environments.values() {
            for (key, value) in &environment.variables {
                all_vars.insert(key.clone(), value.clone());
            }
        }

        all_vars
    }

    /// Clone environment
    pub fn clone_environment(
        &mut self,
        source_id: &EntityId,
        new_name: String,
    ) -> Result<EntityId, String> {
        let source_env = self
            .environments
            .get(source_id)
            .ok_or_else(|| format!("Environment with ID {} not found", source_id))?;

        let mut new_env = source_env.clone();
        new_env.id = uuid::Uuid::new_v4().to_string();
        new_env.name = new_name;
        new_env.active = false;
        new_env.created_at = Utc::now();
        new_env.updated_at = Utc::now();

        Ok(self.add_environment(new_env))
    }

    /// Merge environments (combine variables)
    pub fn merge_environments(
        &mut self,
        environment_ids: &[EntityId],
        merged_name: String,
    ) -> Result<EntityId, String> {
        let mut merged_variables = HashMap::new();

        for env_id in environment_ids {
            let env = self
                .environments
                .get(env_id)
                .ok_or_else(|| format!("Environment with ID {} not found", env_id))?;

            for (key, value) in &env.variables {
                merged_variables.insert(key.clone(), value.clone());
            }
        }

        let mut merged_env = Environment::new(merged_name);
        merged_env.variables = merged_variables;

        Ok(self.add_environment(merged_env))
    }
}

/// Environment statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentStats {
    /// Total number of environments
    pub total_environments: usize,
    /// Total number of variables across all environments
    pub total_variables: usize,
    /// Number of active environments
    pub active_environments: usize,
}

impl Default for EnvironmentManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Environment variable validation utilities
pub struct EnvironmentValidator;

impl EnvironmentValidator {
    /// Validate variable name format
    pub fn validate_variable_name(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("Variable name cannot be empty".to_string());
        }

        if !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(
                "Variable name can only contain letters, numbers, underscores, and hyphens"
                    .to_string(),
            );
        }

        if name.starts_with('-') || name.ends_with('-') {
            return Err("Variable name cannot start or end with hyphens".to_string());
        }

        Ok(())
    }

    /// Validate variable value (basic checks)
    pub fn validate_variable_value(value: &str) -> Result<(), String> {
        if value.contains('\0') {
            return Err("Variable value cannot contain null characters".to_string());
        }

        Ok(())
    }

    /// Validate color values
    pub fn validate_color(_color: &EnvironmentColor) -> Result<(), String> {
        // Color values are u8, so always valid (0-255)
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_substitution() {
        let mut manager = EnvironmentManager::new();
        let mut env = Environment::new("test".to_string());
        env.set_variable("API_URL".to_string(), "https://api.example.com".to_string());
        env.set_variable("VERSION".to_string(), "1.0.0".to_string());
        manager.add_environment(env);

        let result = manager.substitute_variables("API: {{API_URL}}, Version: {{VERSION}}");
        assert!(result.success);
        assert_eq!(result.value, "API: https://api.example.com, Version: 1.0.0");
    }

    #[test]
    fn test_missing_variable_substitution() {
        let manager = EnvironmentManager::new();
        let result = manager.substitute_variables("Missing: {{MISSING_VAR}}");

        assert!(!result.success);
        assert!(result.errors.contains(&"Variable 'MISSING_VAR' not found".to_string()));
    }
}
