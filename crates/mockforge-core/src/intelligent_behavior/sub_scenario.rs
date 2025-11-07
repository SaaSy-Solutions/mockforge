//! Sub-scenario support for nested state machines
//!
//! Sub-scenarios allow state machines to reference and execute nested state machines,
//! enabling composition of complex workflows from simpler building blocks.

use crate::intelligent_behavior::rules::StateMachine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sub-scenario definition for nested state machine execution
///
/// A sub-scenario is a nested state machine that can be referenced from a parent
/// state machine. It supports input/output mapping to pass data between parent
/// and child state machines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubScenario {
    /// Unique identifier for this sub-scenario
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Nested state machine definition
    pub state_machine: StateMachine,

    /// Input mapping: maps parent state variables to sub-scenario input variables
    ///
    /// Example: `{"parent.status" => "sub.input.status"}`
    #[serde(default)]
    pub input_mapping: HashMap<String, String>,

    /// Output mapping: maps sub-scenario output variables to parent state variables
    ///
    /// Example: `{"sub.output.result" => "parent.result"}`
    #[serde(default)]
    pub output_mapping: HashMap<String, String>,

    /// Optional description
    pub description: Option<String>,
}

impl SubScenario {
    /// Create a new sub-scenario
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        state_machine: StateMachine,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            state_machine,
            input_mapping: HashMap::new(),
            output_mapping: HashMap::new(),
            description: None,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add an input mapping
    ///
    /// Maps a parent state variable to a sub-scenario input variable.
    pub fn with_input_mapping(
        mut self,
        parent_var: impl Into<String>,
        sub_var: impl Into<String>,
    ) -> Self {
        self.input_mapping.insert(parent_var.into(), sub_var.into());
        self
    }

    /// Add an output mapping
    ///
    /// Maps a sub-scenario output variable to a parent state variable.
    pub fn with_output_mapping(
        mut self,
        sub_var: impl Into<String>,
        parent_var: impl Into<String>,
    ) -> Self {
        self.output_mapping.insert(sub_var.into(), parent_var.into());
        self
    }

    /// Get the nested state machine
    pub fn state_machine(&self) -> &StateMachine {
        &self.state_machine
    }

    /// Get the nested state machine mutably
    pub fn state_machine_mut(&mut self) -> &mut StateMachine {
        &mut self.state_machine
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligent_behavior::rules::{StateMachine, StateTransition};

    #[test]
    fn test_sub_scenario_creation() {
        let nested_machine = StateMachine::new(
            "sub_resource",
            vec!["start".to_string(), "end".to_string()],
            "start",
        );

        let sub_scenario = SubScenario::new("sub1", "Test Sub-Scenario", nested_machine)
            .with_description("A test sub-scenario")
            .with_input_mapping("parent.status", "sub.input.status")
            .with_output_mapping("sub.output.result", "parent.result");

        assert_eq!(sub_scenario.id, "sub1");
        assert_eq!(sub_scenario.name, "Test Sub-Scenario");
        assert_eq!(sub_scenario.input_mapping.len(), 1);
        assert_eq!(sub_scenario.output_mapping.len(), 1);
        assert_eq!(
            sub_scenario.input_mapping.get("parent.status"),
            Some(&"sub.input.status".to_string())
        );
    }

    #[test]
    fn test_sub_scenario_serialization() {
        let nested_machine = StateMachine::new(
            "sub_resource",
            vec!["start".to_string(), "end".to_string()],
            "start",
        )
        .add_transition(StateTransition::new("start", "end"));

        let sub_scenario = SubScenario::new("sub1", "Test", nested_machine)
            .with_input_mapping("parent.x", "sub.x");

        let json = serde_json::to_string(&sub_scenario).unwrap();
        let deserialized: SubScenario = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "sub1");
        assert_eq!(deserialized.input_mapping.len(), 1);
    }
}
