//! Test run domain model — one execution of a TestSuite (cloud-enablement
//! task #4 / Phase 2).
//!
//! Workers (mockforge-test-runner — separate crate, future slice) consume
//! `queued` runs from a Redis queue, transition them through `running` to
//! one of `passed` / `failed` / `cancelled` / `errored`, and stream
//! per-step events into `test_run_events` (see migration
//! 20250101000059_test_execution.sql).
//!
//! This module covers the registry-side state: enqueueing a run, listing
//! recent runs, fetching a single run, and updating status from worker
//! callbacks.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRun {
    pub id: Uuid,
    /// Owning resource id. For suite runs this is `test_suites.id`; for
    /// chaos / flow / snapshot / contract / replay / clone runs it points
    /// at the matching domain table — `kind` says which.
    pub suite_id: Uuid,
    pub org_id: Uuid,
    /// What sort of run this is. Mirrors the `kind` vocabulary in
    /// `test_suites` plus the cross-task additions documented in migration
    /// 20250101000059_test_execution.sql.
    pub kind: String,
    pub triggered_by: String,
    #[serde(default)]
    pub triggered_by_user: Option<Uuid>,
    pub status: String,
    pub queued_at: DateTime<Utc>,
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub finished_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub runner_seconds: Option<i32>,
    #[serde(default)]
    pub summary: Option<serde_json::Value>,
    #[serde(default)]
    pub git_ref: Option<String>,
    #[serde(default)]
    pub git_sha: Option<String>,
}

#[cfg(feature = "postgres")]
pub struct EnqueueTestRun<'a> {
    pub suite_id: Uuid,
    pub org_id: Uuid,
    pub kind: &'a str,
    pub triggered_by: &'a str,
    pub triggered_by_user: Option<Uuid>,
    pub git_ref: Option<&'a str>,
    pub git_sha: Option<&'a str>,
}

/// Counts of inflight runs (queued + running) for an org. Used for the
/// concurrency cap (`max_concurrent_runs` plan limit) before enqueueing.
#[derive(Debug, Clone, Copy)]
pub struct InflightRuns {
    pub queued: i64,
    pub running: i64,
}

impl InflightRuns {
    pub fn total(self) -> i64 {
        self.queued + self.running
    }
}

#[cfg(feature = "postgres")]
impl TestRun {
    /// Insert a new run in `queued` status.
    pub async fn enqueue(pool: &PgPool, input: EnqueueTestRun<'_>) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO test_runs
                (suite_id, org_id, kind, triggered_by, triggered_by_user, status,
                 git_ref, git_sha)
            VALUES ($1, $2, $3, $4, $5, 'queued', $6, $7)
            RETURNING *
            "#,
        )
        .bind(input.suite_id)
        .bind(input.org_id)
        .bind(input.kind)
        .bind(input.triggered_by)
        .bind(input.triggered_by_user)
        .bind(input.git_ref)
        .bind(input.git_sha)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM test_runs WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Recent runs of a suite, newest first.
    pub async fn list_by_suite(
        pool: &PgPool,
        suite_id: Uuid,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM test_runs
            WHERE suite_id = $1
            ORDER BY COALESCE(finished_at, queued_at) DESC
            LIMIT $2
            "#,
        )
        .bind(suite_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    /// Cross-suite list for an org. Used by the global "all runs" dashboard.
    pub async fn list_by_org(
        pool: &PgPool,
        org_id: Uuid,
        status_filter: Option<&str>,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        match status_filter {
            Some(status) => {
                sqlx::query_as::<_, Self>(
                    "SELECT * FROM test_runs WHERE org_id = $1 AND status = $2 \
                 ORDER BY COALESCE(finished_at, queued_at) DESC LIMIT $3",
                )
                .bind(org_id)
                .bind(status)
                .bind(limit)
                .fetch_all(pool)
                .await
            }
            None => {
                sqlx::query_as::<_, Self>(
                    "SELECT * FROM test_runs WHERE org_id = $1 \
                 ORDER BY COALESCE(finished_at, queued_at) DESC LIMIT $2",
                )
                .bind(org_id)
                .bind(limit)
                .fetch_all(pool)
                .await
            }
        }
    }

    /// How many runs are queued + running for this org? Used by the
    /// concurrency-cap check (max_concurrent_runs plan limit) before
    /// admitting a new run.
    pub async fn count_inflight(pool: &PgPool, org_id: Uuid) -> sqlx::Result<InflightRuns> {
        let queued: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM test_runs WHERE org_id = $1 AND status = 'queued'",
        )
        .bind(org_id)
        .fetch_one(pool)
        .await?;
        let running: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM test_runs WHERE org_id = $1 AND status = 'running'",
        )
        .bind(org_id)
        .fetch_one(pool)
        .await?;
        Ok(InflightRuns {
            queued: queued.0,
            running: running.0,
        })
    }

    /// Worker-callback transition to `running`. Idempotent: only
    /// transitions when current status is `queued`, otherwise no-op.
    pub async fn mark_running(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE test_runs SET
                status = 'running',
                started_at = NOW()
            WHERE id = $1 AND status = 'queued'
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// Worker-callback transition to a terminal status. Idempotent on the
    /// terminal: a row already in `passed/failed/cancelled/errored` is not
    /// changed.
    pub async fn mark_finished(
        pool: &PgPool,
        id: Uuid,
        status: &str,
        runner_seconds: i32,
        summary: Option<&serde_json::Value>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE test_runs SET
                status = $2,
                finished_at = NOW(),
                runner_seconds = $3,
                summary = COALESCE($4, summary)
            WHERE id = $1 AND status NOT IN ('passed', 'failed', 'cancelled', 'errored')
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(runner_seconds)
        .bind(summary)
        .fetch_optional(pool)
        .await
    }

    /// User-initiated abort. Allowed from `queued` or `running`.
    pub async fn cancel(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE test_runs SET
                status = 'cancelled',
                finished_at = NOW()
            WHERE id = $1 AND status IN ('queued', 'running')
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }
}

/// Convenience: is `status` one of the terminal values? Used by handlers
/// to decide whether a cancel is even meaningful.
pub fn is_terminal_status(status: &str) -> bool {
    matches!(status, "passed" | "failed" | "cancelled" | "errored")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_terminal_status_matches_canonical() {
        assert!(is_terminal_status("passed"));
        assert!(is_terminal_status("failed"));
        assert!(is_terminal_status("cancelled"));
        assert!(is_terminal_status("errored"));
    }

    #[test]
    fn is_terminal_status_rejects_inflight() {
        assert!(!is_terminal_status("queued"));
        assert!(!is_terminal_status("running"));
        assert!(!is_terminal_status(""));
        assert!(!is_terminal_status("PASSED"));
    }

    #[test]
    fn inflight_total_sums_queued_and_running() {
        let i = InflightRuns {
            queued: 3,
            running: 2,
        };
        assert_eq!(i.total(), 5);
    }
}
