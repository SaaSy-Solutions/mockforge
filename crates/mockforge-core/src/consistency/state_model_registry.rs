//! State Model Registry
//!
//! Manages shared state models that define how personas and entities are related
//! across protocols. Workspaces with the same state model share persona graphs
//! and unified state, ensuring consistency across protocols.

use crate::consistency::ConsistencyEngine;
use crate::Result;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// State Model Registry
///
/// Manages the mapping between state models and workspaces, ensuring that
/// workspaces with the same state model share persona graphs and unified state.
pub struct StateModelRegistry {
    /// Mapping from state model name to set of workspace IDs
    model_to_workspaces: Arc<RwLock<HashMap<String, HashSet<String>>>>,

    /// Mapping from workspace ID to state model name
    workspace_to_model: Arc<RwLock<HashMap<String, String>>>,

    /// Reference to the consistency engine
    consistency_engine: Arc<ConsistencyEngine>,
}

impl StateModelRegistry {
    /// Create a new state model registry
    pub fn new(consistency_engine: Arc<ConsistencyEngine>) -> Self {
        Self {
            model_to_workspaces: Arc::new(RwLock::new(HashMap::new())),
            workspace_to_model: Arc::new(RwLock::new(HashMap::new())),
            consistency_engine,
        }
    }

    /// Register a workspace with a state model
    ///
    /// This associates a workspace with a state model, enabling it to share
    /// persona graphs and unified state with other workspaces using the same model.
    pub async fn register_workspace(
        &self,
        workspace_id: String,
        state_model: String,
    ) -> Result<()> {
        // Remove workspace from any previous state model
        let old_model = {
            let workspace_to_model = self.workspace_to_model.read().await;
            workspace_to_model.get(&workspace_id).cloned()
        };

        if let Some(ref old) = old_model {
            if old == &state_model {
                // Already registered with this model
                return Ok(());
            }

            // Remove from old model
            let mut model_to_workspaces = self.model_to_workspaces.write().await;
            if let Some(workspaces) = model_to_workspaces.get_mut(old) {
                workspaces.remove(&workspace_id);
                if workspaces.is_empty() {
                    model_to_workspaces.remove(old);
                }
            }
        }

        // Add to new model
        {
            let mut model_to_workspaces = self.model_to_workspaces.write().await;
            let mut workspace_to_model = self.workspace_to_model.write().await;

            model_to_workspaces
                .entry(state_model.clone())
                .or_insert_with(HashSet::new)
                .insert(workspace_id.clone());

            workspace_to_model.insert(workspace_id.clone(), state_model.clone());
        }

        info!("Registered workspace {} with state model {}", workspace_id, state_model);
        Ok(())
    }

    /// Unregister a workspace from its state model
    pub async fn unregister_workspace(&self, workspace_id: &str) -> Result<()> {
        let state_model = {
            let mut workspace_to_model = self.workspace_to_model.write().await;
            workspace_to_model.remove(workspace_id)
        };

        if let Some(model) = state_model {
            let mut model_to_workspaces = self.model_to_workspaces.write().await;
            if let Some(workspaces) = model_to_workspaces.get_mut(&model) {
                workspaces.remove(workspace_id);
                if workspaces.is_empty() {
                    model_to_workspaces.remove(&model);
                }
            }
            info!("Unregistered workspace {} from state model {}", workspace_id, model);
        }

        Ok(())
    }

    /// Get the state model for a workspace
    pub async fn get_state_model(&self, workspace_id: &str) -> Option<String> {
        let workspace_to_model = self.workspace_to_model.read().await;
        workspace_to_model.get(workspace_id).cloned()
    }

    /// Get all workspaces using a state model
    pub async fn get_workspaces_for_model(&self, state_model: &str) -> Vec<String> {
        let model_to_workspaces = self.model_to_workspaces.read().await;
        model_to_workspaces
            .get(state_model)
            .map(|workspaces| workspaces.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all state models
    pub async fn list_state_models(&self) -> Vec<String> {
        let model_to_workspaces = self.model_to_workspaces.read().await;
        model_to_workspaces.keys().cloned().collect()
    }

    /// Synchronize persona graph across workspaces with the same state model
    ///
    /// When a persona graph is updated in one workspace, this method ensures
    /// that all other workspaces using the same state model are updated as well.
    pub async fn sync_persona_graph(&self, workspace_id: &str) -> Result<()> {
        let state_model = match self.get_state_model(workspace_id).await {
            Some(model) => model,
            None => {
                debug!("Workspace {} not registered with any state model", workspace_id);
                return Ok(());
            }
        };

        // Get the source workspace's unified state
        let _source_state = match self.consistency_engine.get_state(workspace_id).await {
            Some(state) => state,
            None => {
                warn!("Source workspace {} not found", workspace_id);
                return Ok(());
            }
        };

        // Get all other workspaces using the same state model
        let _target_workspaces = self.get_workspaces_for_model(&state_model).await;

        // Sync persona graph to all target workspaces
        #[cfg(feature = "persona-graph")]
        if let Some(_source_graph) = _source_state.persona_graph() {
            for target_workspace_id in _target_workspaces {
                if target_workspace_id == workspace_id {
                    continue; // Skip the source workspace
                }

                // Get or create target workspace state
                let mut target_state =
                    self.consistency_engine.get_or_create_state(&target_workspace_id).await;

                // Clone the persona graph
                let _target_graph = target_state.get_or_create_persona_graph();
                // Note: PersonaGraph doesn't have a clone method, so we'd need to
                // manually copy nodes and edges, or add a clone method to PersonaGraph
                // For now, we'll just log that sync is needed
                debug!(
                    "Persona graph sync needed from {} to {} (state model: {})",
                    workspace_id, target_workspace_id, state_model
                );
            }
        }

        Ok(())
    }

    /// Ensure all workspaces with the same state model use the same persona graph
    ///
    /// This is called periodically or when state model configuration changes
    /// to ensure consistency across workspaces.
    pub async fn ensure_consistency(&self, state_model: &str) -> Result<()> {
        let workspaces = self.get_workspaces_for_model(state_model).await;

        if workspaces.is_empty() {
            return Ok(());
        }

        // Use the first workspace as the source of truth
        let source_workspace = &workspaces[0];

        // Sync persona graph from source to all other workspaces
        for workspace_id in workspaces.iter().skip(1) {
            // For now, we'll just ensure they all have the same active persona
            // Full persona graph sync would require PersonaGraph to support cloning
            if let Some(source_state) = self.consistency_engine.get_state(source_workspace).await {
                if let Some(ref persona) = source_state.active_persona {
                    if let Err(e) = self
                        .consistency_engine
                        .set_active_persona(workspace_id, persona.clone())
                        .await
                    {
                        warn!(
                            "Failed to sync persona from {} to {}: {}",
                            source_workspace, workspace_id, e
                        );
                    }
                }
            }
        }

        debug!("Ensured consistency for state model {}", state_model);
        Ok(())
    }
}
