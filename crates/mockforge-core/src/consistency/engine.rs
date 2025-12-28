//! Consistency engine implementation
//!
//! The consistency engine coordinates state across all protocols, ensuring
//! that persona, scenario, reality level, and entity state are synchronized.

use crate::consistency::adapters::ProtocolAdapter;
use crate::consistency::types::{EntityState, ProtocolState, StateChangeEvent, UnifiedState};
use crate::protocol_abstraction::Protocol;
use crate::reality::RealityLevel;
use crate::Result;
// ChaosScenario is defined in mockforge-chaos, but we use serde_json::Value to avoid circular dependency
type ChaosScenario = serde_json::Value;
use mockforge_data::PersonaProfile;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

/// Default broadcast channel capacity for state change events
const DEFAULT_BROADCAST_CAPACITY: usize = 1000;

/// Get the broadcast channel capacity from environment or use default
fn get_broadcast_capacity() -> usize {
    std::env::var("MOCKFORGE_BROADCAST_CAPACITY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_BROADCAST_CAPACITY)
}

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
    ///
    /// The broadcast channel capacity can be configured via the
    /// `MOCKFORGE_BROADCAST_CAPACITY` environment variable.
    pub fn new() -> Self {
        let capacity = get_broadcast_capacity();
        let (event_tx, _) = broadcast::channel(capacity);
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
    pub async fn set_active_scenario(&self, workspace_id: &str, scenario_id: String) -> Result<()> {
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
    pub async fn set_reality_level(&self, workspace_id: &str, level: RealityLevel) -> Result<()> {
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
    /// Also automatically adds the entity to the persona graph if a persona_id is present.
    pub async fn register_entity(&self, workspace_id: &str, entity: EntityState) -> Result<()> {
        let mut states = self.states.write().await;
        let state = states
            .entry(workspace_id.to_string())
            .or_insert_with(|| UnifiedState::new(workspace_id.to_string()));

        let is_new = !state
            .entity_state
            .contains_key(&UnifiedState::entity_key(&entity.entity_type, &entity.entity_id));

        // Add entity to persona graph if persona_id is present
        #[cfg(feature = "persona-graph")]
        if let Some(ref persona_id) = entity.persona_id {
            let graph = state.get_or_create_persona_graph();
            graph.get_or_create_node_with_links(persona_id, &entity.entity_type, None, None);

            // If entity data contains related entity IDs, link them in the graph
            #[cfg(feature = "persona-graph")]
            if let Some(user_id) = entity.data.get("user_id").or_else(|| entity.data.get("userId"))
            {
                if let Some(user_id_str) = user_id.as_str() {
                    let user_persona_id = format!("user:{}", user_id_str);
                    graph.link_entity_types(
                        &user_persona_id,
                        "user",
                        persona_id,
                        &entity.entity_type,
                    );
                }
            }

            // Link orders to payments
            #[cfg(feature = "persona-graph")]
            if entity.entity_type == "payment" {
                if let Some(order_id) =
                    entity.data.get("order_id").or_else(|| entity.data.get("orderId"))
                {
                    if let Some(order_id_str) = order_id.as_str() {
                        let order_persona_id = format!("order:{}", order_id_str);
                        graph.link_entity_types(&order_persona_id, "order", persona_id, "payment");
                    }
                }
            }
        }

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
        states.get(workspace_id)?.get_entity(entity_type, entity_id).cloned()
    }

    /// Find related entities using the persona graph
    ///
    /// Given a persona ID and entity type, finds all related entities of the target type
    /// by traversing the persona graph.
    ///
    /// # Arguments
    /// * `workspace_id` - Workspace identifier
    /// * `persona_id` - Starting persona ID
    /// * `target_entity_type` - Entity type to find (e.g., "order", "payment")
    /// * `relationship_type` - Optional relationship type filter (e.g., "has_orders")
    ///
    /// # Returns
    /// Vector of entity states matching the criteria
    pub async fn find_related_entities(
        &self,
        workspace_id: &str,
        persona_id: &str,
        target_entity_type: &str,
        relationship_type: Option<&str>,
    ) -> Vec<EntityState> {
        let states = self.states.read().await;
        let state = match states.get(workspace_id) {
            Some(s) => s,
            None => return Vec::new(),
        };

        #[cfg(feature = "persona-graph")]
        let graph = match state.persona_graph() {
            Some(g) => g,
            None => return Vec::new(),
        };

        // Find related persona IDs in the graph
        #[cfg(feature = "persona-graph")]
        let related_persona_ids =
            graph.find_related_by_entity_type(persona_id, target_entity_type, relationship_type);

        #[cfg(not(feature = "persona-graph"))]
        let related_persona_ids: Vec<String> = Vec::new();

        // Convert persona IDs to entity states
        let mut related_entities = Vec::new();
        for related_persona_id in related_persona_ids {
            // Extract entity ID from persona ID (format: "entity_type:entity_id")
            if let Some((_, entity_id)) = related_persona_id.split_once(':') {
                if let Some(entity) = state.get_entity(target_entity_type, entity_id) {
                    related_entities.push(entity.clone());
                }
            }
        }

        related_entities
    }

    /// Activate a chaos rule
    pub async fn activate_chaos_rule(&self, workspace_id: &str, rule: ChaosScenario) -> Result<()> {
        let mut states = self.states.write().await;
        let state = states
            .entry(workspace_id.to_string())
            .or_insert_with(|| UnifiedState::new(workspace_id.to_string()));

        // Remove existing rule with same name if present
        // Note: ChaosScenario is serde_json::Value, so we compare by serializing
        if let Some(rule_name) = rule.get("name").and_then(|v| v.as_str()) {
            state
                .active_chaos_rules
                .retain(|r| r.get("name").and_then(|v| v.as_str()) != Some(rule_name));
        }
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
    pub async fn deactivate_chaos_rule(&self, workspace_id: &str, rule_name: &str) -> Result<()> {
        let mut states = self.states.write().await;
        let state = states
            .entry(workspace_id.to_string())
            .or_insert_with(|| UnifiedState::new(workspace_id.to_string()));

        // Note: ChaosScenario is serde_json::Value, so we compare by serializing
        let removed = state
            .active_chaos_rules
            .iter()
            .any(|r| r.get("name").and_then(|v| v.as_str()) == Some(rule_name));

        if removed {
            state
                .active_chaos_rules
                .retain(|r| r.get("name").and_then(|v| v.as_str()) != Some(rule_name));
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
        states.get(workspace_id)?.protocol_states.get(&protocol).cloned()
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
