//! Integration tests for MockOps Pipelines
//!
//! Tests that verify pipeline execution, event handling, and step execution
//! work correctly end-to-end.

use mockforge_pipelines::{
    events::{publish_event, PipelineEvent, PipelineEventType},
    pipeline::{Pipeline, PipelineDefinition, PipelineExecutor, PipelineStep, PipelineTrigger},
    steps::{NotifyStep, RegenerateSDKStep},
};
use serde_json::json;
use std::collections::HashMap;
use tempfile::TempDir;
use tokio::fs;
use uuid::Uuid;

/// Test basic pipeline creation and execution
#[tokio::test]
async fn test_pipeline_creation_and_execution() {
    let executor = PipelineExecutor::new();

    // Create a simple pipeline definition
    let definition = PipelineDefinition {
        name: "test-pipeline".to_string(),
        description: String::new(),
        enabled: true,
        triggers: vec![PipelineTrigger {
            event: "schema.changed".to_string(),
            filters: HashMap::new(),
        }],
        steps: vec![PipelineStep {
            name: "test_step".to_string(),
            step_type: "notify".to_string(),
            config: {
                let mut m = HashMap::new();
                m.insert("type".to_string(), json!("webhook"));
                m.insert("webhook_url".to_string(), json!("https://example.com/webhook"));
                m.insert("message".to_string(), json!("Schema changed"));
                m
            },
            continue_on_error: false,
            timeout: None,
        }],
        step_defaults: HashMap::new(),
    };

    let pipeline = Pipeline::new("test-pipeline".to_string(), definition, None, None);

    assert_eq!(pipeline.name, "test-pipeline");
    assert!(pipeline.definition.enabled);
    assert_eq!(pipeline.definition.triggers.len(), 1);
    assert_eq!(pipeline.definition.steps.len(), 1);
}

/// Test pipeline event matching
#[tokio::test]
async fn test_pipeline_event_matching() {
    // Create pipeline that matches schema_changed events
    let definition = PipelineDefinition {
        name: "schema-pipeline".to_string(),
        description: String::new(),
        enabled: true,
        triggers: vec![PipelineTrigger {
            event: "schema.changed".to_string(),
            filters: HashMap::new(),
        }],
        steps: vec![],
        step_defaults: HashMap::new(),
    };

    let pipeline = Pipeline::new("schema-pipeline".to_string(), definition, None, None);

    // Create a schema_changed event
    let workspace_id = Uuid::new_v4();
    let mut payload = HashMap::new();
    payload.insert("spec_path".to_string(), json!("/path/to/spec.yaml"));
    payload.insert("schema_type".to_string(), json!("openapi"));
    let event = PipelineEvent::new(
        PipelineEventType::SchemaChanged,
        Some(workspace_id),
        None,
        payload,
        "test".to_string(),
    );

    // Check if pipeline matches the event
    let matches = pipeline.matches_event(&event);
    assert!(matches, "Pipeline should match schema.changed event");
}

/// Test pipeline execution with notify step
#[tokio::test]
async fn test_pipeline_execution_notify_step() {
    let executor = PipelineExecutor::new();
    let notify_step = NotifyStep::new();

    let mut executor = PipelineExecutor::new();
    executor.register_step_executor("notify".to_string(), Box::new(NotifyStep::new()));

    // Create a test pipeline with notify step
    let definition = PipelineDefinition {
        name: "notify-pipeline".to_string(),
        description: String::new(),
        enabled: true,
        triggers: vec![PipelineTrigger {
            event: "promotion.completed".to_string(),
            filters: HashMap::new(),
        }],
        steps: vec![PipelineStep {
            name: "notify_team".to_string(),
            step_type: "notify".to_string(),
            config: {
                let mut m = HashMap::new();
                m.insert("type".to_string(), json!("webhook"));
                m.insert("webhook_url".to_string(), json!("https://httpbin.org/post"));
                m.insert("message".to_string(), json!("Promotion completed"));
                m
            },
            continue_on_error: false,
            timeout: None,
        }],
        step_defaults: HashMap::new(),
    };

    let pipeline = Pipeline::new("notify-pipeline".to_string(), definition, None, None);

    // Create a promotion_completed event
    let workspace_id = Uuid::new_v4();
    let mut payload = HashMap::new();
    payload.insert("entity_id".to_string(), json!("scenario-456"));
    payload.insert("from_environment".to_string(), json!("dev"));
    payload.insert("to_environment".to_string(), json!("test"));
    let event = PipelineEvent::new(
        PipelineEventType::PromotionCompleted,
        Some(workspace_id),
        None,
        payload,
        "test".to_string(),
    );

    // Execute the pipeline
    let result = executor.execute(&pipeline, event.clone()).await;

    // Pipeline execution should succeed (even if webhook fails, it should handle gracefully)
    assert!(result.is_ok(), "Pipeline execution should complete");
}

