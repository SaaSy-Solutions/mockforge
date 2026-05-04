//! Capture sessions + behavioral-clone models
//! (cloud-enablement task #6 / Phase 1).
//!
//! Training jobs reuse the #4 worker pool with kind='behavioral_clone';
//! replay reuses test_runs with kind='replay'.
//!
//! See docs/cloud/CLOUD_RECORDER_BEHAVIORAL_CLONING_DESIGN.md.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSession {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub capture_count: i32,
    pub total_bytes: i64,
    #[serde(default)]
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneModel {
    pub id: Uuid,
    pub org_id: Uuid,
    pub workspace_id: Uuid,
    #[serde(default)]
    pub source_session_id: Option<Uuid>,
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub artifact_url: Option<String>,
    #[serde(default)]
    pub metrics: Option<serde_json::Value>,
    #[serde(default)]
    pub runner_seconds: Option<i32>,
    #[serde(default)]
    pub deployed_to: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
impl CaptureSession {
    pub async fn list_by_workspace(pool: &PgPool, workspace_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM capture_sessions WHERE workspace_id = $1 ORDER BY updated_at DESC",
        )
        .bind(workspace_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM capture_sessions WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(
        pool: &PgPool,
        workspace_id: Uuid,
        name: &str,
        description: Option<&str>,
        created_by: Option<Uuid>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO capture_sessions (workspace_id, name, description, created_by)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(workspace_id)
        .bind(name)
        .bind(description)
        .bind(created_by)
        .fetch_one(pool)
        .await
    }

    /// Add a capture to a session. Idempotent (ON CONFLICT DO NOTHING)
    /// + bumps capture_count if a row was inserted.
    pub async fn add_member(
        pool: &PgPool,
        session_id: Uuid,
        capture_id: Uuid,
    ) -> sqlx::Result<bool> {
        let mut tx = pool.begin().await?;
        let inserted = sqlx::query(
            "INSERT INTO capture_session_members (session_id, capture_id) \
             VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(session_id)
        .bind(capture_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if inserted > 0 {
            sqlx::query(
                "UPDATE capture_sessions SET capture_count = capture_count + 1, \
                 updated_at = NOW() WHERE id = $1",
            )
            .bind(session_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(inserted > 0)
    }

    /// Remove a capture. Idempotent on missing rows.
    pub async fn remove_member(
        pool: &PgPool,
        session_id: Uuid,
        capture_id: Uuid,
    ) -> sqlx::Result<bool> {
        let mut tx = pool.begin().await?;
        let removed = sqlx::query(
            "DELETE FROM capture_session_members WHERE session_id = $1 AND capture_id = $2",
        )
        .bind(session_id)
        .bind(capture_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if removed > 0 {
            sqlx::query(
                "UPDATE capture_sessions SET \
                 capture_count = GREATEST(capture_count - 1, 0), \
                 updated_at = NOW() WHERE id = $1",
            )
            .bind(session_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(removed > 0)
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
        let rows = sqlx::query("DELETE FROM capture_sessions WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();
        Ok(rows > 0)
    }
}

#[cfg(feature = "postgres")]
impl CloneModel {
    pub async fn list_by_workspace(pool: &PgPool, workspace_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM clone_models WHERE workspace_id = $1 ORDER BY created_at DESC",
        )
        .bind(workspace_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM clone_models WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Create a clone-model row in `training` state. The training worker
    /// transitions it to `ready` (with artifact_url + metrics + runner_seconds)
    /// once the model is uploaded.
    pub async fn create_training(
        pool: &PgPool,
        org_id: Uuid,
        workspace_id: Uuid,
        source_session_id: Option<Uuid>,
        name: &str,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO clone_models (org_id, workspace_id, source_session_id, name, status)
            VALUES ($1, $2, $3, $4, 'training')
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(workspace_id)
        .bind(source_session_id)
        .bind(name)
        .fetch_one(pool)
        .await
    }

    /// Worker callback. Idempotent: only transitions rows still in
    /// 'training'.
    pub async fn mark_ready(
        pool: &PgPool,
        id: Uuid,
        artifact_url: &str,
        metrics: &serde_json::Value,
        runner_seconds: i32,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE clone_models SET
                status = 'ready',
                artifact_url = $2,
                metrics = $3,
                runner_seconds = $4
            WHERE id = $1 AND status = 'training'
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(artifact_url)
        .bind(metrics)
        .bind(runner_seconds)
        .fetch_optional(pool)
        .await
    }

    pub async fn mark_failed(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "UPDATE clone_models SET status = 'failed' WHERE id = $1 AND status = 'training' \
             RETURNING *",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
        let rows = sqlx::query("DELETE FROM clone_models WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();
        Ok(rows > 0)
    }
}
