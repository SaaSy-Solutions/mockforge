//! Scenario promotion model for environment promotion workflow
//!
//! Tracks scenario promotions between environments (dev → test → prod)
//! with approval workflow support for high-impact changes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Promotion status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromotionStatus {
    /// Promotion is pending approval
    Pending,
    /// Promotion has been approved
    Approved,
    /// Promotion has been rejected
    Rejected,
    /// Promotion has been completed
    Completed,
    /// Promotion failed
    Failed,
}

impl PromotionStatus {
    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            PromotionStatus::Pending => "pending",
            PromotionStatus::Approved => "approved",
            PromotionStatus::Rejected => "rejected",
            PromotionStatus::Completed => "completed",
            PromotionStatus::Failed => "failed",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Some(PromotionStatus::Pending),
            "approved" => Some(PromotionStatus::Approved),
            "rejected" => Some(PromotionStatus::Rejected),
            "completed" => Some(PromotionStatus::Completed),
            "failed" => Some(PromotionStatus::Failed),
            _ => None,
        }
    }
}

impl std::fmt::Display for PromotionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Scenario promotion model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ScenarioPromotion {
    /// Unique identifier
    pub id: Uuid,
    /// Scenario ID being promoted
    pub scenario_id: Uuid,
    /// Scenario version being promoted
    pub scenario_version: String,
    /// Workspace ID where promotion occurs
    pub workspace_id: Uuid,
    /// Source environment (dev or test)
    pub from_environment: String,
    /// Target environment (test or prod)
    pub to_environment: String,
    /// User who initiated the promotion
    pub promoted_by: Uuid,
    /// User who approved the promotion (nullable)
    pub approved_by: Option<Uuid>,
    /// Promotion status
    pub status: String, // Stored as VARCHAR, converted via methods
    /// Whether this promotion requires approval
    pub requires_approval: bool,
    /// Reason why approval is required
    pub approval_required_reason: Option<String>,
    /// Comments from promoter
    pub comments: Option<String>,
    /// Comments from approver
    pub approval_comments: Option<String>,
    /// When promotion was completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl ScenarioPromotion {
    /// Get status as enum
    pub fn status_enum(&self) -> Option<PromotionStatus> {
        PromotionStatus::from_str(&self.status)
    }

    /// Create a new promotion request
    pub async fn create(
        pool: &sqlx::PgPool,
        scenario_id: Uuid,
        scenario_version: &str,
        workspace_id: Uuid,
        from_environment: &str,
        to_environment: &str,
        promoted_by: Uuid,
        requires_approval: bool,
        approval_required_reason: Option<&str>,
        comments: Option<&str>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO scenario_promotions (
                scenario_id, scenario_version, workspace_id, from_environment, to_environment,
                promoted_by, requires_approval, approval_required_reason, comments, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending')
            RETURNING *
            "#,
        )
        .bind(scenario_id)
        .bind(scenario_version)
        .bind(workspace_id)
        .bind(from_environment)
        .bind(to_environment)
        .bind(promoted_by)
        .bind(requires_approval)
        .bind(approval_required_reason)
        .bind(comments)
        .fetch_one(pool)
        .await
    }

    /// Find by ID
    pub async fn find_by_id(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM scenario_promotions WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// List promotions for a workspace
    pub async fn list_by_workspace(
        pool: &sqlx::PgPool,
        workspace_id: Uuid,
        status: Option<PromotionStatus>,
    ) -> sqlx::Result<Vec<Self>> {
        if let Some(status) = status {
            sqlx::query_as::<_, Self>(
                "SELECT * FROM scenario_promotions WHERE workspace_id = $1 AND status = $2 ORDER BY created_at DESC",
            )
            .bind(workspace_id)
            .bind(status.as_str())
            .fetch_all(pool)
            .await
        } else {
            sqlx::query_as::<_, Self>(
                "SELECT * FROM scenario_promotions WHERE workspace_id = $1 ORDER BY created_at DESC",
            )
            .bind(workspace_id)
            .fetch_all(pool)
            .await
        }
    }

    /// List promotions for a scenario
    pub async fn list_by_scenario(
        pool: &sqlx::PgPool,
        scenario_id: Uuid,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM scenario_promotions WHERE scenario_id = $1 ORDER BY created_at DESC",
        )
        .bind(scenario_id)
        .fetch_all(pool)
        .await
    }

