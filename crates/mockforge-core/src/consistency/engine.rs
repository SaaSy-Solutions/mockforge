//! Consistency engine implementation
//!
//! The consistency engine coordinates state across all protocols, ensuring
//! that persona, scenario, reality level, and entity state are synchronized.

use crate::consistency::adapters::ProtocolAdapter;
use crate::consistency::types::{
    EntityState, ProtocolState, StateChangeEvent, UnifiedState,
};
use crate::protocol_abstraction::Protocol;
use crate::reality::RealityLevel;
use crate::Result;
use mockforge_chaos::ChaosScenario;
use mockforge_data::PersonaProfile;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

/// Consistency engine for coordinating state across all protocols
///
/// The engine maintains unified state per workspace and broadcasts state
/// changes to all registered protocol adapters. This ensures that all
/// protocols reflect the same persona, scenario, reality level, and entity state.
pub struct ConsistencyEngine {
    /// Workspace state storage (workspace_id -> UnifiedState)
    states: Arc<RwLock<HashMap<String, UnifiedState>>>,
    /// Event broadcaster for state change notifications
    event_tx: broadcast::Sender<StateChangeEvent>,
    /// Registered protocol adapters
    adapters: Arc<RwLock<Vec<Arc<dyn ProtocolAdapter + Send + Sync>>>>,
}

impl ConsistencyEngine {
    /// Create a new consistency engine
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            adapters: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a protocol adapter
    ///
    /// Adapters are notified of all state changes for their protocol.
    pub async fn register_adapter(&self, adapter: Arc<dyn ProtocolAdapter + Send + Sync>) {
        let mut adapters = self.adapters.write().await;
        adapters.push(adapter);
        info!("Registered protocol adapter");
    }

    /// Get or create unified state for a workspace
    pub async fn get_or_create_state(&self, workspace_id: &str) -> UnifiedState {
        let mut states = self.states.write().await;
        states
            .entry(workspace_id.to_string())
            .or_insert_with(|| UnifiedState::new(workspace_id.to_string()))
            .clone()
    }

    /// Get unified state for a workspace (returns None if not found)
    pub async fn get_state(&self, workspace_id: &str) -> Option<UnifiedState> {
        let states = self.states.read().await;
        states.get(workspace_id).cloned()
    }

    /// Set active persona for a workspace
    ///
    /// This updates the unified state and broadcasts the change to all
    /// protocol adapters, ensuring all protocols use the new persona.
    pub async fn set_active_persona(
        &self,
        workspace_id: &str,
        persona: PersonaProfile,
    ) -> Result<()> {
        let mut states = self.states.write().await;
        let state = states
            .entry(workspace_id.to_string())
            .or_insert_with(|| UnifiedState::new(workspace_id.to_string()));

        state.active_persona = Some(persona.clone());
        state.increment_version();

        let event = StateChangeEvent::PersonaChanged {
            workspace_id: workspace_id.to_string(),
            persona,
        };

        drop(states); // Release lock before broadcasting

        self.broadcast_event(&event).await;
        info!("Set active persona for workspace {}", workspace_id);
        Ok(())
    }

    /// Set active scenario for a workspace
    pub async fn set_active_scenario(
        &self,
        workspace_id: &str,
        scenario_id: String,
    ) -> Result<()> {
        let mut states = self.states.write().await;
        let state = states
            .entry(workspace_id.to_string())
            .or_insert_with(|| UnifiedState::new(workspace_id.to_string()));

        state.active_scenario = Some(scenario_id.clone());
        state.increment_version();

        let event = StateChangeEvent::ScenarioChanged {
            workspace_id: workspace_id.to_string(),
            scenario_id,
        };

        drop(states);

        self.broadcast_event(&event).await;
        info!("Set active scenario for workspace {}", workspace_id);
        Ok(())
    }

