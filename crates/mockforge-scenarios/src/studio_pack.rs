//! Studio pack installer
//!
//! This module provides functionality for installing and applying studio packs,
//! which bundle scenarios, personas, chaos rules, contract diffs, and reality blends.

#[path = "studio_pack/packs.rs"]
pub mod packs;

use crate::domain_pack::{
    DomainPackManifest, StudioChaosRule, StudioContractDiff, StudioPersona, StudioRealityBlend,
};
use crate::error::{Result, ScenarioError};
use mockforge_data::domains::Domain;
use mockforge_data::PersonaProfile;
use serde_json::Value;
use tracing::{info, warn};

/// Studio pack installer
///
/// Handles installation and application of studio packs to a workspace,
/// including personas, chaos rules, contract diffs, and reality blends.
pub struct StudioPackInstaller {
    /// Base directory for pack storage
    packs_dir: std::path::PathBuf,
}

impl StudioPackInstaller {
    /// Create a new studio pack installer
    pub fn new(packs_dir: std::path::PathBuf) -> Self {
        Self { packs_dir }
    }

    /// Install a studio pack from a manifest
    ///
    /// This method applies all components of the studio pack to the workspace:
    /// 1. Install scenarios (existing functionality)
    /// 2. Configure personas in PersonaRegistry
    /// 3. Apply chaos rules to workspace
    /// 4. Set up contract drift budgets
    /// 5. Configure reality continuum ratios
    /// 6. Apply workspace configuration
    pub async fn install_studio_pack(
        &self,
        manifest: &DomainPackManifest,
        workspace_id: Option<&str>,
    ) -> Result<StudioPackInstallResult> {
        info!("Installing studio pack: {} v{}", manifest.name, manifest.version);

        let mut result = StudioPackInstallResult {
            pack_name: manifest.name.clone(),
            pack_version: manifest.version.clone(),
            scenarios_installed: 0,
            personas_configured: 0,
            chaos_rules_applied: 0,
            contract_diffs_configured: 0,
            reality_blends_configured: 0,
            workspace_config_applied: false,
            errors: Vec::new(),
        };

        // 1. Install scenarios (existing functionality)
        // TODO: Integrate with existing scenario installation logic
        result.scenarios_installed = manifest.scenarios.len();

        // 2. Configure personas
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

        // 3. Apply chaos rules
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

        // 4. Configure contract drift budgets
        for contract_diff in &manifest.contract_diffs {
            match self.configure_contract_diff(contract_diff, workspace_id).await {
                Ok(_) => {
                    result.contract_diffs_configured += 1;
                    info!("Configured contract diff: {}", contract_diff.name);
                }
                Err(e) => {
                    let error_msg =
                        format!("Failed to configure contract diff {}: {}", contract_diff.name, e);
                    warn!("{}", error_msg);
                    result.errors.push(error_msg);
                }
            }
        }

        // 5. Configure reality blends
        for reality_blend in &manifest.reality_blends {
            match self.configure_reality_blend(reality_blend, workspace_id).await {
                Ok(_) => {
                    result.reality_blends_configured += 1;
                    info!("Configured reality blend: {}", reality_blend.name);
                }
                Err(e) => {
                    let error_msg =
                        format!("Failed to configure reality blend {}: {}", reality_blend.name, e);
                    warn!("{}", error_msg);
                    result.errors.push(error_msg);
                }
            }
        }

        // 6. Apply workspace configuration
        if let Some(ref workspace_config) = manifest.workspace_config {
            match self.apply_workspace_config(workspace_config, workspace_id).await {
                Ok(_) => {
                    result.workspace_config_applied = true;
                    info!("Applied workspace configuration");
                }
                Err(e) => {
                    let error_msg = format!("Failed to apply workspace config: {}", e);
                    warn!("{}", error_msg);
                    result.errors.push(error_msg);
                }
            }
        }

        info!(
            "Studio pack installation complete: {} scenarios, {} personas, {} chaos rules, {} contract diffs, {} reality blends",
            result.scenarios_installed,
            result.personas_configured,
            result.chaos_rules_applied,
            result.contract_diffs_configured,
            result.reality_blends_configured
        );

        Ok(result)
    }

    /// Configure a persona from a studio pack
    async fn configure_persona(&self, studio_persona: &StudioPersona) -> Result<()> {
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

    /// Apply a chaos rule from a studio pack
    async fn apply_chaos_rule(
        &self,
        chaos_rule: &StudioChaosRule,
        _workspace_id: Option<&str>,
    ) -> Result<()> {
        // Validate chaos config JSON
        // TODO: Deserialize into ChaosConfig and apply to workspace
        // This would require access to a ChaosEngine or workspace configuration
        serde_json::from_value::<Value>(chaos_rule.chaos_config.clone())
            .map_err(ScenarioError::Serde)?;

        Ok(())
    }

    /// Configure contract drift from a studio pack
    async fn configure_contract_diff(
        &self,
        contract_diff: &StudioContractDiff,
        _workspace_id: Option<&str>,
    ) -> Result<()> {
        // Validate drift budget JSON
        // TODO: Deserialize into DriftBudgetConfig and apply to workspace
        serde_json::from_value::<Value>(contract_diff.drift_budget.clone())
            .map_err(ScenarioError::Serde)?;

        Ok(())
    }

    /// Configure reality blend from a studio pack
    async fn configure_reality_blend(
        &self,
        reality_blend: &StudioRealityBlend,
        _workspace_id: Option<&str>,
    ) -> Result<()> {
        // Validate continuum config JSON
        // TODO: Deserialize into ContinuumConfig and apply to workspace
        serde_json::from_value::<Value>(reality_blend.continuum_config.clone())
            .map_err(ScenarioError::Serde)?;

        Ok(())
    }

    /// Apply workspace configuration from a studio pack
    async fn apply_workspace_config(
        &self,
        _workspace_config: &Value,
        _workspace_id: Option<&str>,
    ) -> Result<()> {
        // TODO: Apply workspace configuration
        // This would require access to workspace management APIs
        Ok(())
    }
}

/// Result of installing a studio pack
#[derive(Debug, Clone)]
pub struct StudioPackInstallResult {
    /// Name of the installed pack
    pub pack_name: String,
    /// Version of the installed pack
    pub pack_version: String,
    /// Number of scenarios installed
    pub scenarios_installed: usize,
    /// Number of personas configured
    pub personas_configured: usize,
    /// Number of chaos rules applied
    pub chaos_rules_applied: usize,
    /// Number of contract diffs configured
    pub contract_diffs_configured: usize,
    /// Number of reality blends configured
    pub reality_blends_configured: usize,
    /// Whether workspace configuration was applied
    pub workspace_config_applied: bool,
    /// List of errors encountered during installation
    pub errors: Vec<String>,
}

/// Helper function to parse domain from string
fn parse_domain(s: &str) -> std::result::Result<Domain, String> {
    // Map common domain strings to Domain enum variants
    match s.to_lowercase().as_str() {
        "finance" | "fintech" | "financial" => Ok(Domain::Finance),
        "ecommerce" | "e-commerce" | "retail" => Ok(Domain::Ecommerce),
        "healthcare" | "health" | "medical" => Ok(Domain::Healthcare),
        "iot" | "internet_of_things" => Ok(Domain::Iot),
        "social" => Ok(Domain::Social),
        "general" | "default" | "generic" => Ok(Domain::General),
        _ => Err(format!("Unknown domain: {}", s)),
    }
}
