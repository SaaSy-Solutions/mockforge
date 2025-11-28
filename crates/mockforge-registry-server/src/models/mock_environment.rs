//! Mock environment model for environment-specific configurations
//!
//! Mock environments (dev/test/prod) allow workspaces to have different
//! configurations for reality levels, chaos profiles, and drift budgets
//! per environment, similar to application environments.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Mock environment name (dev/test/prod)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MockEnvironmentName {
    Dev,
    Test,
    Prod,
}

impl MockEnvironmentName {
    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            MockEnvironmentName::Dev => "dev",
            MockEnvironmentName::Test => "test",
            MockEnvironmentName::Prod => "prod",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dev" => Some(MockEnvironmentName::Dev),
            "test" => Some(MockEnvironmentName::Test),
            "prod" => Some(MockEnvironmentName::Prod),
            _ => None,
        }
    }
}

impl std::fmt::Display for MockEnvironmentName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Mock environment model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct MockEnvironment {
    /// Unique identifier
    pub id: Uuid,
    /// Workspace ID (references workspace, may be in collab DB or registry)
    pub workspace_id: Uuid,
    /// Environment name (dev/test/prod)
    pub name: String, // Stored as VARCHAR, converted via methods
    /// Environment-specific reality configuration (JSONB)
    pub reality_config: serde_json::Value,
    /// Environment-specific chaos engineering configuration (JSONB)
    pub chaos_config: serde_json::Value,
    /// Environment-specific drift budget configuration (JSONB)
    pub drift_budget_config: serde_json::Value,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl MockEnvironment {
    /// Get environment name as enum
    pub fn environment_name(&self) -> Option<MockEnvironmentName> {
        MockEnvironmentName::from_str(&self.name)
    }

    /// Create a new mock environment
    pub async fn create(
        pool: &sqlx::PgPool,
        workspace_id: Uuid,
        name: MockEnvironmentName,
        reality_config: Option<serde_json::Value>,
        chaos_config: Option<serde_json::Value>,
        drift_budget_config: Option<serde_json::Value>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO mock_environments (
                workspace_id, name, reality_config, chaos_config, drift_budget_config
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(workspace_id)
        .bind(name.as_str())
        .bind(reality_config.unwrap_or_else(|| serde_json::json!({})))
        .bind(chaos_config.unwrap_or_else(|| serde_json::json!({})))
        .bind(drift_budget_config.unwrap_or_else(|| serde_json::json!({})))
        .fetch_one(pool)
        .await
    }

    /// Find by ID
    pub async fn find_by_id(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM mock_environments WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Find by workspace ID and environment name
    pub async fn find_by_workspace_and_name(
        pool: &sqlx::PgPool,
        workspace_id: Uuid,
        name: MockEnvironmentName,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM mock_environments WHERE workspace_id = $1 AND name = $2",
        )
        .bind(workspace_id)
        .bind(name.as_str())
        .fetch_optional(pool)
        .await
    }

    /// List all environments for a workspace
    pub async fn list_by_workspace(
        pool: &sqlx::PgPool,
        workspace_id: Uuid,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM mock_environments WHERE workspace_id = $1 ORDER BY name",
        )
        .bind(workspace_id)
        .fetch_all(pool)
        .await
    }

    /// Update environment configuration
    pub async fn update_config(
        &self,
        pool: &sqlx::PgPool,
        reality_config: Option<serde_json::Value>,
        chaos_config: Option<serde_json::Value>,
        drift_budget_config: Option<serde_json::Value>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE mock_environments
            SET
                reality_config = COALESCE($1, reality_config),
                chaos_config = COALESCE($2, chaos_config),
                drift_budget_config = COALESCE($3, drift_budget_config),
                updated_at = NOW()
            WHERE id = $4
            RETURNING *
            "#,
        )
        .bind(reality_config)
        .bind(chaos_config)
        .bind(drift_budget_config)
        .bind(self.id)
        .fetch_one(pool)
        .await
    }

    /// Delete environment
    pub async fn delete(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM mock_environments WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Delete all environments for a workspace
    pub async fn delete_by_workspace(pool: &sqlx::PgPool, workspace_id: Uuid) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM mock_environments WHERE workspace_id = $1")
            .bind(workspace_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
