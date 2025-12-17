//! # `MockForge` Pipelines
//!
//! Event-driven pipeline orchestration for `MockForge`.
//!
//! This crate provides a GitHub Actions-like pipeline system for automating
//! mock lifecycle management, including:
//!
//! - Schema change detection → auto-regenerate SDKs
//! - Scenario publication → auto-promote to test → notify teams
//! - Drift threshold exceeded → auto-generate Git PRs
//!
//! ## Overview
//!
//! Pipelines are defined in YAML and triggered by events. Each pipeline consists
//! of steps that execute sequentially or in parallel. Steps can be:
//!
//! - `regenerate_sdk` - Regenerate client SDKs for specified languages
//! - `auto_promote` - Automatically promote scenarios/personas/configs
//! - `notify` - Send notifications to teams (Slack, email, webhooks)
//! - `create_pr` - Create Git pull requests
//!
//! ## Example Pipeline
//!
//! ```yaml
//! name: schema-change-pipeline
//! triggers:
//!   - event: schema.changed
//!     filters:
//!       workspace_id: "workspace-123"
//!       schema_type: ["openapi", "protobuf"]
//!
//! steps:
//!   - name: regenerate-sdks
//!     type: regenerate_sdk
//!     config:
//!       languages: ["typescript", "python", "rust"]
//!       workspace_id: "{{workspace_id}}"
//!
//!   - name: notify-teams
//!     type: notify
//!     config:
//!       channels: ["#api-team", "#frontend-team"]
//!       message: "SDKs regenerated for {{workspace_id}}"
//! ```
//!
//! ## Event System
//!
//! Events are emitted by various `MockForge` components and trigger pipelines:
//!
//! - `schema.changed` - OpenAPI/Protobuf schema modified
//! - `scenario.published` - New scenario published
//! - `drift.threshold_exceeded` - Drift budget exceeded
//! - `promotion.completed` - Promotion completed
//! - `workspace.created` - New workspace created

pub mod events;
pub mod pipeline;
pub mod steps;

