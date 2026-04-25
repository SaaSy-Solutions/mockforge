//! Workspace environment + variable models (cloud mode).

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
    pub color_hex: String,
    pub color_name: String,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WorkspaceEnvVariable {
    pub environment_id: Uuid,
    pub name: String,
    pub value: String,
    pub is_secret: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Matches the self-hosted environment list shape consumed by EnvironmentManager.tsx.
#[derive(Debug, Clone, Serialize)]
pub struct EnvironmentSummaryResponse {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub variable_count: i64,
    pub is_global: bool,
    pub active: bool,
    pub color: Option<ColorResponse>,
    pub order: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColorResponse {
    pub hex: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VariableResponse {
    pub id: String,
    pub key: String,
    pub value: String,
    pub encrypted: bool,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
}

impl WorkspaceEnvironment {
    pub fn color_response(&self) -> Option<ColorResponse> {
        if self.color_hex.is_empty() && self.color_name.is_empty() {
            None
        } else {
            Some(ColorResponse {
                hex: self.color_hex.clone(),
                name: self.color_name.clone(),
            })
        }
    }
}

impl WorkspaceEnvVariable {
    pub fn to_response(&self) -> VariableResponse {
        VariableResponse {
            // Self-hosted uses a synthetic id; we build a stable one from env_id + name.
            id: format!("{}:{}", self.environment_id, self.name),
            key: self.name.clone(),
            value: self.value.clone(),
            encrypted: self.is_secret,
            created_at: self.created_at,
        }
    }
}

#[cfg(feature = "postgres")]
impl WorkspaceEnvironment {
    pub async fn list_by_workspace(
        pool: &sqlx::PgPool,
        workspace_id: Uuid,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM workspace_environments
               WHERE workspace_id = $1
               ORDER BY sort_order, created_at"#,
        )
        .bind(workspace_id)
        .fetch_all(pool)
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
        color_hex: &str,
        color_name: &str,
    ) -> sqlx::Result<Self> {
        // New environments go at the end of the sort order.
        let next_order: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM workspace_environments WHERE workspace_id = $1",
        )
        .bind(workspace_id)
        .fetch_one(pool)
        .await?;

        sqlx::query_as::<_, Self>(
            r#"INSERT INTO workspace_environments
                   (workspace_id, name, description, color_hex, color_name, sort_order)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING *"#,
        )
        .bind(workspace_id)
        .bind(name)
        .bind(description)
        .bind(color_hex)
        .bind(color_name)
        .bind(next_order)
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &sqlx::PgPool,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        color_hex: Option<&str>,
        color_name: Option<&str>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE workspace_environments
               SET name        = COALESCE($2, name),
                   description = COALESCE($3, description),
                   color_hex   = COALESCE($4, color_hex),
                   color_name  = COALESCE($5, color_name),
                   updated_at  = NOW()
               WHERE id = $1
               RETURNING *"#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(color_hex)
        .bind(color_name)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM workspace_environments WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Deactivate every environment in the workspace, then mark the target active.
    /// Intended to run inside a transaction so "only one active" stays invariant.
    pub async fn set_active(
        pool: &sqlx::PgPool,
        workspace_id: Uuid,
        environment_id: Uuid,
    ) -> sqlx::Result<Option<Self>> {
        let mut tx = pool.begin().await?;
        sqlx::query(
            "UPDATE workspace_environments SET is_active = FALSE, updated_at = NOW() WHERE workspace_id = $1",
        )
        .bind(workspace_id)
        .execute(&mut *tx)
        .await?;

        let env = sqlx::query_as::<_, Self>(
            r#"UPDATE workspace_environments
               SET is_active = TRUE, updated_at = NOW()
               WHERE id = $1 AND workspace_id = $2
               RETURNING *"#,
        )
        .bind(environment_id)
        .bind(workspace_id)
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(env)
    }

    /// Reassign sort_order to match the supplied id list.
    pub async fn reorder(
        pool: &sqlx::PgPool,
        workspace_id: Uuid,
        ordered_ids: &[Uuid],
    ) -> sqlx::Result<()> {
        let mut tx = pool.begin().await?;
        for (idx, id) in ordered_ids.iter().enumerate() {
            sqlx::query(
                r#"UPDATE workspace_environments
                   SET sort_order = $3, updated_at = NOW()
                   WHERE id = $1 AND workspace_id = $2"#,
            )
            .bind(id)
            .bind(workspace_id)
            .bind(idx as i32)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn variable_count(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<i64> {
        sqlx::query_scalar("SELECT COUNT(*) FROM workspace_env_variables WHERE environment_id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
    }
}

#[cfg(feature = "postgres")]
impl WorkspaceEnvVariable {
    pub async fn list_by_environment(
        pool: &sqlx::PgPool,
        environment_id: Uuid,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM workspace_env_variables
               WHERE environment_id = $1
               ORDER BY name"#,
        )
        .bind(environment_id)
        .fetch_all(pool)
        .await
    }

    pub async fn upsert(
        pool: &sqlx::PgPool,
        environment_id: Uuid,
        name: &str,
        value: &str,
        is_secret: bool,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"INSERT INTO workspace_env_variables (environment_id, name, value, is_secret)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT (environment_id, name)
               DO UPDATE SET value     = EXCLUDED.value,
                             is_secret = EXCLUDED.is_secret,
                             updated_at = NOW()
               RETURNING *"#,
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
    ) -> sqlx::Result<bool> {
        let rows = sqlx::query(
            "DELETE FROM workspace_env_variables WHERE environment_id = $1 AND name = $2",
        )
        .bind(environment_id)
        .bind(name)
        .execute(pool)
        .await?
        .rows_affected();
        Ok(rows > 0)
    }
}