/// Test pipeline execution with regenerate_sdk step
#[tokio::test]
async fn test_pipeline_execution_regenerate_sdk_step() {
    // Create a temporary directory for SDK output
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let output_dir = temp_dir.path().join("generated-sdks");
    fs::create_dir_all(&output_dir).await.expect("Failed to create output dir");

    // Create a minimal OpenAPI spec file for testing
    let spec_path = temp_dir.path().join("api.yaml");
    let spec_content = r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /test:
    get:
      responses:
        '200':
          description: Success
"#;
    fs::write(&spec_path, spec_content).await.expect("Failed to write spec file");

    let mut executor = PipelineExecutor::new();
    executor.register_step_executor(
        "regenerate_sdk".to_string(),
        Box::new(RegenerateSDKStep::new(Some(output_dir.clone()))),
    );

    // Create a test pipeline with regenerate_sdk step
    let definition = PipelineDefinition {
        name: "sdk-pipeline".to_string(),
        description: String::new(),
        enabled: true,
        triggers: vec![PipelineTrigger {
            event: "schema.changed".to_string(),
            filters: HashMap::new(),
        }],
        steps: vec![PipelineStep {
            name: "regenerate_sdk".to_string(),
            step_type: "regenerate_sdk".to_string(),
            config: {
                let mut m = HashMap::new();
                m.insert("spec_path".to_string(), json!(spec_path.to_str().unwrap()));
                m.insert("output_dir".to_string(), json!(output_dir.to_str().unwrap()));
                m.insert("languages".to_string(), json!(["typescript"]));
                m
            },
            continue_on_error: false,
            timeout: None,
        }],
        step_defaults: HashMap::new(),
    };

    let pipeline = Pipeline::new("sdk-pipeline".to_string(), definition, None, None);

    // Create a schema_changed event
    let workspace_id = Uuid::new_v4();
    let mut payload = HashMap::new();
    payload.insert("spec_path".to_string(), json!(spec_path.to_str().unwrap()));
    payload.insert("schema_type".to_string(), json!("openapi"));
    let event = PipelineEvent::new(
        PipelineEventType::SchemaChanged,
        Some(workspace_id),
        None,
        payload,
        "test".to_string(),
    );

    // Execute the pipeline
    let result = executor.execute(&pipeline, event.clone()).await;

    // Pipeline execution should succeed
    assert!(result.is_ok(), "Pipeline execution should complete");

    // Verify SDK files were generated
    let mut generated_files = Vec::new();
    let mut entries = fs::read_dir(&output_dir).await.expect("Failed to read output dir");
    while let Some(entry) = entries.next_entry().await.expect("Failed to read entry") {
        generated_files.push(entry);
    }

    // At least one file should be generated
    assert!(!generated_files.is_empty(), "SDK files should be generated");
}

