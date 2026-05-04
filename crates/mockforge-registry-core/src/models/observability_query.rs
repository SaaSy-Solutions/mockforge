//! Saved observability queries + dashboards (cloud-enablement task #2 / Phase 1).
//!
//! Cross-deployment query handlers themselves come in a follow-up slice;
//! this module owns the persistence layer for users' named filters and
//! dashboard layouts.
//!
//! See docs/cloud/CLOUD_OBSERVABILITY_DESIGN.md.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilitySavedQuery {
    pub id: Uuid,
    pub org_id: Uuid,
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// `logs` | `traces` | `metrics`. Open string so future signal types
    /// don't need a schema change.
    pub kind: String,
    pub filters: serde_json::Value,
    #[serde(default)]
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityDashboard {
    pub id: Uuid,
    pub org_id: Uuid,
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub layout: serde_json::Value,
    pub queries: serde_json::Value,
    #[serde(default)]
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
pub struct CreateSavedQuery<'a> {
    pub org_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub kind: &'a str,
    pub filters: &'a serde_json::Value,
    pub created_by: Option<Uuid>,
}

#[cfg(feature = "postgres")]
pub struct CreateDashboard<'a> {
    pub org_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub layout: &'a serde_json::Value,
    pub queries: &'a serde_json::Value,
    pub created_by: Option<Uuid>,
}

#[cfg(feature = "postgres")]
impl ObservabilitySavedQuery {
    pub const VALID_KINDS: &'static [&'static str] = &["logs", "traces", "metrics"];

    pub fn is_valid_kind(kind: &str) -> bool {
        Self::VALID_KINDS.contains(&kind)
    }

    pub async fn list_by_org(
        pool: &PgPool,
        org_id: Uuid,
        kind: Option<&str>,
    ) -> sqlx::Result<Vec<Self>> {
        match kind {
            Some(k) => {
                sqlx::query_as::<_, Self>(
                    "SELECT * FROM observability_saved_queries \
                 WHERE org_id = $1 AND kind = $2 ORDER BY updated_at DESC",
                )
                .bind(org_id)
                .bind(k)
                .fetch_all(pool)
                .await
            }
            None => {
                sqlx::query_as::<_, Self>(
                    "SELECT * FROM observability_saved_queries \
                 WHERE org_id = $1 ORDER BY updated_at DESC",
                )
                .bind(org_id)
                .fetch_all(pool)
                .await
            }
        }
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM observability_saved_queries WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(pool: &PgPool, input: CreateSavedQuery<'_>) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO observability_saved_queries
                (org_id, workspace_id, name, description, kind, filters, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(input.org_id)
        .bind(input.workspace_id)
        .bind(input.name)
        .bind(input.description)
        .bind(input.kind)
        .bind(input.filters)
        .bind(input.created_by)
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        name: Option<&str>,
        filters: Option<&serde_json::Value>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE observability_saved_queries SET
                name = COALESCE($2, name),
                filters = COALESCE($3, filters),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(filters)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
        let rows = sqlx::query("DELETE FROM observability_saved_queries WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();
        Ok(rows > 0)
    }
}

#[cfg(feature = "postgres")]
impl ObservabilityDashboard {
    pub async fn list_by_org(pool: &PgPool, org_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM observability_dashboards WHERE org_id = $1 ORDER BY updated_at DESC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM observability_dashboards WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(pool: &PgPool, input: CreateDashboard<'_>) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO observability_dashboards
                (org_id, workspace_id, name, description, layout, queries, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(input.org_id)
        .bind(input.workspace_id)
        .bind(input.name)
        .bind(input.description)
        .bind(input.layout)
        .bind(input.queries)
        .bind(input.created_by)
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        name: Option<&str>,
        layout: Option<&serde_json::Value>,
        queries: Option<&serde_json::Value>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE observability_dashboards SET
                name = COALESCE($2, name),
                layout = COALESCE($3, layout),
                queries = COALESCE($4, queries),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(layout)
        .bind(queries)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
        let rows = sqlx::query("DELETE FROM observability_dashboards WHERE id = $1")
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
    fn saved_query_kinds_recognized() {
        assert!(ObservabilitySavedQuery::is_valid_kind("logs"));
        assert!(ObservabilitySavedQuery::is_valid_kind("traces"));
        assert!(ObservabilitySavedQuery::is_valid_kind("metrics"));
    }

    #[test]
    fn saved_query_kinds_rejected() {
        assert!(!ObservabilitySavedQuery::is_valid_kind(""));
        assert!(!ObservabilitySavedQuery::is_valid_kind("LOGS"));
        assert!(!ObservabilitySavedQuery::is_valid_kind("audit"));
    }
}
