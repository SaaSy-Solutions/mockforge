//! Pipeline definition and execution
//!
//! Pipelines are defined in YAML and executed when matching events are received.

use crate::events::{PipelineEvent, PipelineEventType};
use crate::steps::{PipelineStepExecutor, StepContext, StepResult};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Pipeline trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineTrigger {
    /// Event type that triggers this pipeline
    pub event: String,
    /// Filters to match events
    #[serde(default)]
    pub filters: HashMap<String, serde_json::Value>,
}

/// Pipeline step configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    /// Step name (for logging and identification)
    pub name: String,
    /// Step type (`regenerate_sdk`, `auto_promote`, notify, `create_pr`)
    pub step_type: String,
    /// Step configuration (type-specific)
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
    /// Whether to continue on error
    #[serde(default = "default_continue_on_error")]
    pub continue_on_error: bool,
    /// Timeout in seconds (None = no timeout)
    #[serde(default)]
    pub timeout: Option<u64>,
}

const fn default_continue_on_error() -> bool {
    false
}

/// Pipeline definition (YAML structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineDefinition {
    /// Pipeline name
    pub name: String,
    /// Pipeline description
    #[serde(default)]
    pub description: String,
    /// Triggers that activate this pipeline
    pub triggers: Vec<PipelineTrigger>,
    /// Steps to execute
    pub steps: Vec<PipelineStep>,
    /// Whether pipeline is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Pipeline-level defaults for steps (e.g., PR configuration)
    /// These defaults are merged with step-specific config, with step config taking precedence
    #[serde(default)]
    pub step_defaults: HashMap<String, HashMap<String, serde_json::Value>>,
}

const fn default_enabled() -> bool {
    true
}

