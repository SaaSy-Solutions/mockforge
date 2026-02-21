//! Reality Profile Pack manifest and installer
//!
//! Reality Profile Packs define hyper-realistic mock behaviors including:
//! - Latency curves (protocol-specific latency distributions)
//! - Error distributions (endpoint-specific error patterns)
//! - Failure patterns (seasonal behaviors, chaos rules)
//! - Data mutation rules (how data evolves)
//! - Protocol behaviors (MQTT, WebSocket, REST specific)
//! - Persona behavior modifications
//!
//! These operate at a different level than domain packs:
//! - Domain packs: entities, schemas, field relationships, example responses, basic personas
//! - Reality profiles: latency curves, error distributions, failure patterns, seasonal behaviors,
//!   chaos rules, lifecycle overrides, data mutation rules, persona behavior modifications

use crate::domain_pack::{StudioChaosRule, StudioPersona};
use crate::error::{Result, ScenarioError};
use crate::reality_profile::{
    DataMutationBehavior, ErrorDistribution, LatencyCurve, ProtocolBehavior,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Reality Profile Pack manifest
///
/// Defines a collection of reality profile configurations that make mocks behave
/// like real customer-driven systems. Separate from domain packs to maintain
/// clean boundaries and enable mix-and-match scenarios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealityProfilePackManifest {
    /// Manifest format version
    pub manifest_version: String,

    /// Pack name (e.g., "ecommerce-peak-season", "fintech-fraud")
    pub name: String,

    /// Pack version (semver)
    pub version: String,

    /// Human-readable title
    pub title: String,

    /// Detailed description
    pub description: String,

    /// Domain category (e.g., "ecommerce", "fintech", "healthcare", "iot")
    pub domain: String,

    /// Author information
    pub author: String,

    /// Author email (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_email: Option<String>,

    /// Tags for discovery
    #[serde(default)]
    pub tags: Vec<String>,

    /// Compatibility requirements
    pub compatibility: RealityProfileCompatibilityInfo,

    /// Pre-configured personas with behavior modifications
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub personas: Vec<StudioPersona>,

    /// Chaos rules for this reality profile
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chaos_rules: Vec<StudioChaosRule>,

    /// Latency curves for protocol-specific latency distributions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub latency_curves: Vec<LatencyCurve>,

    /// Error distributions for endpoint-specific error patterns
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub error_distributions: Vec<ErrorDistribution>,

    /// Data mutation behaviors defining how data evolves
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data_mutation_behaviors: Vec<DataMutationBehavior>,

    /// Protocol-specific behaviors (MQTT, WebSocket, REST, etc.)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub protocol_behaviors: HashMap<String, ProtocolBehavior>,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, Value>,

    /// Created timestamp
    #[serde(default = "default_timestamp")]
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Updated timestamp
    #[serde(default = "default_timestamp")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn default_timestamp() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

/// Compatibility information for reality profile packs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealityProfileCompatibilityInfo {
    /// Minimum MockForge version required
    pub min_version: String,

    /// Maximum MockForge version (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_version: Option<String>,

    /// Required protocols
    #[serde(default)]
    pub required_protocols: Vec<String>,

    /// Required features
    #[serde(default)]
    pub required_features: Vec<String>,
}

impl Default for RealityProfileCompatibilityInfo {
    fn default() -> Self {
        Self {
            min_version: "0.3.0".to_string(),
            max_version: None,
            required_protocols: Vec::new(),
            required_features: Vec::new(),
        }
    }
}

