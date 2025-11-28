//! Auto-promote step
//!
//! Automatically promotes scenarios, personas, or configs to the next environment.

use super::{PipelineStepExecutor, StepContext, StepResult};
use anyhow::{Context, Result};
use mockforge_core::workspace::scenario_promotion::PromotionEntityType;
use mockforge_core::PromotionService;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Auto-promote step executor
pub struct AutoPromoteStep {
    /// Promotion service (optional, injected by caller)
    promotion_service: Option<Arc<dyn PromotionService>>,
}

impl AutoPromoteStep {
    /// Create a new auto-promote step without promotion service
    #[must_use]
    pub fn new() -> Self {
        Self {
            promotion_service: None,
        }
    }

    /// Create a new auto-promote step with promotion service
    pub fn with_promotion_service(service: Arc<dyn PromotionService>) -> Self {
        Self {
            promotion_service: Some(service),
        }
    }
}

impl Default for AutoPromoteStep {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PipelineStepExecutor for AutoPromoteStep {
    fn step_type(&self) -> &'static str {
        "auto_promote"
    }

    async fn execute(&self, context: StepContext) -> Result<StepResult> {
        info!(
            execution_id = %context.execution_id,
            step_name = %context.step_name,
            "Executing auto_promote step"
        );

        // Extract configuration
        let entity_type = context
            .config
            .get("entity_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'entity_type' in step config"))?;

        let entity_id = context
            .config
            .get("entity_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'entity_id' in step config"))?;

        let from_env = context
            .config
            .get("from_environment")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'from_environment' in step config"))?;

        let to_env = context
            .config
            .get("to_environment")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'to_environment' in step config"))?;

        // Parse entity type
        let promotion_entity_type = match entity_type {
            "scenario" => PromotionEntityType::Scenario,
            "persona" => PromotionEntityType::Persona,
            "config" => PromotionEntityType::Config,
            _ => return Err(anyhow::anyhow!("Invalid entity_type: {entity_type}")),
        };

        // Parse environments
        let from_environment =
            mockforge_core::workspace::mock_environment::MockEnvironmentName::from_str(from_env)
                .ok_or_else(|| anyhow::anyhow!("Invalid from_environment: {from_env}"))?;
        let to_environment =
            mockforge_core::workspace::mock_environment::MockEnvironmentName::from_str(to_env)
                .ok_or_else(|| anyhow::anyhow!("Invalid to_environment: {to_env}"))?;

        // Get workspace ID from event
        let workspace_id = context
            .event
            .workspace_id
            .ok_or_else(|| anyhow::anyhow!("Event missing workspace_id"))?;

        // Parse entity ID
        let entity_uuid = Uuid::parse_str(entity_id).context("Invalid entity_id format")?;

        debug!(
            execution_id = %context.execution_id,
            entity_type = ?promotion_entity_type,
            entity_id = %entity_uuid,
            from_env = ?from_environment,
            to_env = ?to_environment,
            "Auto-promoting entity"
        );

        // Perform promotion if service is available
        if let Some(ref service) = self.promotion_service {
            // Get promoted_by from event payload or use system user
            let promoted_by = context
                .event
                .payload
                .get("promoted_by")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(|| {
                    // Use a system user ID - in production this should come from event context
                    Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap()
                });

            // Get entity version from config or event
            let entity_version = context
                .config
                .get("entity_version")
                .or_else(|| context.event.payload.get("version"))
                .and_then(|v| v.as_str())
                .map(ToString::to_string);

            // Get comments from config
            let comments =
                context.config.get("comments").and_then(|v| v.as_str()).map(ToString::to_string);

            match service
                .promote_entity(
                    workspace_id,
                    promotion_entity_type,
                    entity_id.to_string(),
                    entity_version,
                    from_environment,
                    to_environment,
                    promoted_by,
                    comments,
                )
                .await
            {
                Ok(promotion_id) => {
                    info!(
                        execution_id = %context.execution_id,
                        promotion_id = %promotion_id,
                        "Successfully auto-promoted entity"
                    );

                    let mut output = HashMap::new();
                    output.insert(
                        "promotion_id".to_string(),
                        Value::String(promotion_id.to_string()),
                    );
                    output
                        .insert("entity_type".to_string(), Value::String(entity_type.to_string()));
                    output.insert("entity_id".to_string(), Value::String(entity_id.to_string()));
                    output.insert(
                        "from_environment".to_string(),
                        Value::String(from_env.to_string()),
                    );
                    output.insert("to_environment".to_string(), Value::String(to_env.to_string()));
                    output.insert("status".to_string(), Value::String("promoted".to_string()));

                    Ok(StepResult::success_with_output(output))
                }
                Err(e) => {
                    error!(
                        execution_id = %context.execution_id,
                        error = %e,
                        "Failed to auto-promote entity"
                    );
                    Err(anyhow::anyhow!("Promotion failed: {e}"))
                }
            }
        } else {
            // No promotion service available - log warning but don't fail
            warn!(
                execution_id = %context.execution_id,
                "Auto-promote step executed but no promotion service configured"
            );

            let mut output = HashMap::new();
            output.insert("entity_type".to_string(), Value::String(entity_type.to_string()));
            output.insert("entity_id".to_string(), Value::String(entity_id.to_string()));
            output.insert("from_environment".to_string(), Value::String(from_env.to_string()));
            output.insert("to_environment".to_string(), Value::String(to_env.to_string()));
            output.insert("status".to_string(), Value::String("skipped_no_service".to_string()));

            Ok(StepResult::success_with_output(output))
        }
    }
}
