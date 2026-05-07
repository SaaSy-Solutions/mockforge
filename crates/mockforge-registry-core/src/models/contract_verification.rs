//! Contract Diff / Verification / Fitness models
//! (cloud-enablement task #8 / Phase 1).
//!
//! Probe runs reuse the #4 worker pool with kind values 'contract_diff'
//! / 'verification_suite' / 'fitness_evaluation'. Drift findings raise
//! incidents through the #3 IncidentBus once integrated.
//!
//! See docs/cloud/CLOUD_CONTRACT_VERIFICATION_DESIGN.md.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoredService {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub openapi_spec_url: Option<String>,
    #[serde(default)]
    pub openapi_spec_inline: Option<serde_json::Value>,
    #[serde(default)]
    pub auth_config: Option<serde_json::Value>,
    pub traffic_source: String,
    #[serde(default)]
    pub traffic_source_ref: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractDiffRun {
    pub id: Uuid,
    pub monitored_service_id: Uuid,
    pub triggered_by: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    #[serde(default)]
    pub finished_at: Option<DateTime<Utc>>,
    pub breaking_changes_count: i32,
    pub non_breaking_changes_count: i32,
    #[serde(default)]
    pub summary: Option<serde_json::Value>,
}

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractDiffFinding {
    pub id: Uuid,
    pub run_id: Uuid,
    pub severity: String,
    pub endpoint: String,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub field_path: Option<String>,
    pub description: String,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub suggested_fix: Option<serde_json::Value>,
}

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessFunction {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub kind: String,
    pub config: serde_json::Value,
    #[serde(default)]
    pub last_evaluated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_status: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationSuite {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub contract_check_ids: Vec<Uuid>,
    pub fitness_function_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
pub struct CreateMonitoredService<'a> {
    pub workspace_id: Uuid,
    pub name: &'a str,
    pub base_url: &'a str,
    pub openapi_spec_url: Option<&'a str>,
    pub openapi_spec_inline: Option<&'a serde_json::Value>,
    pub auth_config: Option<&'a serde_json::Value>,
    pub traffic_source: &'a str,
    pub traffic_source_ref: Option<&'a str>,
}

#[cfg(feature = "postgres")]
impl MonitoredService {
    pub const VALID_TRAFFIC_SOURCES: &'static [&'static str] =
        &["logs", "capture_session", "probe"];

    pub fn is_valid_traffic_source(s: &str) -> bool {
        Self::VALID_TRAFFIC_SOURCES.contains(&s)
    }

    pub async fn list_by_workspace(pool: &PgPool, workspace_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM monitored_services WHERE workspace_id = $1 ORDER BY name",
        )
        .bind(workspace_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM monitored_services WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(pool: &PgPool, input: CreateMonitoredService<'_>) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO monitored_services
                (workspace_id, name, base_url, openapi_spec_url, openapi_spec_inline,
                 auth_config, traffic_source, traffic_source_ref)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(input.workspace_id)
        .bind(input.name)
        .bind(input.base_url)
        .bind(input.openapi_spec_url)
        .bind(input.openapi_spec_inline)
        .bind(input.auth_config)
        .bind(input.traffic_source)
        .bind(input.traffic_source_ref)
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
        let rows = sqlx::query("DELETE FROM monitored_services WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();
        Ok(rows > 0)
    }
}

#[cfg(feature = "postgres")]
impl FitnessFunction {
    pub const VALID_KINDS: &'static [&'static str] = &[
        "latency_threshold",
        "error_rate",
        "contract_stability",
        "custom_query",
    ];

    pub fn is_valid_kind(s: &str) -> bool {
        Self::VALID_KINDS.contains(&s)
    }

    pub async fn list_by_workspace(pool: &PgPool, workspace_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM fitness_functions WHERE workspace_id = $1 ORDER BY name",
        )
        .bind(workspace_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM fitness_functions WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(
        pool: &PgPool,
        workspace_id: Uuid,
        name: &str,
        kind: &str,
        config: &serde_json::Value,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO fitness_functions (workspace_id, name, kind, config)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(workspace_id)
        .bind(name)
        .bind(kind)
        .bind(config)
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
        let rows = sqlx::query("DELETE FROM fitness_functions WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();
        Ok(rows > 0)
    }

    /// Replace the mutable fields (name, kind, config) on an existing
    /// fitness function. Returns `Ok(None)` if the row doesn't exist
    /// rather than erroring — caller can map that to a 404. Bumps
    /// `updated_at` (no DB trigger covers this column on the table).
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        name: &str,
        kind: &str,
        config: &serde_json::Value,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE fitness_functions
            SET name = $2, kind = $3, config = $4, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(kind)
        .bind(config)
        .fetch_optional(pool)
        .await
    }
}

#[cfg(feature = "postgres")]
impl ContractDiffRun {
    pub async fn list_by_service(
        pool: &PgPool,
        service_id: Uuid,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM contract_diff_runs WHERE monitored_service_id = $1 \
             ORDER BY started_at DESC LIMIT $2",
        )
        .bind(service_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM contract_diff_runs WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }
}

#[cfg(feature = "postgres")]
impl ContractDiffFinding {
    pub async fn list_by_run(pool: &PgPool, run_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM contract_diff_findings WHERE run_id = $1 \
             ORDER BY CASE severity \
                 WHEN 'breaking' THEN 0 \
                 WHEN 'non_breaking' THEN 1 \
                 WHEN 'cosmetic' THEN 2 \
                 ELSE 3 END",
        )
        .bind(run_id)
        .fetch_all(pool)
        .await
    }
}

#[cfg(feature = "postgres")]
impl VerificationSuite {
    pub async fn list_by_workspace(pool: &PgPool, workspace_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM verification_suites WHERE workspace_id = $1 ORDER BY name",
        )
        .bind(workspace_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM verification_suites WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(
        pool: &PgPool,
        workspace_id: Uuid,
        name: &str,
        contract_check_ids: &[Uuid],
        fitness_function_ids: &[Uuid],
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO verification_suites
                (workspace_id, name, contract_check_ids, fitness_function_ids)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(workspace_id)
        .bind(name)
        .bind(contract_check_ids)
        .bind(fitness_function_ids)
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
        let rows = sqlx::query("DELETE FROM verification_suites WHERE id = $1")
            .bind(id)
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
    fn traffic_sources_recognized() {
        for s in MonitoredService::VALID_TRAFFIC_SOURCES {
            assert!(MonitoredService::is_valid_traffic_source(s));
        }
        assert!(!MonitoredService::is_valid_traffic_source("WAL"));
    }

    #[test]
    fn fitness_kinds_recognized() {
        for k in FitnessFunction::VALID_KINDS {
            assert!(FitnessFunction::is_valid_kind(k));
        }
        assert!(!FitnessFunction::is_valid_kind("LATENCY_THRESHOLD"));
        assert!(!FitnessFunction::is_valid_kind(""));
    }
}
