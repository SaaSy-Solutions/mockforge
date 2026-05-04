//! Cloud recorder proxy — Phase 5 of the cloud-runs roadmap.
//!
//! A `CloudProxySession` is a live forwarding endpoint pinned to one
//! upstream URL. The `session_token` (a long random string) goes in
//! the proxy URL — it's the only auth incoming traffic carries, so the
//! token must be treated as a secret.
//!
//! Each request the proxy forwards lands in `cloud_proxy_captures` for
//! later inspection. Bodies are capped at 1 MB; larger bodies are
//! truncated and flagged.
//!
//! See `docs/cloud/CLOUD_RECORDER_BEHAVIORAL_CLONING_DESIGN.md` for the
//! upstream design context (this module adds a third capture source
//! alongside the existing 'hosted' and 'local' sources in
//! `runtime_captures`).

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudProxySession {
    pub id: Uuid,
    pub org_id: Uuid,
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
    /// Opaque secret embedded in the proxy URL. Treat as sensitive.
    pub session_token: String,
    pub upstream_url: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    #[serde(default)]
    pub revoked_at: Option<DateTime<Utc>>,
    pub capture_count: i64,
    pub total_bytes: i64,
}

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudProxyCapture {
    pub id: i64,
    pub session_id: Uuid,
    pub org_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub query_string: Option<String>,
    pub request_headers: String,
    #[serde(default)]
    pub request_body: Option<String>,
    pub request_body_encoding: String,
    pub request_body_truncated: bool,
    pub request_size_bytes: i64,
    #[serde(default)]
    pub response_status: Option<i32>,
    #[serde(default)]
    pub response_headers: Option<String>,
    #[serde(default)]
    pub response_body: Option<String>,
    #[serde(default)]
    pub response_body_encoding: Option<String>,
    pub response_body_truncated: bool,
    #[serde(default)]
    pub response_size_bytes: Option<i64>,
    pub duration_ms: i64,
    #[serde(default)]
    pub upstream_error: Option<String>,
    #[serde(default)]
    pub client_ip: Option<String>,
}

/// Maximum request/response body bytes the proxy will persist. Beyond
/// this, the body is truncated and `*_truncated` is set true. Keeping
/// this conservative — Postgres TEXT can hold more, but we don't want
/// proxy traffic to balloon the table to GBs.
pub const PROXY_BODY_MAX_BYTES: usize = 1024 * 1024;

/// Default session lifetime when the caller doesn't override it.
pub const DEFAULT_SESSION_TTL_HOURS: i64 = 24;

/// Hard upper bound on session lifetime — keeps stale tokens from
/// hanging around indefinitely.
pub const MAX_SESSION_TTL_HOURS: i64 = 24 * 7;

/// Generate a cryptographically random session token. 32 bytes of
/// entropy, encoded URL-safe — fits in a path segment.
pub fn generate_session_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    // Hex is 64 chars — safe for URL paths and easy to copy/paste.
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(feature = "postgres")]
pub struct CreateCloudProxySession<'a> {
    pub org_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub upstream_url: &'a str,
    pub name: Option<&'a str>,
    pub created_by: Option<Uuid>,
    pub ttl_hours: i64,
}

