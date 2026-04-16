//! Workspace environment model — Postman-style environments per cloud workspace.
//!
//! Distinct from [`crate::models::mock_environment`], which models the
//! constrained dev/test/prod tier used by the scenario-promotion workflow.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WorkspaceEnvironment {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub description: String,
    pub color: Option<serde_json::Value>,
    pub is_global: bool,
    pub is_active: bool,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WorkspaceEnvironmentVariable {
    pub id: Uuid,
    pub environment_id: Uuid,
    pub name: String,
    pub value: String,
    pub is_secret: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Summary row joined with the variable count for `GET .../environments`.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WorkspaceEnvironmentSummary {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub description: String,
    pub color: Option<serde_json::Value>,
    pub is_global: bool,
    pub is_active: bool,
    pub display_order: i32,
    pub variable_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
impl WorkspaceEnvironment {
    /// List environments for a workspace, ordered by `display_order` then
    /// `created_at`. Includes the variable count via a correlated subquery.
    pub async fn list_with_counts(
        pool: &sqlx::PgPool,
        workspace_id: Uuid,
    ) -> sqlx::Result<Vec<WorkspaceEnvironmentSummary>> {
        sqlx::query_as::<_, WorkspaceEnvironmentSummary>(
            r#"
            SELECT
                e.id,
                e.workspace_id,
                e.name,
                e.description,
                e.color,
                e.is_global,
                e.is_active,
                e.display_order,
                COALESCE((
                    SELECT COUNT(*) FROM workspace_environment_variables v
                    WHERE v.environment_id = e.id
                ), 0) AS variable_count,
                e.created_at,
                e.updated_at
            FROM workspace_environments e
            WHERE e.workspace_id = $1
            ORDER BY e.display_order ASC, e.created_at ASC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(pool)
        .await
    }

    /// Ensure the workspace has a global environment. Creates one if missing.
    /// Idempotent. Used to lazily seed environments on first read.
    pub async fn ensure_global(pool: &sqlx::PgPool, workspace_id: Uuid) -> sqlx::Result<Self> {
        if let Some(existing) = sqlx::query_as::<_, Self>(
            "SELECT * FROM workspace_environments WHERE workspace_id = $1 AND is_global = true",
        )
        .bind(workspace_id)
        .fetch_optional(pool)
        .await?
        {
            return Ok(existing);
        }

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO workspace_environments (workspace_id, name, description, is_global, is_active, display_order)
            VALUES ($1, 'Globals', 'Workspace-wide variables shared across environments.', true, true, 0)
            ON CONFLICT (workspace_id, name) DO NOTHING
            RETURNING *
            "#,
        )
        .bind(workspace_id)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM workspace_environments WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(
        pool: &sqlx::PgPool,
        workspace_id: Uuid,
        name: &str,
        description: &str,
        color: Option<&serde_json::Value>,
    ) -> sqlx::Result<Self> {
        // New environments go at the end of the display order.
        let next_order: (Option<i32>,) = sqlx::query_as(
            "SELECT MAX(display_order) + 1 FROM workspace_environments WHERE workspace_id = $1",
        )
        .bind(workspace_id)
        .fetch_one(pool)
        .await?;

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO workspace_environments
                (workspace_id, name, description, color, display_order)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(workspace_id)
        .bind(name)
        .bind(description)
        .bind(color)
        .bind(next_order.0.unwrap_or(1))
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &sqlx::PgPool,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        color: Option<&serde_json::Value>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE workspace_environments
            SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                color = COALESCE($4, color),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(color)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM workspace_environments WHERE id = $1 AND is_global = false")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Activate one environment within a workspace; deactivates the rest in
    /// the same transaction so the partial unique index isn't violated.
    pub async fn set_active(
        pool: &sqlx::PgPool,
        workspace_id: Uuid,
        environment_id: Uuid,
    ) -> sqlx::Result<()> {
        let mut tx = pool.begin().await?;
        sqlx::query(
            "UPDATE workspace_environments SET is_active = false WHERE workspace_id = $1 AND is_active = true",
        )
        .bind(workspace_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE workspace_environments SET is_active = true, updated_at = NOW() WHERE id = $1 AND workspace_id = $2",
        )
        .bind(environment_id)
        .bind(workspace_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }

    /// Reorder environments. Order positions are assigned by index; unknown
    /// IDs are silently skipped (avoids partial-failure mid-reorder).
    pub async fn reorder(
        pool: &sqlx::PgPool,
        workspace_id: Uuid,
        environment_ids: &[Uuid],
    ) -> sqlx::Result<()> {
        let mut tx = pool.begin().await?;
        for (idx, env_id) in environment_ids.iter().enumerate() {
            sqlx::query(
                "UPDATE workspace_environments SET display_order = $1, updated_at = NOW() WHERE id = $2 AND workspace_id = $3",
            )
            .bind(idx as i32)
            .bind(env_id)
            .bind(workspace_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await
    }
}

#[cfg(feature = "postgres")]
impl WorkspaceEnvironmentVariable {
    pub async fn list_for_environment(
        pool: &sqlx::PgPool,
        environment_id: Uuid,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM workspace_environment_variables WHERE environment_id = $1 ORDER BY name ASC",
        )
        .bind(environment_id)
        .fetch_all(pool)
        .await
    }

    /// Upsert a variable. The unique (environment_id, name) index makes this
    /// the natural way to express "set this variable".
    pub async fn upsert(
        pool: &sqlx::PgPool,
        environment_id: Uuid,
        name: &str,
        value: &str,
        is_secret: bool,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO workspace_environment_variables (environment_id, name, value, is_secret)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (environment_id, name)
            DO UPDATE SET value = EXCLUDED.value, is_secret = EXCLUDED.is_secret, updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(environment_id)
        .bind(name)
        .bind(value)
        .bind(is_secret)
        .fetch_one(pool)
        .await
    }

    pub async fn delete(
        pool: &sqlx::PgPool,
        environment_id: Uuid,
        name: &str,
    ) -> sqlx::Result<u64> {
        let result = sqlx::query(
            "DELETE FROM workspace_environment_variables WHERE environment_id = $1 AND name = $2",
        )
        .bind(environment_id)
        .bind(name)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }
}
