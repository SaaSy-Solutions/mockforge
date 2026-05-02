//! Fixture model for managing mock response fixtures in cloud mode

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CloudFixture {
    pub id: Uuid,
    pub org_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub path: String,
    pub method: String,
    pub content: Option<serde_json::Value>,
    pub tags: serde_json::Value,
    pub route_path: Option<String>,
    pub protocol: Option<String>,
    pub created_by: Uuid,
    /// Resolved username of the creator. Populated by queries that LEFT JOIN
    /// `users`; absent on raw row reads.
    #[sqlx(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by_username: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
impl CloudFixture {
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
        path: &str,
        method: &str,
        content: Option<&serde_json::Value>,
        protocol: Option<&str>,
        tags: Option<&serde_json::Value>,
        workspace_id: Option<Uuid>,
        route_path: Option<&str>,
    ) -> sqlx::Result<Self> {
        let tags_value = tags.cloned().unwrap_or_else(|| serde_json::Value::Array(Vec::new()));
        sqlx::query_as::<_, Self>(
            r#"
            WITH inserted AS (
                INSERT INTO fixtures (
                    org_id, workspace_id, name, description, path, method,
                    content, protocol, tags, route_path, created_by
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                RETURNING *
            )
            SELECT i.*, u.username AS created_by_username
            FROM inserted i
            LEFT JOIN users u ON u.id = i.created_by
            "#,
        )
        .bind(org_id)
        .bind(workspace_id)
        .bind(name)
        .bind(description)
        .bind(path)
        .bind(method)
        .bind(content)
        .bind(protocol)
        .bind(tags_value)
        .bind(route_path)
        .bind(created_by)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT f.*, u.username AS created_by_username
            FROM fixtures f
            LEFT JOIN users u ON u.id = f.created_by
            WHERE f.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// List fixtures in an organization, optionally filtered to a single
    /// workspace. Pass `workspace_id = None` to return everything in the org.
    pub async fn find_by_org(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        workspace_id: Option<Uuid>,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT f.*, u.username AS created_by_username
            FROM fixtures f
            LEFT JOIN users u ON u.id = f.created_by
            WHERE f.org_id = $1
              AND ($2::uuid IS NULL OR f.workspace_id = $2)
            ORDER BY f.created_at DESC
            "#,
        )
        .bind(org_id)
        .bind(workspace_id)
        .fetch_all(pool)
        .await
    }

    /// Update mutable fields on a fixture. `workspace_id` is tri-state:
    /// `None` leaves it untouched; `Some(None)` clears it; `Some(Some(id))`
    /// reassigns it.
    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        pool: &sqlx::PgPool,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        path: Option<&str>,
        method: Option<&str>,
        content: Option<&serde_json::Value>,
        protocol: Option<&str>,
        tags: Option<&serde_json::Value>,
        route_path: Option<&str>,
        workspace_id: Option<Option<Uuid>>,
    ) -> sqlx::Result<Option<Self>> {
        // workspace_id update flag: when None, leave column untouched; when
        // Some(_), apply the wrapped value (which may itself be NULL).
        let (workspace_set, workspace_value) = match workspace_id {
            Some(value) => (true, value),
            None => (false, None),
        };

        sqlx::query_as::<_, Self>(
            r#"
            WITH updated AS (
                UPDATE fixtures
                SET
                    name = COALESCE($2, name),
                    description = COALESCE($3, description),
                    path = COALESCE($4, path),
                    method = COALESCE($5, method),
                    content = COALESCE($6, content),
                    protocol = COALESCE($7, protocol),
                    tags = COALESCE($8, tags),
                    route_path = COALESCE($9, route_path),
                    workspace_id = CASE WHEN $10 THEN $11 ELSE workspace_id END,
                    updated_at = NOW()
                WHERE id = $1
                RETURNING *
            )
            SELECT u_row.*, u.username AS created_by_username
            FROM updated u_row
            LEFT JOIN users u ON u.id = u_row.created_by
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(path)
        .bind(method)
        .bind(content)
        .bind(protocol)
        .bind(tags)
        .bind(route_path)
        .bind(workspace_set)
        .bind(workspace_value)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM fixtures WHERE id = $1").bind(id).execute(pool).await?;
        Ok(())
    }

    /// Bulk delete by id, scoped to an org for safety. Returns the IDs that
    /// were actually deleted (filters out any not in the org or already gone).
    pub async fn delete_many(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        ids: &[Uuid],
    ) -> sqlx::Result<Vec<Uuid>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            DELETE FROM fixtures
            WHERE id = ANY($1) AND org_id = $2
            RETURNING id
            "#,
        )
        .bind(ids)
        .bind(org_id)
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }
}
