//! Workspace mock request model (cloud mode), plus execution history.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WorkspaceRequest {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub folder_id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub method: String,
    pub path: String,
    pub status_code: i32,
    pub response_body: String,
    pub request_headers: serde_json::Value,
    pub response_headers: serde_json::Value,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestSummaryResponse {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub method: String,
    pub path: String,
    pub status_code: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WorkspaceRequest {
    pub fn to_summary(&self) -> RequestSummaryResponse {
        RequestSummaryResponse {
            id: self.id,
            name: self.name.clone(),
            description: self.description.clone(),
            method: self.method.clone(),
            path: self.path.clone(),
            status_code: self.status_code,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[cfg(feature = "postgres")]
impl WorkspaceRequest {
    pub async fn list_by_workspace(
        pool: &sqlx::PgPool,
        workspace_id: Uuid,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM workspace_requests
               WHERE workspace_id = $1 AND folder_id IS NULL
               ORDER BY sort_order, created_at"#,
        )
        .bind(workspace_id)
        .fetch_all(pool)
        .await
    }

    pub async fn list_by_folder(pool: &sqlx::PgPool, folder_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM workspace_requests
               WHERE folder_id = $1
               ORDER BY sort_order, created_at"#,
        )
        .bind(folder_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM workspace_requests WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        pool: &sqlx::PgPool,
        workspace_id: Uuid,
        folder_id: Option<Uuid>,
        name: &str,
        description: &str,
        method: &str,
        path: &str,
        status_code: i32,
        response_body: &str,
        request_headers: &serde_json::Value,
        response_headers: &serde_json::Value,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"INSERT INTO workspace_requests
                   (workspace_id, folder_id, name, description, method, path,
                    status_code, response_body, request_headers, response_headers)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               RETURNING *"#,
        )
        .bind(workspace_id)
        .bind(folder_id)
        .bind(name)
        .bind(description)
        .bind(method)
        .bind(path)
        .bind(status_code)
        .bind(response_body)
        .bind(request_headers)
        .bind(response_headers)
        .fetch_one(pool)
        .await
    }

    pub async fn count_in_workspace(pool: &sqlx::PgPool, workspace_id: Uuid) -> sqlx::Result<i64> {
        sqlx::query_scalar("SELECT COUNT(*) FROM workspace_requests WHERE workspace_id = $1")
            .bind(workspace_id)
            .fetch_one(pool)
            .await
    }
}

/// Execution history — one row per `/execute` invocation.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WorkspaceRequestHistory {
    pub id: Uuid,
    pub request_id: Uuid,
    pub workspace_id: Uuid,
    pub executed_by: Option<Uuid>,
    pub executed_at: DateTime<Utc>,
    pub request_method: String,
    pub request_path: String,
    pub request_headers: serde_json::Value,
    pub request_body: Option<String>,
    pub response_status_code: i32,
    pub response_headers: serde_json::Value,
    pub response_body: Option<String>,
    pub response_time_ms: i32,
    pub response_size_bytes: i32,
    pub error_message: Option<String>,
}

/// Matches the `ResponseHistoryEntry` TypeScript interface consumed by `ResponseHistory.tsx`.
#[derive(Debug, Clone, Serialize)]
pub struct HistoryEntryResponse {
    pub executed_at: DateTime<Utc>,
    pub request_method: String,
    pub request_path: String,
    pub request_headers: serde_json::Value,
    pub request_body: Option<String>,
    pub response_status_code: i32,
    pub response_headers: serde_json::Value,
    pub response_body: Option<String>,
    pub response_time_ms: i32,
    pub response_size_bytes: i32,
    pub error_message: Option<String>,
}

impl WorkspaceRequestHistory {
    pub fn to_response(&self) -> HistoryEntryResponse {
        HistoryEntryResponse {
            executed_at: self.executed_at,
            request_method: self.request_method.clone(),
            request_path: self.request_path.clone(),
            request_headers: self.request_headers.clone(),
            request_body: self.request_body.clone(),
            response_status_code: self.response_status_code,
            response_headers: self.response_headers.clone(),
            response_body: self.response_body.clone(),
            response_time_ms: self.response_time_ms,
            response_size_bytes: self.response_size_bytes,
            error_message: self.error_message.clone(),
        }
    }
}

#[cfg(feature = "postgres")]
impl WorkspaceRequestHistory {
    /// Return up to `limit` most-recent executions for a request, newest first.
    pub async fn list_for_request(
        pool: &sqlx::PgPool,
        request_id: Uuid,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM workspace_request_history
               WHERE request_id = $1
               ORDER BY executed_at DESC
               LIMIT $2"#,
        )
        .bind(request_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    /// Record a single execution.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert(
        pool: &sqlx::PgPool,
        request_id: Uuid,
        workspace_id: Uuid,
        executed_by: Option<Uuid>,
        request_method: &str,
        request_path: &str,
        request_headers: &serde_json::Value,
        request_body: Option<&str>,
        response_status_code: i32,
        response_headers: &serde_json::Value,
        response_body: Option<&str>,
        response_time_ms: i32,
        response_size_bytes: i32,
        error_message: Option<&str>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"INSERT INTO workspace_request_history
                   (request_id, workspace_id, executed_by,
                    request_method, request_path, request_headers, request_body,
                    response_status_code, response_headers, response_body,
                    response_time_ms, response_size_bytes, error_message)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
               RETURNING *"#,
        )
        .bind(request_id)
        .bind(workspace_id)
        .bind(executed_by)
        .bind(request_method)
        .bind(request_path)
        .bind(request_headers)
        .bind(request_body)
        .bind(response_status_code)
        .bind(response_headers)
        .bind(response_body)
        .bind(response_time_ms)
        .bind(response_size_bytes)
        .bind(error_message)
        .fetch_one(pool)
        .await
    }

    pub async fn count_for_request(pool: &sqlx::PgPool, request_id: Uuid) -> sqlx::Result<i64> {
        sqlx::query_scalar("SELECT COUNT(*) FROM workspace_request_history WHERE request_id = $1")
            .bind(request_id)
            .fetch_one(pool)
            .await
    }
}
