//! Workspace folder model (cloud mode). Mirrors the self-hosted folder API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WorkspaceFolder {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub parent_folder_id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FolderSummaryResponse {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub parent_id: Option<Uuid>,
    pub subfolder_count: i64,
    pub request_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
impl WorkspaceFolder {
    pub async fn list_by_workspace(
        pool: &sqlx::PgPool,
        workspace_id: Uuid,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM workspace_folders
               WHERE workspace_id = $1
               ORDER BY sort_order, created_at"#,
        )
        .bind(workspace_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM workspace_folders WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(
        pool: &sqlx::PgPool,
        workspace_id: Uuid,
        parent_folder_id: Option<Uuid>,
        name: &str,
        description: &str,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"INSERT INTO workspace_folders
                   (workspace_id, parent_folder_id, name, description)
               VALUES ($1, $2, $3, $4)
               RETURNING *"#,
        )
        .bind(workspace_id)
        .bind(parent_folder_id)
        .bind(name)
        .bind(description)
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM workspace_folders WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn to_summary_response(
        &self,
        pool: &sqlx::PgPool,
    ) -> sqlx::Result<FolderSummaryResponse> {
        let subfolder_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM workspace_folders WHERE parent_folder_id = $1",
        )
        .bind(self.id)
        .fetch_one(pool)
        .await?;

        let request_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM workspace_requests WHERE folder_id = $1")
                .bind(self.id)
                .fetch_one(pool)
                .await?;

        Ok(FolderSummaryResponse {
            id: self.id,
            name: self.name.clone(),
            description: self.description.clone(),
            parent_id: self.parent_folder_id,
            subfolder_count,
            request_count,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}