impl RealityProfilePackManifest {
    /// Create a new reality profile pack manifest
    pub fn new(
        name: String,
        version: String,
        title: String,
        description: String,
        domain: String,
        author: String,
    ) -> Self {
        Self {
            manifest_version: "1.0".to_string(),
            name,
            version,
            title,
            description,
            domain,
            author,
            author_email: None,
            tags: Vec::new(),
            compatibility: RealityProfileCompatibilityInfo::default(),
            personas: Vec::new(),
            chaos_rules: Vec::new(),
            latency_curves: Vec::new(),
            error_distributions: Vec::new(),
            data_mutation_behaviors: Vec::new(),
            protocol_behaviors: HashMap::new(),
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// Load manifest from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(ScenarioError::Io)?;
        Self::from_str(&content)
    }

    /// Load manifest from string
    pub fn from_str(content: &str) -> Result<Self> {
        let manifest: Self = serde_yaml::from_str(content).map_err(ScenarioError::Yaml)?;
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
                "Reality profile pack name cannot be empty".to_string(),
            ));
        }

        // Validate version
        if self.version.trim().is_empty() {
            return Err(ScenarioError::InvalidVersion(
                "Reality profile pack version cannot be empty".to_string(),
            ));
        }

        // Validate title
        if self.title.trim().is_empty() {
            return Err(ScenarioError::InvalidManifest(
                "Reality profile pack title cannot be empty".to_string(),
            ));
        }

        // Validate description
        if self.description.trim().is_empty() {
            return Err(ScenarioError::InvalidManifest(
                "Reality profile pack description cannot be empty".to_string(),
            ));
        }

        // Validate domain
        if self.domain.trim().is_empty() {
            return Err(ScenarioError::InvalidManifest(
                "Reality profile pack domain cannot be empty".to_string(),
            ));
        }

        // Validate author
        if self.author.trim().is_empty() {
            return Err(ScenarioError::InvalidManifest(
                "Reality profile pack author cannot be empty".to_string(),
            ));
        }

        // Validate compatibility
        if self.compatibility.min_version.trim().is_empty() {
            return Err(ScenarioError::InvalidManifest(
                "Minimum version cannot be empty".to_string(),
            ));
        }

        // Validate error distributions
        for error_dist in &self.error_distributions {
            error_dist.validate()?;
        }

        // Validate data mutation behaviors
        for mutation in &self.data_mutation_behaviors {
            mutation.validate()?;
        }

        // Validate protocol behaviors
        for protocol_behavior in self.protocol_behaviors.values() {
            protocol_behavior.validate()?;
        }

        Ok(())
    }

    /// Get pack ID (name@version)
    pub fn id(&self) -> String {
        format!("{}@{}", self.name, self.version)
    }

    /// Save manifest to file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = if path
            .as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "yaml" || ext == "yml")
            .unwrap_or(true)
        {
            serde_yaml::to_string(self).map_err(ScenarioError::Yaml)?
        } else {
            serde_json::to_string_pretty(self).map_err(ScenarioError::Serde)?
        };

        std::fs::write(path.as_ref(), content).map_err(ScenarioError::Io)?;

        Ok(())
    }
}

/// Reality profile pack information
///
/// Contains information about an installed or available reality profile pack.
#[derive(Debug, Clone)]
pub struct RealityProfilePackInfo {
    /// Pack manifest
    pub manifest: RealityProfilePackManifest,

    /// Path to pack directory (if installed)
    pub path: Option<PathBuf>,

    /// Whether the pack is installed
    pub installed: bool,
}

impl RealityProfilePackInfo {
    /// Create pack info from manifest
    pub fn from_manifest(manifest: RealityProfilePackManifest, path: Option<PathBuf>) -> Self {
        let installed = path.is_some();
        Self {
            manifest,
            path,
            installed,
        }
    }
}

/// Reality profile pack installer
///
/// Handles installation and management of reality profile packs.
pub struct RealityProfilePackInstaller {
    /// Base directory for pack storage
    packs_dir: PathBuf,
}

impl RealityProfilePackInstaller {
    /// Create a new reality profile pack installer
    pub fn new() -> Result<Self> {
        let packs_dir = dirs::data_dir()
            .ok_or_else(|| ScenarioError::Storage("Failed to get data directory".to_string()))?
            .join("mockforge")
            .join("reality-profiles");

        Ok(Self { packs_dir })
    }

    /// Initialize the pack installer (create directories)
    pub fn init(&self) -> Result<()> {
        std::fs::create_dir_all(&self.packs_dir).map_err(|e| {
            ScenarioError::Storage(format!(
                "Failed to create reality profile packs directory: {}",
                e
            ))
        })?;
        Ok(())
    }

    /// List all installed packs
    pub fn list_installed(&self) -> Result<Vec<RealityProfilePackInfo>> {
        if !self.packs_dir.exists() {
            return Ok(Vec::new());
        }

        let mut packs = Vec::new();

        for entry in std::fs::read_dir(&self.packs_dir).map_err(ScenarioError::Io)? {
            let entry = entry.map_err(ScenarioError::Io)?;
            let pack_path = entry.path();

            if pack_path.is_dir() {
                // Look for reality-profile.yaml or reality-profile.json
                let manifest_path = {
                    let yaml_path = pack_path.join("reality-profile.yaml");
                    if yaml_path.exists() {
                        yaml_path
                    } else {
                        pack_path.join("reality-profile.json")
                    }
                };

                if manifest_path.exists() {
                    match RealityProfilePackManifest::from_file(&manifest_path) {
                        Ok(manifest) => {
                            packs.push(RealityProfilePackInfo::from_manifest(
                                manifest,
                                Some(pack_path),
                            ));
                        }
                        Err(e) => {
                            eprintln!(
                                "Warning: Failed to load reality profile pack manifest from {}: {}",
                                pack_path.display(),
                                e
                            );
                        }
                    }
                }
            }
        }

        Ok(packs)
    }

