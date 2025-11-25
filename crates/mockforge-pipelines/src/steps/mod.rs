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
