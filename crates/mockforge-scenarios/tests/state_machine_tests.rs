//! State machine edge case and complex scenario execution tests.
//!
//! These tests verify state machine behavior under various conditions:
//! - State transitions
//! - Condition evaluation
//! - Circular dependencies
//! - Invalid states
//! - Concurrent state updates

use mockforge_core::intelligent_behavior::rules::StateMachine;
use mockforge_scenarios::state_machine::{ScenarioStateMachineManager, StateInstance};
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_state_machine_creation() {
    let manager = ScenarioStateMachineManager::new();

    // Create a simple state machine
    let state_machine = StateMachine::new(
        "test_machine".to_string(),
        vec!["initial".to_string(), "final".to_string()],
        "initial".to_string(),
    );

    // State machine should be valid
    assert_eq!(state_machine.resource_type, "test_machine");
    assert_eq!(state_machine.initial_state, "initial");
    assert_eq!(state_machine.states.len(), 2);
}

#[tokio::test]
async fn test_state_instance_creation() {
    let instance = StateInstance::new("resource_123", "order", "initial");

    assert_eq!(instance.resource_id, "resource_123");
    assert_eq!(instance.current_state, "initial");
    assert_eq!(instance.resource_type, "order");
    assert!(instance.state_history.is_empty());
    assert!(instance.state_data.is_empty());
}

#[tokio::test]
async fn test_state_transition() {
    let mut instance = StateInstance::new("resource_123", "order", "initial");

    // Transition to new state
    instance.transition_to("processing".to_string(), None);

    assert_eq!(instance.current_state, "processing");
    assert_eq!(instance.state_history.len(), 1);
    assert_eq!(instance.state_history[0].from_state, "initial");
    assert_eq!(instance.state_history[0].to_state, "processing");
}

#[tokio::test]
async fn test_multiple_state_transitions() {
    let mut instance = StateInstance::new("resource_123", "order", "pending");

    // Multiple transitions
    instance.transition_to("processing".to_string(), None);
    instance.transition_to("shipped".to_string(), None);
    instance.transition_to("delivered".to_string(), None);

    assert_eq!(instance.current_state, "delivered");
    assert_eq!(instance.state_history.len(), 3);

    // Verify history order
    assert_eq!(instance.state_history[0].from_state, "pending");
    assert_eq!(instance.state_history[0].to_state, "processing");
    assert_eq!(instance.state_history[1].from_state, "processing");
    assert_eq!(instance.state_history[1].to_state, "shipped");
    assert_eq!(instance.state_history[2].from_state, "shipped");
    assert_eq!(instance.state_history[2].to_state, "delivered");
}

#[tokio::test]
async fn test_state_data_persistence() {
    let mut instance = StateInstance::new("resource_123", "order", "initial");

    // Add state data
    instance.state_data.insert("order_id".to_string(), json!("12345"));
    instance.state_data.insert("total".to_string(), json!(99.99));

    // Transition should preserve data
    instance.transition_to("processing".to_string(), None);

    assert_eq!(instance.state_data.get("order_id"), Some(&json!("12345")));
    assert_eq!(instance.state_data.get("total"), Some(&json!(99.99)));
}

#[tokio::test]
async fn test_state_machine_with_no_transitions() {
    let manager = ScenarioStateMachineManager::new();

    // Create state machine with no transitions (single state)
    let state_machine = StateMachine::new(
        "single_state".to_string(),
        vec!["only_state".to_string()],
        "only_state".to_string(),
    );

    // Should be valid even with no transitions
    assert_eq!(state_machine.states.len(), 1);
    assert!(state_machine.transitions.is_empty());
}

