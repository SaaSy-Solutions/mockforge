//! Organization invitations (cloud-enablement task #15 / Phase 1).
//!
//! Email-based invites: an admin issues one, the system emails a magic
//! link with a token, and the recipient redeems it to join the org.
//! Tokens are hashed at rest (mirrors api_tokens).
//!
//! See docs/cloud/CLOUD_USER_MANAGEMENT_CONSOLIDATION.md.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgInvitation {
    pub id: Uuid,
    pub org_id: Uuid,
    pub email: String,
    pub role: String,
    /// Hashed at rest. The plaintext is only seen at create-time.
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub token_prefix: String,
    #[serde(default)]
    pub invited_by: Option<Uuid>,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    #[serde(default)]
    pub accepted_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub accepted_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Default invitation lifetime. 7 days mirrors the standard SaaS
/// "click the link before it expires" pattern.
pub const DEFAULT_INVITATION_TTL_DAYS: i64 = 7;

/// Inputs for `OrgInvitation::create`. Bundled into a struct so the
/// 7-column INSERT doesn't trip clippy's too_many_arguments lint.
#[cfg(feature = "postgres")]
pub struct CreateInvitation<'a> {
    pub org_id: Uuid,
    pub email: &'a str,
    pub role: &'a str,
    pub token_hash: &'a str,
    pub token_prefix: &'a str,
    pub invited_by: Option<Uuid>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[cfg(feature = "postgres")]
impl OrgInvitation {
    pub const VALID_ROLES: &'static [&'static str] = &["owner", "admin", "member"];

    pub fn is_valid_role(role: &str) -> bool {
        Self::VALID_ROLES.contains(&role)
    }

    pub async fn list_by_org(pool: &PgPool, org_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM org_invitations WHERE org_id = $1 \
             ORDER BY status = 'pending' DESC, created_at DESC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM org_invitations WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Create an invitation row. Caller computes the token_hash + prefix
    /// (handler does this via `generate_token`). `expires_at` defaults to
    /// now + DEFAULT_INVITATION_TTL_DAYS when None.
    pub async fn create(pool: &PgPool, input: CreateInvitation<'_>) -> sqlx::Result<Self> {
        let expires_at = input
            .expires_at
            .unwrap_or_else(|| Utc::now() + Duration::days(DEFAULT_INVITATION_TTL_DAYS));
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO org_invitations
                (org_id, email, role, token_hash, token_prefix, invited_by, expires_at)
            VALUES ($1, lower($2), $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(input.org_id)
        .bind(input.email)
        .bind(input.role)
        .bind(input.token_hash)
        .bind(input.token_prefix)
        .bind(input.invited_by)
        .bind(expires_at)
        .fetch_one(pool)
        .await
    }

    /// Cancel a pending invitation. Idempotent — already-accepted/cancelled
    /// rows are not changed.
    pub async fn cancel(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "UPDATE org_invitations SET status = 'cancelled', updated_at = NOW() \
             WHERE id = $1 AND status = 'pending' RETURNING *",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// Reissue with a new expiry. Caller provides a fresh token. Only
    /// transitions rows still in 'pending'.
    pub async fn resend(
        pool: &PgPool,
        id: Uuid,
        new_token_hash: &str,
        new_token_prefix: &str,
    ) -> sqlx::Result<Option<Self>> {
        let new_expiry = Utc::now() + Duration::days(DEFAULT_INVITATION_TTL_DAYS);
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE org_invitations SET
                token_hash = $2,
                token_prefix = $3,
                expires_at = $4,
                updated_at = NOW()
            WHERE id = $1 AND status = 'pending'
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(new_token_hash)
        .bind(new_token_prefix)
        .bind(new_expiry)
        .fetch_optional(pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_validation() {
        assert!(OrgInvitation::is_valid_role("owner"));
        assert!(OrgInvitation::is_valid_role("admin"));
        assert!(OrgInvitation::is_valid_role("member"));
        assert!(!OrgInvitation::is_valid_role("ADMIN"));
        assert!(!OrgInvitation::is_valid_role(""));
        assert!(!OrgInvitation::is_valid_role("guest"));
    }
}
