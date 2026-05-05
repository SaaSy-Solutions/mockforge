//! Plugin attachment to a hosted mock deployment.
//!
//! Each row says: "plugin P at version V is attached to deployment D
//! with permissions G and config C." The plugin-host fetches the
//! enabled rows for its deployment on boot and on manifest reload.
//!
//! The permission grant payload (`permissions_json`) follows the
//! shape defined in `docs/plugins/security/cloud-trust-permissions-rfc.md`
//! §4.2 — strawman:
//!
//! ```json
//! {
//!   "egress":   { "allow": ["*.stripe.com"], "deny_all_others": true },
//!   "env":      { "read": ["MY_PUBLIC_FLAG"] },
//!   "request":  { "read_body": true, "modify_body": true,
//!                 "read_headers": ["x-trace-id"],
//!                 "modify_headers": ["x-rewritten-by"] },
//!   "response": { "read_body": true, "modify_body": true,
//!                 "modify_status": false },
//!   "storage":  { "kv_namespace": null }
//! }
//! ```
//!
//! Default is deny-all (empty object). The handler that creates the
//! row validates the grant against the manifest's declared
//! capabilities — the runtime enforces `manifest ∩ grant`.
//!
//! Schema: migration 20250101000074_cloud_plugin_attachments.sql.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostedMockPlugin {
    pub id: Uuid,
    pub deployment_id: Uuid,
    pub plugin_id: Uuid,
    pub plugin_version_id: Uuid,
    pub config_json: serde_json::Value,
    pub permissions_json: serde_json::Value,
    pub enabled: bool,
    pub attached_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub attached_by: Option<Uuid>,
}

#[cfg(feature = "postgres")]
pub struct AttachHostedMockPlugin<'a> {
    pub deployment_id: Uuid,
    pub plugin_id: Uuid,
    pub plugin_version_id: Uuid,
    pub config_json: &'a serde_json::Value,
    pub permissions_json: &'a serde_json::Value,
    pub enabled: bool,
    pub attached_by: Option<Uuid>,
}

#[cfg(feature = "postgres")]
impl HostedMockPlugin {
    /// Attach (or re-attach) a plugin to a deployment. UPSERT on the
    /// `(deployment_id, plugin_id)` UNIQUE constraint — re-attach of
    /// the same plugin updates the version, config, and grant.
    pub async fn attach(pool: &PgPool, input: AttachHostedMockPlugin<'_>) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO hosted_mock_plugins (
                deployment_id, plugin_id, plugin_version_id,
                config_json, permissions_json, enabled, attached_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (deployment_id, plugin_id) DO UPDATE SET
                plugin_version_id = EXCLUDED.plugin_version_id,
                config_json = EXCLUDED.config_json,
                permissions_json = EXCLUDED.permissions_json,
                enabled = EXCLUDED.enabled,
                attached_by = EXCLUDED.attached_by,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(input.deployment_id)
        .bind(input.plugin_id)
        .bind(input.plugin_version_id)
        .bind(input.config_json)
        .bind(input.permissions_json)
        .bind(input.enabled)
        .bind(input.attached_by)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM hosted_mock_plugins WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn list_by_deployment(pool: &PgPool, deployment_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM hosted_mock_plugins
            WHERE deployment_id = $1
            ORDER BY attached_at ASC
            "#,
        )
        .bind(deployment_id)
        .fetch_all(pool)
        .await
    }

    /// Enabled-only listing. The plugin-host calls this on boot to
    /// build its load manifest.
    pub async fn list_enabled_by_deployment(
        pool: &PgPool,
        deployment_id: Uuid,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM hosted_mock_plugins
            WHERE deployment_id = $1 AND enabled = TRUE
            ORDER BY attached_at ASC
            "#,
        )
        .bind(deployment_id)
        .fetch_all(pool)
        .await
    }

    pub async fn count_active_by_deployment(
        pool: &PgPool,
        deployment_id: Uuid,
    ) -> sqlx::Result<i64> {
        let row: (Option<i64>,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::BIGINT
            FROM hosted_mock_plugins
            WHERE deployment_id = $1 AND enabled = TRUE
            "#,
        )
        .bind(deployment_id)
        .fetch_one(pool)
        .await?;
        Ok(row.0.unwrap_or(0))
    }

    /// Soft toggle. Detach (hard delete) is `delete`.
    pub async fn set_enabled(pool: &PgPool, id: Uuid, enabled: bool) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE hosted_mock_plugins
            SET enabled = $2,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(enabled)
        .fetch_optional(pool)
        .await
    }

    /// Hard detach. Audit trail is preserved separately via
    /// `audit_logs` (event type `plugin_detached`); this row goes away.
    pub async fn delete(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
        let rows = sqlx::query("DELETE FROM hosted_mock_plugins WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();
        Ok(rows > 0)
    }
}
