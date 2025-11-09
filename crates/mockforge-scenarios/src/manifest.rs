//! Scenario manifest schema and validation
//!
//! Defines the structure of scenario manifests that describe complete mock system
//! configurations including metadata, compatibility, and file organization.

use crate::error::{Result, ScenarioError};
use chrono::{DateTime, Utc};
use mockforge_core::intelligent_behavior::{rules::StateMachine, VisualLayout};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Scenario manifest structure
///
/// The manifest contains all metadata about a scenario, including its files,
/// compatibility requirements, and dependencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioManifest {
    /// Manifest format version
    pub manifest_version: String,

    /// Scenario name (unique identifier)
    pub name: String,

    /// Scenario version (semver)
    pub version: String,

    /// Human-readable title
    pub title: String,

    /// Detailed description
    pub description: String,

    /// Author information
    pub author: String,

    /// Author email (optional)
    #[serde(default)]
    pub author_email: Option<String>,

    /// Category classification
    pub category: ScenarioCategory,

    /// Tags for discovery
    #[serde(default)]
    pub tags: Vec<String>,

    /// Compatibility requirements
    pub compatibility: CompatibilityInfo,

    /// List of files included in the scenario
    pub files: Vec<String>,

    /// Path to README file
    #[serde(default)]
    pub readme: Option<String>,

    /// Example usage instructions
    #[serde(default)]
    pub example_usage: Option<String>,

    /// Required features
    #[serde(default)]
    pub required_features: Vec<String>,

    /// Optional plugin dependencies
    #[serde(default)]
    pub plugin_dependencies: Vec<PluginDependency>,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,

    /// Created timestamp
    #[serde(default = "default_timestamp")]
    pub created_at: DateTime<Utc>,

    /// Updated timestamp
    #[serde(default = "default_timestamp")]
    pub updated_at: DateTime<Utc>,

    /// State machines defined in this scenario
    ///
    /// These state machines can be used to model resource lifecycles,
    /// workflow states, and complex state transitions.
    #[serde(default)]
    pub state_machines: Vec<StateMachine>,

    /// Visual layout graphs for state machines
    ///
    /// Maps state machine resource_type to its visual layout representation.
    /// This allows the editor to restore node positions and visual structure.
    #[serde(default)]
    pub state_machine_graphs: HashMap<String, VisualLayout>,

    /// VBR entity definitions
    ///
    /// If provided, these entities will be created in the VBR engine
    /// when the scenario is applied to a workspace.
    #[serde(default)]
    pub vbr_entities: Option<Vec<crate::vbr_integration::VbrEntityDefinition>>,

    /// MockAI configuration
    ///
    /// If provided, this MockAI configuration will be merged with existing
    /// config when the scenario is applied to a workspace.
    #[serde(default)]
    pub mockai_config: Option<crate::mockai_integration::MockAIConfigDefinition>,
}

fn default_timestamp() -> DateTime<Utc> {
    Utc::now()
}

impl ScenarioManifest {
    /// Create a new scenario manifest
    pub fn new(name: String, version: String, title: String, description: String) -> Self {
        Self {
            manifest_version: "1.0".to_string(),
            name,
            version,
            title,
            description,
            author: "community".to_string(),
            author_email: None,
            category: ScenarioCategory::Other,
            tags: Vec::new(),
            compatibility: CompatibilityInfo::default(),
            files: Vec::new(),
            readme: None,
            example_usage: None,
            required_features: Vec::new(),
            plugin_dependencies: Vec::new(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            state_machines: Vec::new(),
            state_machine_graphs: HashMap::new(),
            vbr_entities: None,
            mockai_config: None,
        }
    }

    /// Load manifest from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| ScenarioError::Io(e))?;
        Self::from_str(&content)
    }

