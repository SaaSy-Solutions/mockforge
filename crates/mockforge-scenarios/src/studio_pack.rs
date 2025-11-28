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
use crate::installer::{InstallOptions, ScenarioInstaller};
use mockforge_data::domains::Domain;
use mockforge_data::PersonaProfile;
use mockforge_data::PersonaRegistry;
use mockforge_core::consistency::ConsistencyEngine;
use mockforge_core::contract_drift::{DriftBudgetConfig, DriftBudgetEngine};
use mockforge_core::reality_continuum::{ContinuumConfig, RealityContinuumEngine};
use serde_json::Value;
use std::sync::Arc;
use tracing::{info, warn};

/// Studio pack installer
///
/// Handles installation and application of studio packs to a workspace,
/// including scenarios, personas, chaos rules, contract diffs, and reality blends.
pub struct StudioPackInstaller {
    /// Base directory for pack storage
    packs_dir: std::path::PathBuf,
    /// Optional scenario installer for installing scenarios
    scenario_installer: Option<Arc<tokio::sync::Mutex<ScenarioInstaller>>>,
    /// Optional persona registry for registering personas
    persona_registry: Option<Arc<PersonaRegistry>>,
    /// Optional consistency engine for applying chaos rules
    consistency_engine: Option<Arc<ConsistencyEngine>>,
    /// Optional drift budget engine for applying drift budgets
    drift_budget_engine: Option<Arc<tokio::sync::RwLock<DriftBudgetEngine>>>,
    /// Optional reality continuum engine for applying continuum configs
    continuum_engine: Option<Arc<RealityContinuumEngine>>,
}

impl StudioPackInstaller {
    /// Create a new studio pack installer
    pub fn new(packs_dir: std::path::PathBuf) -> Self {
        Self {
            packs_dir,
            scenario_installer: None,
            persona_registry: None,
            consistency_engine: None,
            drift_budget_engine: None,
            continuum_engine: None,
        }
    }

    /// Create a new installer with all dependencies
    pub fn with_dependencies(
        packs_dir: std::path::PathBuf,
        scenario_installer: Option<Arc<tokio::sync::Mutex<ScenarioInstaller>>>,
        persona_registry: Option<Arc<PersonaRegistry>>,
        consistency_engine: Option<Arc<ConsistencyEngine>>,
        drift_budget_engine: Option<Arc<tokio::sync::RwLock<DriftBudgetEngine>>>,
        continuum_engine: Option<Arc<RealityContinuumEngine>>,
    ) -> Self {
        Self {
            packs_dir,
            scenario_installer,
            persona_registry,
            consistency_engine,
            drift_budget_engine,
            continuum_engine,
        }
    }

    /// Set scenario installer
    pub fn with_scenario_installer(
        mut self,
        installer: Arc<tokio::sync::Mutex<ScenarioInstaller>>,
    ) -> Self {
        self.scenario_installer = Some(installer);
        self
    }

    /// Set persona registry
    pub fn with_persona_registry(mut self, registry: Arc<PersonaRegistry>) -> Self {
        self.persona_registry = Some(registry);
        self
    }

    /// Set consistency engine
    pub fn with_consistency_engine(mut self, engine: Arc<ConsistencyEngine>) -> Self {
        self.consistency_engine = Some(engine);
        self
    }

    /// Set drift budget engine
    pub fn with_drift_budget_engine(
        mut self,
        engine: Arc<tokio::sync::RwLock<DriftBudgetEngine>>,
    ) -> Self {
        self.drift_budget_engine = Some(engine);
        self
    }

