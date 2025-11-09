//! VBR (Virtual Backend Reality) integration for scenarios
//!
//! Provides functionality to include VBR entity definitions in scenarios
//! and apply them when scenarios are installed or used.

use crate::error::{Result, ScenarioError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// VBR entity definition for scenarios
///
/// Defines a VBR entity that can be included in a scenario package.
/// The entity will be created in the VBR engine when the scenario is applied.
///
/// Note: VBR schema is stored as JSON to avoid circular dependencies.
/// The actual VBR integration is handled at a higher level (CLI or application code).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VbrEntityDefinition {
    /// Entity name
    pub name: String,

    /// VBR schema definition (stored as JSON to avoid circular dependencies)
    pub schema: serde_json::Value,

    /// Optional seed data file path (relative to scenario root)
    ///
    /// If provided, the entity will be seeded with data from this file
    /// when the scenario is applied. The file should be JSON or YAML
    /// format with an array of entity records.
    #[serde(default)]
    pub seed_data_path: Option<String>,

    /// Optional state machine for this entity
    ///
    /// If provided, the entity will use this state machine for
    /// lifecycle management and state transitions.
    #[serde(default)]
    pub state_machine: Option<mockforge_core::intelligent_behavior::rules::StateMachine>,
}

impl VbrEntityDefinition {
    /// Create a new VBR entity definition
    pub fn new(name: String, schema: serde_json::Value) -> Self {
        Self {
            name,
            schema,
            seed_data_path: None,
            state_machine: None,
        }
    }

    /// Set the seed data path
    pub fn with_seed_data(mut self, path: String) -> Self {
        self.seed_data_path = Some(path);
        self
    }

    /// Set the state machine
    pub fn with_state_machine(
        mut self,
        state_machine: mockforge_core::intelligent_behavior::rules::StateMachine,
    ) -> Self {
        self.state_machine = Some(state_machine);
        self
    }

    /// Load seed data from file
    pub fn load_seed_data(&self, scenario_root: &PathBuf) -> Result<Vec<serde_json::Value>> {
        let seed_path = match &self.seed_data_path {
            Some(path) => scenario_root.join(path),
            None => return Ok(Vec::new()),
        };

        if !seed_path.exists() {
            return Err(ScenarioError::NotFound(format!(
                "Seed data file not found: {}",
                seed_path.display()
            )));
        }

        let content = std::fs::read_to_string(&seed_path).map_err(|e| ScenarioError::Io(e))?;

        // Try to parse as JSON first, then YAML
        let data: Vec<serde_json::Value> = if seed_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "json")
            .unwrap_or(false)
        {
            serde_json::from_str(&content).map_err(|e| ScenarioError::Serde(e))?
        } else {
            serde_yaml::from_str(&content).map_err(|e| ScenarioError::Yaml(e))?
        };

        Ok(data)
    }
}

/// VBR integration configuration
///
/// Contains configuration for applying VBR entities from a scenario.
#[derive(Debug, Clone)]
pub struct VbrIntegrationConfig {
    /// Whether to create entities if they don't exist
    pub create_entities: bool,

    /// Whether to seed data if seed files are provided
    pub seed_data: bool,

    /// Whether to merge with existing entities (or replace)
    pub merge_mode: VbrMergeMode,
}

/// VBR merge mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VbrMergeMode {
    /// Replace existing entities with scenario entities
    Replace,

    /// Merge scenario entities with existing (prefer existing)
    MergePreferExisting,

    /// Merge scenario entities with existing (prefer scenario)
    MergePreferScenario,
}

impl Default for VbrIntegrationConfig {
    fn default() -> Self {
        Self {
            create_entities: true,
            seed_data: true,
            merge_mode: VbrMergeMode::MergePreferExisting,
        }
    }
}

/// Apply VBR entities from a scenario to a VBR engine
///
/// This function takes a list of VBR entity definitions and applies them
/// to an existing VBR engine, creating entities and seeding data as needed.
///
/// Note: This is a placeholder function. The actual implementation should be
/// in the CLI or application code that has access to both mockforge-scenarios
/// and mockforge-vbr to avoid circular dependencies.
pub async fn apply_vbr_entities(
    _entities: &[VbrEntityDefinition],
    _engine: &mut (),
    _scenario_root: &PathBuf,
    _config: &VbrIntegrationConfig,
) -> Result<()> {
    // This function is a placeholder. The actual VBR integration should be
    // implemented in the CLI or application layer to avoid circular dependencies.
    // The CLI has access to both mockforge-scenarios and mockforge-vbr.
    Err(ScenarioError::Generic(
        "VBR entity application must be implemented in the CLI or application layer".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vbr_entity_definition() {
        let schema = serde_json::json!({
            "base": {
                "name": "TestEntity",
                "fields": [{
                    "name": "id",
                    "field_type": "string",
                    "required": true
                }]
            },
            "primary_key": ["id"]
        });

        let entity_def = VbrEntityDefinition::new("TestEntity".to_string(), schema);
        assert_eq!(entity_def.name, "TestEntity");
        assert!(entity_def.seed_data_path.is_none());
    }
}