/// Pipeline metadata (stored in database)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    /// Pipeline ID
    pub id: Uuid,
    /// Pipeline name
    pub name: String,
    /// Pipeline definition
    pub definition: PipelineDefinition,
    /// Workspace ID (if workspace-specific)
    pub workspace_id: Option<Uuid>,
    /// Organization ID (if org-wide)
    pub org_id: Option<Uuid>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl Pipeline {
    /// Create a new pipeline
    #[must_use]
    pub fn new(
        name: String,
        definition: PipelineDefinition,
        workspace_id: Option<Uuid>,
        org_id: Option<Uuid>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            definition,
            workspace_id,
            org_id,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if pipeline matches an event
    #[must_use]
    pub fn matches_event(&self, event: &PipelineEvent) -> bool {
        if !self.definition.enabled {
            return false;
        }

        // Check workspace/org scope
        if let Some(workspace_id) = self.workspace_id {
            if event.workspace_id != Some(workspace_id) {
                return false;
            }
        }

        if let Some(org_id) = self.org_id {
            if event.org_id != Some(org_id) {
                return false;
            }
        }

        // Check triggers
        for trigger in &self.definition.triggers {
            if let Some(event_type) = PipelineEventType::from_str(&trigger.event) {
                if event.event_type == event_type {
                    // Check filters
                    if self.matches_filters(&trigger.filters, event) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check if event matches trigger filters
    fn matches_filters(
        &self,
        filters: &HashMap<String, serde_json::Value>,
        event: &PipelineEvent,
    ) -> bool {
        for (key, value) in filters {
            match key.as_str() {
                "workspace_id" => {
                    if let Some(ws_id) = event.workspace_id {
                        let filter_id = value.as_str().and_then(|s| Uuid::parse_str(s).ok());
                        if filter_id != Some(ws_id) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                "schema_type" => {
                    if let Some(schema_type) = event.payload.get("schema_type") {
                        if schema_type != value {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                _ => {
                    // Check payload fields
                    if let Some(payload_value) = event.payload.get(key) {
                        if payload_value != value {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
            }
        }
        true
    }
}

/// Pipeline execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineExecutionStatus {
    /// Execution started
    Started,
    /// Execution in progress
    Running,
    /// Execution completed successfully
    Completed,
    /// Execution failed
    Failed,
    /// Execution cancelled
    Cancelled,
}

/// Step execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionResult {
    /// Step name
    pub step_name: String,
    /// Step type
    pub step_type: String,
    /// Status
    pub status: String,
    /// Started timestamp
    pub started_at: DateTime<Utc>,
    /// Completed timestamp
    pub completed_at: Option<DateTime<Utc>>,
    /// Output (step-specific)
    pub output: Option<HashMap<String, serde_json::Value>>,
    /// Error message (if failed)
    pub error_message: Option<String>,
}

/// Pipeline execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineExecution {
    /// Execution ID
    pub id: Uuid,
    /// Pipeline ID
    pub pipeline_id: Uuid,
    /// Triggering event
    pub trigger_event: PipelineEvent,
    /// Execution status
    pub status: PipelineExecutionStatus,
    /// Started timestamp
    pub started_at: DateTime<Utc>,
    /// Completed timestamp
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Execution log (step results)
    pub execution_log: Vec<StepExecutionResult>,
}

/// Pipeline executor
pub struct PipelineExecutor {
    /// Step executors by type
    step_executors: HashMap<String, Box<dyn PipelineStepExecutor + Send + Sync>>,
    /// Template engine for variable substitution
    handlebars: Handlebars<'static>,
}

impl PipelineExecutor {
    /// Create a new pipeline executor
    #[must_use]
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(false);

        Self {
            step_executors: HashMap::new(),
            handlebars,
        }
    }

    /// Register a step executor
    pub fn register_step_executor(
        &mut self,
        step_type: String,
        executor: Box<dyn PipelineStepExecutor + Send + Sync>,
    ) {
        self.step_executors.insert(step_type, executor);
    }

    /// Execute a pipeline
    pub async fn execute(
        &self,
        pipeline: &Pipeline,
        event: PipelineEvent,
    ) -> Result<PipelineExecution> {
        let execution_id = Uuid::new_v4();
        let started_at = Utc::now();

        info!(
            execution_id = %execution_id,
            pipeline_id = %pipeline.id,
            pipeline_name = %pipeline.name,
            event_type = ?event.event_type,
            "Starting pipeline execution"
        );

        let mut execution_log = Vec::new();
        let mut status = PipelineExecutionStatus::Running;

        // Execute steps sequentially
        for step in &pipeline.definition.steps {
            let step_result = self.execute_step(step, &event, &execution_id, pipeline).await;

            let step_execution = StepExecutionResult {
                step_name: step.name.clone(),
                step_type: step.step_type.clone(),
                status: match &step_result {
                    Ok(_) => "completed".to_string(),
                    Err(_) => "failed".to_string(),
                },
                started_at: Utc::now(),
                completed_at: Some(Utc::now()),
                output: step_result.as_ref().ok().and_then(|r| r.output.clone()),
                error_message: step_result.as_ref().err().map(ToString::to_string),
            };

            execution_log.push(step_execution);

            match step_result {
                Ok(_) => {
                    debug!(
                        execution_id = %execution_id,
                        step_name = %step.name,
                        "Step completed successfully"
                    );
                }
                Err(e) => {
                    error!(
                        execution_id = %execution_id,
                        step_name = %step.name,
                        error = %e,
                        "Step failed"
                    );

                    if step.continue_on_error {
                        warn!(
                            execution_id = %execution_id,
                            step_name = %step.name,
                            "Step failed but continuing due to continue_on_error=true"
                        );
                    } else {
                        status = PipelineExecutionStatus::Failed;
                        break;
                    }
                }
            }
        }

        if status == PipelineExecutionStatus::Running {
            status = PipelineExecutionStatus::Completed;
        }

        let completed_at = Some(Utc::now());

        info!(
            execution_id = %execution_id,
            pipeline_id = %pipeline.id,
            status = ?status,
            "Pipeline execution completed"
        );

        Ok(PipelineExecution {
            id: execution_id,
            pipeline_id: pipeline.id,
            trigger_event: event,
            status,
            started_at,
            completed_at,
            error_message: None,
            execution_log,
        })
    }

    /// Execute a single step
    async fn execute_step(
        &self,
        step: &PipelineStep,
        event: &PipelineEvent,
        execution_id: &Uuid,
        pipeline: &Pipeline,
    ) -> Result<StepResult> {
        // Get pipeline-level defaults for this step type (if any)
        let pipeline_defaults = pipeline
            .definition
            .step_defaults
            .get(&step.step_type)
            .cloned()
            .unwrap_or_default();

        // Merge pipeline defaults with step config (step config takes precedence)
        let mut merged_config = pipeline_defaults.clone();
        for (key, value) in &step.config {
            merged_config.insert(key.clone(), value.clone());
        }

        // Render template variables in merged config
        let rendered_config = self.render_config(&merged_config, event)?;

        // Get step executor
        let executor = self
            .step_executors
            .get(&step.step_type)
            .ok_or_else(|| anyhow::anyhow!("Unknown step type: {}", step.step_type))?;

        // Create step context with workspace and pipeline IDs
        let context = StepContext {
            execution_id: *execution_id,
            event: event.clone(),
            config: rendered_config,
            step_name: step.name.clone(),
            workspace_id: pipeline.workspace_id,
            pipeline_id: Some(pipeline.id),
            pipeline_defaults,
        };

        // Execute with timeout if specified
        let result = if let Some(timeout_secs) = step.timeout {
            tokio::time::timeout(
                std::time::Duration::from_secs(timeout_secs),
                executor.execute(context),
            )
            .await
            .map_err(|_| anyhow::anyhow!("Step timed out after {timeout_secs} seconds"))?
        } else {
            executor.execute(context).await
        };

        result.context(format!("Step '{}' failed", step.name))
    }

    /// Render template variables in config
    fn render_config(
        &self,
        config: &HashMap<String, serde_json::Value>,
        event: &PipelineEvent,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut rendered = HashMap::new();

        // Build template context
        let mut template_data = serde_json::Map::new();
        template_data.insert(
            "workspace_id".to_string(),
            serde_json::to_value(event.workspace_id.map(|id| id.to_string()).unwrap_or_default())?,
        );
        template_data.insert(
            "org_id".to_string(),
            serde_json::to_value(event.org_id.map(|id| id.to_string()).unwrap_or_default())?,
        );
        template_data
            .insert("event_type".to_string(), serde_json::to_value(event.event_type.as_str())?);

        // Add payload to template context
        for (key, value) in &event.payload {
            template_data.insert(key.clone(), value.clone());
        }

        // Render each config value
        for (key, value) in config {
            let rendered_value = match value {
                serde_json::Value::String(s) => {
                    // Try to render as template
                    match self.handlebars.render_template(s, &template_data) {
                        Ok(rendered) => serde_json::Value::String(rendered),
                        Err(_) => value.clone(), // If template rendering fails, use original
                    }
                }
                serde_json::Value::Object(obj) => {
                    // Recursively render nested objects
                    let mut rendered_obj = serde_json::Map::new();
                    for (k, v) in obj {
                        let nested_config = {
                            let mut m = HashMap::new();
                            m.insert(k.clone(), v.clone());
                            self.render_config(&m, event)?
                        };
                        if let Some(rendered_v) = nested_config.get(k) {
                            rendered_obj.insert(k.clone(), rendered_v.clone());
                        }
                    }
                    serde_json::Value::Object(rendered_obj)
                }
                _ => value.clone(),
            };
            rendered.insert(key.clone(), rendered_value);
        }

        Ok(rendered)
    }
}

impl Default for PipelineExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_matches_event() {
        let definition = PipelineDefinition {
            name: "test-pipeline".to_string(),
            description: String::new(),
            triggers: vec![PipelineTrigger {
                event: "schema.changed".to_string(),
                filters: {
                    let mut f = HashMap::new();
                    f.insert(
                        "schema_type".to_string(),
                        serde_json::Value::String("openapi".to_string()),
                    );
                    f
                },
            }],
            steps: vec![],
            enabled: true,
            step_defaults: HashMap::new(),
        };

        let workspace_id = Uuid::new_v4();
        let pipeline = Pipeline::new("test".to_string(), definition, Some(workspace_id), None);

        let mut payload = HashMap::new();
        payload.insert("schema_type".to_string(), serde_json::Value::String("openapi".to_string()));
        let event =
            PipelineEvent::schema_changed(workspace_id, "openapi".to_string(), HashMap::new());

        assert!(pipeline.matches_event(&event));
    }

    #[test]
    fn test_pipeline_new() {
        let workspace_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();

        let definition = PipelineDefinition {
            name: "my-pipeline".to_string(),
            description: "A test pipeline".to_string(),
            triggers: vec![],
            steps: vec![],
            enabled: true,
            step_defaults: HashMap::new(),
        };

        let pipeline =
            Pipeline::new("my-pipeline".to_string(), definition, Some(workspace_id), Some(org_id));

        assert_eq!(pipeline.name, "my-pipeline");
        assert_eq!(pipeline.workspace_id, Some(workspace_id));
        assert_eq!(pipeline.org_id, Some(org_id));
        assert!(pipeline.id != Uuid::nil());
    }

    #[test]
    fn test_pipeline_disabled_does_not_match() {
        let definition = PipelineDefinition {
            name: "disabled-pipeline".to_string(),
            description: String::new(),
            triggers: vec![PipelineTrigger {
                event: "schema.changed".to_string(),
                filters: HashMap::new(),
            }],
            steps: vec![],
            enabled: false,
            step_defaults: HashMap::new(),
        };

        let workspace_id = Uuid::new_v4();
        let pipeline = Pipeline::new("test".to_string(), definition, Some(workspace_id), None);

        let event =
            PipelineEvent::schema_changed(workspace_id, "openapi".to_string(), HashMap::new());

        assert!(!pipeline.matches_event(&event));
    }

    #[test]
    fn test_pipeline_workspace_mismatch() {
        let definition = PipelineDefinition {
            name: "test-pipeline".to_string(),
            description: String::new(),
            triggers: vec![PipelineTrigger {
                event: "schema.changed".to_string(),
                filters: HashMap::new(),
            }],
            steps: vec![],
            enabled: true,
            step_defaults: HashMap::new(),
        };

        let workspace_id = Uuid::new_v4();
        let other_workspace_id = Uuid::new_v4();

        let pipeline = Pipeline::new("test".to_string(), definition, Some(workspace_id), None);

        // Event from different workspace should not match
        let event = PipelineEvent::schema_changed(
            other_workspace_id,
            "openapi".to_string(),
            HashMap::new(),
        );

        assert!(!pipeline.matches_event(&event));
    }

    #[test]
    fn test_pipeline_trigger_serialize_deserialize() {
        let trigger = PipelineTrigger {
            event: "schema.changed".to_string(),
            filters: {
                let mut f = HashMap::new();
                f.insert(
                    "schema_type".to_string(),
                    serde_json::Value::String("openapi".to_string()),
                );
                f
            },
        };

        let json = serde_json::to_string(&trigger).unwrap();
        let deserialized: PipelineTrigger = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.event, trigger.event);
        assert_eq!(deserialized.filters.len(), 1);
    }

    #[test]
    fn test_pipeline_step_serialize_deserialize() {
        let step = PipelineStep {
            name: "test-step".to_string(),
            step_type: "notify".to_string(),
            config: {
                let mut c = HashMap::new();
                c.insert("channels".to_string(), serde_json::json!(["#team-channel"]));
                c
            },
            continue_on_error: true,
            timeout: Some(60),
        };

        let json = serde_json::to_string(&step).unwrap();
        let deserialized: PipelineStep = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, step.name);
        assert_eq!(deserialized.step_type, step.step_type);
        assert!(deserialized.continue_on_error);
        assert_eq!(deserialized.timeout, Some(60));
    }

    #[test]
    fn test_pipeline_step_default_continue_on_error() {
        let json = r#"{
            "name": "test-step",
            "step_type": "notify",
            "config": {}
        }"#;

        let step: PipelineStep = serde_json::from_str(json).unwrap();
        assert!(!step.continue_on_error);
        assert_eq!(step.timeout, None);
    }

    #[test]
    fn test_pipeline_definition_serialize_deserialize() {
        let definition = PipelineDefinition {
            name: "full-pipeline".to_string(),
            description: "A complete pipeline".to_string(),
            triggers: vec![
                PipelineTrigger {
                    event: "schema.changed".to_string(),
                    filters: HashMap::new(),
                },
                PipelineTrigger {
                    event: "scenario.published".to_string(),
                    filters: HashMap::new(),
                },
            ],
            steps: vec![
                PipelineStep {
                    name: "step1".to_string(),
                    step_type: "notify".to_string(),
                    config: HashMap::new(),
                    continue_on_error: false,
                    timeout: None,
                },
                PipelineStep {
                    name: "step2".to_string(),
                    step_type: "regenerate_sdk".to_string(),
                    config: HashMap::new(),
                    continue_on_error: true,
                    timeout: Some(300),
                },
            ],
            enabled: true,
            step_defaults: HashMap::new(),
        };

        let json = serde_json::to_string(&definition).unwrap();
        let deserialized: PipelineDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, definition.name);
        assert_eq!(deserialized.triggers.len(), 2);
        assert_eq!(deserialized.steps.len(), 2);
    }

    #[test]
    fn test_pipeline_definition_defaults() {
        let json = r#"{
            "name": "minimal-pipeline",
            "triggers": [],
            "steps": []
        }"#;

        let definition: PipelineDefinition = serde_json::from_str(json).unwrap();
        assert!(definition.enabled);
        assert!(definition.description.is_empty());
        assert!(definition.step_defaults.is_empty());
    }

    #[test]
    fn test_pipeline_execution_status_serialize() {
        let status = PipelineExecutionStatus::Running;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"running\"");

        let status = PipelineExecutionStatus::Completed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"completed\"");

        let status = PipelineExecutionStatus::Failed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"failed\"");
    }

    #[test]
    fn test_pipeline_execution_status_eq() {
        assert_eq!(PipelineExecutionStatus::Started, PipelineExecutionStatus::Started);
        assert_ne!(PipelineExecutionStatus::Started, PipelineExecutionStatus::Running);
    }

    #[test]
    fn test_step_execution_result_serialize() {
        let result = StepExecutionResult {
            step_name: "test-step".to_string(),
            step_type: "notify".to_string(),
            status: "completed".to_string(),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            output: Some({
                let mut m = HashMap::new();
                m.insert("key".to_string(), serde_json::json!("value"));
                m
            }),
            error_message: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test-step"));
        assert!(json.contains("completed"));
    }

    #[test]
    fn test_pipeline_executor_new() {
        let executor = PipelineExecutor::new();
        assert!(executor.step_executors.is_empty());
    }

    #[test]
    fn test_pipeline_executor_default() {
        let executor = PipelineExecutor::default();
        assert!(executor.step_executors.is_empty());
    }

    #[test]
    fn test_pipeline_matches_no_filters() {
        let definition = PipelineDefinition {
            name: "test-pipeline".to_string(),
            description: String::new(),
            triggers: vec![PipelineTrigger {
                event: "schema.changed".to_string(),
                filters: HashMap::new(), // No filters
            }],
            steps: vec![],
            enabled: true,
            step_defaults: HashMap::new(),
        };

        let workspace_id = Uuid::new_v4();
        let pipeline = Pipeline::new("test".to_string(), definition, Some(workspace_id), None);

        let event =
            PipelineEvent::schema_changed(workspace_id, "openapi".to_string(), HashMap::new());

        assert!(pipeline.matches_event(&event));
    }

    #[test]
    fn test_pipeline_matches_wrong_event_type() {
        let definition = PipelineDefinition {
            name: "test-pipeline".to_string(),
            description: String::new(),
            triggers: vec![PipelineTrigger {
                event: "schema.changed".to_string(),
                filters: HashMap::new(),
            }],
            steps: vec![],
            enabled: true,
            step_defaults: HashMap::new(),
        };

        let workspace_id = Uuid::new_v4();
        let pipeline = Pipeline::new("test".to_string(), definition, Some(workspace_id), None);

        // Different event type
        let event = PipelineEvent::scenario_published(
            workspace_id,
            Uuid::new_v4(),
            "scenario".to_string(),
            None,
        );

        assert!(!pipeline.matches_event(&event));
    }

    #[test]
    fn test_pipeline_matches_filter_mismatch() {
        let definition = PipelineDefinition {
            name: "test-pipeline".to_string(),
            description: String::new(),
            triggers: vec![PipelineTrigger {
                event: "schema.changed".to_string(),
                filters: {
                    let mut f = HashMap::new();
                    f.insert(
                        "schema_type".to_string(),
                        serde_json::Value::String("protobuf".to_string()),
                    );
                    f
                },
            }],
            steps: vec![],
            enabled: true,
            step_defaults: HashMap::new(),
        };

        let workspace_id = Uuid::new_v4();
        let pipeline = Pipeline::new("test".to_string(), definition, Some(workspace_id), None);

        // Event with different schema_type
        let event =
            PipelineEvent::schema_changed(workspace_id, "openapi".to_string(), HashMap::new());

        assert!(!pipeline.matches_event(&event));
    }

    #[test]
    fn test_pipeline_global_scope() {
        // Pipeline without workspace_id should match any workspace
        let definition = PipelineDefinition {
            name: "global-pipeline".to_string(),
            description: String::new(),
            triggers: vec![PipelineTrigger {
                event: "schema.changed".to_string(),
                filters: HashMap::new(),
            }],
            steps: vec![],
            enabled: true,
            step_defaults: HashMap::new(),
        };

        let pipeline = Pipeline::new("test".to_string(), definition, None, None);

        let workspace_id = Uuid::new_v4();
        let event =
            PipelineEvent::schema_changed(workspace_id, "openapi".to_string(), HashMap::new());

        assert!(pipeline.matches_event(&event));
    }

    #[test]
    fn test_pipeline_with_step_defaults() {
        let mut step_defaults = HashMap::new();
        let mut notify_defaults = HashMap::new();
        notify_defaults
            .insert("webhook_url".to_string(), serde_json::json!("https://default.webhook"));
        step_defaults.insert("notify".to_string(), notify_defaults);

        let definition = PipelineDefinition {
            name: "pipeline-with-defaults".to_string(),
            description: String::new(),
            triggers: vec![],
            steps: vec![],
            enabled: true,
            step_defaults,
        };

        let json = serde_json::to_string(&definition).unwrap();
        let deserialized: PipelineDefinition = serde_json::from_str(&json).unwrap();

        assert!(deserialized.step_defaults.contains_key("notify"));
    }

    #[test]
    fn test_pipeline_clone() {
        let definition = PipelineDefinition {
            name: "test".to_string(),
            description: "desc".to_string(),
            triggers: vec![],
            steps: vec![],
            enabled: true,
            step_defaults: HashMap::new(),
        };

        let pipeline = Pipeline::new("test".to_string(), definition, None, None);
        let cloned = pipeline.clone();

        assert_eq!(pipeline.id, cloned.id);
        assert_eq!(pipeline.name, cloned.name);
    }

    #[test]
    fn test_pipeline_execution_clone() {
        let workspace_id = Uuid::new_v4();
        let execution = PipelineExecution {
            id: Uuid::new_v4(),
            pipeline_id: Uuid::new_v4(),
            trigger_event: PipelineEvent::schema_changed(
                workspace_id,
                "openapi".to_string(),
                HashMap::new(),
            ),
            status: PipelineExecutionStatus::Completed,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            error_message: None,
            execution_log: vec![],
        };

        let cloned = execution.clone();
        assert_eq!(execution.id, cloned.id);
        assert_eq!(execution.status, cloned.status);
    }
}