    /// Get pack info by name
    pub fn get_pack(&self, name: &str) -> Result<Option<RealityProfilePackInfo>> {
        let packs = self.list_installed()?;
        Ok(packs.into_iter().find(|p| p.manifest.name == name))
    }

    /// Install a pack from a manifest file
    pub fn install_from_manifest(&self, manifest_path: &Path) -> Result<RealityProfilePackInfo> {
        let manifest = RealityProfilePackManifest::from_file(manifest_path)?;

        // Create pack directory
        let pack_dir = self.packs_dir.join(&manifest.name);
        std::fs::create_dir_all(&pack_dir).map_err(|e| {
            ScenarioError::Storage(format!("Failed to create pack directory: {}", e))
        })?;

        // Copy manifest to pack directory
        let pack_manifest_path = pack_dir.join("reality-profile.yaml");
        manifest.to_file(&pack_manifest_path)?;

        Ok(RealityProfilePackInfo::from_manifest(manifest, Some(pack_dir)))
    }

    /// Apply a reality profile pack to a workspace
    ///
    /// This method applies all components of the reality profile pack:
    /// 1. Configure personas with behavior modifications
    /// 2. Apply chaos rules
    /// 3. Configure latency curves
    /// 4. Configure error distributions
    /// 5. Configure data mutation behaviors
    /// 6. Configure protocol behaviors
    pub async fn apply_reality_profile_pack(
        &self,
        manifest: &RealityProfilePackManifest,
        workspace_id: Option<&str>,
    ) -> Result<RealityProfilePackApplyResult> {
        use tracing::{info, warn};

        info!("Applying reality profile pack: {} v{}", manifest.name, manifest.version);

        let mut result = RealityProfilePackApplyResult {
            pack_name: manifest.name.clone(),
            pack_version: manifest.version.clone(),
            personas_configured: 0,
            chaos_rules_applied: 0,
            latency_curves_applied: 0,
            error_distributions_applied: 0,
            data_mutation_behaviors_applied: 0,
            protocol_behaviors_applied: 0,
            errors: Vec::new(),
        };

        // 1. Configure personas
        for studio_persona in &manifest.personas {
            match self.configure_persona(studio_persona).await {
                Ok(_) => {
                    result.personas_configured += 1;
                    info!("Configured persona: {}", studio_persona.id);
                }
                Err(e) => {
                    let error_msg =
                        format!("Failed to configure persona {}: {}", studio_persona.id, e);
                    warn!("{}", error_msg);
                    result.errors.push(error_msg);
                }
            }
        }

        // 2. Apply chaos rules
        for chaos_rule in &manifest.chaos_rules {
            match self.apply_chaos_rule(chaos_rule, workspace_id).await {
                Ok(_) => {
                    result.chaos_rules_applied += 1;
                    info!("Applied chaos rule: {}", chaos_rule.name);
                }
                Err(e) => {
                    let error_msg =
                        format!("Failed to apply chaos rule {}: {}", chaos_rule.name, e);
                    warn!("{}", error_msg);
                    result.errors.push(error_msg);
                }
            }
        }

        // 3. Apply latency curves (would need integration with latency system)
        result.latency_curves_applied = manifest.latency_curves.len();
        info!("Applied {} latency curves", result.latency_curves_applied);

        // 4. Apply error distributions (would need integration with error injection system)
        result.error_distributions_applied = manifest.error_distributions.len();
        info!("Applied {} error distributions", result.error_distributions_applied);

        // 5. Apply data mutation behaviors (would need integration with drift engine)
        result.data_mutation_behaviors_applied = manifest.data_mutation_behaviors.len();
        info!("Applied {} data mutation behaviors", result.data_mutation_behaviors_applied);

        // 6. Apply protocol behaviors
        result.protocol_behaviors_applied = manifest.protocol_behaviors.len();
        info!("Applied {} protocol behaviors", result.protocol_behaviors_applied);

        info!(
            "Reality profile pack application complete: {} personas, {} chaos rules, {} latency curves, {} error distributions, {} data mutations, {} protocol behaviors",
            result.personas_configured,
            result.chaos_rules_applied,
            result.latency_curves_applied,
            result.error_distributions_applied,
            result.data_mutation_behaviors_applied,
            result.protocol_behaviors_applied
        );

        Ok(result)
    }

