//! HTTP protocol adapter for consistency engine
//!
//! This adapter integrates HTTP protocol with the consistency engine,
//! ensuring HTTP responses reflect the unified state (persona, scenario, etc.)

use mockforge_core::consistency::adapters::ProtocolAdapter;
use mockforge_core::consistency::types::{PersonaProfile, ProtocolState, StateChangeEvent};
use mockforge_core::consistency::ConsistencyEngine;
use mockforge_core::protocol_abstraction::Protocol;
use mockforge_core::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// HTTP protocol adapter for consistency engine
///
/// This adapter listens to state change events from the consistency engine
/// and updates HTTP middleware/handlers to reflect the unified state.
pub struct HttpAdapter {
    /// Reference to the consistency engine
    engine: Arc<ConsistencyEngine>,
    /// Current persona for each workspace
    workspace_personas: Arc<RwLock<std::collections::HashMap<String, Option<PersonaProfile>>>>,
    /// Current scenario for each workspace
    workspace_scenarios: Arc<RwLock<std::collections::HashMap<String, Option<String>>>>,
}

impl HttpAdapter {
    /// Create a new HTTP adapter
    pub fn new(engine: Arc<ConsistencyEngine>) -> Self {
        Self {
            engine,
            workspace_personas: Arc::new(RwLock::new(std::collections::HashMap::new())),
            workspace_scenarios: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Get current persona for a workspace
    pub async fn get_persona(&self, workspace_id: &str) -> Option<PersonaProfile> {
        let personas = self.workspace_personas.read().await;
        personas.get(workspace_id).cloned().flatten()
    }

    /// Get current scenario for a workspace
    pub async fn get_scenario(&self, workspace_id: &str) -> Option<String> {
        let scenarios = self.workspace_scenarios.read().await;
        scenarios.get(workspace_id).cloned().flatten()
    }
}

#[async_trait::async_trait]
impl ProtocolAdapter for HttpAdapter {
    fn protocol(&self) -> Protocol {
        Protocol::Http
    }

    async fn on_state_change(&self, event: &StateChangeEvent) -> Result<()> {
        match event {
            StateChangeEvent::PersonaChanged {
                workspace_id,
                persona,
            } => {
                let mut personas = self.workspace_personas.write().await;
                personas.insert(workspace_id.clone(), Some(persona.clone()));
                info!(
                    "HTTP adapter: Updated persona for workspace {} to {}",
                    workspace_id, persona.id
                );
            }
            StateChangeEvent::ScenarioChanged {
                workspace_id,
                scenario_id,
            } => {
                let mut scenarios = self.workspace_scenarios.write().await;
                scenarios.insert(workspace_id.clone(), Some(scenario_id.clone()));
                info!(
                    "HTTP adapter: Updated scenario for workspace {} to {}",
                    workspace_id, scenario_id
                );
            }
            StateChangeEvent::RealityLevelChanged { workspace_id, .. } => {
                debug!("HTTP adapter: Reality level changed for workspace {}", workspace_id);
                // HTTP responses will use reality level from unified state when generating responses
            }
            StateChangeEvent::RealityRatioChanged { workspace_id, .. } => {
                debug!("HTTP adapter: Reality ratio changed for workspace {}", workspace_id);
                // HTTP responses will use reality ratio from unified state when blending responses
            }
            StateChangeEvent::EntityCreated {
                workspace_id,
                entity,
            } => {
                debug!(
                    "HTTP adapter: Entity created {}:{} for workspace {}",
                    entity.entity_type, entity.entity_id, workspace_id
                );
                // Entity is now available for HTTP endpoints to query
            }
            StateChangeEvent::EntityUpdated {
                workspace_id,
                entity,
            } => {
                debug!(
                    "HTTP adapter: Entity updated {}:{} for workspace {}",
                    entity.entity_type, entity.entity_id, workspace_id
                );
                // Updated entity is now available for HTTP endpoints
            }
            StateChangeEvent::ChaosRuleActivated { workspace_id, rule } => {
                // Note: ChaosScenario is now serde_json::Value, so we extract the name field
                let rule_name = rule.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                info!(
                    "HTTP adapter: Chaos rule '{}' activated for workspace {}",
                    rule_name, workspace_id
                );
                // Chaos rule will be applied to HTTP responses via chaos middleware
            }
            StateChangeEvent::ChaosRuleDeactivated {
                workspace_id,
                rule_name,
            } => {
                info!(
                    "HTTP adapter: Chaos rule '{}' deactivated for workspace {}",
                    rule_name, workspace_id
                );
                // Chaos rule will be removed from HTTP responses
            }
        }
        Ok(())
    }

    async fn get_current_state(&self, workspace_id: &str) -> Result<Option<ProtocolState>> {
        // Get protocol state from consistency engine
        let state = self.engine.get_protocol_state(workspace_id, Protocol::Http).await;
        Ok(state)
    }

    async fn apply_persona(&self, workspace_id: &str, persona: &PersonaProfile) -> Result<()> {
        let mut personas = self.workspace_personas.write().await;
        personas.insert(workspace_id.to_string(), Some(persona.clone()));
        info!("HTTP adapter: Applied persona {} to workspace {}", persona.id, workspace_id);
        Ok(())
    }

    async fn apply_scenario(&self, workspace_id: &str, scenario_id: &str) -> Result<()> {
        let mut scenarios = self.workspace_scenarios.write().await;
        scenarios.insert(workspace_id.to_string(), Some(scenario_id.to_string()));
        info!("HTTP adapter: Applied scenario {} to workspace {}", scenario_id, workspace_id);
        Ok(())
    }
}
