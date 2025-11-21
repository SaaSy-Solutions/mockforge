//! Debug context integrator for collecting context from multiple subsystems
//!
//! This module provides functionality to collect debugging context from various
//! MockForge subsystems (Reality, Contracts, Scenarios, Personas, Chaos) and
//! combine them into a unified DebugContext for AI-guided debugging.

use crate::ai_studio::debug_context::{
    ChaosContext, ChaosRule, ContractContext, ContractValidationResult, DebugContext,
    DriftHistoryEntry, FailureInjectionConfig, PersonaContext, RealityContext, ScenarioContext,
};
use crate::reality::{RealityConfig, RealityEngine, RealityLevel};
use crate::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// Trait for accessing reality subsystem
#[async_trait]
pub trait RealityAccessor: Send + Sync {
    /// Get current reality level
    async fn get_level(&self) -> Option<RealityLevel>;
    /// Get reality configuration
    async fn get_config(&self) -> Option<RealityConfig>;
}

/// Trait for accessing contract subsystem
#[async_trait]
pub trait ContractAccessor: Send + Sync {
    /// Get contract validation results
    async fn get_validation_result(&self, workspace_id: Option<&str>) -> Result<Option<ContractValidationResult>>;
    /// Get contract enforcement mode
    async fn get_enforcement_mode(&self, workspace_id: Option<&str>) -> Result<String>;
    /// Get drift history
    async fn get_drift_history(&self, workspace_id: Option<&str>) -> Result<Vec<DriftHistoryEntry>>;
    /// Get active contract paths
    async fn get_active_contracts(&self, workspace_id: Option<&str>) -> Result<Vec<String>>;
}

/// Trait for accessing scenario subsystem
#[async_trait]
pub trait ScenarioAccessor: Send + Sync {
    /// Get active scenario ID
    async fn get_active_scenario(&self, workspace_id: Option<&str>) -> Result<Option<String>>;
    /// Get current state machine state
    async fn get_current_state(&self, workspace_id: Option<&str>, scenario_id: Option<&str>) -> Result<Option<String>>;
    /// Get available state transitions
    async fn get_available_transitions(&self, workspace_id: Option<&str>, scenario_id: Option<&str>) -> Result<Vec<String>>;
    /// Get scenario configuration
    async fn get_scenario_config(&self, workspace_id: Option<&str>, scenario_id: Option<&str>) -> Result<Option<Value>>;
}

/// Trait for accessing persona subsystem
#[async_trait]
pub trait PersonaAccessor: Send + Sync {
    /// Get active persona ID
    async fn get_active_persona_id(&self, workspace_id: Option<&str>) -> Result<Option<String>>;
    /// Get persona details
    async fn get_persona_details(&self, workspace_id: Option<&str>, persona_id: Option<&str>) -> Result<Option<PersonaDetails>>;
}

/// Persona details for debug context
#[derive(Debug, Clone)]
pub struct PersonaDetails {
    /// Persona ID
    pub id: String,
    /// Persona name
    pub name: Option<String>,
    /// Persona traits (key-value pairs)
    pub traits: HashMap<String, String>,
    /// Persona domain (e.g., "ecommerce", "saas")
    pub domain: Option<String>,
    /// Persona backstory/narrative
    pub backstory: Option<String>,
    /// Persona relationships to other entities
    pub relationships: HashMap<String, Vec<String>>,
    /// Current lifecycle state (if applicable)
    pub lifecycle_state: Option<String>,
}

/// Trait for accessing chaos subsystem
#[async_trait]
pub trait ChaosAccessor: Send + Sync {
    /// Get chaos configuration
    async fn get_chaos_config(&self, workspace_id: Option<&str>) -> Result<Option<ChaosConfig>>;
}

/// Chaos configuration structure
/// Chaos configuration for debug context
#[derive(Debug, Clone)]
pub struct ChaosConfig {
    /// Whether chaos engineering is enabled
    pub enabled: bool,
    /// Active chaos rules
    pub active_rules: Vec<ChaosRule>,
    /// Failure injection configuration
    pub failure_injection: Option<FailureInjectionConfig>,
    /// Chaos tags
    pub tags: Vec<String>,
}

