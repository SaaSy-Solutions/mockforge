//! Integration tests for Behavioral Cloning v1 feature
//!
//! Tests flow recording, compilation, replay (strict/flex modes), and export/import.

use chrono::Utc;
use mockforge_recorder::behavioral_cloning::{
    BehavioralScenario, BehavioralScenarioReplayEngine, BehavioralScenarioStep, Flow,
    FlowGroupingStrategy, FlowRecordingConfig, FlowStep, ReplayResponse, StateVariable,
};
use mockforge_recorder::models::{Protocol, RecordedRequest, RecordedResponse};
use serde_json::json;
use std::collections::HashMap;

/// Test flow recording configuration
#[tokio::test]
async fn test_flow_recording_config() {
    let config = FlowRecordingConfig {
        group_by: FlowGroupingStrategy::TraceId,
        time_window_seconds: 300,
        enabled: true,
    };

    assert_eq!(config.group_by, FlowGroupingStrategy::TraceId);
    assert_eq!(config.time_window_seconds, 300);
    assert!(config.enabled);
}

/// Test flow grouping strategies
#[tokio::test]
async fn test_flow_grouping_strategies() {
    // Test trace_id grouping
    let trace_id = FlowGroupingStrategy::TraceId;
    assert_eq!(trace_id, FlowGroupingStrategy::from_str("trace_id"));

    // Test session_id grouping
    let session_id = FlowGroupingStrategy::SessionId;
    assert_eq!(session_id, FlowGroupingStrategy::from_str("session_id"));

    // Test IP+time window grouping
    let ip_time = FlowGroupingStrategy::IpTimeWindow;
    assert_eq!(ip_time, FlowGroupingStrategy::from_str("ip_time_window"));
}

/// Test flow creation
#[tokio::test]
async fn test_flow_creation() {
    let flow = Flow {
        id: "flow-1".to_string(),
        name: Some("checkout_flow".to_string()),
        description: Some("E-commerce checkout flow".to_string()),
        steps: vec![],
        created_at: Utc::now(),
        tags: vec!["ecommerce".to_string(), "checkout".to_string()],
        metadata: HashMap::new(),
    };

    assert_eq!(flow.id, "flow-1");
    assert_eq!(flow.name, Some("checkout_flow".to_string()));
    assert_eq!(flow.steps.len(), 0);
}

/// Test flow step creation
#[tokio::test]
async fn test_flow_step_creation() {
    let step = FlowStep {
        request_id: "req-1".to_string(),
        step_index: 0,
        step_label: Some("login".to_string()),
        timing_ms: None,
    };

    assert_eq!(step.request_id, "req-1");
    assert_eq!(step.step_index, 0);
    assert_eq!(step.step_label, Some("login".to_string()));
}

/// Test behavioral scenario creation
#[tokio::test]
async fn test_behavioral_scenario_creation() {
    let scenario = BehavioralScenario::new("scenario-1", "checkout_success")
        .with_description("Successful checkout scenario")
        .with_strict_mode(true);

    assert_eq!(scenario.id, "scenario-1");
    assert_eq!(scenario.name, "checkout_success");
    assert_eq!(scenario.description, Some("Successful checkout scenario".to_string()));
    assert!(scenario.strict_mode);
    assert_eq!(scenario.steps.len(), 0);
}

/// Test behavioral scenario step creation
#[tokio::test]
async fn test_behavioral_scenario_step() {
    let request = RecordedRequest {
        id: "req-1".to_string(),
        protocol: Protocol::Http,
        method: "POST".to_string(),
        path: "/api/login".to_string(),
        query_params: None,
        headers: json!({}).to_string(),
        body: Some(json!({"email": "user@example.com"}).to_string()),
        body_encoding: "utf8".to_string(),
        timestamp: Utc::now(),
        client_ip: Some("127.0.0.1".to_string()),
        trace_id: Some("trace-123".to_string()),
        span_id: None,
        duration_ms: None,
        status_code: None,
        tags: None,
    };

    let response = RecordedResponse {
        request_id: "req-1".to_string(),
        status_code: 200,
        headers: json!({}).to_string(),
        body: Some(json!({"user_id": "123", "token": "abc"}).to_string()),
        body_encoding: "utf8".to_string(),
        size_bytes: 100,
        timestamp: Utc::now(),
    };

    let step = BehavioralScenarioStep {
        step_id: "step-1".to_string(),
        label: Some("login".to_string()),
        request,
        response,
        timing_ms: None,
        extracts: HashMap::new(),
        depends_on: vec![],
    };

    assert_eq!(step.step_id, "step-1");
    assert_eq!(step.label, Some("login".to_string()));
}