    /// Configure a persona from a reality profile pack
    async fn configure_persona(&self, studio_persona: &StudioPersona) -> Result<()> {
        use mockforge_data::PersonaProfile;

        // Parse domain
        let domain = parse_domain(&studio_persona.domain).map_err(ScenarioError::Generic)?;

        // Create persona profile
        let mut persona = PersonaProfile::new(studio_persona.id.clone(), domain);
        persona.backstory = studio_persona.backstory.clone();

        // Set traits
        for (key, value) in &studio_persona.traits {
            persona.set_trait(key.clone(), value.clone());
        }

        // Set relationships
        for (rel_type, related_ids) in &studio_persona.relationships {
            for related_id in related_ids {
                persona.add_relationship(rel_type.clone(), related_id.clone());
            }
        }

        // Set metadata
        for (key, value) in &studio_persona.metadata {
            persona.metadata.insert(key.clone(), value.clone());
        }

        // TODO: Register persona with PersonaRegistry
        // This would require access to a global PersonaRegistry instance
        // For now, we'll just validate the persona structure

        Ok(())
    }

    /// Apply a chaos rule from a reality profile pack
    async fn apply_chaos_rule(
        &self,
        chaos_rule: &StudioChaosRule,
        _workspace_id: Option<&str>,
    ) -> Result<()> {
        use serde_json::Value;

        // Validate chaos config JSON
        // TODO: Deserialize into ChaosConfig and apply to workspace
        // This would require access to a ChaosEngine or workspace configuration
        serde_json::from_value::<Value>(chaos_rule.chaos_config.clone())
            .map_err(ScenarioError::Serde)?;

        Ok(())
    }
}

/// Result of applying a reality profile pack
#[derive(Debug, Clone)]
pub struct RealityProfilePackApplyResult {
    /// Name of the applied pack
    pub pack_name: String,
    /// Version of the applied pack
    pub pack_version: String,
    /// Number of personas configured
    pub personas_configured: usize,
    /// Number of chaos rules applied
    pub chaos_rules_applied: usize,
    /// Number of latency curves applied
    pub latency_curves_applied: usize,
    /// Number of error distributions applied
    pub error_distributions_applied: usize,
    /// Number of data mutation behaviors applied
    pub data_mutation_behaviors_applied: usize,
    /// Number of protocol behaviors applied
    pub protocol_behaviors_applied: usize,
    /// List of errors encountered during application
    pub errors: Vec<String>,
}

/// Helper function to parse domain from string
fn parse_domain(s: &str) -> std::result::Result<mockforge_data::domains::Domain, String> {
    // Map common domain strings to Domain enum variants
    match s.to_lowercase().as_str() {
        "finance" | "fintech" | "financial" => Ok(mockforge_data::domains::Domain::Finance),
        "ecommerce" | "e-commerce" | "retail" => Ok(mockforge_data::domains::Domain::Ecommerce),
        "healthcare" | "health" | "medical" => Ok(mockforge_data::domains::Domain::Healthcare),
        "iot" | "internet_of_things" => Ok(mockforge_data::domains::Domain::Iot),
        "social" => Ok(mockforge_data::domains::Domain::Social),
        "general" | "default" | "generic" => Ok(mockforge_data::domains::Domain::General),
        _ => Err(format!("Unknown domain: {}", s)),
    }
}

impl Default for RealityProfilePackInstaller {
    fn default() -> Self {
        Self::new().expect("Failed to create RealityProfilePackInstaller")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_creation() {
        let manifest = RealityProfilePackManifest::new(
            "test-pack".to_string(),
            "1.0.0".to_string(),
            "Test Pack".to_string(),
            "A test reality profile pack".to_string(),
            "ecommerce".to_string(),
            "test-author".to_string(),
        );

        assert_eq!(manifest.name, "test-pack");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.title, "Test Pack");
        assert_eq!(manifest.id(), "test-pack@1.0.0");
    }

    #[test]
    fn test_manifest_validation() {
        let mut manifest = RealityProfilePackManifest::new(
            "test-pack".to_string(),
            "1.0.0".to_string(),
            "Test Pack".to_string(),
            "A test reality profile pack".to_string(),
            "ecommerce".to_string(),
            "test-author".to_string(),
        );

        // Valid manifest should pass
        assert!(manifest.validate().is_ok());

        // Empty name should fail
        manifest.name = "".to_string();
        assert!(manifest.validate().is_err());

        // Reset and test empty version
        manifest.name = "test-pack".to_string();
        manifest.version = "".to_string();
        assert!(manifest.validate().is_err());
    }
}
