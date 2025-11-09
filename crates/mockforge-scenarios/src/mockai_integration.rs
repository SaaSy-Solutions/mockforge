//! MockAI integration for scenarios
//!
//! Provides functionality to include MockAI configuration in scenarios
//! and apply them when scenarios are installed or used.

use crate::error::{Result, ScenarioError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// MockAI configuration for scenarios
///
/// Defines MockAI configuration that can be included in a scenario package.
/// The configuration will be merged with existing config when the scenario is applied.
///
/// Note: MockAI config is stored as JSON to avoid circular dependencies.
/// The actual MockAI integration is handled at a higher level (CLI or application code).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockAIConfigDefinition {
    /// MockAI configuration (stored as JSON to avoid circular dependencies)
    pub config: serde_json::Value,

    /// Optional path to behavior rules file (relative to scenario root)
    ///
    /// If provided, behavior rules will be loaded from this file.
    /// The file should be JSON or YAML format.
    #[serde(default)]
    pub behavior_rules_path: Option<String>,

    /// Optional path to example pairs file (relative to scenario root)
    ///
    /// If provided, MockAI will learn from these example pairs.
    /// The file should be JSON or YAML format with an array of example pairs.
    #[serde(default)]
    pub example_pairs_path: Option<String>,
}

impl MockAIConfigDefinition {
    /// Create a new MockAI config definition
    pub fn new(config: serde_json::Value) -> Self {
        Self {
            config,
            behavior_rules_path: None,
            example_pairs_path: None,
        }
    }

    /// Set the behavior rules path
    pub fn with_behavior_rules(mut self, path: String) -> Self {
        self.behavior_rules_path = Some(path);
        self
    }

    /// Set the example pairs path
    pub fn with_example_pairs(mut self, path: String) -> Self {
        self.example_pairs_path = Some(path);
        self
    }

    /// Load behavior rules from file
    pub fn load_behavior_rules(
        &self,
        scenario_root: &PathBuf,
    ) -> Result<Option<serde_json::Value>> {
        let rules_path = match &self.behavior_rules_path {
            Some(path) => scenario_root.join(path),
            None => return Ok(None),
        };

        if !rules_path.exists() {
            return Err(ScenarioError::NotFound(format!(
                "Behavior rules file not found: {}",
                rules_path.display()
            )));
        }

        let content = std::fs::read_to_string(&rules_path).map_err(|e| ScenarioError::Io(e))?;

        // Try to parse as JSON first, then YAML
        let rules: serde_json::Value = if rules_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "json")
            .unwrap_or(false)
        {
            serde_json::from_str(&content).map_err(|e| ScenarioError::Serde(e))?
        } else {
            serde_yaml::from_str(&content).map_err(|e| ScenarioError::Yaml(e))?
        };

        Ok(Some(rules))
    }

    /// Load example pairs from file
    pub fn load_example_pairs(
        &self,
        scenario_root: &PathBuf,
    ) -> Result<Option<Vec<serde_json::Value>>> {
        let pairs_path = match &self.example_pairs_path {
            Some(path) => scenario_root.join(path),
            None => return Ok(None),
        };

        if !pairs_path.exists() {
            return Err(ScenarioError::NotFound(format!(
                "Example pairs file not found: {}",
                pairs_path.display()
            )));
        }

        let content = std::fs::read_to_string(&pairs_path).map_err(|e| ScenarioError::Io(e))?;

        // Try to parse as JSON first, then YAML
        let pairs: Vec<serde_json::Value> = if pairs_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "json")
            .unwrap_or(false)
        {
            serde_json::from_str(&content).map_err(|e| ScenarioError::Serde(e))?
        } else {
            serde_yaml::from_str(&content).map_err(|e| ScenarioError::Yaml(e))?
        };

        Ok(Some(pairs))
    }
}

/// MockAI merge mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockAIMergeMode {
    /// Replace existing MockAI config with scenario config
    Replace,

    /// Merge scenario config with existing (prefer existing)
    MergePreferExisting,

    /// Merge scenario config with existing (prefer scenario)
    MergePreferScenario,
}

/// MockAI integration configuration
///
/// Contains configuration for applying MockAI config from a scenario.
#[derive(Debug, Clone)]
pub struct MockAIIntegrationConfig {
    /// Whether to apply MockAI config
    pub apply_config: bool,

    /// Whether to load behavior rules if provided
    pub load_behavior_rules: bool,

    /// Whether to load example pairs if provided
    pub load_example_pairs: bool,

    /// Merge mode for config
    pub merge_mode: MockAIMergeMode,
}

impl Default for MockAIIntegrationConfig {
    fn default() -> Self {
        Self {
            apply_config: true,
            load_behavior_rules: true,
            load_example_pairs: true,
            merge_mode: MockAIMergeMode::MergePreferScenario,
        }
    }
}

/// Apply MockAI configuration from a scenario
///
/// This function takes a MockAI config definition and applies it.
/// Note: This is a placeholder function. The actual implementation should be
/// in the CLI or application code that has access to both mockforge-scenarios
/// and mockforge-core to avoid circular dependencies.
pub async fn apply_mockai_config(
    _config_def: &MockAIConfigDefinition,
    _scenario_root: &PathBuf,
    _integration_config: &MockAIIntegrationConfig,
) -> Result<serde_json::Value> {
    // This function is a placeholder. The actual MockAI integration should be
    // implemented in the CLI or application layer to avoid circular dependencies.
    // The CLI has access to both mockforge-scenarios and mockforge-core.
    Err(ScenarioError::Generic(
        "MockAI config application must be implemented in the CLI or application layer".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mockai_config_definition() {
        let config = serde_json::json!({
            "enabled": true,
            "behavior_model": {
                "llm_provider": "ollama",
                "model": "llama3.2"
            }
        });

        let config_def = MockAIConfigDefinition::new(config);
        assert!(config_def.behavior_rules_path.is_none());
        assert!(config_def.example_pairs_path.is_none());
    }
}
