//! Test execution domain models — `test_suites` table only in this slice.
//!
//! `TestRun`, `TestRunEvent`, `TestSchedule`, `TestRunArtifact` come in
//! follow-up slices once the worker pool exists (mockforge-test-runner).
//! Their schemas already live in migration 20250101000059.
//!
//! See `docs/cloud/CLOUD_TEST_EXECUTION_DESIGN.md` for the full design.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

/// A user-authored test/chaos/scenario suite.
///
/// `kind` is open (TEXT) so additional cloud-enablement tasks can reuse this
/// table without schema migrations:
/// - `unit` | `integration` | `conformance` | `bench` | `owasp` (#4)
/// - `chaos_campaign` (#7)
/// - `behavioral_clone` (#6)
/// - `contract_diff` | `verification_suite` | `fitness_evaluation` (#8)
/// - `scenario` | `orchestration` | `state_machine` | `chain` (#9)
/// - `snapshot_capture` | `snapshot_restore` (#10)
#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuite {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub kind: String,
    pub config: serde_json::Value,
    /// When set, runs target a different workspace (e.g., test the staging
    /// workspace from a CI workspace).
    #[serde(default)]
    pub target_workspace_id: Option<Uuid>,
    #[serde(default)]
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Inputs for `TestSuite::create`. Bundled into a struct because the
/// row has 7 user-supplied columns and the bare-arg version trips
/// clippy's `too_many_arguments` lint.
#[cfg(feature = "postgres")]
pub struct CreateTestSuite<'a> {
    pub workspace_id: Uuid,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub kind: &'a str,
    pub config: &'a serde_json::Value,
    pub target_workspace_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
}

#[cfg(feature = "postgres")]
impl TestSuite {
    /// List all suites in a workspace, optionally filtered by `kind`.
    pub async fn list_by_workspace(
        pool: &PgPool,
        workspace_id: Uuid,
        kind: Option<&str>,
    ) -> sqlx::Result<Vec<Self>> {
        match kind {
            Some(k) => {
                sqlx::query_as::<_, Self>(
                    "SELECT * FROM test_suites WHERE workspace_id = $1 AND kind = $2 \
                 ORDER BY updated_at DESC",
                )
                .bind(workspace_id)
                .bind(k)
                .fetch_all(pool)
                .await
            }
            None => {
                sqlx::query_as::<_, Self>(
                    "SELECT * FROM test_suites WHERE workspace_id = $1 ORDER BY updated_at DESC",
                )
                .bind(workspace_id)
                .fetch_all(pool)
                .await
            }
        }
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM test_suites WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(pool: &PgPool, input: CreateTestSuite<'_>) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO test_suites
                (workspace_id, name, description, kind, config, target_workspace_id, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(input.workspace_id)
        .bind(input.name)
        .bind(input.description)
        .bind(input.kind)
        .bind(input.config)
        .bind(input.target_workspace_id)
        .bind(input.created_by)
        .fetch_one(pool)
        .await
    }

    /// Patch-style update: any `Some(_)` field overwrites; `None` leaves it.
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        name: Option<&str>,
        description: Option<Option<&str>>,
        config: Option<&serde_json::Value>,
        target_workspace_id: Option<Option<Uuid>>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE test_suites SET
                name = COALESCE($2, name),
                description = CASE WHEN $3::bool THEN $4 ELSE description END,
                config = COALESCE($5, config),
                target_workspace_id = CASE WHEN $6::bool THEN $7 ELSE target_workspace_id END,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description.is_some())
        .bind(description.flatten())
        .bind(config)
        .bind(target_workspace_id.is_some())
        .bind(target_workspace_id.flatten())
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
        let rows = sqlx::query("DELETE FROM test_suites WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();
        Ok(rows > 0)
    }
}

/// Cron-driven schedule for a TestSuite. The schedule worker
/// (registry-server::workers::test_schedule_runner) scans this table
/// every minute, identifies rows whose next-fire time has passed, and
/// triggers runs the same way the public POST /test-suites/{id}/runs
/// path does (test_runs row + Redis queue push).
#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSchedule {
    pub id: Uuid,
    pub suite_id: Uuid,
    pub cron: String,
    pub timezone: String,
    pub enabled: bool,
    #[serde(default)]
    pub last_triggered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
impl TestSchedule {
    pub async fn create(
        pool: &PgPool,
        suite_id: Uuid,
        cron: &str,
        timezone: &str,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO test_schedules (suite_id, cron, timezone, enabled)
            VALUES ($1, $2, $3, TRUE)
            RETURNING *
            "#,
        )
        .bind(suite_id)
        .bind(cron)
        .bind(timezone)
        .fetch_one(pool)
        .await
    }

    pub async fn list_by_suite(pool: &PgPool, suite_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM test_schedules WHERE suite_id = $1 ORDER BY created_at",
        )
        .bind(suite_id)
        .fetch_all(pool)
        .await
    }

    /// All enabled schedules. The worker filters in-memory rather than in
    /// SQL because cron-expression evaluation lives in Rust.
    pub async fn list_enabled(pool: &PgPool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM test_schedules WHERE enabled = TRUE")
            .fetch_all(pool)
            .await
    }

    pub async fn set_enabled(pool: &PgPool, id: Uuid, enabled: bool) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "UPDATE test_schedules SET enabled = $2 WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(enabled)
        .fetch_optional(pool)
        .await
    }

    /// Worker-callback: schedule fired, advance the cursor. Idempotent on
    /// the (id, fired_at) pair via the WHERE clause — re-running with an
    /// older timestamp is a no-op so a worker restart won't double-fire.
    pub async fn mark_triggered(
        pool: &PgPool,
        id: Uuid,
        fired_at: DateTime<Utc>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE test_schedules
               SET last_triggered_at = $2
             WHERE id = $1
               AND (last_triggered_at IS NULL OR last_triggered_at < $2)
             RETURNING *
            "#,
        )
        .bind(id)
        .bind(fired_at)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
        let rows = sqlx::query("DELETE FROM test_schedules WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();
        Ok(rows > 0)
    }
}
