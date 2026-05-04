//! Time Travel snapshots (cloud-enablement task #10 / Phase 1).
//!
//! Each row is a workspace snapshot. The actual capture/restore work
//! runs on the #4 Test Execution worker pool with new test_runs.kind
//! values; this module just owns the metadata + storage references.
//!
//! Status lifecycle: `capturing` → `ready` (or `failed`); the retention
//! worker eventually transitions ready snapshots past their `expires_at`
//! to `expired` and reclaims the blob.
//!
//! See docs/cloud/CLOUD_TIME_TRAVEL_DESIGN.md.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: Uuid,
    pub workspace_id: Uuid,
    #[serde(default)]
    pub hosted_deployment_id: Option<Uuid>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub triggered_by: String,
    #[serde(default)]
    pub triggered_by_user: Option<Uuid>,
    pub status: String,
    #[serde(default)]
    pub storage_url: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<i64>,
    #[serde(default)]
    pub manifest: Option<serde_json::Value>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub captured_at: Option<DateTime<Utc>>,
}

#[cfg(feature = "postgres")]
pub struct CreateSnapshot<'a> {
    pub workspace_id: Uuid,
    pub hosted_deployment_id: Option<Uuid>,
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub triggered_by: &'a str,
    pub triggered_by_user: Option<Uuid>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[cfg(feature = "postgres")]
impl Snapshot {
    pub async fn list_by_workspace(
        pool: &PgPool,
        workspace_id: Uuid,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM snapshots WHERE workspace_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(workspace_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM snapshots WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Insert a snapshot row in `capturing` status. The capture worker
    /// transitions it to `ready` (with storage_url + size_bytes + manifest
    /// + captured_at) once the blob is safely uploaded.
    pub async fn create(pool: &PgPool, input: CreateSnapshot<'_>) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO snapshots
                (workspace_id, hosted_deployment_id, name, description,
                 triggered_by, triggered_by_user, status, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, 'capturing', $7)
            RETURNING *
            "#,
        )
        .bind(input.workspace_id)
        .bind(input.hosted_deployment_id)
        .bind(input.name)
        .bind(input.description)
        .bind(input.triggered_by)
        .bind(input.triggered_by_user)
        .bind(input.expires_at)
        .fetch_one(pool)
        .await
    }

    /// Worker callback: snapshot blob is durably stored; transition to
    /// `ready`. Idempotent — only updates rows currently in `capturing`.
    pub async fn mark_ready(
        pool: &PgPool,
        id: Uuid,
        storage_url: &str,
        size_bytes: i64,
        manifest: &serde_json::Value,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE snapshots SET
                status = 'ready',
                storage_url = $2,
                size_bytes = $3,
                manifest = $4,
                captured_at = NOW()
            WHERE id = $1 AND status = 'capturing'
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(storage_url)
        .bind(size_bytes)
        .bind(manifest)
        .fetch_optional(pool)
        .await
    }

    /// Worker callback for the failure path. Idempotent.
    pub async fn mark_failed(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "UPDATE snapshots SET status = 'failed' WHERE id = $1 AND status = 'capturing' \
             RETURNING *",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
        let rows = sqlx::query("DELETE FROM snapshots WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();
        Ok(rows > 0)
    }

    /// Retention worker callback: transition `ready` snapshots whose
    /// `expires_at` has passed to `expired`. Returns the snapshots
    /// affected so the worker can reclaim their blobs after the row
    /// flip lands. Idempotent: rows already in `expired` are skipped.
    pub async fn mark_expired_batch(pool: &PgPool, limit: i64) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE snapshots SET status = 'expired'
             WHERE id IN (
                 SELECT id FROM snapshots
                  WHERE status = 'ready'
                    AND expires_at IS NOT NULL
                    AND expires_at <= NOW()
                  ORDER BY expires_at ASC
                  LIMIT $1
             )
             RETURNING *
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    /// Workspace-scoped count, used for the `max_snapshots` plan-limit
    /// check before allowing a new capture.
    pub async fn count_by_workspace(pool: &PgPool, workspace_id: Uuid) -> sqlx::Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM snapshots WHERE workspace_id = $1")
            .bind(workspace_id)
            .fetch_one(pool)
            .await?;
        Ok(row.0)
    }

    /// Sum of size_bytes for all `ready` snapshots in a workspace —
    /// used by the storage-quota gauge update path.
    pub async fn sum_ready_bytes_by_workspace(
        pool: &PgPool,
        workspace_id: Uuid,
    ) -> sqlx::Result<i64> {
        let row: (Option<i64>,) = sqlx::query_as(
            "SELECT COALESCE(SUM(size_bytes), 0)::BIGINT \
             FROM snapshots WHERE workspace_id = $1 AND status = 'ready'",
        )
        .bind(workspace_id)
        .fetch_one(pool)
        .await?;
        Ok(row.0.unwrap_or(0))
    }
}
