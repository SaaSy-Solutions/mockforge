//! Login attempt tracking for rate limiting

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Login attempt record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LoginAttempt {
    pub id: Uuid,
    pub email: String,
    pub ip_address: Option<String>,
    pub success: bool,
    pub created_at: DateTime<Utc>,
}

impl LoginAttempt {
    /// Record a login attempt
    pub async fn record(
        pool: &sqlx::PgPool,
        email: &str,
        ip_address: Option<&str>,
        success: bool,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO login_attempts (email, ip_address, success)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(email)
        .bind(ip_address)
        .bind(success)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Count failed login attempts in the last N minutes
    pub async fn count_recent_failures(
        pool: &sqlx::PgPool,
        email: &str,
        minutes: i64,
    ) -> sqlx::Result<i64> {
        let since = Utc::now() - chrono::Duration::minutes(minutes);
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM login_attempts
            WHERE email = $1 AND success = FALSE AND created_at > $2
            "#,
        )
        .bind(email)
        .bind(since)
        .fetch_one(pool)
        .await?;
        Ok(count.0)
    }

    /// Count failed login attempts by IP in the last N minutes
    pub async fn count_recent_failures_by_ip(
        pool: &sqlx::PgPool,
        ip_address: &str,
        minutes: i64,
    ) -> sqlx::Result<i64> {
        let since = Utc::now() - chrono::Duration::minutes(minutes);
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM login_attempts
            WHERE ip_address = $1 AND success = FALSE AND created_at > $2
            "#,
        )
        .bind(ip_address)
        .bind(since)
        .fetch_one(pool)
        .await?;
        Ok(count.0)
    }

    /// Check if account should be locked (too many failed attempts)
    pub async fn is_locked(
        pool: &sqlx::PgPool,
        email: &str,
        max_attempts: i64,
        lockout_minutes: i64,
    ) -> sqlx::Result<bool> {
        let failures = Self::count_recent_failures(pool, email, lockout_minutes).await?;
        Ok(failures >= max_attempts)
    }

    /// Clean up old login attempts (older than N days)
    pub async fn cleanup_old(
        pool: &sqlx::PgPool,
        days: i64,
    ) -> sqlx::Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let result = sqlx::query("DELETE FROM login_attempts WHERE created_at < $1")
            .bind(cutoff)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}
