//! Unified flows resource (cloud-enablement task #9 / Phase 1).
//!
//! One model, four kinds — `scenario`, `orchestration`, `state_machine`,
//! `chain`. The editor UX differs per kind, but the persistence,
//! versioning, and runs lifecycle are identical, so they share a table.
//! Runs reuse `test_runs` with the matching `kind` value (see #4).
//!
//! Versioning: every save inserts a new `flow_versions` row and updates
//! `flows.current_version_id` in the same transaction. Old versions stay
//! around for rollback.
//!
//! See docs/cloud/CLOUD_SCENARIO_ORCHESTRATION_DESIGN.md.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flow {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub current_version_id: Option<Uuid>,
    pub is_published_to_marketplace: bool,
    #[serde(default)]
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowVersion {
    pub id: Uuid,
    pub flow_id: Uuid,
    pub version_number: i32,
    pub config: serde_json::Value,
    #[serde(default)]
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
pub struct CreateFlow<'a> {
    pub workspace_id: Uuid,
    pub kind: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub config: &'a serde_json::Value,
    pub created_by: Option<Uuid>,
}

#[cfg(feature = "postgres")]
impl Flow {
    /// Recognized `kind` values. Open enum — additions are non-breaking.
    pub const VALID_KINDS: &'static [&'static str] =
        &["scenario", "orchestration", "state_machine", "chain"];

    pub fn is_valid_kind(kind: &str) -> bool {
        Self::VALID_KINDS.contains(&kind)
    }

    /// List flows in a workspace, optionally filtered by kind. Newest
    /// updates first.
    pub async fn list_by_workspace(
        pool: &PgPool,
        workspace_id: Uuid,
        kind: Option<&str>,
    ) -> sqlx::Result<Vec<Self>> {
        match kind {
            Some(k) => {
                sqlx::query_as::<_, Self>(
                    "SELECT * FROM flows WHERE workspace_id = $1 AND kind = $2 \
                 ORDER BY updated_at DESC",
                )
                .bind(workspace_id)
                .bind(k)
                .fetch_all(pool)
                .await
            }
            None => {
                sqlx::query_as::<_, Self>(
                    "SELECT * FROM flows WHERE workspace_id = $1 ORDER BY updated_at DESC",
                )
                .bind(workspace_id)
                .fetch_all(pool)
                .await
            }
        }
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM flows WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Create a flow + its first flow_version in a single transaction.
    /// `flows.current_version_id` is set to the new version's id before
    /// commit so callers always see a valid pointer.
    pub async fn create_with_initial_version(
        pool: &PgPool,
        input: CreateFlow<'_>,
    ) -> sqlx::Result<(Self, FlowVersion)> {
        let mut tx = pool.begin().await?;

        // 1. Insert the flow row (current_version_id is NULL for now).
        let flow: Self = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO flows (workspace_id, kind, name, description, created_by)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(input.workspace_id)
        .bind(input.kind)
        .bind(input.name)
        .bind(input.description)
        .bind(input.created_by)
        .fetch_one(&mut *tx)
        .await?;

        // 2. Insert the first version.
        let version: FlowVersion = sqlx::query_as::<_, FlowVersion>(
            r#"
            INSERT INTO flow_versions (flow_id, version_number, config, created_by)
            VALUES ($1, 1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(flow.id)
        .bind(input.config)
        .bind(input.created_by)
        .fetch_one(&mut *tx)
        .await?;

        // 3. Point the flow at it.
        let flow: Self = sqlx::query_as::<_, Self>(
            "UPDATE flows SET current_version_id = $1 WHERE id = $2 RETURNING *",
        )
        .bind(version.id)
        .bind(flow.id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok((flow, version))
    }

    /// Save a new version. Bumps `version_number` to max+1, inserts the
    /// row, and updates `flows.current_version_id`. Returns the new
    /// version. The old `current_version_id` value stays in
    /// `flow_versions` — that's the rollback target.
    pub async fn save_new_version(
        pool: &PgPool,
        flow_id: Uuid,
        config: &serde_json::Value,
        created_by: Option<Uuid>,
    ) -> sqlx::Result<FlowVersion> {
        let mut tx = pool.begin().await?;

        let next_version: (i32,) = sqlx::query_as(
            "SELECT COALESCE(MAX(version_number), 0) + 1 FROM flow_versions WHERE flow_id = $1",
        )
        .bind(flow_id)
        .fetch_one(&mut *tx)
        .await?;

        let version: FlowVersion = sqlx::query_as::<_, FlowVersion>(
            r#"
            INSERT INTO flow_versions (flow_id, version_number, config, created_by)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(flow_id)
        .bind(next_version.0)
        .bind(config)
        .bind(created_by)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("UPDATE flows SET current_version_id = $1, updated_at = NOW() WHERE id = $2")
            .bind(version.id)
            .bind(flow_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(version)
    }

    pub async fn rename(
        pool: &PgPool,
        id: Uuid,
        name: Option<&str>,
        description: Option<Option<&str>>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE flows SET
                name = COALESCE($2, name),
                description = CASE WHEN $3::bool THEN $4 ELSE description END,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description.is_some())
        .bind(description.flatten())
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
        let rows = sqlx::query("DELETE FROM flows WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();
        Ok(rows > 0)
    }
}

#[cfg(feature = "postgres")]
impl FlowVersion {
    pub async fn list_by_flow(pool: &PgPool, flow_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM flow_versions WHERE flow_id = $1 ORDER BY version_number DESC",
        )
        .bind(flow_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM flow_versions WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_kinds_recognized() {
        assert!(Flow::is_valid_kind("scenario"));
        assert!(Flow::is_valid_kind("orchestration"));
        assert!(Flow::is_valid_kind("state_machine"));
        assert!(Flow::is_valid_kind("chain"));
    }

    #[test]
    fn unknown_kinds_rejected() {
        assert!(!Flow::is_valid_kind(""));
        assert!(!Flow::is_valid_kind("Scenario"));
        assert!(!Flow::is_valid_kind("flow"));
        assert!(!Flow::is_valid_kind("snapshot_capture")); // belongs to #10, not here
    }
}