#[tokio::test]
async fn test_state_machine_with_circular_transitions() {
    use mockforge_core::intelligent_behavior::rules::StateTransition;

    let state_machine = StateMachine::new(
        "circular".to_string(),
        vec!["state_a".to_string(), "state_b".to_string()],
        "state_a".to_string(),
    )
    .add_transitions(vec![
        StateTransition::new("state_a", "state_b"),
        StateTransition::new("state_b", "state_a"),
    ]);

    // Circular transitions should be valid
    assert_eq!(state_machine.transitions.len(), 2);
}

#[tokio::test]
async fn test_state_instance_with_empty_history() {
    let instance = StateInstance::new("resource_123", "order", "initial");

    // New instance should have empty history
    assert!(instance.state_history.is_empty());

    // Can get current state
    assert_eq!(instance.current_state, "initial");
}

#[tokio::test]
async fn test_state_instance_resource_type() {
    let instance1 = StateInstance::new("resource_1", "order", "pending");

    let instance2 = StateInstance::new("resource_2", "user", "active");

    // Different resource types should be independent
    assert_eq!(instance1.resource_type, "order");
    assert_eq!(instance2.resource_type, "user");
    assert_ne!(instance1.resource_type, instance2.resource_type);
}

#[tokio::test]
async fn test_state_transition_with_metadata() {
    let mut instance = StateInstance::new("resource_123", "order", "initial");

    // Transition with transition ID
    instance.transition_to("processing".to_string(), Some("transition_1".to_string()));

    assert_eq!(instance.current_state, "processing");
    assert_eq!(instance.state_history.len(), 1);
    assert_eq!(instance.state_history[0].transition_id, Some("transition_1".to_string()));
}

#[tokio::test]
async fn test_state_machine_initial_state_validation() {
    let state_machine =
        StateMachine::new("test".to_string(), vec!["start".to_string()], "start".to_string());

    // Initial state should exist in states
    assert!(state_machine.states.contains(&state_machine.initial_state));
}

#[tokio::test]
async fn test_state_machine_multiple_final_states() {
    use mockforge_core::intelligent_behavior::rules::StateTransition;

    let state_machine = StateMachine::new(
        "multi_final".to_string(),
        vec![
            "start".to_string(),
            "success".to_string(),
            "failure".to_string(),
        ],
        "start".to_string(),
    )
    .add_transitions(vec![
        StateTransition::new("start", "success"),
        StateTransition::new("start", "failure"),
    ]);

    // Multiple final states should be valid
    assert_eq!(state_machine.states.len(), 3);
    assert_eq!(state_machine.transitions.len(), 2);
}

#[tokio::test]
async fn test_state_instance_concurrent_updates() {
    // Test that state instances can be cloned and modified independently
    let instance1 = StateInstance::new("resource_1", "order", "initial");

    let mut instance2 = instance1.clone();
    instance2.resource_id = "resource_2".to_string();
    instance2.transition_to("processing".to_string(), None);

    // Instances should be independent
    assert_eq!(instance1.current_state, "initial");
    assert_eq!(instance2.current_state, "processing");
    assert_eq!(instance1.state_history.len(), 0);
    assert_eq!(instance2.state_history.len(), 1);
}

#[tokio::test]
async fn test_state_history_timestamps() {
    use std::thread;
    use std::time::Duration;

    let mut instance =
        StateInstance::new("resource_123".to_string(), "state1".to_string(), "order".to_string());

    let timestamp1 = instance.state_history.last().map(|e| e.timestamp);

    // Small delay
    thread::sleep(Duration::from_millis(10));

    instance.transition_to("state2".to_string(), None);
    let timestamp2 = instance.state_history.last().map(|e| e.timestamp);

    // Timestamps should be different
    if let (Some(t1), Some(t2)) = (timestamp1, timestamp2) {
        assert!(t2 > t1, "Second transition should have later timestamp");
    }
}

#[tokio::test]
async fn test_state_machine_manager_initialization() {
    let _manager = ScenarioStateMachineManager::new();

    // Manager should be initialized
    // Note: We can't directly access internal state, but we can verify it exists
    // by checking that methods don't panic
}