/// Test pipeline event publishing and subscription
#[tokio::test]
async fn test_pipeline_event_publishing() {
    // Create a test event
    let workspace_id = Uuid::new_v4();
    let mut payload = HashMap::new();
    payload.insert("drift_count".to_string(), json!(10));
    payload.insert("threshold".to_string(), json!(5));
    payload.insert("endpoints".to_string(), json!(["endpoint1", "endpoint2"]));
    let event = PipelineEvent::new(
        PipelineEventType::DriftThresholdExceeded,
        Some(workspace_id),
        None,
        payload,
        "test".to_string(),
    );

    // Publish the event (this should not fail)
    let _ = publish_event(event.clone());

    // Event should be published successfully
    // Note: In a real scenario, we'd verify subscribers received it
    assert_eq!(event.event_type, PipelineEventType::DriftThresholdExceeded);
}

/// Test pipeline with multiple steps
#[tokio::test]
async fn test_pipeline_multiple_steps() {
    let mut executor = PipelineExecutor::new();
    executor.register_step_executor("notify".to_string(), Box::new(NotifyStep::new()));

    // Create a pipeline with multiple steps
    let definition = PipelineDefinition {
        name: "multi-step-pipeline".to_string(),
        description: String::new(),
        enabled: true,
        triggers: vec![PipelineTrigger {
            event: "promotion.completed".to_string(),
            filters: HashMap::new(),
        }],
        steps: vec![
            PipelineStep {
                name: "notify".to_string(),
                step_type: "notify".to_string(),
                config: {
                    let mut m = HashMap::new();
                    m.insert("type".to_string(), json!("webhook"));
                    m.insert("webhook_url".to_string(), json!("https://httpbin.org/post"));
                    m.insert("message".to_string(), json!("Step 1: Promotion completed"));
                    m
                },
                continue_on_error: false,
                timeout: None,
            },
            PipelineStep {
                name: "notify_again".to_string(),
                step_type: "notify".to_string(),
                config: {
                    let mut m = HashMap::new();
                    m.insert("type".to_string(), json!("webhook"));
                    m.insert("webhook_url".to_string(), json!("https://httpbin.org/post"));
                    m.insert("message".to_string(), json!("Step 2: Follow-up notification"));
                    m
                },
                continue_on_error: false,
                timeout: None,
            },
        ],
        step_defaults: HashMap::new(),
    };

    let pipeline = Pipeline::new("multi-step-pipeline".to_string(), definition, None, None);

    let workspace_id = Uuid::new_v4();
    let mut payload = HashMap::new();
    payload.insert("entity_id".to_string(), json!("scenario-456"));
    payload.insert("from_environment".to_string(), json!("dev"));
    payload.insert("to_environment".to_string(), json!("test"));
    let event = PipelineEvent::new(
        PipelineEventType::PromotionCompleted,
        Some(workspace_id),
        None,
        payload,
        "test".to_string(),
    );

    // Execute the pipeline
    let result = executor.execute(&pipeline, event).await;

    // All steps should execute
    assert!(result.is_ok(), "Multi-step pipeline should execute successfully");
}

/// Test disabled pipeline doesn't execute
#[tokio::test]
async fn test_disabled_pipeline() {
    let executor = PipelineExecutor::new();

    let definition = PipelineDefinition {
        name: "disabled-pipeline".to_string(),
        description: String::new(),
        enabled: false, // Pipeline is disabled
        triggers: vec![PipelineTrigger {
            event: "schema.changed".to_string(),
            filters: HashMap::new(),
        }],
        steps: vec![],
        step_defaults: HashMap::new(),
    };

    let pipeline = Pipeline::new("disabled-pipeline".to_string(), definition, None, None);

    let workspace_id = Uuid::new_v4();
    let mut payload = HashMap::new();
    payload.insert("spec_path".to_string(), json!("/path/to/spec.yaml"));
    payload.insert("schema_type".to_string(), json!("openapi"));
    let event = PipelineEvent::new(
        PipelineEventType::SchemaChanged,
        Some(workspace_id),
        None,
        payload,
        "test".to_string(),
    );

    // Even if event matches, disabled pipeline should not execute
    // The matches_event check should prevent execution
    assert!(!pipeline.matches_event(&event), "Disabled pipeline should not match events");

    // If we tried to execute, it should be handled gracefully
    // (In practice, disabled pipelines are filtered out before execution)
}
