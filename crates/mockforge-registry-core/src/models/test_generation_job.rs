//! Cloud Test Generator async-job rows.
//!
//! Backs `/api/v1/workspaces/{id}/test-generation/jobs` and friends (#469).
//! Phase 1 ships the data plane only — rows are created in 'queued' state
//! and stay there until Phase 2 wires the background worker that calls the
//! org's BYOK LLM provider.
//!
//! Schema: migration `20250101000076_test_generation_jobs.sql`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

/// Terminal states a job can finish in. Stored in DB as the raw string
/// values via `status: String`; this enum mirrors them for client code
/// that wants exhaustive matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestGenerationJobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl TestGenerationJobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestGenerationJob {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub org_id: Uuid,
    /// One of `TestGenerationJobStatus`'s string forms.
    pub status: String,
    pub prompt: String,
    pub captures_filter: serde_json::Value,
    /// Populated once status = 'succeeded'.
    pub result: Option<serde_json::Value>,
    /// Populated once status = 'failed' or 'cancelled'.
    pub error: Option<String>,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
}

/// Insert payload for [`TestGenerationJob::create`].
#[cfg(feature = "postgres")]
pub struct CreateTestGenerationJob<'a> {
    pub workspace_id: Uuid,
    pub org_id: Uuid,
    pub prompt: &'a str,
    pub captures_filter: &'a serde_json::Value,
    pub created_by: Option<Uuid>,
}

#[cfg(feature = "postgres")]
impl TestGenerationJob {
    /// Create a new job in 'queued' state. The Phase 2 worker is
    /// responsible for transitioning it through 'running' → terminal.
    pub async fn create(pool: &PgPool, input: CreateTestGenerationJob<'_>) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO cloud_test_generation_jobs
                (workspace_id, org_id, prompt, captures_filter, created_by)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, workspace_id, org_id, status, prompt, captures_filter,
                      result, error, queued_at, started_at, finished_at, created_by
            "#,
        )
        .bind(input.workspace_id)
        .bind(input.org_id)
        .bind(input.prompt)
        .bind(input.captures_filter)
        .bind(input.created_by)
        .fetch_one(pool)
        .await
    }

    /// Look up a single job by id, scoped to the caller's workspace so
    /// cross-workspace IDs return None even when the row exists.
    pub async fn find_in_workspace(
        pool: &PgPool,
        workspace_id: Uuid,
        job_id: Uuid,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT id, workspace_id, org_id, status, prompt, captures_filter,
                   result, error, queued_at, started_at, finished_at, created_by
            FROM cloud_test_generation_jobs
            WHERE id = $1 AND workspace_id = $2
            "#,
        )
        .bind(job_id)
        .bind(workspace_id)
        .fetch_optional(pool)
        .await
    }

    /// List jobs for a workspace, newest first. `limit` caps the page
    /// size — callers should pass a sane upper bound (the handler caps
    /// at 100).
    pub async fn list_by_workspace(
        pool: &PgPool,
        workspace_id: Uuid,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT id, workspace_id, org_id, status, prompt, captures_filter,
                   result, error, queued_at, started_at, finished_at, created_by
            FROM cloud_test_generation_jobs
            WHERE workspace_id = $1
            ORDER BY queued_at DESC
            LIMIT $2
            "#,
        )
        .bind(workspace_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    /// Atomically claim the oldest queued job, flipping it to 'running'.
    /// Returns the claimed row, or `None` if no job is queued.
    ///
    /// Uses `FOR UPDATE SKIP LOCKED` so concurrent workers (a future
    /// scale-out won't claim the same row) skip rather than block on a
    /// locked candidate. Phase 3 only runs a single worker process per
    /// registry pod, but the locking pattern future-proofs us.
    pub async fn claim_next_queued(pool: &PgPool) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE cloud_test_generation_jobs
            SET status = 'running', started_at = NOW()
            WHERE id = (
                SELECT id FROM cloud_test_generation_jobs
                WHERE status = 'queued'
                ORDER BY queued_at ASC
                LIMIT 1
                FOR UPDATE SKIP LOCKED
            )
            RETURNING id, workspace_id, org_id, status, prompt, captures_filter,
                      result, error, queued_at, started_at, finished_at, created_by
            "#,
        )
        .fetch_optional(pool)
        .await
    }

    /// Persist a successful generation result and flip status to
    /// 'succeeded'. No-op if the job is no longer in 'running' state
    /// (e.g., the user cancelled while the worker was mid-flight) — the
    /// rows_affected check makes this safe under that race.
    pub async fn complete_success(
        pool: &PgPool,
        job_id: Uuid,
        result: &serde_json::Value,
    ) -> sqlx::Result<bool> {
        let rows = sqlx::query(
            r#"
            UPDATE cloud_test_generation_jobs
            SET status = 'succeeded',
                result = $2,
                finished_at = NOW()
            WHERE id = $1 AND status = 'running'
            "#,
        )
        .bind(job_id)
        .bind(result)
        .execute(pool)
        .await?
        .rows_affected();
        Ok(rows > 0)
    }

    /// Persist a failure reason and flip status to 'failed'. Same
    /// no-op-on-race semantics as `complete_success`.
    pub async fn complete_failure(pool: &PgPool, job_id: Uuid, error: &str) -> sqlx::Result<bool> {
        let rows = sqlx::query(
            r#"
            UPDATE cloud_test_generation_jobs
            SET status = 'failed',
                error = $2,
                finished_at = NOW()
            WHERE id = $1 AND status = 'running'
            "#,
        )
        .bind(job_id)
        .bind(error)
        .execute(pool)
        .await?
        .rows_affected();
        Ok(rows > 0)
    }

    /// Cancel a queued/running job. No-op if the job is already terminal.
    /// Returns Ok(true) on a state change, Ok(false) if the job was
    /// already terminal or not found.
    pub async fn cancel(pool: &PgPool, workspace_id: Uuid, job_id: Uuid) -> sqlx::Result<bool> {
        let rows = sqlx::query(
            r#"
            UPDATE cloud_test_generation_jobs
            SET status = 'cancelled',
                finished_at = NOW(),
                error = 'Cancelled by user'
            WHERE id = $1
              AND workspace_id = $2
              AND status IN ('queued', 'running')
            "#,
        )
        .bind(job_id)
        .bind(workspace_id)
        .execute(pool)
        .await?
        .rows_affected();
        Ok(rows > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_round_trips_via_serde() {
        for s in [
            TestGenerationJobStatus::Queued,
            TestGenerationJobStatus::Running,
            TestGenerationJobStatus::Succeeded,
            TestGenerationJobStatus::Failed,
            TestGenerationJobStatus::Cancelled,
        ] {
            let json = serde_json::to_string(&s).unwrap();
            let back: TestGenerationJobStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(s, back, "round-trip failed for {s:?}");
            // serde lowercase matches DB string form.
            assert_eq!(json.trim_matches('"'), s.as_str());
        }
    }

    #[test]
    fn is_terminal_matches_expected_states() {
        assert!(!TestGenerationJobStatus::Queued.is_terminal());
        assert!(!TestGenerationJobStatus::Running.is_terminal());
        assert!(TestGenerationJobStatus::Succeeded.is_terminal());
        assert!(TestGenerationJobStatus::Failed.is_terminal());
        assert!(TestGenerationJobStatus::Cancelled.is_terminal());
    }
}
