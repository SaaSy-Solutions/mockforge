//! Scenario state machine manager
//!
//! Provides functionality for loading, validating, and executing state machines
//! from scenario manifests. Manages active state instances and real-time state tracking.

use crate::error::{Result, ScenarioError};
use crate::manifest::ScenarioManifest;
use mockforge_core::intelligent_behavior::{
    condition_evaluator::{ConditionError, ConditionEvaluator, ConditionResult},
    history::HistoryManager,
    rules::{StateMachine, StateTransition},
    sub_scenario::SubScenario,
    visual_layout::VisualLayout,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Active state instance for a state machine
///
/// Tracks the current state of a specific resource instance within a state machine.
#[derive(Debug, Clone)]
pub struct StateInstance {
    /// Resource identifier (e.g., entity ID)
    pub resource_id: String,

    /// Current state
    pub current_state: String,

    /// State machine resource type
    pub resource_type: String,

    /// State history (for undo/redo and debugging)
    pub state_history: Vec<StateHistoryEntry>,

    /// Custom state data (key-value pairs)
    pub state_data: HashMap<String, Value>,
}

/// Entry in state history
#[derive(Debug, Clone)]
pub struct StateHistoryEntry {
    /// Previous state
    pub from_state: String,

    /// New state
    pub to_state: String,

    /// Timestamp of transition
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Transition that was used
    pub transition_id: Option<String>,
}

impl StateInstance {
    /// Create a new state instance
    pub fn new(
        resource_id: impl Into<String>,
        resource_type: impl Into<String>,
        initial_state: impl Into<String>,
    ) -> Self {
        Self {
            resource_id: resource_id.into(),
            current_state: initial_state.into(),
            resource_type: resource_type.into(),
            state_history: Vec::new(),
            state_data: HashMap::new(),
        }
    }

    /// Transition to a new state
    pub fn transition_to(&mut self, to_state: impl Into<String>, transition_id: Option<String>) {
        let from_state = self.current_state.clone();
        let to_state = to_state.into();

        self.state_history.push(StateHistoryEntry {
            from_state: from_state.clone(),
            to_state: to_state.clone(),
            timestamp: chrono::Utc::now(),
            transition_id,
        });

        self.current_state = to_state;
    }

    /// Get the current state
    pub fn current_state(&self) -> &str {
        &self.current_state
    }

    /// Set state data
    pub fn set_data(&mut self, key: impl Into<String>, value: Value) {
        self.state_data.insert(key.into(), value);
    }

    /// Get state data
    pub fn get_data(&self, key: &str) -> Option<&Value> {
        self.state_data.get(key)
    }
}

/// Manager for scenario state machines
///
/// Handles loading state machines from scenario manifests, validating them,
/// executing state transitions, and managing active state instances.
pub struct ScenarioStateMachineManager {
    /// Loaded state machines by resource type
    state_machines: Arc<RwLock<HashMap<String, StateMachine>>>,

    /// Active state instances by resource ID
    instances: Arc<RwLock<HashMap<String, StateInstance>>>,

    /// Visual layouts by resource type
    visual_layouts: Arc<RwLock<HashMap<String, VisualLayout>>>,

    /// History managers for undo/redo (by resource type)
    history_managers: Arc<RwLock<HashMap<String, HistoryManager>>>,
}

impl ScenarioStateMachineManager {
    /// Create a new state machine manager
    pub fn new() -> Self {
        Self {
            state_machines: Arc::new(RwLock::new(HashMap::new())),
            instances: Arc::new(RwLock::new(HashMap::new())),
            visual_layouts: Arc::new(RwLock::new(HashMap::new())),
            history_managers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load state machines from a scenario manifest
    ///
    /// Validates and loads all state machines defined in the manifest,
    /// along with their visual layouts.
    pub async fn load_from_manifest(&self, manifest: &ScenarioManifest) -> Result<()> {
        info!(
            "Loading {} state machines from scenario '{}'",
            manifest.state_machines.len(),
            manifest.name
        );

        let mut state_machines = self.state_machines.write().await;
        let mut visual_layouts = self.visual_layouts.write().await;

        for state_machine in &manifest.state_machines {
            // Validate state machine
            self.validate_state_machine(state_machine)?;

            // Store state machine
            let resource_type = state_machine.resource_type.clone();
            state_machines.insert(resource_type.clone(), state_machine.clone());

            // Store visual layout if available
            if let Some(layout) = &state_machine.visual_layout {
                visual_layouts.insert(resource_type.clone(), layout.clone());
            }

            // Also check state_machine_graphs for additional layouts
            if let Some(layout) = manifest.state_machine_graphs.get(&resource_type) {
                visual_layouts.insert(resource_type.clone(), layout.clone());
            }

            info!("Loaded state machine for resource type '{}'", resource_type);
        }

        Ok(())
    }

    /// Validate a state machine
    ///
    /// Checks that:
    /// - Initial state exists in states list
    /// - All transitions reference valid states
    /// - Sub-scenario references are valid
    /// - No circular dependencies in sub-scenarios
    pub fn validate_state_machine(&self, state_machine: &StateMachine) -> Result<()> {
        // Check initial state exists
        if !state_machine.states.contains(&state_machine.initial_state) {
            return Err(ScenarioError::InvalidManifest(format!(
                "State machine '{}' has initial state '{}' that is not in states list",
                state_machine.resource_type, state_machine.initial_state
            )));
        }

        // Validate transitions
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
            self.validate_state_machine(&sub_scenario.state_machine)?;
        }

        Ok(())
    }

    /// Get a state machine by resource type
    pub async fn get_state_machine(&self, resource_type: &str) -> Option<StateMachine> {
        let state_machines = self.state_machines.read().await;
        state_machines.get(resource_type).cloned()
    }

    /// Get visual layout for a state machine
    pub async fn get_visual_layout(&self, resource_type: &str) -> Option<VisualLayout> {
        let layouts = self.visual_layouts.read().await;
        layouts.get(resource_type).cloned()
    }

    /// Create a new state instance for a resource
    ///
    /// Initializes a new state instance with the initial state from the state machine.
    pub async fn create_instance(
        &self,
        resource_id: impl Into<String>,
        resource_type: impl Into<String>,
    ) -> Result<()> {
        let resource_id = resource_id.into();
        let resource_type = resource_type.into();

        // Get state machine
        let state_machine = self.get_state_machine(&resource_type).await.ok_or_else(|| {
            ScenarioError::InvalidManifest(format!(
                "No state machine found for resource type '{}'",
                resource_type
            ))
        })?;

        // Create instance with initial state
        let instance = StateInstance::new(
            resource_id.clone(),
            resource_type.clone(),
            state_machine.initial_state.clone(),
        );

        let mut instances = self.instances.write().await;
        instances.insert(resource_id, instance);

        Ok(())
    }

    /// Get current state of a resource instance
    pub async fn get_current_state(&self, resource_id: &str) -> Option<String> {
        let instances = self.instances.read().await;
        instances.get(resource_id).map(|i| i.current_state.clone())
    }

    /// Execute a state transition
    ///
    /// Attempts to transition a resource instance from its current state to a new state.
    /// Validates the transition is allowed and evaluates any conditions.
    pub async fn execute_transition(
        &self,
        resource_id: &str,
        to_state: impl Into<String>,
        context: Option<HashMap<String, Value>>,
    ) -> Result<()> {
        let to_state = to_state.into();
        let mut instances = self.instances.write().await;

        let instance = instances.get_mut(resource_id).ok_or_else(|| {
            ScenarioError::InvalidManifest(format!(
                "No state instance found for resource '{}'",
                resource_id
            ))
        })?;

        // Get state machine
        let state_machine =
            self.get_state_machine(&instance.resource_type).await.ok_or_else(|| {
                ScenarioError::InvalidManifest(format!(
                    "No state machine found for resource type '{}'",
                    instance.resource_type
                ))
            })?;

        // Find valid transition
        let transition = state_machine
            .transitions
            .iter()
            .find(|t| t.from_state == instance.current_state && t.to_state == to_state);

        let transition = transition.ok_or_else(|| {
            ScenarioError::InvalidManifest(format!(
                "No valid transition from '{}' to '{}' for resource '{}'",
                instance.current_state, to_state, resource_id
            ))
        })?;

        // Evaluate condition if present
        if let Some(ref condition_expr) = transition.condition_expression {
            let mut evaluator = ConditionEvaluator::new();

            // Add context variables
            if let Some(ref ctx) = context {
                for (key, value) in ctx {
                    evaluator.set_variable(key.clone(), value.clone());
                }
            }

            // Add instance state data
            for (key, value) in &instance.state_data {
                evaluator.set_variable(key.clone(), value.clone());
            }

            // Evaluate condition
            match evaluator.evaluate(condition_expr) {
                Ok(true) => {
                    // Condition passed, proceed with transition
                }
                Ok(false) => {
                    return Err(ScenarioError::InvalidManifest(format!(
                        "Transition condition not met: {}",
                        condition_expr
                    )));
                }
                Err(e) => {
                    return Err(ScenarioError::InvalidManifest(format!(
                        "Error evaluating transition condition: {}",
                        e
                    )));
                }
            }
        }

        // Execute sub-scenario if referenced
        if let Some(ref sub_scenario_id) = transition.sub_scenario_ref {
            if let Some(sub_scenario) = state_machine.get_sub_scenario(sub_scenario_id) {
                debug!("Executing sub-scenario '{}' for transition", sub_scenario_id);

                // Execute sub-scenario with input/output mapping
                match self
                    .execute_sub_scenario(
                        sub_scenario,
                        &instance.state_data,
                        &sub_scenario.state_machine.resource_type,
                    )
                    .await
                {
                    Ok(output_data) => {
                        // Apply output mapping: copy sub-scenario outputs to parent instance
                        for (sub_var, parent_var) in &sub_scenario.output_mapping {
                            if let Some(value) = output_data.get(sub_var) {
                                instance.state_data.insert(parent_var.clone(), value.clone());
                                debug!(
                                    "Mapped sub-scenario output '{}' to parent variable '{}'",
                                    sub_var, parent_var
                                );
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Sub-scenario execution failed: {}", e);
                        // Continue with transition even if sub-scenario fails
                        // (could be made configurable in the future)
                    }
                }
            }
        }

        // Perform transition
        instance.transition_to(
            to_state.clone(),
            Some(format!("{}-{}", instance.current_state, to_state)),
        );

        // Update history manager
        let mut history_managers = self.history_managers.write().await;
        let history = history_managers
            .entry(instance.resource_type.clone())
            .or_insert_with(HistoryManager::new);
        // Note: We'd push the state machine to history here if we were tracking edits
        // For now, we're just tracking execution state

        info!(
            "Resource '{}' transitioned from '{}' to '{}'",
            resource_id, instance.current_state, to_state
        );

        Ok(())
    }

    /// Get all state instances
    pub async fn list_instances(&self) -> Vec<StateInstance> {
        let instances = self.instances.read().await;
        instances.values().cloned().collect()
    }

    /// Get state instance by resource ID
    pub async fn get_instance(&self, resource_id: &str) -> Option<StateInstance> {
        let instances = self.instances.read().await;
        instances.get(resource_id).cloned()
    }

    /// Delete a state instance
    pub async fn delete_instance(&self, resource_id: &str) -> bool {
        let mut instances = self.instances.write().await;
        instances.remove(resource_id).is_some()
    }

    /// Get next possible states for a resource
    ///
    /// Returns all states that can be reached from the current state of the resource.
    pub async fn get_next_states(&self, resource_id: &str) -> Result<Vec<String>> {
        let instances = self.instances.read().await;
        let instance = instances.get(resource_id).ok_or_else(|| {
            ScenarioError::InvalidManifest(format!(
                "No state instance found for resource '{}'",
                resource_id
            ))
        })?;

        let state_machine =
            self.get_state_machine(&instance.resource_type).await.ok_or_else(|| {
                ScenarioError::InvalidManifest(format!(
                    "No state machine found for resource type '{}'",
                    instance.resource_type
                ))
            })?;

        Ok(state_machine.next_states(&instance.current_state))
    }

    /// Set visual layout for a state machine
    pub async fn set_visual_layout(&self, resource_type: &str, layout: VisualLayout) {
        let mut layouts = self.visual_layouts.write().await;
        layouts.insert(resource_type.to_string(), layout);
    }

    /// Clear all state machines and instances
    pub async fn clear(&self) {
        let mut state_machines = self.state_machines.write().await;
        let mut instances = self.instances.write().await;
        let mut layouts = self.visual_layouts.write().await;
        let mut history = self.history_managers.write().await;

        state_machines.clear();
        instances.clear();
        layouts.clear();
        history.clear();
    }

    /// Delete a state machine by resource type
    ///
    /// Removes the state machine and its visual layout.
    /// Also removes all instances associated with this resource type.
    pub async fn delete_state_machine(&self, resource_type: &str) -> bool {
        let mut state_machines = self.state_machines.write().await;
        let mut visual_layouts = self.visual_layouts.write().await;
        let mut instances = self.instances.write().await;
        let mut history_managers = self.history_managers.write().await;

        // Remove state machine
        let removed = state_machines.remove(resource_type).is_some();

        // Remove visual layout
        visual_layouts.remove(resource_type);

        // Remove all instances for this resource type
        instances.retain(|_, instance| instance.resource_type != resource_type);

        // Remove history manager
        history_managers.remove(resource_type);

        if removed {
            info!("Deleted state machine for resource type '{}'", resource_type);
        }

        removed
    }

    /// List all state machines
    ///
    /// Returns a list of all loaded state machines with their resource types.
    pub async fn list_state_machines(&self) -> Vec<(String, StateMachine)> {
        let state_machines = self.state_machines.read().await;
        state_machines.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// Get all state machines and visual layouts for export
    ///
    /// Returns all state machines and their associated visual layouts
    /// in a format suitable for export.
    pub async fn export_all(&self) -> (Vec<StateMachine>, HashMap<String, VisualLayout>) {
        let state_machines = self.state_machines.read().await;
        let visual_layouts = self.visual_layouts.read().await;

        let machines: Vec<StateMachine> = state_machines.values().cloned().collect();
        let layouts: HashMap<String, VisualLayout> =
            visual_layouts.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        (machines, layouts)
    }

    /// Execute a sub-scenario with input/output mapping
    ///
    /// Creates a nested state instance, applies input mapping, executes the sub-scenario
    /// state machine until completion, and returns the output data for output mapping.
    async fn execute_sub_scenario(
        &self,
        sub_scenario: &SubScenario,
        parent_state_data: &HashMap<String, Value>,
        sub_resource_type: &str,
    ) -> Result<HashMap<String, Value>> {
        // Generate unique resource ID for sub-scenario instance
        let sub_instance_id = format!("sub-{}-{}", sub_scenario.id, Uuid::new_v4());

        // Create sub-scenario instance with initial state
        let mut sub_instance = StateInstance::new(
            sub_instance_id.clone(),
            sub_resource_type.to_string(),
            sub_scenario.state_machine.initial_state.clone(),
        );

        // Apply input mapping: copy values from parent to sub-scenario
        for (parent_var, sub_var) in &sub_scenario.input_mapping {
            // Resolve parent variable value
            // Support dot notation for nested access (e.g., "parent.status" or just "status")
            let value = if parent_var.contains('.') {
                // Try to resolve nested path
                let parts: Vec<&str> = parent_var.split('.').collect();
                if parts.len() == 2 && parts[0] == "parent" {
                    parent_state_data.get(parts[1]).cloned()
                } else {
                    // Try direct lookup
                    parent_state_data.get(parent_var).cloned()
                }
            } else {
                parent_state_data.get(parent_var).cloned()
            };

            if let Some(val) = value {
                sub_instance.set_data(sub_var.clone(), val.clone());
                debug!(
                    "Mapped parent variable '{}' to sub-scenario variable '{}'",
                    parent_var, sub_var
                );
            } else {
                warn!(
                    "Parent variable '{}' not found in state data, skipping input mapping",
                    parent_var
                );
            }
        }

        // Store sub-instance temporarily
        {
            let mut instances = self.instances.write().await;
            instances.insert(sub_instance_id.clone(), sub_instance.clone());
        }

        // Execute sub-scenario state machine until it reaches a final state
        // A final state is one that has no outgoing transitions
        let mut max_iterations = 100; // Prevent infinite loops
        let mut iteration = 0;

        loop {
            if iteration >= max_iterations {
                warn!("Sub-scenario '{}' exceeded maximum iterations, stopping", sub_scenario.id);
                break;
            }
            iteration += 1;

            // Get current state
            let current_state = sub_instance.current_state.clone();

            // Check if this is a final state (no outgoing transitions)
            let has_outgoing = sub_scenario
                .state_machine
                .transitions
                .iter()
                .any(|t| t.from_state == current_state);

            if !has_outgoing {
                debug!(
                    "Sub-scenario '{}' reached final state '{}'",
                    sub_scenario.id, current_state
                );
                break;
            }

            // Find valid transitions from current state
            let possible_transitions: Vec<_> = sub_scenario
                .state_machine
                .transitions
                .iter()
                .filter(|t| t.from_state == current_state)
                .collect();

            if possible_transitions.is_empty() {
                debug!(
                    "Sub-scenario '{}' has no valid transitions from state '{}', stopping",
                    sub_scenario.id, current_state
                );
                break;
            }

            // Select transition (for now, take the first valid one)
            // In the future, this could support probability-based selection or condition evaluation
            let selected_transition = possible_transitions[0];
            let next_state = selected_transition.to_state.clone();

            // Evaluate condition if present
            if let Some(ref condition_expr) = selected_transition.condition_expression {
                let mut evaluator = ConditionEvaluator::new();

                // Add sub-instance state data to evaluator
                for (key, value) in &sub_instance.state_data {
                    evaluator.set_variable(key.clone(), value.clone());
                }

                // Evaluate condition
                match evaluator.evaluate(condition_expr) {
                    Ok(true) => {
                        // Condition passed, proceed with transition
                    }
                    Ok(false) => {
                        // Condition failed, try next transition or stop
                        debug!(
                            "Sub-scenario transition condition not met: {}, trying next transition",
                            condition_expr
                        );
                        if possible_transitions.len() > 1 {
                            // Try next transition
                            let next_transition = possible_transitions[1];
                            let next_state = next_transition.to_state.clone();
                            sub_instance.transition_to(next_state, None);
                        } else {
                            // No more transitions, stop
                            break;
                        }
                        continue;
                    }
                    Err(e) => {
                        warn!(
                            "Error evaluating sub-scenario transition condition: {}, stopping",
                            e
                        );
                        break;
                    }
                }
            }

            // Perform transition
            sub_instance.transition_to(next_state.clone(), None);
            debug!(
                "Sub-scenario '{}' transitioned from '{}' to '{}'",
                sub_scenario.id, current_state, next_state
            );

            // Update stored instance
            {
                let mut instances = self.instances.write().await;
                if let Some(stored) = instances.get_mut(&sub_instance_id) {
                    *stored = sub_instance.clone();
                }
            }
        }

        // Get final state data from sub-instance
        let output_data = sub_instance.state_data.clone();

        // Clean up sub-instance
        {
            let mut instances = self.instances.write().await;
            instances.remove(&sub_instance_id);
        }

        info!(
            "Sub-scenario '{}' completed after {} iterations, final state: '{}'",
            sub_scenario.id, iteration, sub_instance.current_state
        );

        Ok(output_data)
    }
}

impl Default for ScenarioStateMachineManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_core::intelligent_behavior::rules::{StateMachine, StateTransition};

    fn create_test_state_machine() -> StateMachine {
        StateMachine::new(
            "order",
            vec![
                "pending".to_string(),
                "processing".to_string(),
                "shipped".to_string(),
            ],
            "pending",
        )
        .add_transition(StateTransition::new("pending", "processing"))
        .add_transition(StateTransition::new("processing", "shipped"))
    }

    #[tokio::test]
    async fn test_load_state_machine() {
        let manager = ScenarioStateMachineManager::new();
        let mut manifest = ScenarioManifest::new(
            "test".to_string(),
            "1.0.0".to_string(),
            "Test".to_string(),
            "Test scenario".to_string(),
        );
        manifest.state_machines.push(create_test_state_machine());

        let result = manager.load_from_manifest(&manifest).await;
        assert!(result.is_ok());

        let state_machine = manager.get_state_machine("order").await;
        assert!(state_machine.is_some());
        assert_eq!(state_machine.unwrap().resource_type, "order");
    }

    #[tokio::test]
    async fn test_create_and_transition() {
        let manager = ScenarioStateMachineManager::new();
        let mut manifest = ScenarioManifest::new(
            "test".to_string(),
            "1.0.0".to_string(),
            "Test".to_string(),
            "Test scenario".to_string(),
        );
        manifest.state_machines.push(create_test_state_machine());

        manager.load_from_manifest(&manifest).await.unwrap();
        manager.create_instance("order-1", "order").await.unwrap();

        let state = manager.get_current_state("order-1").await;
        assert_eq!(state, Some("pending".to_string()));

        manager.execute_transition("order-1", "processing", None).await.unwrap();
        let state = manager.get_current_state("order-1").await;
        assert_eq!(state, Some("processing".to_string()));
    }

    #[tokio::test]
    async fn test_conditional_transition() {
        let manager = ScenarioStateMachineManager::new();
        let state_machine = StateMachine::new(
            "order",
            vec![
                "pending".to_string(),
                "approved".to_string(),
                "rejected".to_string(),
            ],
            "pending",
        )
        .add_transition(
            StateTransition::new("pending", "approved").with_condition_expression("amount > 100"),
        )
        .add_transition(
            StateTransition::new("pending", "rejected").with_condition_expression("amount <= 100"),
        );

        let mut manifest = ScenarioManifest::new(
            "test".to_string(),
            "1.0.0".to_string(),
            "Test".to_string(),
            "Test scenario".to_string(),
        );
        manifest.state_machines.push(state_machine);

        manager.load_from_manifest(&manifest).await.unwrap();
        manager.create_instance("order-1", "order").await.unwrap();

        // Test with condition that passes
        let mut context = HashMap::new();
        context.insert("amount".to_string(), Value::Number(serde_json::Number::from(150)));
        manager.execute_transition("order-1", "approved", Some(context)).await.unwrap();
        assert_eq!(manager.get_current_state("order-1").await, Some("approved".to_string()));
    }
}
