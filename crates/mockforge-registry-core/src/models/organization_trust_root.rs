//! Per-org Ed25519 trust roots for org-private cloud plugin signing.
//!
//! See `docs/plugins/security/cloud-trust-permissions-rfc.md` §7.1.
//! Org-private plugins (not in the public marketplace) must be signed
//! by a key the org admin has registered as a trust root. Revoked
//! roots reject new attaches immediately and existing running plugins
//! fail re-verification on next boot.
//!
//! Schema: migration 20250101000074_cloud_plugin_attachments.sql.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationTrustRoot {
    pub id: Uuid,
    pub org_id: Uuid,
    /// Raw Ed25519 public key bytes (32 bytes).
    pub public_key: Vec<u8>,
    pub name: String,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_reason: Option<String>,
    pub revoked_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
}

#[cfg(feature = "postgres")]
pub struct CreateOrganizationTrustRoot<'a> {
    pub org_id: Uuid,
    pub public_key: &'a [u8],
    pub name: &'a str,
    pub created_by: Option<Uuid>,
}

#[cfg(feature = "postgres")]
impl OrganizationTrustRoot {
    /// Whether this root is currently usable for verifying plugin
    /// signatures. Convenience: matches the partial index
    /// `idx_org_trust_roots_active`.
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }

    pub async fn create(
        pool: &PgPool,
        input: CreateOrganizationTrustRoot<'_>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO organization_trust_roots (org_id, public_key, name, created_by)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(input.org_id)
        .bind(input.public_key)
        .bind(input.name)
        .bind(input.created_by)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM organization_trust_roots WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn list_by_org(pool: &PgPool, org_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM organization_trust_roots WHERE org_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    /// Active (non-revoked) roots only. The plugin-host fetches this
    /// when verifying org-private plugin signatures.
    pub async fn list_active_by_org(pool: &PgPool, org_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM organization_trust_roots
            WHERE org_id = $1 AND revoked_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    pub async fn revoke(
        pool: &PgPool,
        id: Uuid,
        reason: Option<&str>,
        revoked_by: Option<Uuid>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE organization_trust_roots
            SET revoked_at = NOW(),
                revoked_reason = $2,
                revoked_by = $3
            WHERE id = $1 AND revoked_at IS NULL
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(reason)
        .bind(revoked_by)
        .fetch_optional(pool)
        .await
    }
}