/// Test state variable extraction
#[tokio::test]
async fn test_state_variable() {
    let variable = StateVariable {
        name: "user_id".to_string(),
        json_path: "$.user_id".to_string(),
        extracted_from_step: "step-1".to_string(),
        default_value: None,
    };

    assert_eq!(variable.name, "user_id");
    assert_eq!(variable.json_path, "$.user_id");
    assert_eq!(variable.extracted_from_step, "step-1");
}

/// Test scenario with state variables
#[tokio::test]
async fn test_scenario_with_state_variables() {
    let variable = StateVariable {
        name: "user_id".to_string(),
        json_path: "$.user_id".to_string(),
        extracted_from_step: "step-1".to_string(),
        default_value: None,
    };

    let scenario = BehavioralScenario::new("scenario-1", "checkout").add_state_variable(variable);

    assert_eq!(scenario.state_variables.len(), 1);
    assert!(scenario.state_variables.contains_key("user_id"));
}

/// Test strict mode vs flex mode
#[tokio::test]
async fn test_strict_vs_flex_mode() {
    let strict_scenario = BehavioralScenario::new("scenario-1", "strict").with_strict_mode(true);

    let flex_scenario = BehavioralScenario::new("scenario-2", "flex").with_strict_mode(false);

    assert!(strict_scenario.strict_mode);
    assert!(!flex_scenario.strict_mode);
}

/// Test replay engine creation
#[tokio::test]
async fn test_replay_engine_creation() {
    let engine = BehavioralScenarioReplayEngine::new();
    // Engine should be created successfully without panicking
    let _ = &engine;
}

/// Test scenario activation
#[tokio::test]
async fn test_scenario_activation() {
    let engine = BehavioralScenarioReplayEngine::new();
    let scenario = BehavioralScenario::new("scenario-1", "test");

    let result = engine.activate_scenario(scenario).await;
    assert!(result.is_ok());
}

/// Test scenario deactivation
#[tokio::test]
async fn test_scenario_deactivation() {
    let engine = BehavioralScenarioReplayEngine::new();
    let scenario = BehavioralScenario::new("scenario-1", "test");

    // Activate first
    engine.activate_scenario(scenario).await.unwrap();

    // Then deactivate
    let result = engine.deactivate_scenario("scenario-1").await;
    assert!(result.is_ok());
}

/// Test flow with multiple steps
#[tokio::test]
async fn test_flow_with_steps() {
    let mut flow = Flow {
        id: "flow-1".to_string(),
        name: Some("multi_step_flow".to_string()),
        description: None,
        steps: vec![],
        created_at: Utc::now(),
        tags: vec![],
        metadata: HashMap::new(),
    };

    flow.steps.push(FlowStep {
        request_id: "req-1".to_string(),
        step_index: 0,
        step_label: Some("login".to_string()),
        timing_ms: None,
    });

    flow.steps.push(FlowStep {
        request_id: "req-2".to_string(),
        step_index: 1,
        step_label: Some("list_products".to_string()),
        timing_ms: Some(150),
    });

    assert_eq!(flow.steps.len(), 2);
    assert_eq!(flow.steps[0].step_label, Some("login".to_string()));
    assert_eq!(flow.steps[1].step_label, Some("list_products".to_string()));
    assert_eq!(flow.steps[1].timing_ms, Some(150));
}

/// Test scenario with multiple steps
#[tokio::test]
async fn test_scenario_with_steps() {
    let request1 = RecordedRequest {
        id: "req-1".to_string(),
        protocol: Protocol::Http,
        method: "POST".to_string(),
        path: "/api/login".to_string(),
        query_params: None,
        headers: json!({}).to_string(),
        body: Some(json!({"email": "user@example.com"}).to_string()),
        body_encoding: "utf8".to_string(),
        timestamp: Utc::now(),
        client_ip: None,
        trace_id: None,
        span_id: None,
        duration_ms: None,
        status_code: None,
        tags: None,
    };

    let response1 = RecordedResponse {
        request_id: "req-1".to_string(),
        status_code: 200,
        headers: json!({}).to_string(),
        body: Some(json!({"user_id": "123"}).to_string()),
        body_encoding: "utf8".to_string(),
        size_bytes: 50,
        timestamp: Utc::now(),
    };

    let step1 = BehavioralScenarioStep {
        step_id: "step-1".to_string(),
        label: Some("login".to_string()),
        request: request1,
        response: response1,
        timing_ms: None,
        extracts: {
            let mut map = HashMap::new();
            map.insert("user_id".to_string(), "$.user_id".to_string());
            map
        },
        depends_on: vec![],
    };

    let scenario = BehavioralScenario::new("scenario-1", "test").add_step(step1);

    assert_eq!(scenario.steps.len(), 1);
}

