//! Pipeline step executors
//!
//! Each step type has its own executor that implements `PipelineStepExecutor`.

pub mod auto_promote;
pub mod create_pr;
pub mod notify;
pub mod regenerate_sdk;

pub use auto_promote::AutoPromoteStep;
pub use create_pr::CreatePRStep;
pub use notify::NotifyStep;
pub use regenerate_sdk::RegenerateSDKStep;

use crate::events::PipelineEvent;
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Step execution context
#[derive(Debug, Clone)]
pub struct StepContext {
    /// Execution ID
    pub execution_id: Uuid,
    /// Triggering event
    pub event: PipelineEvent,
    /// Step configuration (with template variables rendered)
    pub config: HashMap<String, Value>,
    /// Step name
    pub step_name: String,
    /// Workspace ID (for workspace-level configuration lookups)
    pub workspace_id: Option<Uuid>,
    /// Pipeline ID (for pipeline-level configuration lookups)
    pub pipeline_id: Option<Uuid>,
    /// Pipeline-level defaults (merged from pipeline definition)
    pub pipeline_defaults: HashMap<String, Value>,
}

/// Step execution result
#[derive(Debug, Clone)]
pub struct StepResult {
    /// Step output (step-specific data)
    pub output: Option<HashMap<String, Value>>,
    /// Success message
    pub message: Option<String>,
}

impl StepResult {
    /// Create a successful result
    #[must_use]
    pub const fn success(output: Option<HashMap<String, Value>>, message: Option<String>) -> Self {
        Self { output, message }
    }

    /// Create a successful result with output
    #[must_use]
    pub const fn success_with_output(output: HashMap<String, Value>) -> Self {
        Self {
            output: Some(output),
            message: None,
        }
    }

    /// Create a successful result with message
    #[must_use]
    pub const fn success_with_message(message: String) -> Self {
        Self {
            output: None,
            message: Some(message),
        }
    }
}

/// Trait for pipeline step executors
#[async_trait::async_trait]
pub trait PipelineStepExecutor: Send + Sync {
    /// Execute the step
    async fn execute(&self, context: StepContext) -> Result<StepResult>;

    /// Get step type name
    fn step_type(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{PipelineEvent, PipelineEventType};

    #[test]
    fn test_step_result_success() {
        let result = StepResult::success(None, Some("done".to_string()));
        assert!(result.output.is_none());
        assert_eq!(result.message, Some("done".to_string()));
    }

    #[test]
    fn test_step_result_success_with_output() {
        let mut output = HashMap::new();
        output.insert("key".to_string(), serde_json::json!("value"));

        let result = StepResult::success_with_output(output);
        assert!(result.output.is_some());
        assert!(result.message.is_none());
        assert_eq!(result.output.unwrap().get("key"), Some(&serde_json::json!("value")));
    }

    #[test]
    fn test_step_result_success_with_message() {
        let result = StepResult::success_with_message("Operation completed".to_string());
        assert!(result.output.is_none());
        assert_eq!(result.message, Some("Operation completed".to_string()));
    }

    #[test]
    fn test_step_context_creation() {
        let workspace_id = Uuid::new_v4();
        let pipeline_id = Uuid::new_v4();
        let execution_id = Uuid::new_v4();

        let event = PipelineEvent::new(
            PipelineEventType::SchemaChanged,
            Some(workspace_id),
            None,
            HashMap::new(),
            "test".to_string(),
        );

        let mut config = HashMap::new();
        config.insert("key".to_string(), serde_json::json!("value"));

        let context = StepContext {
            execution_id,
            event: event.clone(),
            config,
            step_name: "test-step".to_string(),
            workspace_id: Some(workspace_id),
            pipeline_id: Some(pipeline_id),
            pipeline_defaults: HashMap::new(),
        };

        assert_eq!(context.execution_id, execution_id);
        assert_eq!(context.step_name, "test-step");
        assert_eq!(context.workspace_id, Some(workspace_id));
        assert_eq!(context.pipeline_id, Some(pipeline_id));
    }

    #[test]
    fn test_step_context_clone() {
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
            event,
            config: HashMap::new(),
            step_name: "test-step".to_string(),
            workspace_id: Some(workspace_id),
            pipeline_id: None,
            pipeline_defaults: HashMap::new(),
        };

        let cloned = context.clone();
        assert_eq!(context.execution_id, cloned.execution_id);
        assert_eq!(context.step_name, cloned.step_name);
    }

    #[test]
    fn test_step_result_clone() {
        let mut output = HashMap::new();
        output.insert("key".to_string(), serde_json::json!("value"));

        let result = StepResult {
            output: Some(output),
            message: Some("done".to_string()),
        };

        let cloned = result.clone();
        assert_eq!(result.message, cloned.message);
        assert!(cloned.output.is_some());
    }

    #[test]
    fn test_step_context_debug() {
        let execution_id = Uuid::new_v4();
        let event = PipelineEvent::new(
            PipelineEventType::SchemaChanged,
            None,
            None,
            HashMap::new(),
            "test".to_string(),
        );

        let context = StepContext {
            execution_id,
            event,
            config: HashMap::new(),
            step_name: "test".to_string(),
            workspace_id: None,
            pipeline_id: None,
            pipeline_defaults: HashMap::new(),
        };

        let debug = format!("{:?}", context);
        assert!(debug.contains("StepContext"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_step_result_debug() {
        let result = StepResult::success_with_message("test".to_string());
        let debug = format!("{:?}", result);
        assert!(debug.contains("StepResult"));
    }
}
