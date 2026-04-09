//! Workspace model for managing mock API workspace definitions in cloud mode

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Workspace {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub description: String,
    pub is_active: bool,
    pub settings: serde_json::Value,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Summary response matching the frontend WorkspaceSummary interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSummaryResponse {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
    pub config_count: i64,
    pub service_count: i64,
    pub request_count: i64,
    pub folder_count: i64,
}

#[cfg(feature = "postgres")]
impl Workspace {
    /// Create a new workspace
    pub async fn create(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO workspaces (org_id, name, description, created_by)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(name)
        .bind(description)
        .bind(created_by)
        .fetch_one(pool)
        .await
    }

    /// Find a workspace by ID
    pub async fn find_by_id(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM workspaces WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Find all workspaces for an organization
    pub async fn find_by_org(pool: &sqlx::PgPool, org_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM workspaces WHERE org_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    /// Update a workspace
    pub async fn update(
        pool: &sqlx::PgPool,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        is_active: Option<bool>,
        settings: Option<&serde_json::Value>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE workspaces
            SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                is_active = COALESCE($4, is_active),
                settings = COALESCE($5, settings),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(is_active)
        .bind(settings)
        .fetch_optional(pool)
        .await
    }

    /// Delete a workspace
    pub async fn delete(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM workspaces WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Convert to summary response (frontend-compatible format)
    pub fn to_summary(&self) -> WorkspaceSummaryResponse {
        WorkspaceSummaryResponse {
            id: self.id,
            name: self.name.clone(),
            description: self.description.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            is_active: self.is_active,
            config_count: 0,
            service_count: 0,
            request_count: 0,
            folder_count: 0,
        }
    }
}