/// Test scenario export/import (JSON serialization)
#[tokio::test]
async fn test_scenario_serialization() {
    let scenario = BehavioralScenario::new("scenario-1", "test")
        .with_description("Test scenario")
        .with_strict_mode(true);

    // Serialize to JSON
    let json = serde_json::to_string(&scenario);
    assert!(json.is_ok());

    // Deserialize from JSON
    let json_str = json.unwrap();
    let deserialized: Result<BehavioralScenario, _> = serde_json::from_str(&json_str);
    assert!(deserialized.is_ok());

    let deserialized_scenario = deserialized.unwrap();
    assert_eq!(deserialized_scenario.id, scenario.id);
    assert_eq!(deserialized_scenario.name, scenario.name);
    assert_eq!(deserialized_scenario.strict_mode, scenario.strict_mode);
}

/// Test flow export/import (JSON serialization)
#[tokio::test]
async fn test_flow_serialization() {
    let flow = Flow {
        id: "flow-1".to_string(),
        name: Some("test_flow".to_string()),
        description: Some("Test flow".to_string()),
        steps: vec![FlowStep {
            request_id: "req-1".to_string(),
            step_index: 0,
            step_label: Some("step1".to_string()),
            timing_ms: Some(100),
        }],
        created_at: Utc::now(),
        tags: vec!["test".to_string()],
        metadata: HashMap::new(),
    };

    // Serialize to JSON
    let json = serde_json::to_string(&flow);
    assert!(json.is_ok());

    // Deserialize from JSON
    let json_str = json.unwrap();
    let deserialized: Result<Flow, _> = serde_json::from_str(&json_str);
    assert!(deserialized.is_ok());

    let deserialized_flow = deserialized.unwrap();
    assert_eq!(deserialized_flow.id, flow.id);
    assert_eq!(deserialized_flow.steps.len(), flow.steps.len());
}

/// Test scenario with dependencies
#[tokio::test]
async fn test_scenario_step_dependencies() {
    let request = RecordedRequest {
        id: "req-1".to_string(),
        protocol: Protocol::Http,
        method: "GET".to_string(),
        path: "/api/orders".to_string(),
        query_params: None,
        headers: json!({}).to_string(),
        body: None,
        body_encoding: "utf8".to_string(),
        timestamp: Utc::now(),
        client_ip: None,
        trace_id: None,
        span_id: None,
        duration_ms: None,
        status_code: None,
        tags: None,
    };

    let response = RecordedResponse {
        request_id: "req-1".to_string(),
        status_code: 200,
        headers: json!({}).to_string(),
        body: Some(json!([]).to_string()),
        body_encoding: "utf8".to_string(),
        size_bytes: 50,
        timestamp: Utc::now(),
    };

    let step = BehavioralScenarioStep {
        step_id: "step-2".to_string(),
        label: Some("list_orders".to_string()),
        request,
        response,
        timing_ms: Some(200),
        extracts: HashMap::new(),
        depends_on: vec!["step-1".to_string()], // Depends on login step
    };

    assert_eq!(step.depends_on.len(), 1);
    assert_eq!(step.depends_on[0], "step-1");
}

/// Test flow recording config defaults
#[tokio::test]
async fn test_flow_recording_config_defaults() {
    let config = FlowRecordingConfig::default();

    assert_eq!(config.group_by, FlowGroupingStrategy::TraceId);
    assert_eq!(config.time_window_seconds, 300);
    assert!(config.enabled);
}

/// Test scenario tags
#[tokio::test]
async fn test_scenario_tags() {
    let mut scenario = BehavioralScenario::new("scenario-1", "test");
    scenario.tags.push("ecommerce".to_string());
    scenario.tags.push("checkout".to_string());

    assert_eq!(scenario.tags.len(), 2);
    assert!(scenario.tags.contains(&"ecommerce".to_string()));
    assert!(scenario.tags.contains(&"checkout".to_string()));
}