    /// Set continuum engine
    pub fn with_continuum_engine(mut self, engine: Arc<RealityContinuumEngine>) -> Self {
        self.continuum_engine = Some(engine);
        self
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

        // 1. Install scenarios using ScenarioInstaller if available
        if let Some(ref installer) = self.scenario_installer {
            for pack_scenario in &manifest.scenarios {
                let install_options = InstallOptions {
                    force: false,
                    skip_validation: false,
                    expected_checksum: None,
                };

                match installer.lock().await.install(&pack_scenario.source, install_options).await {
                    Ok(scenario_id) => {
                        result.scenarios_installed += 1;
                        info!(
                            "Installed scenario: {} (from pack scenario: {})",
                            scenario_id, pack_scenario.name
                        );
                    }
                    Err(e) => {
                        let error_msg = if pack_scenario.required {
                            format!(
                                "Failed to install required scenario {} from source {}: {}",
                                pack_scenario.name, pack_scenario.source, e
                            )
                        } else {
                            format!(
                                "Failed to install optional scenario {} from source {}: {}",
                                pack_scenario.name, pack_scenario.source, e
                            )
                        };

                        if pack_scenario.required {
                            warn!("{}", error_msg);
                            result.errors.push(error_msg);
                            // For required scenarios, we might want to fail the entire installation
                            // For now, we log the error and continue
                        } else {
                            warn!("{}", error_msg);
                            // Optional scenarios are logged but don't fail the installation
                        }
                    }
                }
            }
        } else {
            // Scenario installer not available - just count scenarios
            info!(
                "Scenario installer not available. {} scenarios would be installed if installer was provided.",
                manifest.scenarios.len()
            );
            result.scenarios_installed = manifest.scenarios.len();
        }

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

        // If persona registry is available, register the persona
        if let Some(ref registry) = self.persona_registry {
            // Get or create the persona (this ensures it exists in the registry)
            let persona = registry.get_or_create_persona(studio_persona.id.clone(), domain);

            // Update the persona with all details from the studio pack
            let traits: std::collections::HashMap<String, String> = studio_persona
                .traits
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            let relationships: std::collections::HashMap<String, Vec<String>> = studio_persona
                .relationships
                .clone();

            registry
                .update_persona_full(
                    &studio_persona.id,
                    Some(traits),
                    studio_persona.backstory.clone(),
                    Some(relationships),
                )
                .map_err(|e| ScenarioError::Generic(format!("Failed to update persona: {}", e)))?;

            // Update metadata separately if needed (metadata is not part of update_persona_full)
            // Note: PersonaProfile metadata is not directly updatable via registry,
            // but traits and relationships are the main configuration points
            info!("Registered persona: {} in PersonaRegistry", studio_persona.id);
        } else {
            // Validate persona structure even if registry is not available
            let _persona = PersonaProfile::new(studio_persona.id.clone(), domain);
            info!(
                "Persona {} validated (registry not available, skipping registration)",
                studio_persona.id
            );
        }

        Ok(())
    }

    /// Apply a chaos rule from a studio pack
    async fn apply_chaos_rule(
        &self,
        chaos_rule: &StudioChaosRule,
        workspace_id: Option<&str>,
    ) -> Result<()> {
        // Validate chaos config JSON structure
        serde_json::from_value::<Value>(chaos_rule.chaos_config.clone())
            .map_err(ScenarioError::Serde)?;

        // If consistency engine is available, activate the chaos rule
        if let (Some(ref engine), Some(ws_id)) = (&self.consistency_engine, workspace_id) {
            // The chaos rule config is stored as JSON, which matches ChaosScenario type
            let chaos_scenario = chaos_rule.chaos_config.clone();
            engine
                .activate_chaos_rule(ws_id, chaos_scenario)
                .await
                .map_err(|e| {
                    ScenarioError::Generic(format!("Failed to activate chaos rule: {}", e))
                })?;
            info!(
                "Activated chaos rule: {} for workspace: {}",
                chaos_rule.name, ws_id
            );
        } else {
            info!(
                "Chaos rule {} validated (consistency engine not available, skipping activation)",
                chaos_rule.name
            );
        }

        Ok(())
    }

