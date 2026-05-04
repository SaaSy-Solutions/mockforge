//! Tunnel reservation + session models (cloud-enablement task #5 / Phase 1).
//!
//! Reservations are durable subdomain claims (with optional custom-domain
//! attachment); sessions are the per-connect bandwidth/request roll-ups
//! the relay binary writes back. The relay (separate deployment, future
//! slice) is the only writer to `tunnel_sessions`; the registry CRUDs
//! `tunnel_reservations` and increments `usage_counters.tunnel_bytes_used`
//! based on session reports.
//!
//! Schema: migration 20250101000061_tunnels.sql.
//! Design: docs/cloud/CLOUD_TUNNELS_DESIGN.md.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelReservation {
    pub id: Uuid,
    pub org_id: Uuid,
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
    pub name: String,
    pub subdomain: String,
    #[serde(default)]
    pub custom_domain: Option<String>,
    pub custom_domain_verified: bool,
    #[serde(default)]
    pub custom_domain_verified_at: Option<DateTime<Utc>>,
    pub status: String,
    #[serde(default)]
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
pub struct CreateTunnelReservation<'a> {
    pub org_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub name: &'a str,
    pub subdomain: &'a str,
    pub custom_domain: Option<&'a str>,
    pub created_by: Option<Uuid>,
}

#[cfg(feature = "postgres")]
impl TunnelReservation {
    pub async fn list_by_org(pool: &PgPool, org_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM tunnel_reservations WHERE org_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM tunnel_reservations WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Look up by subdomain. Used by the relay's auth handshake to map
    /// an incoming connection to a reservation row.
    pub async fn find_by_subdomain(pool: &PgPool, subdomain: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM tunnel_reservations WHERE subdomain = $1")
            .bind(subdomain)
            .fetch_optional(pool)
            .await
    }

    /// How many reservations does an org have? Used for the
    /// `max_tunnel_reservations` plan-limit check before insert.
    pub async fn count_by_org(pool: &PgPool, org_id: Uuid) -> sqlx::Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM tunnel_reservations WHERE org_id = $1")
                .bind(org_id)
                .fetch_one(pool)
                .await?;
        Ok(row.0)
    }

    pub async fn create(pool: &PgPool, input: CreateTunnelReservation<'_>) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO tunnel_reservations
                (org_id, workspace_id, name, subdomain, custom_domain, status, created_by)
            VALUES ($1, $2, $3, $4, $5, 'reserved', $6)
            RETURNING *
            "#,
        )
        .bind(input.org_id)
        .bind(input.workspace_id)
        .bind(input.name)
        .bind(input.subdomain)
        .bind(input.custom_domain)
        .bind(input.created_by)
        .fetch_one(pool)
        .await
    }

    /// PATCH-style update.
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        name: Option<&str>,
        custom_domain: Option<Option<&str>>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE tunnel_reservations SET
                name = COALESCE($2, name),
                custom_domain = CASE WHEN $3::bool THEN $4 ELSE custom_domain END,
                custom_domain_verified = CASE WHEN $3::bool THEN FALSE ELSE custom_domain_verified END,
                custom_domain_verified_at = CASE WHEN $3::bool THEN NULL ELSE custom_domain_verified_at END,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(custom_domain.is_some())
        .bind(custom_domain.flatten())
        .fetch_optional(pool)
        .await
    }

    /// Mark a reservation's custom domain as verified. Called after the
    /// DNS proof check passes. Idempotent.
    pub async fn mark_custom_domain_verified(
        pool: &PgPool,
        id: Uuid,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE tunnel_reservations SET
                custom_domain_verified = TRUE,
                custom_domain_verified_at = COALESCE(custom_domain_verified_at, NOW()),
                updated_at = NOW()
            WHERE id = $1 AND custom_domain IS NOT NULL
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
        let rows = sqlx::query("DELETE FROM tunnel_reservations WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();
        Ok(rows > 0)
    }
}

/// Subdomain validity check used by the create handler before hitting the
/// unique-index conflict. Pure function so the rules are testable and
/// shared with any future CLI / SDK validators.
///
/// Rules: 3-40 chars, lowercase ASCII alphanumeric or hyphen, must start
/// and end with an alphanumeric. Lowercase-only matches DNS conventions
/// and avoids case-sensitivity surprises in the relay's host-header
/// lookup.
pub fn is_valid_subdomain(s: &str) -> bool {
    if !(3..=40).contains(&s.len()) {
        return false;
    }
    let bytes = s.as_bytes();
    let is_lower_alnum = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit();
    if !is_lower_alnum(bytes[0]) || !is_lower_alnum(bytes[bytes.len() - 1]) {
        return false;
    }
    s.bytes().all(|b| is_lower_alnum(b) || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subdomain_valid_simple() {
        assert!(is_valid_subdomain("api"));
        assert!(is_valid_subdomain("stage-api"));
        assert!(is_valid_subdomain("ray123"));
        assert!(is_valid_subdomain("a1b2c3"));
    }

    #[test]
    fn subdomain_too_short_or_long() {
        assert!(!is_valid_subdomain("ab")); // <3
        assert!(!is_valid_subdomain(&"a".repeat(41))); // >40
    }

    #[test]
    fn subdomain_must_start_and_end_alphanumeric() {
        assert!(!is_valid_subdomain("-api"));
        assert!(!is_valid_subdomain("api-"));
        assert!(!is_valid_subdomain("-api-"));
    }

    #[test]
    fn subdomain_disallows_special_chars() {
        assert!(!is_valid_subdomain("api.v1"));
        assert!(!is_valid_subdomain("api_v1"));
        assert!(!is_valid_subdomain("api/v1"));
        assert!(!is_valid_subdomain("API")); // uppercase
    }

    #[test]
    fn subdomain_allows_internal_hyphens() {
        assert!(is_valid_subdomain("a-b-c-d"));
        assert!(is_valid_subdomain("staging-api-v2"));
    }
}