    /// Set reality level for a workspace
    pub async fn set_reality_level(
        &self,
        workspace_id: &str,
        level: RealityLevel,
    ) -> Result<()> {
        let mut states = self.states.write().await;
        let state = states
            .entry(workspace_id.to_string())
            .or_insert_with(|| UnifiedState::new(workspace_id.to_string()));

        state.reality_level = level;
        state.increment_version();

        let event = StateChangeEvent::RealityLevelChanged {
            workspace_id: workspace_id.to_string(),
            level,
        };

        drop(states);

        self.broadcast_event(&event).await;
        debug!("Set reality level {:?} for workspace {}", level, workspace_id);
        Ok(())
    }

    /// Set reality continuum ratio for a workspace
    pub async fn set_reality_ratio(&self, workspace_id: &str, ratio: f64) -> Result<()> {
        let ratio = ratio.clamp(0.0, 1.0);
        let mut states = self.states.write().await;
        let state = states
            .entry(workspace_id.to_string())
            .or_insert_with(|| UnifiedState::new(workspace_id.to_string()));

        state.reality_continuum_ratio = ratio;
        state.increment_version();

        let event = StateChangeEvent::RealityRatioChanged {
            workspace_id: workspace_id.to_string(),
            ratio,
        };

        drop(states);

        self.broadcast_event(&event).await;
        debug!("Set reality ratio {} for workspace {}", ratio, workspace_id);
        Ok(())
    }

    /// Register or update an entity
    ///
    /// Entities are tracked across all protocols. When an entity is created
    /// via HTTP, it becomes immediately available in GraphQL, gRPC, etc.
    pub async fn register_entity(
        &self,
        workspace_id: &str,
        entity: EntityState,
    ) -> Result<()> {
        let mut states = self.states.write().await;
        let state = states
            .entry(workspace_id.to_string())
            .or_insert_with(|| UnifiedState::new(workspace_id.to_string()));

        let is_new = !state.entity_state.contains_key(&UnifiedState::entity_key(
            &entity.entity_type,
            &entity.entity_id,
        ));

        let entity_clone = entity.clone();
        state.register_entity(entity_clone.clone());

        let event = if is_new {
            StateChangeEvent::EntityCreated {
                workspace_id: workspace_id.to_string(),
                entity: entity_clone,
            }
        } else {
            StateChangeEvent::EntityUpdated {
                workspace_id: workspace_id.to_string(),
                entity: entity_clone,
            }
        };

        drop(states);

        self.broadcast_event(&event).await;
        debug!(
            "Registered entity {}:{} for workspace {}",
            entity.entity_type, entity.entity_id, workspace_id
        );
        Ok(())
    }