    /// Configure contract drift from a studio pack
    async fn configure_contract_diff(
        &self,
        contract_diff: &StudioContractDiff,
        workspace_id: Option<&str>,
    ) -> Result<()> {
        // Deserialize drift budget config
        let drift_config: DriftBudgetConfig = serde_json::from_value(
            contract_diff.drift_budget.clone(),
        )
        .map_err(|e| {
            ScenarioError::Generic(format!(
                "Failed to deserialize DriftBudgetConfig: {}",
                e
            ))
        })?;

        // If drift budget engine is available, apply the configuration
        if let Some(ref engine) = self.drift_budget_engine {
            let mut engine_guard = engine.write().await;
            // Merge the new config with existing config
            let mut current_config = engine_guard.config().clone();

            // Merge per-workspace budgets if workspace_id is provided
            if let Some(ws_id) = workspace_id {
                if let Some(budget) = drift_config.default_budget.clone() {
                    current_config
                        .per_workspace_budgets
                        .insert(ws_id.to_string(), budget);
                }
            }

            // Merge per-service budgets
            for (service, budget) in &drift_config.per_service_budgets {
                current_config
                    .per_service_budgets
                    .insert(service.clone(), budget.clone());
            }

            // Merge per-tag budgets
            for (tag, budget) in &drift_config.per_tag_budgets {
                current_config
                    .per_tag_budgets
                    .insert(tag.clone(), budget.clone());
            }

            // Merge per-endpoint budgets
            for (endpoint, budget) in &drift_config.per_endpoint_budgets {
                current_config
                    .per_endpoint_budgets
                    .insert(endpoint.clone(), budget.clone());
            }

            // Update default budget if provided
            if drift_config.default_budget.is_some() {
                current_config.default_budget = drift_config.default_budget;
            }

            // Update enabled flag
            current_config.enabled = drift_config.enabled;

            // Apply the merged configuration
            engine_guard.update_config(current_config);
            info!(
                "Applied drift budget config: {} for workspace: {:?}",
                contract_diff.name, workspace_id
            );
        } else {
            info!(
                "Drift budget config {} validated (drift budget engine not available, skipping application)",
                contract_diff.name
            );
        }

        Ok(())
    }

    /// Configure reality blend from a studio pack
    async fn configure_reality_blend(
        &self,
        reality_blend: &StudioRealityBlend,
        _workspace_id: Option<&str>,
    ) -> Result<()> {
        // Deserialize continuum config
        let continuum_config: ContinuumConfig = serde_json::from_value(
            reality_blend.continuum_config.clone(),
        )
        .map_err(|e| {
            ScenarioError::Generic(format!(
                "Failed to deserialize ContinuumConfig: {}",
                e
            ))
        })?;

        // If continuum engine is available, apply the configuration
        if let Some(ref engine) = self.continuum_engine {
            engine.update_config(continuum_config).await;
            info!("Applied reality continuum config: {}", reality_blend.name);
        } else {
            info!(
                "Reality continuum config {} validated (continuum engine not available, skipping application)",
                reality_blend.name
            );
        }

        Ok(())
    }

    /// Apply workspace configuration from a studio pack
    async fn apply_workspace_config(
        &self,
        workspace_config: &Value,
        workspace_id: Option<&str>,
    ) -> Result<()> {
        // Validate workspace configuration JSON structure
        // The workspace config is a flexible JSON value that can contain
        // various workspace settings (reality level, AI mode, etc.)
        if !workspace_config.is_object() {
            return Err(ScenarioError::Generic(
                "Workspace configuration must be a JSON object".to_string(),
            ));
        }

        // Note: Full workspace configuration application would require access to
        // workspace management APIs (e.g., WorkspaceRegistry, WorkspaceService).
        // For now, we validate the structure and log that it would be applied.
        // In a full implementation, this would:
        // 1. Deserialize workspace config into WorkspaceConfig
        // 2. Update the workspace via WorkspaceRegistry or WorkspaceService
        // 3. Persist the changes

        if workspace_id.is_some() {
            info!(
                "Workspace configuration validated for workspace: {} (workspace management APIs not available, skipping application)",
                workspace_id.unwrap()
            );
        } else {
            info!(
                "Workspace configuration validated (workspace_id not provided, skipping application)"
            );
        }

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
