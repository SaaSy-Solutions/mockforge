//! Promotion workflow management
//!
//! Handles promotion of scenarios, personas, and configs between environments
//! with history tracking and `GitOps` integration.

use crate::error::{CollabError, Result};
use chrono::{DateTime, Utc};
use mockforge_core::pr_generation::{
    PRFileChange, PRFileChangeType, PRGenerator, PRProvider, PRRequest,
};
use mockforge_core::workspace::mock_environment::MockEnvironmentName;
use mockforge_core::workspace::scenario_promotion::{
    PromotionEntityType, PromotionHistory, PromotionHistoryEntry, PromotionRequest, PromotionStatus,
};
use mockforge_core::PromotionService as PromotionServiceTrait;
use serde_json;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use uuid::Uuid;

/// `GitOps` configuration for promotions
#[derive(Debug, Clone)]
pub struct PromotionGitOpsConfig {
    /// Whether `GitOps` is enabled
    pub enabled: bool,
    /// PR generator (if enabled)
    pub pr_generator: Option<PRGenerator>,
    /// Repository path for workspace config (relative to repo root)
    pub config_path: Option<String>,
}

impl PromotionGitOpsConfig {
    /// Create a new `GitOps` config
    #[must_use]
    pub fn new(
        enabled: bool,
        provider: PRProvider,
        owner: String,
        repo: String,
        token: Option<String>,
        base_branch: String,
        config_path: Option<String>,
    ) -> Self {
        let pr_generator = if let (true, Some(token)) = (enabled, token) {
            Some(match provider {
                PRProvider::GitHub => PRGenerator::new_github(owner, repo, token, base_branch),
                PRProvider::GitLab => PRGenerator::new_gitlab(owner, repo, token, base_branch),
            })
        } else {
            None
        };

        Self {
            enabled,
            pr_generator,
            config_path,
        }
    }

    /// Create disabled `GitOps` config
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            pr_generator: None,
            config_path: None,
        }
    }
}

/// Promotion service for managing promotions between environments
pub struct PromotionService {
    db: Pool<Sqlite>,
    gitops: Arc<PromotionGitOpsConfig>,
}