    /// Load manifest from string
    pub fn from_str(content: &str) -> Result<Self> {
        let manifest: Self = serde_yaml::from_str(content).map_err(|e| ScenarioError::Yaml(e))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate the manifest
    pub fn validate(&self) -> Result<()> {
        // Validate manifest version
        if self.manifest_version != "1.0" {
            return Err(ScenarioError::InvalidManifest(format!(
                "Unsupported manifest version: {}",
                self.manifest_version
            )));
        }

        // Validate name
        if self.name.trim().is_empty() {
            return Err(ScenarioError::InvalidManifest(
                "Scenario name cannot be empty".to_string(),
            ));
        }

        // Validate version (basic semver check)
        if self.version.trim().is_empty() {
            return Err(ScenarioError::InvalidVersion(
                "Scenario version cannot be empty".to_string(),
            ));
        }

        // Validate title
        if self.title.trim().is_empty() {
            return Err(ScenarioError::InvalidManifest(
                "Scenario title cannot be empty".to_string(),
            ));
        }

        // Validate description
        if self.description.trim().is_empty() {
            return Err(ScenarioError::InvalidManifest(
                "Scenario description cannot be empty".to_string(),
            ));
        }

        // Validate author
        if self.author.trim().is_empty() {
            return Err(ScenarioError::InvalidManifest(
                "Scenario author cannot be empty".to_string(),
            ));
        }

        // Validate compatibility
        self.compatibility.validate()?;

        // Validate plugin dependencies
        for dep in &self.plugin_dependencies {
            dep.validate()?;
        }

        // Validate state machines
        for state_machine in &self.state_machines {
            // Ensure initial state exists in states list
            if !state_machine.states.contains(&state_machine.initial_state) {
                return Err(ScenarioError::InvalidManifest(format!(
                    "State machine '{}' has initial state '{}' that is not in states list",
                    state_machine.resource_type, state_machine.initial_state
                )));
            }

            // Validate transitions reference valid states
            for transition in &state_machine.transitions {
                if !state_machine.states.contains(&transition.from_state) {
                    return Err(ScenarioError::InvalidManifest(format!(
                        "State machine '{}' has transition from invalid state '{}'",
                        state_machine.resource_type, transition.from_state
                    )));
                }
                if !state_machine.states.contains(&transition.to_state) {
                    return Err(ScenarioError::InvalidManifest(format!(
                        "State machine '{}' has transition to invalid state '{}'",
                        state_machine.resource_type, transition.to_state
                    )));
                }

                // Validate sub-scenario references
                if let Some(ref sub_scenario_id) = transition.sub_scenario_ref {
                    if state_machine.get_sub_scenario(sub_scenario_id).is_none() {
                        return Err(ScenarioError::InvalidManifest(format!(
                            "State machine '{}' references non-existent sub-scenario '{}'",
                            state_machine.resource_type, sub_scenario_id
                        )));
                    }
                }
            }

            // Validate sub-scenarios recursively
            for sub_scenario in &state_machine.sub_scenarios {
                // Validate sub-scenario has valid state machine
                if !sub_scenario
                    .state_machine
                    .states
                    .contains(&sub_scenario.state_machine.initial_state)
                {
                    return Err(ScenarioError::InvalidManifest(format!(
                        "Sub-scenario '{}' in state machine '{}' has invalid initial state",
                        sub_scenario.id, state_machine.resource_type
                    )));
                }
            }
        }

        Ok(())
    }

    /// Get scenario ID (name@version)
    pub fn id(&self) -> String {
        format!("{}@{}", self.name, self.version)
    }

    /// Check if scenario requires a specific protocol
    pub fn requires_protocol(&self, protocol: &str) -> bool {
        self.compatibility.protocols.contains(&protocol.to_string())
    }

    /// Check if scenario requires a specific feature
    pub fn requires_feature(&self, feature: &str) -> bool {
        self.required_features.contains(&feature.to_string())
    }
}

/// Scenario category classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ScenarioCategory {
    /// E-commerce scenarios
    Ecommerce,
    /// Chat and messaging scenarios
    Chat,
    /// Weather and location services
    Weather,
    /// Social media scenarios
    Social,
    /// Payment processing scenarios
    Payment,
    /// Authentication and authorization scenarios
    Auth,
    /// Analytics and reporting scenarios
    Analytics,
    /// IoT and device scenarios
    Iot,
    /// Other/miscellaneous scenarios
    Other,
}

/// Compatibility information for scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityInfo {
    /// Minimum MockForge version required
    pub min_version: String,

    /// Maximum MockForge version (optional)
    #[serde(default)]
    pub max_version: Option<String>,

    /// Required features
    #[serde(default)]
    pub required_features: Vec<String>,

    /// Required protocols
    #[serde(default)]
    pub protocols: Vec<String>,
}

impl Default for CompatibilityInfo {
    fn default() -> Self {
        Self {
            min_version: "0.2.0".to_string(),
            max_version: None,
            required_features: Vec::new(),
            protocols: vec!["http".to_string()],
        }
    }
}

impl CompatibilityInfo {
    /// Validate compatibility info
    pub fn validate(&self) -> Result<()> {
        if self.min_version.trim().is_empty() {
            return Err(ScenarioError::InvalidManifest(
                "Minimum version cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

/// Plugin dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// Plugin ID
    pub plugin_id: String,

    /// Version requirement (semver)
    pub version: String,

    /// Whether this dependency is optional
    #[serde(default)]
    pub optional: bool,
}

impl PluginDependency {
    /// Validate plugin dependency
    pub fn validate(&self) -> Result<()> {
        if self.plugin_id.trim().is_empty() {
            return Err(ScenarioError::InvalidManifest("Plugin ID cannot be empty".to_string()));
        }

        if self.version.trim().is_empty() {
            return Err(ScenarioError::InvalidManifest(format!(
                "Plugin dependency {} version cannot be empty",
                self.plugin_id
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_creation() {
        let manifest = ScenarioManifest::new(
            "test-scenario".to_string(),
            "1.0.0".to_string(),
            "Test Scenario".to_string(),
            "A test scenario".to_string(),
        );

        assert_eq!(manifest.name, "test-scenario");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.title, "Test Scenario");
    }

    #[test]
    fn test_manifest_validation() {
        let mut manifest = ScenarioManifest::new(
            "test-scenario".to_string(),
            "1.0.0".to_string(),
            "Test Scenario".to_string(),
            "A test scenario".to_string(),
        );

        // Valid manifest should pass
        assert!(manifest.validate().is_ok());

        // Empty name should fail
        manifest.name = "".to_string();
        assert!(manifest.validate().is_err());

        // Reset and test empty version
        manifest.name = "test-scenario".to_string();
        manifest.version = "".to_string();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_scenario_id() {
        let manifest = ScenarioManifest::new(
            "test-scenario".to_string(),
            "1.0.0".to_string(),
            "Test Scenario".to_string(),
            "A test scenario".to_string(),
        );

        assert_eq!(manifest.id(), "test-scenario@1.0.0");
    }
}