/// Debug context integrator
///
/// Collects context from multiple subsystems and combines them into a unified DebugContext.
/// All accessors are optional - if not provided, the corresponding context will be empty/default.
pub struct DebugContextIntegrator {
    /// Optional reality accessor
    reality_accessor: Option<Box<dyn RealityAccessor>>,
    /// Optional contract accessor
    contract_accessor: Option<Box<dyn ContractAccessor>>,
    /// Optional scenario accessor
    scenario_accessor: Option<Box<dyn ScenarioAccessor>>,
    /// Optional persona accessor
    persona_accessor: Option<Box<dyn PersonaAccessor>>,
    /// Optional chaos accessor
    chaos_accessor: Option<Box<dyn ChaosAccessor>>,
}

impl DebugContextIntegrator {
    /// Create a new debug context integrator
    pub fn new() -> Self {
        Self {
            reality_accessor: None,
            contract_accessor: None,
            scenario_accessor: None,
            persona_accessor: None,
            chaos_accessor: None,
        }
    }

    /// Set reality accessor
    pub fn with_reality(mut self, accessor: Box<dyn RealityAccessor>) -> Self {
        self.reality_accessor = Some(accessor);
        self
    }

    /// Set contract accessor
    pub fn with_contract(mut self, accessor: Box<dyn ContractAccessor>) -> Self {
        self.contract_accessor = Some(accessor);
        self
    }

    /// Set scenario accessor
    pub fn with_scenario(mut self, accessor: Box<dyn ScenarioAccessor>) -> Self {
        self.scenario_accessor = Some(accessor);
        self
    }

    /// Set persona accessor
    pub fn with_persona(mut self, accessor: Box<dyn PersonaAccessor>) -> Self {
        self.persona_accessor = Some(accessor);
        self
    }

    /// Set chaos accessor
    pub fn with_chaos(mut self, accessor: Box<dyn ChaosAccessor>) -> Self {
        self.chaos_accessor = Some(accessor);
        self
    }

    /// Collect unified context from all subsystems
    ///
    /// This method queries all available subsystems and combines their contexts
    /// into a single DebugContext structure.
    pub async fn collect_unified_context(&self, workspace_id: Option<&str>) -> Result<DebugContext> {
        let reality = self.collect_reality_context().await?;
        let contract = self.collect_contract_context(workspace_id).await?;
        let scenario = self.collect_scenario_context(workspace_id).await?;
        let persona = self.collect_persona_context(workspace_id).await?;
        let chaos = self.collect_chaos_context(workspace_id).await?;

        Ok(DebugContext {
            reality,
            contract,
            scenario,
            persona,
            chaos,
            collected_at: chrono::Utc::now(),
        })
    }

    /// Collect reality subsystem context
    async fn collect_reality_context(&self) -> Result<RealityContext> {
        if let Some(accessor) = &self.reality_accessor {
            let level = accessor.get_level().await;
            let config = accessor.get_config().await;

            if let Some(config) = config {
                Ok(RealityContext::from_config(&config))
            } else if let Some(level) = level {
                // Create minimal context from level only
                Ok(RealityContext {
                    level: Some(level),
                    level_name: Some(level.name().to_string()),
                    ..Default::default()
                })
            } else {
                Ok(RealityContext::default())
            }
        } else {
            Ok(RealityContext::default())
        }
    }