impl PromotionService {
    /// Create a new promotion service
    #[must_use]
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self {
            db,
            gitops: Arc::new(PromotionGitOpsConfig::disabled()),
        }
    }

    /// Create a new promotion service with `GitOps` support
    #[must_use]
    pub fn with_gitops(db: Pool<Sqlite>, gitops: PromotionGitOpsConfig) -> Self {
        Self {
            db,
            gitops: Arc::new(gitops),
        }
    }

    /// Run database migrations for promotion tables
    ///
    /// This ensures the `promotion_history` and `environment_permission_policies` tables exist.
    /// Should be called during service initialization.
    pub async fn run_migrations(&self) -> Result<()> {
        // Promotion-specific tables (promotion_history, environment_permission_policies)
        // are created by the collab server's main SQLx migration system.
        // No additional promotion-specific migrations needed at this time.
        Ok(())
    }

    /// Record a promotion in the history and optionally create a `GitOps` PR
    pub async fn record_promotion(
        &self,
        request: &PromotionRequest,
        promoted_by: Uuid,
        status: PromotionStatus,
        workspace_config: Option<serde_json::Value>,
    ) -> Result<Uuid> {
        let promotion_id = Uuid::new_v4();
        let now = Utc::now();

        let metadata_json = if request.metadata.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&request.metadata)?)
        };

        // Record promotion in database
        // Note: promotion_history table uses TEXT columns, so convert UUIDs to strings
        let promotion_id_str = promotion_id.to_string();
        let promoted_by_str = promoted_by.to_string();
        let entity_type_str = request.entity_type.to_string();
        let from_env_str = request.from_environment.as_str().to_string();
        let to_env_str = request.to_environment.as_str().to_string();
        let status_str = status.to_string();
        let now_str = now.to_rfc3339();

        sqlx::query!(
            r#"
            INSERT INTO promotion_history (
                id, workspace_id, entity_type, entity_id, entity_version,
                from_environment, to_environment, promoted_by, status,
                comments, metadata, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            promotion_id_str,
            request.workspace_id,
            entity_type_str,
            request.entity_id,
            request.entity_version,
            from_env_str,
            to_env_str,
            promoted_by_str,
            status_str,
            request.comments,
            metadata_json,
            now_str,
            now_str,
        )
        .execute(&self.db)
        .await
        .map_err(|e| CollabError::DatabaseError(format!("Failed to record promotion: {e}")))?;

        // Create GitOps PR if enabled and workspace config is provided
        if self.gitops.enabled && workspace_config.is_some() {
            if let Err(e) = self
                .create_promotion_pr(&promotion_id, request, workspace_config.unwrap())
                .await
            {
                tracing::warn!("Failed to create GitOps PR for promotion {}: {}", promotion_id, e);
                // Don't fail the promotion if PR creation fails
            }
        }

        Ok(promotion_id)
    }

    /// Create a `GitOps` PR for a promotion
    async fn create_promotion_pr(
        &self,
        promotion_id: &Uuid,
        request: &PromotionRequest,
        workspace_config: serde_json::Value,
    ) -> Result<()> {
        let pr_generator = self
            .gitops
            .pr_generator
            .as_ref()
            .ok_or_else(|| CollabError::Internal("PR generator not configured".to_string()))?;

        // Generate PR title and body
        let title = format!(
            "Promote {} '{}' from {} to {}",
            request.entity_type,
            request.entity_id,
            request.from_environment.as_str(),
            request.to_environment.as_str(),
        );

        let mut body = format!(
            "## Promotion: {} → {}\n\n",
            request.from_environment.as_str(),
            request.to_environment.as_str(),
        );
        body.push_str(&format!("**Entity Type:** {}\n", request.entity_type));
        body.push_str(&format!("**Entity ID:** {}\n", request.entity_id));
        if let Some(version) = &request.entity_version {
            body.push_str(&format!("**Version:** {version}\n"));
        }
        if let Some(comments) = &request.comments {
            body.push_str(&format!("\n**Comments:**\n{comments}\n"));
        }
        body.push_str("\n---\n\n");
        body.push_str("*This PR was automatically generated by MockForge promotion workflow.*");

        // Determine config file path
        let default_path = format!("workspaces/{}/config.yaml", request.workspace_id);
        let config_path = self.gitops.config_path.as_deref().unwrap_or(&default_path);

        // Serialize workspace config to JSON (YAML can be converted later if needed)
        let config_json = serde_json::to_string_pretty(&workspace_config)
            .map_err(|e| CollabError::Internal(format!("Failed to serialize config: {e}")))?;

        // Create file change (use .json extension or keep .yaml if path specifies it)
        let file_path = if config_path.ends_with(".yaml") || config_path.ends_with(".yml") {
            config_path.to_string()
        } else {
            format!("{config_path}.json")
        };

        let file_change = PRFileChange {
            path: file_path,
            content: config_json,
            change_type: PRFileChangeType::Update,
        };

        // Create PR request
        let pr_request = PRRequest {
            title,
            body,
            branch: format!(
                "mockforge/promotion-{}-{}-{}",
                request.entity_type,
                request.entity_id,
                &promotion_id.to_string()[..8]
            ),
            files: vec![file_change],
            labels: vec![
                "automated".to_string(),
                "promotion".to_string(),
                format!("env-{}", request.to_environment.as_str()),
            ],
            reviewers: vec![],
        };

        // Create PR
        match pr_generator.create_pr(pr_request).await {
            Ok(pr_result) => {
                // Update promotion with PR URL
                self.update_promotion_pr_url(*promotion_id, pr_result.url.clone()).await?;
                tracing::info!(
                    "Created GitOps PR {} for promotion {}",
                    pr_result.url,
                    promotion_id
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to create PR for promotion {}: {}", promotion_id, e);
                Err(CollabError::Internal(format!("Failed to create PR: {e}")))
            }
        }
    }

    /// Update promotion status (e.g., when approved or rejected)
    pub async fn update_promotion_status(
        &self,
        promotion_id: Uuid,
        status: PromotionStatus,
        approved_by: Option<Uuid>,
    ) -> Result<()> {
        let now = Utc::now();
        let status_str = status.to_string();
        let approved_by_str = approved_by.map(|u| u.to_string());
        let now_str = now.to_rfc3339();
        let promotion_id_str = promotion_id.to_string();

        sqlx::query!(
            r#"
            UPDATE promotion_history
            SET status = ?, approved_by = ?, updated_at = ?
            WHERE id = ?
            "#,
            status_str,
            approved_by_str,
            now_str,
            promotion_id_str,
        )
        .execute(&self.db)
        .await
        .map_err(|e| {
            CollabError::DatabaseError(format!("Failed to update promotion status: {e}"))
        })?;

        // Emit pipeline event when promotion is completed
        if status == PromotionStatus::Completed {
            #[cfg(feature = "pipelines")]
            {
                use mockforge_pipelines::events::{publish_event, PipelineEvent};
                use sqlx::Row;

                // Get workspace_id from database (stored as TEXT)
                let workspace_id_row =
                    sqlx::query("SELECT workspace_id FROM promotion_history WHERE id = ?")
                        .bind(&promotion_id_str)
                        .fetch_optional(&self.db)
                        .await
                        .ok()
                        .flatten();

                if let Some(row) = workspace_id_row {
                    if let Ok(workspace_id_str) = row.try_get::<String, _>("workspace_id") {
                        if let Ok(ws_id) = Uuid::parse_str(&workspace_id_str) {
                            // Get promotion details for event
                            if let Some(promotion) = self.get_promotion_by_id(promotion_id).await? {
                                let event = PipelineEvent::promotion_completed(
                                    ws_id,
                                    promotion_id,
                                    promotion.entity_type.to_string(),
                                    promotion.from_environment.as_str().to_string(),
                                    promotion.to_environment.as_str().to_string(),
                                );

                                if let Err(e) = publish_event(event) {
                                    tracing::warn!(
                                        "Failed to publish promotion completed event: {}",
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Update promotion with `GitOps` PR URL
    pub async fn update_promotion_pr_url(&self, promotion_id: Uuid, pr_url: String) -> Result<()> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let promotion_id_str = promotion_id.to_string();

        sqlx::query!(
            r#"
            UPDATE promotion_history
            SET pr_url = ?, updated_at = ?
            WHERE id = ?
            "#,
            pr_url,
            now_str,
            promotion_id_str,
        )
        .execute(&self.db)
        .await
        .map_err(|e| {
            CollabError::DatabaseError(format!("Failed to update promotion PR URL: {e}"))
        })?;

        Ok(())
    }

    /// Get a promotion by ID
    pub async fn get_promotion_by_id(
        &self,
        promotion_id: Uuid,
    ) -> Result<Option<PromotionHistoryEntry>> {
        let promotion_id_str = promotion_id.to_string();

        use sqlx::Row;
        let row = sqlx::query(
            r"
            SELECT
                id, entity_type, entity_id, entity_version, workspace_id,
                from_environment, to_environment, promoted_by, approved_by,
                status, comments, pr_url, metadata, created_at, updated_at
            FROM promotion_history
            WHERE id = ?
            ",
        )
        .bind(&promotion_id_str)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| CollabError::DatabaseError(format!("Failed to get promotion: {e}")))?;

        if let Some(row) = row {
            let id: String = row.get("id");
            let entity_type_str: String = row.get("entity_type");
            let entity_id: String = row.get("entity_id");
            let entity_version: Option<String> = row.get("entity_version");
            let _workspace_id: String = row.get("workspace_id");
            let from_environment: String = row.get("from_environment");
            let to_environment: String = row.get("to_environment");
            let promoted_by: String = row.get("promoted_by");
            let approved_by: Option<String> = row.get("approved_by");
            let status_str: String = row.get("status");
            let comments: Option<String> = row.get("comments");
            let pr_url: Option<String> = row.get("pr_url");
            let metadata: Option<String> = row.get("metadata");
            let created_at: String = row.get("created_at");

            let from_env = MockEnvironmentName::from_str(&from_environment).ok_or_else(|| {
                CollabError::Internal(format!("Invalid from_environment: {from_environment}"))
            })?;
            let to_env = MockEnvironmentName::from_str(&to_environment).ok_or_else(|| {
                CollabError::Internal(format!("Invalid to_environment: {to_environment}"))
            })?;
            let status = match status_str.as_str() {
                "pending" => PromotionStatus::Pending,
                "approved" => PromotionStatus::Approved,
                "rejected" => PromotionStatus::Rejected,
                "completed" => PromotionStatus::Completed,
                "failed" => PromotionStatus::Failed,
                _ => return Err(CollabError::Internal(format!("Invalid status: {status_str}"))),
            };
            let entity_type = match entity_type_str.as_str() {
                "scenario" => PromotionEntityType::Scenario,
                "persona" => PromotionEntityType::Persona,
                "config" => PromotionEntityType::Config,
                _ => {
                    return Err(CollabError::Internal(format!(
                        "Invalid entity_type: {entity_type_str}"
                    )))
                }
            };

            let metadata_map = if let Some(meta_str) = metadata {
                serde_json::from_str(&meta_str).unwrap_or_default()
            } else {
                std::collections::HashMap::new()
            };

            let timestamp = DateTime::parse_from_rfc3339(&created_at)
                .map_err(|e| CollabError::Internal(format!("Invalid timestamp: {e}")))?
                .with_timezone(&Utc);

            Ok(Some(PromotionHistoryEntry {
                promotion_id: id,
                entity_type,
                entity_id,
                entity_version,
                from_environment: from_env,
                to_environment: to_env,
                promoted_by,
                approved_by,
                status,
                timestamp,
                comments,
                pr_url,
                metadata: metadata_map,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get promotion history for an entity
    pub async fn get_promotion_history(
        &self,
        workspace_id: &str,
        entity_type: PromotionEntityType,
        entity_id: &str,
    ) -> Result<PromotionHistory> {
        let entity_type_str = entity_type.to_string();
        let rows = sqlx::query!(
            r#"
            SELECT
                id, entity_type, entity_id, entity_version,
                from_environment, to_environment, promoted_by, approved_by,
                status, comments, pr_url, metadata, created_at, updated_at
            FROM promotion_history
            WHERE workspace_id = ? AND entity_type = ? AND entity_id = ?
            ORDER BY created_at ASC
            "#,
            workspace_id,
            entity_type_str,
            entity_id,
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| CollabError::DatabaseError(format!("Failed to get promotion history: {e}")))?;

        let promotions: Result<Vec<PromotionHistoryEntry>> = rows
            .into_iter()
            .map(|row| {
                let from_env =
                    MockEnvironmentName::from_str(&row.from_environment).ok_or_else(|| {
                        CollabError::Internal(format!(
                            "Invalid from_environment: {}",
                            row.from_environment
                        ))
                    })?;
                let to_env =
                    MockEnvironmentName::from_str(&row.to_environment).ok_or_else(|| {
                        CollabError::Internal(format!(
                            "Invalid to_environment: {}",
                            row.to_environment
                        ))
                    })?;
                let status = match row.status.as_str() {
                    "pending" => PromotionStatus::Pending,
                    "approved" => PromotionStatus::Approved,
                    "rejected" => PromotionStatus::Rejected,
                    "completed" => PromotionStatus::Completed,
                    "failed" => PromotionStatus::Failed,
                    _ => {
                        return Err(CollabError::Internal(format!(
                            "Invalid status: {}",
                            row.status
                        )))
                    }
                };
                let entity_type = match row.entity_type.as_str() {
                    "scenario" => PromotionEntityType::Scenario,
                    "persona" => PromotionEntityType::Persona,
                    "config" => PromotionEntityType::Config,
                    _ => {
                        return Err(CollabError::Internal(format!(
                            "Invalid entity_type: {}",
                            row.entity_type
                        )))
                    }
                };

                let metadata = if let Some(meta_str) = row.metadata {
                    serde_json::from_str(&meta_str).unwrap_or_default()
                } else {
                    std::collections::HashMap::new()
                };

                let timestamp = DateTime::parse_from_rfc3339(&row.created_at)
                    .map_err(|e| CollabError::Internal(format!("Invalid timestamp: {e}")))?
                    .with_timezone(&Utc);

                Ok(PromotionHistoryEntry {
                    promotion_id: row.id,
                    entity_type,
                    entity_id: row.entity_id,
                    entity_version: row.entity_version,
                    from_environment: from_env,
                    to_environment: to_env,
                    promoted_by: row.promoted_by,
                    approved_by: row.approved_by,
                    status,
                    timestamp,
                    comments: row.comments,
                    pr_url: row.pr_url,
                    metadata,
                })
            })
            .collect();

        Ok(PromotionHistory {
            entity_type,
            entity_id: entity_id.to_string(),
            workspace_id: workspace_id.to_string(),
            promotions: promotions?,
        })
    }

    /// Get all promotions for a workspace
    pub async fn get_workspace_promotions(
        &self,
        workspace_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<PromotionHistoryEntry>> {
        let limit = limit.unwrap_or(100);

        let rows = sqlx::query!(
            r#"
            SELECT
                id, entity_type, entity_id, entity_version,
                from_environment, to_environment, promoted_by, approved_by,
                status, comments, pr_url, metadata, created_at, updated_at
            FROM promotion_history
            WHERE workspace_id = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
            workspace_id,
            limit,
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| {
            CollabError::DatabaseError(format!("Failed to get workspace promotions: {e}"))
        })?;

        let promotions: Result<Vec<PromotionHistoryEntry>> = rows
            .into_iter()
            .map(|row| {
                let from_env =
                    MockEnvironmentName::from_str(&row.from_environment).ok_or_else(|| {
                        CollabError::Internal(format!(
                            "Invalid from_environment: {}",
                            row.from_environment
                        ))
                    })?;
                let to_env =
                    MockEnvironmentName::from_str(&row.to_environment).ok_or_else(|| {
                        CollabError::Internal(format!(
                            "Invalid to_environment: {}",
                            row.to_environment
                        ))
                    })?;
                let status = match row.status.as_str() {
                    "pending" => PromotionStatus::Pending,
                    "approved" => PromotionStatus::Approved,
                    "rejected" => PromotionStatus::Rejected,
                    "completed" => PromotionStatus::Completed,
                    "failed" => PromotionStatus::Failed,
                    _ => {
                        return Err(CollabError::Internal(format!(
                            "Invalid status: {}",
                            row.status
                        )))
                    }
                };
                let entity_type = match row.entity_type.as_str() {
                    "scenario" => PromotionEntityType::Scenario,
                    "persona" => PromotionEntityType::Persona,
                    "config" => PromotionEntityType::Config,
                    _ => {
                        return Err(CollabError::Internal(format!(
                            "Invalid entity_type: {}",
                            row.entity_type
                        )))
                    }
                };

                let metadata = if let Some(meta_str) = row.metadata {
                    serde_json::from_str(&meta_str).unwrap_or_default()
                } else {
                    std::collections::HashMap::new()
                };

                let timestamp = DateTime::parse_from_rfc3339(&row.created_at)
                    .map_err(|e| CollabError::Internal(format!("Invalid timestamp: {e}")))?
                    .with_timezone(&Utc);

                Ok(PromotionHistoryEntry {
                    promotion_id: row.id,
                    entity_type,
                    entity_id: row.entity_id,
                    entity_version: row.entity_version,
                    from_environment: from_env,
                    to_environment: to_env,
                    promoted_by: row.promoted_by,
                    approved_by: row.approved_by,
                    status,
                    timestamp,
                    comments: row.comments,
                    pr_url: row.pr_url,
                    metadata,
                })
            })
            .collect();

        promotions
    }

    /// Get pending promotions requiring approval
    pub async fn get_pending_promotions(
        &self,
        workspace_id: Option<&str>,
    ) -> Result<Vec<PromotionHistoryEntry>> {
        // Use runtime query to handle conditional workspace_id
        let rows = if let Some(ws_id) = workspace_id {
            sqlx::query(
                r"
                SELECT
                    id, entity_type, entity_id, entity_version,
                    from_environment, to_environment, promoted_by, approved_by,
                    status, comments, pr_url, metadata, created_at, updated_at
                FROM promotion_history
                WHERE workspace_id = ? AND status = 'pending'
                ORDER BY created_at ASC
                ",
            )
            .bind(ws_id)
            .fetch_all(&self.db)
            .await
            .map_err(|e| {
                CollabError::DatabaseError(format!("Failed to get pending promotions: {e}"))
            })?
        } else {
            sqlx::query(
                r"
                SELECT
                    id, entity_type, entity_id, entity_version,
                    from_environment, to_environment, promoted_by, approved_by,
                    status, comments, pr_url, metadata, created_at, updated_at
                FROM promotion_history
                WHERE status = 'pending'
                ORDER BY created_at ASC
                ",
            )
            .fetch_all(&self.db)
            .await
            .map_err(|e| {
                CollabError::DatabaseError(format!("Failed to get pending promotions: {e}"))
            })?
        };

        use sqlx::Row;
        let promotions: Result<Vec<PromotionHistoryEntry>> = rows
            .into_iter()
            .map(|row: sqlx::sqlite::SqliteRow| {
                let id: String = row.get("id");
                let entity_type_str: String = row.get("entity_type");
                let entity_id: String = row.get("entity_id");
                let entity_version: Option<String> = row.get("entity_version");
                let from_environment: String = row.get("from_environment");
                let to_environment: String = row.get("to_environment");
                let promoted_by: String = row.get("promoted_by");
                let approved_by: Option<String> = row.get("approved_by");
                let comments: Option<String> = row.get("comments");
                let pr_url: Option<String> = row.get("pr_url");
                let metadata: Option<String> = row.get("metadata");
                let created_at: String = row.get("created_at");

                let from_env =
                    MockEnvironmentName::from_str(&from_environment).ok_or_else(|| {
                        CollabError::Internal(format!(
                            "Invalid from_environment: {from_environment}"
                        ))
                    })?;
                let to_env = MockEnvironmentName::from_str(&to_environment).ok_or_else(|| {
                    CollabError::Internal(format!("Invalid to_environment: {to_environment}"))
                })?;
                let status = PromotionStatus::Pending;
                let entity_type = match entity_type_str.as_str() {
                    "scenario" => PromotionEntityType::Scenario,
                    "persona" => PromotionEntityType::Persona,
                    "config" => PromotionEntityType::Config,
                    _ => {
                        return Err(CollabError::Internal(format!(
                            "Invalid entity_type: {entity_type_str}"
                        )))
                    }
                };

                let metadata_map = if let Some(meta_str) = metadata {
                    serde_json::from_str(&meta_str).unwrap_or_default()
                } else {
                    std::collections::HashMap::new()
                };

                let timestamp = DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| CollabError::Internal(format!("Invalid timestamp: {e}")))?
                    .with_timezone(&Utc);

                Ok(PromotionHistoryEntry {
                    promotion_id: id,
                    entity_type,
                    entity_id,
                    entity_version,
                    from_environment: from_env,
                    to_environment: to_env,
                    promoted_by,
                    approved_by,
                    status,
                    timestamp,
                    comments,
                    pr_url,
                    metadata: metadata_map,
                })
            })
            .collect();

        promotions
    }
}

// Implement PromotionService trait for PromotionService
#[async_trait::async_trait]
impl PromotionServiceTrait for PromotionService {
    async fn promote_entity(
        &self,
        workspace_id: Uuid,
        entity_type: PromotionEntityType,
        entity_id: String,
        entity_version: Option<String>,
        from_environment: MockEnvironmentName,
        to_environment: MockEnvironmentName,
        promoted_by: Uuid,
        comments: Option<String>,
    ) -> mockforge_core::Result<Uuid> {
        let request = PromotionRequest {
            entity_type,
            entity_id: entity_id.clone(),
            entity_version,
            workspace_id: workspace_id.to_string(),
            from_environment,
            to_environment,
            requires_approval: false, // Auto-promotions don't require approval
            approval_required_reason: None,
            comments,
            metadata: std::collections::HashMap::new(),
        };

        // Auto-complete the promotion (no approval needed for auto-promotions)
        self.record_promotion(&request, promoted_by, PromotionStatus::Completed, None)
            .await
            .map_err(|e| mockforge_core::Error::generic(format!("Promotion failed: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_core::pr_generation::PRProvider;
    use mockforge_core::workspace::mock_environment::MockEnvironmentName;
    use mockforge_core::workspace::scenario_promotion::{
        PromotionEntityType, PromotionRequest, PromotionStatus,
    };
    use sqlx::SqlitePool;

    async fn setup_test_db() -> Pool<Sqlite> {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS promotion_history (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                entity_version TEXT,
                from_environment TEXT NOT NULL,
                to_environment TEXT NOT NULL,
                promoted_by TEXT NOT NULL,
                approved_by TEXT,
                status TEXT NOT NULL,
                comments TEXT,
                pr_url TEXT,
                metadata TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[test]
    fn test_promotion_gitops_config_new() {
        let config = PromotionGitOpsConfig::new(
            true,
            PRProvider::GitHub,
            "owner".to_string(),
            "repo".to_string(),
            Some("token".to_string()),
            "main".to_string(),
            Some("config.yaml".to_string()),
        );

        assert!(config.enabled);
        assert!(config.pr_generator.is_some());
        assert_eq!(config.config_path, Some("config.yaml".to_string()));
    }

    #[test]
    fn test_promotion_gitops_config_new_without_token() {
        let config = PromotionGitOpsConfig::new(
            true,
            PRProvider::GitHub,
            "owner".to_string(),
            "repo".to_string(),
            None,
            "main".to_string(),
            None,
        );

        assert!(config.enabled);
        assert!(config.pr_generator.is_none());
        assert_eq!(config.config_path, None);
    }

    #[test]
    fn test_promotion_gitops_config_disabled() {
        let config = PromotionGitOpsConfig::disabled();

        assert!(!config.enabled);
        assert!(config.pr_generator.is_none());
        assert_eq!(config.config_path, None);
    }

    #[test]
    fn test_promotion_gitops_config_gitlab() {
        let config = PromotionGitOpsConfig::new(
            true,
            PRProvider::GitLab,
            "owner".to_string(),
            "repo".to_string(),
            Some("token".to_string()),
            "main".to_string(),
            None,
        );

        assert!(config.enabled);
        assert!(config.pr_generator.is_some());
    }

    #[tokio::test]
    async fn test_promotion_service_new() {
        let pool = setup_test_db().await;
        let service = PromotionService::new(pool);

        // Verify service is created with disabled gitops
        assert!(!service.gitops.enabled);
    }

    #[tokio::test]
    async fn test_promotion_service_with_gitops() {
        let pool = setup_test_db().await;
        let gitops = PromotionGitOpsConfig::disabled();
        let service = PromotionService::with_gitops(pool, gitops);

        assert!(!service.gitops.enabled);
    }

    #[tokio::test]
    async fn test_run_migrations() {
        let pool = setup_test_db().await;
        let service = PromotionService::new(pool);

        let result = service.run_migrations().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_record_promotion_success() {
        let pool = setup_test_db().await;
        let service = PromotionService::new(pool);

        let request = PromotionRequest {
            entity_type: PromotionEntityType::Scenario,
            entity_id: "test-scenario".to_string(),
            entity_version: Some("v1".to_string()),
            workspace_id: Uuid::new_v4().to_string(),
            from_environment: MockEnvironmentName::Dev,
            to_environment: MockEnvironmentName::Test,
            requires_approval: false,
            approval_required_reason: None,
            comments: Some("Test promotion".to_string()),
            metadata: std::collections::HashMap::new(),
        };

        let user_id = Uuid::new_v4();
        let result = service
            .record_promotion(&request, user_id, PromotionStatus::Pending, None)
            .await;

        assert!(result.is_ok());
        let promotion_id = result.unwrap();

        // Verify the promotion was recorded
        let promotion = service.get_promotion_by_id(promotion_id).await.unwrap();
        assert!(promotion.is_some());
        let promotion = promotion.unwrap();
        assert_eq!(promotion.entity_id, "test-scenario");
        assert_eq!(promotion.status, PromotionStatus::Pending);
    }

    #[tokio::test]
    async fn test_record_promotion_with_metadata() {
        let pool = setup_test_db().await;
        let service = PromotionService::new(pool);

        let mut metadata = std::collections::HashMap::new();
        metadata.insert("key1".to_string(), serde_json::Value::String("value1".to_string()));
        metadata.insert("key2".to_string(), serde_json::Value::String("value2".to_string()));

        let request = PromotionRequest {
            entity_type: PromotionEntityType::Persona,
            entity_id: "test-persona".to_string(),
            entity_version: None,
            workspace_id: Uuid::new_v4().to_string(),
            from_environment: MockEnvironmentName::Test,
            to_environment: MockEnvironmentName::Prod,
            requires_approval: true,
            approval_required_reason: Some("Production deployment".to_string()),
            comments: None,
            metadata,
        };

        let user_id = Uuid::new_v4();
        let result = service
            .record_promotion(&request, user_id, PromotionStatus::Pending, None)
            .await;

        assert!(result.is_ok());
        let promotion_id = result.unwrap();

        // Verify metadata was stored
        let promotion = service.get_promotion_by_id(promotion_id).await.unwrap();
        assert!(promotion.is_some());
        let promotion = promotion.unwrap();
        assert_eq!(promotion.metadata.len(), 2);
        assert_eq!(promotion.metadata.get("key1").unwrap(), "value1");
    }

    #[tokio::test]
    async fn test_update_promotion_status() {
        let pool = setup_test_db().await;
        let service = PromotionService::new(pool);

        let request = PromotionRequest {
            entity_type: PromotionEntityType::Config,
            entity_id: "test-config".to_string(),
            entity_version: None,
            workspace_id: Uuid::new_v4().to_string(),
            from_environment: MockEnvironmentName::Dev,
            to_environment: MockEnvironmentName::Test,
            requires_approval: true,
            approval_required_reason: None,
            comments: None,
            metadata: std::collections::HashMap::new(),
        };

        let user_id = Uuid::new_v4();
        let promotion_id = service
            .record_promotion(&request, user_id, PromotionStatus::Pending, None)
            .await
            .unwrap();

        // Update status
        let approver_id = Uuid::new_v4();
        let result = service
            .update_promotion_status(promotion_id, PromotionStatus::Approved, Some(approver_id))
            .await;

        assert!(result.is_ok());

        // Verify update
        let promotion = service.get_promotion_by_id(promotion_id).await.unwrap();
        assert!(promotion.is_some());
        let promotion = promotion.unwrap();
        assert_eq!(promotion.status, PromotionStatus::Approved);
        assert_eq!(promotion.approved_by, Some(approver_id.to_string()));
    }

    #[tokio::test]
    async fn test_update_promotion_pr_url() {
        let pool = setup_test_db().await;
        let service = PromotionService::new(pool);

        let request = PromotionRequest {
            entity_type: PromotionEntityType::Scenario,
            entity_id: "test-scenario".to_string(),
            entity_version: None,
            workspace_id: Uuid::new_v4().to_string(),
            from_environment: MockEnvironmentName::Dev,
            to_environment: MockEnvironmentName::Test,
            requires_approval: false,
            approval_required_reason: None,
            comments: None,
            metadata: std::collections::HashMap::new(),
        };

        let user_id = Uuid::new_v4();
        let promotion_id = service
            .record_promotion(&request, user_id, PromotionStatus::Pending, None)
            .await
            .unwrap();

        // Update PR URL
        let pr_url = "https://github.com/owner/repo/pull/123".to_string();
        let result = service.update_promotion_pr_url(promotion_id, pr_url.clone()).await;

        assert!(result.is_ok());

        // Verify update
        let promotion = service.get_promotion_by_id(promotion_id).await.unwrap();
        assert!(promotion.is_some());
        let promotion = promotion.unwrap();
        assert_eq!(promotion.pr_url, Some(pr_url));
    }

    #[tokio::test]
    async fn test_get_promotion_by_id_not_found() {
        let pool = setup_test_db().await;
        let service = PromotionService::new(pool);

        let result = service.get_promotion_by_id(Uuid::new_v4()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_promotion_history() {
        let pool = setup_test_db().await;
        let service = PromotionService::new(pool);

        let workspace_id = Uuid::new_v4();
        let entity_id = "test-scenario";

        // Create multiple promotions for the same entity
        for i in 0..3 {
            let request = PromotionRequest {
                entity_type: PromotionEntityType::Scenario,
                entity_id: entity_id.to_string(),
                entity_version: Some(format!("v{}", i)),
                workspace_id: workspace_id.to_string(),
                from_environment: MockEnvironmentName::Dev,
                to_environment: MockEnvironmentName::Test,
                requires_approval: false,
                approval_required_reason: None,
                comments: Some(format!("Promotion {}", i)),
                metadata: std::collections::HashMap::new(),
            };

            let user_id = Uuid::new_v4();
            service
                .record_promotion(&request, user_id, PromotionStatus::Completed, None)
                .await
                .unwrap();
        }

        // Get history
        let history = service
            .get_promotion_history(
                &workspace_id.to_string(),
                PromotionEntityType::Scenario,
                entity_id,
            )
            .await
            .unwrap();

        assert_eq!(history.promotions.len(), 3);
        assert_eq!(history.entity_id, entity_id);
        assert_eq!(history.workspace_id, workspace_id.to_string());
    }

    #[tokio::test]
    async fn test_get_workspace_promotions() {
        let pool = setup_test_db().await;
        let service = PromotionService::new(pool);

        let workspace_id = Uuid::new_v4();

        // Create promotions for different entities
        for entity_type in &[
            PromotionEntityType::Scenario,
            PromotionEntityType::Persona,
            PromotionEntityType::Config,
        ] {
            let request = PromotionRequest {
                entity_type: *entity_type,
                entity_id: format!("test-{}", entity_type),
                entity_version: None,
                workspace_id: workspace_id.to_string(),
                from_environment: MockEnvironmentName::Dev,
                to_environment: MockEnvironmentName::Test,
                requires_approval: false,
                approval_required_reason: None,
                comments: None,
                metadata: std::collections::HashMap::new(),
            };

            let user_id = Uuid::new_v4();
            service
                .record_promotion(&request, user_id, PromotionStatus::Completed, None)
                .await
                .unwrap();
        }

        // Get all workspace promotions
        let promotions =
            service.get_workspace_promotions(&workspace_id.to_string(), None).await.unwrap();

        assert_eq!(promotions.len(), 3);
    }

    #[tokio::test]
    async fn test_get_workspace_promotions_with_limit() {
        let pool = setup_test_db().await;
        let service = PromotionService::new(pool);

        let workspace_id = Uuid::new_v4();

        // Create 5 promotions
        for i in 0..5 {
            let request = PromotionRequest {
                entity_type: PromotionEntityType::Scenario,
                entity_id: format!("test-{}", i),
                entity_version: None,
                workspace_id: workspace_id.to_string(),
                from_environment: MockEnvironmentName::Dev,
                to_environment: MockEnvironmentName::Test,
                requires_approval: false,
                approval_required_reason: None,
                comments: None,
                metadata: std::collections::HashMap::new(),
            };

            let user_id = Uuid::new_v4();
            service
                .record_promotion(&request, user_id, PromotionStatus::Completed, None)
                .await
                .unwrap();
        }

        // Get with limit
        let promotions = service
            .get_workspace_promotions(&workspace_id.to_string(), Some(3))
            .await
            .unwrap();

        assert_eq!(promotions.len(), 3);
    }

    #[tokio::test]
    async fn test_get_pending_promotions() {
        let pool = setup_test_db().await;
        let service = PromotionService::new(pool);

        let workspace_id = Uuid::new_v4();

        // Create promotions with different statuses
        for (i, status) in [
            PromotionStatus::Pending,
            PromotionStatus::Approved,
            PromotionStatus::Pending,
            PromotionStatus::Completed,
        ]
        .iter()
        .enumerate()
        {
            let request = PromotionRequest {
                entity_type: PromotionEntityType::Scenario,
                entity_id: format!("test-{}", i),
                entity_version: None,
                workspace_id: workspace_id.to_string(),
                from_environment: MockEnvironmentName::Dev,
                to_environment: MockEnvironmentName::Test,
                requires_approval: true,
                approval_required_reason: None,
                comments: None,
                metadata: std::collections::HashMap::new(),
            };

            let user_id = Uuid::new_v4();
            service.record_promotion(&request, user_id, *status, None).await.unwrap();
        }

        // Get pending promotions
        let pending =
            service.get_pending_promotions(Some(&workspace_id.to_string())).await.unwrap();

        assert_eq!(pending.len(), 2);
        for promotion in &pending {
            assert_eq!(promotion.status, PromotionStatus::Pending);
        }
    }

    #[tokio::test]
    async fn test_get_pending_promotions_all_workspaces() {
        let pool = setup_test_db().await;
        let service = PromotionService::new(pool);

        // Create promotions in different workspaces
        for _ in 0..3 {
            let workspace_id = Uuid::new_v4();
            let request = PromotionRequest {
                entity_type: PromotionEntityType::Scenario,
                entity_id: "test-scenario".to_string(),
                entity_version: None,
                workspace_id: workspace_id.to_string(),
                from_environment: MockEnvironmentName::Dev,
                to_environment: MockEnvironmentName::Test,
                requires_approval: true,
                approval_required_reason: None,
                comments: None,
                metadata: std::collections::HashMap::new(),
            };

            let user_id = Uuid::new_v4();
            service
                .record_promotion(&request, user_id, PromotionStatus::Pending, None)
                .await
                .unwrap();
        }

        // Get all pending promotions
        let pending = service.get_pending_promotions(None).await.unwrap();

        assert_eq!(pending.len(), 3);
    }

    #[tokio::test]
    async fn test_promotion_service_trait_promote_entity() {
        let pool = setup_test_db().await;
        let service = PromotionService::new(pool);

        let workspace_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let result = service
            .promote_entity(
                workspace_id,
                PromotionEntityType::Scenario,
                "test-scenario".to_string(),
                Some("v1".to_string()),
                MockEnvironmentName::Dev,
                MockEnvironmentName::Test,
                user_id,
                Some("Auto promotion".to_string()),
            )
            .await;

        assert!(result.is_ok());
        let promotion_id = result.unwrap();

        // Verify the promotion was created with Completed status
        let promotion = service.get_promotion_by_id(promotion_id).await.unwrap();
        assert!(promotion.is_some());
        let promotion = promotion.unwrap();
        assert_eq!(promotion.status, PromotionStatus::Completed);
        assert_eq!(promotion.entity_id, "test-scenario");
    }

    #[tokio::test]
    async fn test_all_promotion_statuses() {
        let pool = setup_test_db().await;
        let service = PromotionService::new(pool);

        let statuses = vec![
            PromotionStatus::Pending,
            PromotionStatus::Approved,
            PromotionStatus::Rejected,
            PromotionStatus::Completed,
            PromotionStatus::Failed,
        ];

        for status in statuses {
            let request = PromotionRequest {
                entity_type: PromotionEntityType::Scenario,
                entity_id: format!("test-{}", status),
                entity_version: None,
                workspace_id: Uuid::new_v4().to_string(),
                from_environment: MockEnvironmentName::Dev,
                to_environment: MockEnvironmentName::Test,
                requires_approval: false,
                approval_required_reason: None,
                comments: None,
                metadata: std::collections::HashMap::new(),
            };

            let user_id = Uuid::new_v4();
            let promotion_id =
                service.record_promotion(&request, user_id, status, None).await.unwrap();

            // Verify status was stored correctly
            let promotion = service.get_promotion_by_id(promotion_id).await.unwrap();
            assert!(promotion.is_some());
            assert_eq!(promotion.unwrap().status, status);
        }
    }

    #[tokio::test]
    async fn test_all_entity_types() {
        let pool = setup_test_db().await;
        let service = PromotionService::new(pool);

        let entity_types = vec![
            PromotionEntityType::Scenario,
            PromotionEntityType::Persona,
            PromotionEntityType::Config,
        ];

        for entity_type in entity_types {
            let request = PromotionRequest {
                entity_type,
                entity_id: format!("test-{}", entity_type),
                entity_version: None,
                workspace_id: Uuid::new_v4().to_string(),
                from_environment: MockEnvironmentName::Dev,
                to_environment: MockEnvironmentName::Test,
                requires_approval: false,
                approval_required_reason: None,
                comments: None,
                metadata: std::collections::HashMap::new(),
            };

            let user_id = Uuid::new_v4();
            let promotion_id = service
                .record_promotion(&request, user_id, PromotionStatus::Completed, None)
                .await
                .unwrap();

            // Verify entity type was stored correctly
            let promotion = service.get_promotion_by_id(promotion_id).await.unwrap();
            assert!(promotion.is_some());
            assert_eq!(promotion.unwrap().entity_type, entity_type);
        }
    }
}