    /// Approve a promotion
    pub async fn approve(
        &self,
        pool: &sqlx::PgPool,
        approved_by: Uuid,
        approval_comments: Option<&str>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE scenario_promotions
            SET
                status = 'approved',
                approved_by = $1,
                approval_comments = $2,
                updated_at = NOW()
            WHERE id = $3
            RETURNING *
            "#,
        )
        .bind(approved_by)
        .bind(approval_comments)
        .bind(self.id)
        .fetch_one(pool)
        .await
    }

    /// Reject a promotion
    pub async fn reject(
        &self,
        pool: &sqlx::PgPool,
        rejected_by: Uuid,
        rejection_reason: &str,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE scenario_promotions
            SET
                status = 'rejected',
                approved_by = $1,
                approval_comments = $2,
                updated_at = NOW()
            WHERE id = $3
            RETURNING *
            "#,
        )
        .bind(rejected_by)
        .bind(Some(rejection_reason))
        .bind(self.id)
        .fetch_one(pool)
        .await
    }

    /// Mark promotion as completed
    pub async fn mark_completed(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE scenario_promotions
            SET
                status = 'completed',
                completed_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await
    }

    /// Mark promotion as failed
    pub async fn mark_failed(
        pool: &sqlx::PgPool,
        id: Uuid,
        error_message: &str,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE scenario_promotions
            SET
                status = 'failed',
                approval_comments = $1,
                updated_at = NOW()
            WHERE id = $2
            RETURNING *
            "#,
        )
        .bind(Some(error_message))
        .bind(id)
        .fetch_one(pool)
        .await
    }
}

/// Scenario environment version tracking
///
/// Tracks which scenario version is active in each environment for each workspace.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ScenarioEnvironmentVersion {
    /// Unique identifier
    pub id: Uuid,
    /// Scenario ID
    pub scenario_id: Uuid,
    /// Workspace ID
    pub workspace_id: Uuid,
    /// Environment name (dev/test/prod)
    pub environment: String,
    /// Active scenario version in this environment
    pub scenario_version: String,
    /// When this version was promoted
    pub promoted_at: DateTime<Utc>,
    /// User who promoted this version
    pub promoted_by: Uuid,
    /// Promotion ID that created this record
    pub promotion_id: Option<Uuid>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl ScenarioEnvironmentVersion {
    /// Get or create environment version record
    ///
    /// If a version already exists for this scenario/workspace/environment,
    /// updates it. Otherwise creates a new record.
    pub async fn set_version(
        pool: &sqlx::PgPool,
        scenario_id: Uuid,
        workspace_id: Uuid,
        environment: &str,
        scenario_version: &str,
        promoted_by: Uuid,
        promotion_id: Option<Uuid>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO scenario_environment_versions (
                scenario_id, workspace_id, environment, scenario_version,
                promoted_by, promotion_id
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (scenario_id, workspace_id, environment)
            DO UPDATE SET
                scenario_version = EXCLUDED.scenario_version,
                promoted_at = NOW(),
                promoted_by = EXCLUDED.promoted_by,
                promotion_id = EXCLUDED.promotion_id,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(scenario_id)
        .bind(workspace_id)
        .bind(environment)
        .bind(scenario_version)
        .bind(promoted_by)
        .bind(promotion_id)
        .fetch_one(pool)
        .await
    }

    /// Get current version for a scenario in an environment
    pub async fn get_version(
        pool: &sqlx::PgPool,
        scenario_id: Uuid,
        workspace_id: Uuid,
        environment: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM scenario_environment_versions
            WHERE scenario_id = $1 AND workspace_id = $2 AND environment = $3
            "#,
        )
        .bind(scenario_id)
        .bind(workspace_id)
        .bind(environment)
        .fetch_optional(pool)
        .await
    }

    /// List all environment versions for a scenario in a workspace
    pub async fn list_by_scenario_workspace(
        pool: &sqlx::PgPool,
        scenario_id: Uuid,
        workspace_id: Uuid,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM scenario_environment_versions
            WHERE scenario_id = $1 AND workspace_id = $2
            ORDER BY environment, promoted_at DESC
            "#,
        )
        .bind(scenario_id)
        .bind(workspace_id)
        .fetch_all(pool)
        .await
    }
}