    /// Collect contract subsystem context
    async fn collect_contract_context(&self, workspace_id: Option<&str>) -> Result<ContractContext> {
        if let Some(accessor) = &self.contract_accessor {
            let validation_result = accessor.get_validation_result(workspace_id).await?;
            let enforcement_mode = accessor.get_enforcement_mode(workspace_id).await?;
            let drift_history = accessor.get_drift_history(workspace_id).await?;
            let active_contracts = accessor.get_active_contracts(workspace_id).await?;

            let validation_enabled = validation_result.is_some();
            let validation_errors = validation_result
                .as_ref()
                .map(|r| r.errors.clone())
                .unwrap_or_default();

            Ok(ContractContext {
                validation_enabled,
                validation_result,
                enforcement_mode,
                drift_history,
                active_contracts,
                validation_errors,
            })
        } else {
            Ok(ContractContext::default())
        }
    }

    /// Collect scenario subsystem context
    async fn collect_scenario_context(&self, workspace_id: Option<&str>) -> Result<ScenarioContext> {
        if let Some(accessor) = &self.scenario_accessor {
            let active_scenario = accessor.get_active_scenario(workspace_id).await?;
            let current_state = if let Some(ref scenario_id) = active_scenario {
                accessor.get_current_state(workspace_id, Some(scenario_id)).await?
            } else {
                None
            };
            let available_transitions = if let Some(ref scenario_id) = active_scenario {
                accessor.get_available_transitions(workspace_id, Some(scenario_id)).await?
            } else {
                Vec::new()
            };
            let scenario_config = if let Some(ref scenario_id) = active_scenario {
                accessor.get_scenario_config(workspace_id, Some(scenario_id)).await?
            } else {
                None
            };

            Ok(ScenarioContext {
                active_scenario,
                current_state,
                available_transitions,
                scenario_config,
            })
        } else {
            Ok(ScenarioContext::default())
        }
    }

    /// Collect persona subsystem context
    async fn collect_persona_context(&self, workspace_id: Option<&str>) -> Result<PersonaContext> {
        if let Some(accessor) = &self.persona_accessor {
            let active_persona_id = accessor.get_active_persona_id(workspace_id).await?;
            let persona_details = if let Some(ref persona_id) = active_persona_id {
                accessor.get_persona_details(workspace_id, Some(persona_id)).await?
            } else {
                None
            };

            if let Some(details) = persona_details {
                Ok(PersonaContext {
                    active_persona_id: Some(details.id.clone()),
                    active_persona_name: details.name,
                    traits: details.traits,
                    domain: details.domain,
                    backstory: details.backstory,
                    relationships: details.relationships,
                    lifecycle_state: details.lifecycle_state,
                })
            } else if let Some(persona_id) = active_persona_id {
                // We have an ID but no details
                Ok(PersonaContext {
                    active_persona_id: Some(persona_id),
                    ..Default::default()
                })
            } else {
                Ok(PersonaContext::default())
            }
        } else {
            Ok(PersonaContext::default())
        }
    }

    /// Collect chaos subsystem context
    async fn collect_chaos_context(&self, workspace_id: Option<&str>) -> Result<ChaosContext> {
        if let Some(accessor) = &self.chaos_accessor {
            if let Some(config) = accessor.get_chaos_config(workspace_id).await? {
                Ok(ChaosContext {
                    enabled: config.enabled,
                    active_rules: config.active_rules,
                    failure_injection: config.failure_injection,
                    tags: config.tags,
                })
            } else {
                Ok(ChaosContext::default())
            }
        } else {
            Ok(ChaosContext::default())
        }
    }
}

impl Default for DebugContextIntegrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Implementation of RealityAccessor for RealityEngine
pub struct RealityEngineAccessor {
    engine: std::sync::Arc<tokio::sync::RwLock<RealityEngine>>,
}

impl RealityEngineAccessor {
    /// Create a new reality engine accessor
    pub fn new(engine: std::sync::Arc<tokio::sync::RwLock<RealityEngine>>) -> Self {
        Self { engine }
    }
}

#[async_trait]
impl RealityAccessor for RealityEngineAccessor {
    async fn get_level(&self) -> Option<RealityLevel> {
        Some(self.engine.read().await.get_level().await)
    }

    async fn get_config(&self) -> Option<RealityConfig> {
        Some(self.engine.read().await.get_config().await)
    }
}