#[cfg(feature = "postgres")]
impl CloudProxySession {
    pub async fn create(pool: &PgPool, input: CreateCloudProxySession<'_>) -> sqlx::Result<Self> {
        let token = generate_session_token();
        let ttl = input.ttl_hours.clamp(1, MAX_SESSION_TTL_HOURS);
        let expires_at = Utc::now() + Duration::hours(ttl);

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO cloud_proxy_sessions
                (org_id, workspace_id, session_token, upstream_url, name, created_by, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(input.org_id)
        .bind(input.workspace_id)
        .bind(&token)
        .bind(input.upstream_url)
        .bind(input.name)
        .bind(input.created_by)
        .bind(expires_at)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM cloud_proxy_sessions WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Look up by the public-facing token. Filters out revoked rows so
    /// the proxy handler can treat `None` as "the token is no longer
    /// valid" — expired sessions still return Some so the handler can
    /// emit a clear 410 Gone with the expiry timestamp.
    pub async fn find_by_token(pool: &PgPool, token: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM cloud_proxy_sessions WHERE session_token = $1 AND revoked_at IS NULL",
        )
        .bind(token)
        .fetch_optional(pool)
        .await
    }

    /// List active (non-revoked) sessions for an org, newest first.
    pub async fn list_for_org(pool: &PgPool, org_id: Uuid, limit: i64) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM cloud_proxy_sessions
            WHERE org_id = $1 AND revoked_at IS NULL
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(org_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    /// Mark a session revoked. The row is preserved so capture history
    /// stays queryable; future proxy traffic with this token returns
    /// 410 Gone.
    pub async fn revoke(pool: &PgPool, id: Uuid, org_id: Uuid) -> sqlx::Result<bool> {
        let res = sqlx::query(
            r#"
            UPDATE cloud_proxy_sessions
               SET revoked_at = NOW()
             WHERE id = $1 AND org_id = $2 AND revoked_at IS NULL
            "#,
        )
        .bind(id)
        .bind(org_id)
        .execute(pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    /// Bump cached counters after a capture is persisted. Called from
    /// the proxy handler — invariant maintained: rows in
    /// cloud_proxy_captures with this session_id sum to capture_count
    /// and total_bytes. Best-effort; a missed update doesn't break the
    /// proxy itself.
    pub async fn record_capture(pool: &PgPool, id: Uuid, bytes: i64) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            UPDATE cloud_proxy_sessions
               SET capture_count = capture_count + 1,
                   total_bytes  = total_bytes + $2
             WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(bytes)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none() && self.expires_at > Utc::now()
    }
}

#[cfg(feature = "postgres")]
pub struct InsertCloudProxyCapture<'a> {
    pub session_id: Uuid,
    pub org_id: Uuid,
    pub method: &'a str,
    pub path: &'a str,
    pub query_string: Option<&'a str>,
    pub request_headers: &'a str,
    pub request_body: Option<&'a str>,
    pub request_body_encoding: &'a str,
    pub request_body_truncated: bool,
    pub request_size_bytes: i64,
    pub response_status: Option<i32>,
    pub response_headers: Option<&'a str>,
    pub response_body: Option<&'a str>,
    pub response_body_encoding: Option<&'a str>,
    pub response_body_truncated: bool,
    pub response_size_bytes: Option<i64>,
    pub duration_ms: i64,
    pub upstream_error: Option<&'a str>,
    pub client_ip: Option<&'a str>,
}

#[cfg(feature = "postgres")]
impl CloudProxyCapture {
    pub async fn insert(pool: &PgPool, input: InsertCloudProxyCapture<'_>) -> sqlx::Result<i64> {
        let row: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO cloud_proxy_captures (
                session_id, org_id, method, path, query_string,
                request_headers, request_body, request_body_encoding,
                request_body_truncated, request_size_bytes,
                response_status, response_headers, response_body,
                response_body_encoding, response_body_truncated, response_size_bytes,
                duration_ms, upstream_error, client_ip
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19
            )
            RETURNING id
            "#,
        )
        .bind(input.session_id)
        .bind(input.org_id)
        .bind(input.method)
        .bind(input.path)
        .bind(input.query_string)
        .bind(input.request_headers)
        .bind(input.request_body)
        .bind(input.request_body_encoding)
        .bind(input.request_body_truncated)
        .bind(input.request_size_bytes)
        .bind(input.response_status)
        .bind(input.response_headers)
        .bind(input.response_body)
        .bind(input.response_body_encoding)
        .bind(input.response_body_truncated)
        .bind(input.response_size_bytes)
        .bind(input.duration_ms)
        .bind(input.upstream_error)
        .bind(input.client_ip)
        .fetch_one(pool)
        .await?;
        Ok(row.0)
    }

    pub async fn list_for_session(
        pool: &PgPool,
        session_id: Uuid,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM cloud_proxy_captures
            WHERE session_id = $1
            ORDER BY occurred_at DESC
            LIMIT $2
            "#,
        )
        .bind(session_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_token_is_deterministic_length() {
        // 32 bytes hex-encoded = 64 chars.
        for _ in 0..50 {
            let t = generate_session_token();
            assert_eq!(t.len(), 64);
            assert!(t.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn generated_tokens_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..1000 {
            assert!(seen.insert(generate_session_token()));
        }
    }
}