    /// Get entity by type and ID
    pub async fn get_entity(
        &self,
        workspace_id: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> Option<EntityState> {
        let states = self.states.read().await;
        states
            .get(workspace_id)?
            .get_entity(entity_type, entity_id)
            .cloned()
    }

    /// Activate a chaos rule
    pub async fn activate_chaos_rule(
        &self,
        workspace_id: &str,
        rule: ChaosScenario,
    ) -> Result<()> {
        let mut states = self.states.write().await;
        let state = states
            .entry(workspace_id.to_string())
            .or_insert_with(|| UnifiedState::new(workspace_id.to_string()));

        // Remove existing rule with same name if present
        state
            .active_chaos_rules
            .retain(|r| r.name != rule.name);
        state.active_chaos_rules.push(rule.clone());
        state.increment_version();

        let event = StateChangeEvent::ChaosRuleActivated {
            workspace_id: workspace_id.to_string(),
            rule,
        };

        drop(states);

        self.broadcast_event(&event).await;
        info!("Activated chaos rule for workspace {}", workspace_id);
        Ok(())
    }

    /// Deactivate a chaos rule
    pub async fn deactivate_chaos_rule(
        &self,
        workspace_id: &str,
        rule_name: &str,
    ) -> Result<()> {
        let mut states = self.states.write().await;
        let state = states
            .entry(workspace_id.to_string())
            .or_insert_with(|| UnifiedState::new(workspace_id.to_string()));

        let removed = state
            .active_chaos_rules
            .iter()
            .any(|r| r.name == rule_name);

        if removed {
            state.active_chaos_rules.retain(|r| r.name != rule_name);
            state.increment_version();

            let event = StateChangeEvent::ChaosRuleDeactivated {
                workspace_id: workspace_id.to_string(),
                rule_name: rule_name.to_string(),
            };

            drop(states);

            self.broadcast_event(&event).await;
            info!("Deactivated chaos rule {} for workspace {}", rule_name, workspace_id);
        }

        Ok(())
    }

    /// Update protocol state
    pub async fn update_protocol_state(
        &self,
        workspace_id: &str,
        protocol: Protocol,
        protocol_state: ProtocolState,
    ) -> Result<()> {
        let mut states = self.states.write().await;
        let state = states
            .entry(workspace_id.to_string())
            .or_insert_with(|| UnifiedState::new(workspace_id.to_string()));

        state.protocol_states.insert(protocol, protocol_state);
        state.increment_version();

        Ok(())
    }

    /// Get protocol state
    pub async fn get_protocol_state(
        &self,
        workspace_id: &str,
        protocol: Protocol,
    ) -> Option<ProtocolState> {
        let states = self.states.read().await;
        states
            .get(workspace_id)?
            .protocol_states
            .get(&protocol)
            .cloned()
    }

    /// Subscribe to state change events
    ///
    /// Returns a receiver that will receive all state change events for
    /// the specified workspace (or all workspaces if None).
    pub fn subscribe_to_events(
        &self,
        _workspace_id: Option<&str>,
    ) -> broadcast::Receiver<StateChangeEvent> {
        self.event_tx.subscribe()
    }

    /// Restore unified state from a snapshot
    ///
    /// This replaces the current state for a workspace with the provided state.
    /// All protocol adapters will be notified of the state changes.
    pub async fn restore_state(&self, state: UnifiedState) -> Result<()> {
        let workspace_id = state.workspace_id.clone();
        let mut states = self.states.write().await;
        states.insert(workspace_id.clone(), state.clone());
        drop(states);

        // Broadcast state change events for all components
        if let Some(ref persona) = state.active_persona {
            self.broadcast_event(&StateChangeEvent::PersonaChanged {
                workspace_id: workspace_id.clone(),
                persona: persona.clone(),
            })
            .await;
        }

        if let Some(ref scenario_id) = state.active_scenario {
            self.broadcast_event(&StateChangeEvent::ScenarioChanged {
                workspace_id: workspace_id.clone(),
                scenario_id: scenario_id.clone(),
            })
            .await;
        }

        self.broadcast_event(&StateChangeEvent::RealityLevelChanged {
            workspace_id: workspace_id.clone(),
            level: state.reality_level,
        })
        .await;

        self.broadcast_event(&StateChangeEvent::RealityRatioChanged {
            workspace_id: workspace_id.clone(),
            ratio: state.reality_continuum_ratio,
        })
        .await;

        // Broadcast entity events
        for entity in state.entity_state.values() {
            self.broadcast_event(&StateChangeEvent::EntityCreated {
                workspace_id: workspace_id.clone(),
                entity: entity.clone(),
            })
            .await;
        }

        // Broadcast chaos rule events
        for rule in &state.active_chaos_rules {
            self.broadcast_event(&StateChangeEvent::ChaosRuleActivated {
                workspace_id: workspace_id.clone(),
                rule: rule.clone(),
            })
            .await;
        }

        info!("Restored state for workspace {}", workspace_id);
        Ok(())
    }

    /// Broadcast a state change event to all adapters
    async fn broadcast_event(&self, event: &StateChangeEvent) {
        // Send to event subscribers
        if let Err(e) = self.event_tx.send(event.clone()) {
            warn!("Failed to broadcast state change event: {}", e);
        }

        // Notify all protocol adapters
        let adapters = self.adapters.read().await;
        for adapter in adapters.iter() {
            if let Err(e) = adapter.on_state_change(event).await {
                error!(
                    "Protocol adapter {:?} failed to handle state change: {}",
                    adapter.protocol(),
                    e
                );
            }
        }
    }
}

impl Default for ConsistencyEngine {
    fn default() -> Self {
        Self::new()
    }
}

