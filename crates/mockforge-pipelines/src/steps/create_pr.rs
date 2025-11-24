//! Create PR step
//!
//! Creates a Git pull request with changes.

use super::{PipelineStepExecutor, StepContext, StepResult};
use anyhow::Result;
use mockforge_core::pr_generation::{PRFileChange, PRFileChangeType, PRGenerator, PRRequest};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, error, info};

/// Create PR step executor
///
/// This step creates a Git pull request with drift violations or schema changes.
pub struct CreatePRStep {
    // PR generator (if configured)
    // TODO: Make this configurable per pipeline/workspace
    // pr_generator: Option<Arc<PRGenerator>>,
}

impl CreatePRStep {
    /// Create a new create PR step
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for CreatePRStep {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PipelineStepExecutor for CreatePRStep {
    fn step_type(&self) -> &'static str {
        "create_pr"
    }

    async fn execute(&self, context: StepContext) -> Result<StepResult> {
        info!(
            execution_id = %context.execution_id,
            step_name = %context.step_name,
            "Executing create_pr step"
        );

        // Extract configuration
        let title = context
            .config
            .get("title")
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
            .ok_or_else(|| anyhow::anyhow!("Missing 'title' in step config"))?;

        let body = context
            .config
            .get("body")
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
            .unwrap_or_default();

        let branch = context
            .config
            .get("branch")
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
            .ok_or_else(|| anyhow::anyhow!("Missing 'branch' in step config"))?;

        // Get PR provider and credentials from config
        let provider = context.config.get("provider").and_then(|v| v.as_str()).unwrap_or("github");

        let owner = context
            .config
            .get("owner")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'owner' in step config"))?;

        let repo = context
            .config
            .get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'repo' in step config"))?;

        let token = context
            .config
            .get("token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'token' in step config"))?;

        let base_branch =
            context.config.get("base_branch").and_then(|v| v.as_str()).unwrap_or("main");

        debug!(
            execution_id = %context.execution_id,
            provider = %provider,
            owner = %owner,
            repo = %repo,
            branch = %branch,
            "Creating pull request"
        );

        // Create PR generator
        let pr_generator = match provider {
            "github" => PRGenerator::new_github(
                owner.to_string(),
                repo.to_string(),
                token.to_string(),
                base_branch.to_string(),
            ),
            "gitlab" => PRGenerator::new_gitlab(
                owner.to_string(),
                repo.to_string(),
                token.to_string(),
                base_branch.to_string(),
            ),
            _ => return Err(anyhow::anyhow!("Unsupported PR provider: {provider}")),
        };

        // Extract file changes from config (if any)
        let file_changes = context
            .config
            .get("files")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        if let Some(obj) = v.as_object() {
                            let path = obj.get("path")?.as_str()?;
                            let content = obj.get("content")?.as_str()?;
                            let change_type = obj
                                .get("type")
                                .and_then(|t| t.as_str())
                                .and_then(|t| match t {
                                    "create" => Some(PRFileChangeType::Create),
                                    "update" => Some(PRFileChangeType::Update),
                                    "delete" => Some(PRFileChangeType::Delete),
                                    _ => None,
                                })
                                .unwrap_or(PRFileChangeType::Update);

                            Some(PRFileChange {
                                path: path.to_string(),
                                content: content.to_string(),
                                change_type,
                            })
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Create PR request
        let pr_request = PRRequest {
            title,
            body,
            branch,
            files: file_changes,
            labels: vec!["automated".to_string(), "pipeline".to_string()],
            reviewers: vec![],
        };

        // Create PR
        match pr_generator.create_pr(pr_request).await {
            Ok(pr_result) => {
                info!(
                    execution_id = %context.execution_id,
                    pr_url = %pr_result.url,
                    "Pull request created successfully"
                );

                let mut output = HashMap::new();
                output.insert("pr_url".to_string(), Value::String(pr_result.url));
                output.insert("pr_number".to_string(), Value::Number(pr_result.number.into()));
                output.insert("status".to_string(), Value::String("created".to_string()));

                Ok(StepResult::success_with_output(output))
            }
            Err(e) => {
                error!(
                    execution_id = %context.execution_id,
                    error = %e,
                    "Failed to create pull request"
                );
                Err(anyhow::anyhow!("Failed to create PR: {e}"))
            }
        }
    }
}