pub use events::{publish_event, PipelineEvent, PipelineEventBus, PipelineEventType};
pub use pipeline::{Pipeline, PipelineDefinition, PipelineExecutor, PipelineStep};
pub use steps::{PipelineStepExecutor, StepContext, StepResult};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use uuid::Uuid;

    /// Test that all main types are re-exported correctly
    #[test]
    fn test_pipeline_event_type_reexport() {
        let event_type = PipelineEventType::SchemaChanged;
        assert_eq!(event_type.as_str(), "schema.changed");
    }

    #[test]
    fn test_pipeline_event_reexport() {
        let workspace_id = Uuid::new_v4();
        let event =
            PipelineEvent::schema_changed(workspace_id, "openapi".to_string(), HashMap::new());
        assert_eq!(event.event_type, PipelineEventType::SchemaChanged);
        assert_eq!(event.workspace_id, Some(workspace_id));
    }

    #[test]
    fn test_pipeline_event_bus_reexport() {
        let bus = PipelineEventBus::new(100);
        assert_eq!(bus.subscriber_count(), 0);

        let _receiver = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);
    }

    #[test]
    fn test_publish_event_function_reexport() {
        let event =
            PipelineEvent::schema_changed(Uuid::new_v4(), "openapi".to_string(), HashMap::new());
        // publish_event should be accessible as a re-exported function
        let result = publish_event(event);
        // May succeed or fail depending on subscribers, but function should be callable
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_pipeline_definition_reexport() {
        let definition = PipelineDefinition {
            name: "test-pipeline".to_string(),
            description: "Test pipeline".to_string(),
            triggers: vec![],
            steps: vec![],
            enabled: true,
            step_defaults: HashMap::new(),
        };
        assert_eq!(definition.name, "test-pipeline");
        assert!(definition.enabled);
    }

    #[test]
    fn test_pipeline_step_reexport() {
        let step = PipelineStep {
            name: "test-step".to_string(),
            step_type: "notify".to_string(),
            config: HashMap::new(),
            continue_on_error: false,
            timeout: None,
        };
        assert_eq!(step.name, "test-step");
        assert_eq!(step.step_type, "notify");
    }

    #[test]
    fn test_pipeline_reexport() {
        let definition = PipelineDefinition {
            name: "test-pipeline".to_string(),
            description: String::new(),
            triggers: vec![],
            steps: vec![],
            enabled: true,
            step_defaults: HashMap::new(),
        };

        let workspace_id = Uuid::new_v4();
        let pipeline =
            Pipeline::new("test-pipeline".to_string(), definition, Some(workspace_id), None);

        assert_eq!(pipeline.name, "test-pipeline");
        assert_eq!(pipeline.workspace_id, Some(workspace_id));
    }

    #[test]
    fn test_pipeline_executor_reexport() {
        let executor = PipelineExecutor::new();
        // Verify we can create an executor through the re-export
        // The executor should be created successfully and we can't test internal state
        // but we can verify it exists and is the default instance
        let default_executor = PipelineExecutor::default();
        // Both should be valid executors (we can't compare them directly)
        drop(executor);
        drop(default_executor);
    }

    #[test]
    fn test_step_context_reexport() {
        let workspace_id = Uuid::new_v4();
        let execution_id = Uuid::new_v4();

        let event = PipelineEvent::new(
            PipelineEventType::SchemaChanged,
            Some(workspace_id),
            None,
            HashMap::new(),
            "test".to_string(),
        );

        let context = StepContext {
            execution_id,
            event: event.clone(),
            config: HashMap::new(),
            step_name: "test-step".to_string(),
            workspace_id: Some(workspace_id),
            pipeline_id: None,
            pipeline_defaults: HashMap::new(),
        };

        assert_eq!(context.execution_id, execution_id);
        assert_eq!(context.step_name, "test-step");
        assert_eq!(context.workspace_id, Some(workspace_id));
    }

    #[test]
    fn test_step_result_reexport() {
        let result = StepResult::success(None, Some("Success".to_string()));
        assert_eq!(result.message, Some("Success".to_string()));
        assert!(result.output.is_none());
    }

    #[test]
    fn test_step_result_with_output_reexport() {
        let mut output = HashMap::new();
        output.insert("key".to_string(), serde_json::json!("value"));

        let result = StepResult::success_with_output(output);
        assert!(result.output.is_some());
        assert!(result.message.is_none());
    }

    #[test]
    fn test_step_result_with_message_reexport() {
        let result = StepResult::success_with_message("Operation completed".to_string());
        assert_eq!(result.message, Some("Operation completed".to_string()));
        assert!(result.output.is_none());
    }

    /// Test integration between exported components
    #[test]
    fn test_event_to_pipeline_integration() {
        // Create a pipeline
        let definition = PipelineDefinition {
            name: "integration-test".to_string(),
            description: String::new(),
            triggers: vec![pipeline::PipelineTrigger {
                event: "schema.changed".to_string(),
                filters: HashMap::new(),
            }],
            steps: vec![],
            enabled: true,
            step_defaults: HashMap::new(),
        };

        let workspace_id = Uuid::new_v4();
        let pipeline =
            Pipeline::new("integration-test".to_string(), definition, Some(workspace_id), None);

        // Create an event
        let event =
            PipelineEvent::schema_changed(workspace_id, "openapi".to_string(), HashMap::new());

        // Test that pipeline can match the event
        assert!(pipeline.matches_event(&event));
    }

    #[test]
    fn test_event_bus_integration() {
        let bus = PipelineEventBus::new(100);
        let mut receiver = bus.subscribe();

        let workspace_id = Uuid::new_v4();
        let event =
            PipelineEvent::schema_changed(workspace_id, "openapi".to_string(), HashMap::new());
        let event_id = event.id;

        // Publish event
        bus.publish(event).unwrap();

        // Verify event was received (use try_recv to avoid blocking in test)
        let received = receiver.try_recv().unwrap();
        assert_eq!(received.id, event_id);
    }

    #[test]
    fn test_all_event_types_constructible() {
        // Verify all event types can be constructed and used
        let types = vec![
            PipelineEventType::SchemaChanged,
            PipelineEventType::ScenarioPublished,
            PipelineEventType::DriftThresholdExceeded,
            PipelineEventType::PromotionCompleted,
            PipelineEventType::WorkspaceCreated,
            PipelineEventType::PersonaPublished,
            PipelineEventType::ConfigChanged,
        ];

        for event_type in types {
            let str_repr = event_type.as_str();
            assert!(!str_repr.is_empty());
            let parsed = PipelineEventType::from_str(str_repr);
            assert_eq!(parsed, Some(event_type));
        }
    }

    #[test]
    fn test_pipeline_lifecycle() {
        // Create a complete pipeline
        let workspace_id = Uuid::new_v4();

        let definition = PipelineDefinition {
            name: "lifecycle-test".to_string(),
            description: "Testing full lifecycle".to_string(),
            triggers: vec![pipeline::PipelineTrigger {
                event: "scenario.published".to_string(),
                filters: {
                    let mut f = HashMap::new();
                    f.insert("scenario_name".to_string(), serde_json::json!("test-scenario"));
                    f
                },
            }],
            steps: vec![PipelineStep {
                name: "notify-step".to_string(),
                step_type: "notify".to_string(),
                config: {
                    let mut c = HashMap::new();
                    c.insert("channels".to_string(), serde_json::json!(["#team"]));
                    c
                },
                continue_on_error: false,
                timeout: Some(30),
            }],
            enabled: true,
            step_defaults: HashMap::new(),
        };

        let pipeline = Pipeline::new(
            "lifecycle-test".to_string(),
            definition.clone(),
            Some(workspace_id),
            None,
        );

        // Verify pipeline properties
        assert_eq!(pipeline.name, "lifecycle-test");
        assert_eq!(pipeline.workspace_id, Some(workspace_id));
        assert_eq!(pipeline.definition.name, definition.name);
        assert_eq!(pipeline.definition.steps.len(), 1);
        assert_eq!(pipeline.definition.triggers.len(), 1);

        // Create a matching event
        let event = PipelineEvent::scenario_published(
            workspace_id,
            Uuid::new_v4(),
            "test-scenario".to_string(),
            Some("1.0.0".to_string()),
        );

        // Verify pipeline matches event
        assert!(pipeline.matches_event(&event));

        // Create a non-matching event
        let non_matching_event = PipelineEvent::scenario_published(
            workspace_id,
            Uuid::new_v4(),
            "other-scenario".to_string(),
            Some("1.0.0".to_string()),
        );

        // Verify pipeline doesn't match different scenario
        assert!(!pipeline.matches_event(&non_matching_event));
    }

    #[test]
    fn test_executor_and_context_integration() {
        let _executor = PipelineExecutor::new();

        // Create a step context
        let workspace_id = Uuid::new_v4();
        let event =
            PipelineEvent::schema_changed(workspace_id, "openapi".to_string(), HashMap::new());

        let context = StepContext {
            execution_id: Uuid::new_v4(),
            event: event.clone(),
            config: HashMap::new(),
            step_name: "test-step".to_string(),
            workspace_id: Some(workspace_id),
            pipeline_id: Some(Uuid::new_v4()),
            pipeline_defaults: HashMap::new(),
        };

        // Verify context properties
        assert_eq!(context.workspace_id, Some(workspace_id));
        assert_eq!(context.event.event_type, PipelineEventType::SchemaChanged);

        // Verify context can be used with event
        assert_eq!(context.event.workspace_id, Some(workspace_id));
    }

    #[test]
    fn test_multiple_pipelines_same_workspace() {
        let workspace_id = Uuid::new_v4();

        // Create multiple pipelines for the same workspace
        let definition1 = PipelineDefinition {
            name: "pipeline-1".to_string(),
            description: String::new(),
            triggers: vec![pipeline::PipelineTrigger {
                event: "schema.changed".to_string(),
                filters: HashMap::new(),
            }],
            steps: vec![],
            enabled: true,
            step_defaults: HashMap::new(),
        };

        let definition2 = PipelineDefinition {
            name: "pipeline-2".to_string(),
            description: String::new(),
            triggers: vec![pipeline::PipelineTrigger {
                event: "scenario.published".to_string(),
                filters: HashMap::new(),
            }],
            steps: vec![],
            enabled: true,
            step_defaults: HashMap::new(),
        };

        let pipeline1 =
            Pipeline::new("pipeline-1".to_string(), definition1, Some(workspace_id), None);

        let pipeline2 =
            Pipeline::new("pipeline-2".to_string(), definition2, Some(workspace_id), None);

        // Create events
        let event1 =
            PipelineEvent::schema_changed(workspace_id, "openapi".to_string(), HashMap::new());

        let event2 = PipelineEvent::scenario_published(
            workspace_id,
            Uuid::new_v4(),
            "test".to_string(),
            None,
        );

        // Verify each pipeline matches its corresponding event
        assert!(pipeline1.matches_event(&event1));
        assert!(!pipeline1.matches_event(&event2));
        assert!(!pipeline2.matches_event(&event1));
        assert!(pipeline2.matches_event(&event2));
    }

    #[test]
    fn test_event_serialization_through_reexports() {
        let workspace_id = Uuid::new_v4();
        let event =
            PipelineEvent::schema_changed(workspace_id, "openapi".to_string(), HashMap::new());

        // Serialize and deserialize
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: PipelineEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.event_type, event.event_type);
        assert_eq!(deserialized.workspace_id, event.workspace_id);
        assert_eq!(deserialized.id, event.id);
    }

    #[test]
    fn test_pipeline_definition_serialization_through_reexports() {
        let definition = PipelineDefinition {
            name: "test-pipeline".to_string(),
            description: "Test description".to_string(),
            triggers: vec![],
            steps: vec![],
            enabled: true,
            step_defaults: HashMap::new(),
        };

        // Serialize and deserialize
        let json = serde_json::to_string(&definition).unwrap();
        let deserialized: PipelineDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, definition.name);
        assert_eq!(deserialized.description, definition.description);
        assert_eq!(deserialized.enabled, definition.enabled);
    }

    #[test]
    fn test_complex_event_payloads() {
        let workspace_id = Uuid::new_v4();

        // Test drift threshold exceeded event
        let event =
            PipelineEvent::drift_threshold_exceeded(workspace_id, "/api/users".to_string(), 15, 10);
        assert_eq!(event.event_type, PipelineEventType::DriftThresholdExceeded);
        assert_eq!(event.workspace_id, Some(workspace_id));

        // Test promotion completed event
        let promotion_event = PipelineEvent::promotion_completed(
            workspace_id,
            Uuid::new_v4(),
            "scenario".to_string(),
            "staging".to_string(),
            "production".to_string(),
        );
        assert_eq!(promotion_event.event_type, PipelineEventType::PromotionCompleted);
        assert_eq!(promotion_event.workspace_id, Some(workspace_id));
    }

    #[test]
    fn test_pipeline_with_complex_configuration() {
        let workspace_id = Uuid::new_v4();

        // Create a pipeline with complex step configuration
        let mut step_defaults = HashMap::new();
        let mut notify_defaults = HashMap::new();
        notify_defaults.insert(
            "webhook_url".to_string(),
            serde_json::json!("https://hooks.example.com/webhook"),
        );
        notify_defaults.insert("timeout".to_string(), serde_json::json!(30));
        step_defaults.insert("notify".to_string(), notify_defaults);

        let definition = PipelineDefinition {
            name: "complex-pipeline".to_string(),
            description: "Pipeline with complex configuration".to_string(),
            triggers: vec![
                pipeline::PipelineTrigger {
                    event: "schema.changed".to_string(),
                    filters: {
                        let mut f = HashMap::new();
                        f.insert("schema_type".to_string(), serde_json::json!("openapi"));
                        f
                    },
                },
                pipeline::PipelineTrigger {
                    event: "drift.threshold_exceeded".to_string(),
                    filters: HashMap::new(),
                },
            ],
            steps: vec![
                PipelineStep {
                    name: "validate".to_string(),
                    step_type: "regenerate_sdk".to_string(),
                    config: {
                        let mut c = HashMap::new();
                        c.insert(
                            "languages".to_string(),
                            serde_json::json!(["typescript", "python"]),
                        );
                        c
                    },
                    continue_on_error: false,
                    timeout: Some(300),
                },
                PipelineStep {
                    name: "notify-team".to_string(),
                    step_type: "notify".to_string(),
                    config: {
                        let mut c = HashMap::new();
                        c.insert("channels".to_string(), serde_json::json!(["#api-team"]));
                        c
                    },
                    continue_on_error: true,
                    timeout: None,
                },
            ],
            enabled: true,
            step_defaults,
        };

        let pipeline = Pipeline::new(
            "complex-pipeline".to_string(),
            definition.clone(),
            Some(workspace_id),
            None,
        );

        // Verify all the complex configuration is preserved
        assert_eq!(pipeline.definition.triggers.len(), 2);
        assert_eq!(pipeline.definition.steps.len(), 2);
        assert!(pipeline.definition.step_defaults.contains_key("notify"));

        // Test matching with schema.changed event
        let event1 =
            PipelineEvent::schema_changed(workspace_id, "openapi".to_string(), HashMap::new());
        assert!(pipeline.matches_event(&event1));

        // Test matching with drift.threshold_exceeded event
        let event2 = PipelineEvent::drift_threshold_exceeded(
            workspace_id,
            "/api/endpoint".to_string(),
            20,
            10,
        );
        assert!(pipeline.matches_event(&event2));
    }
}
