//! Waitlist subscriber model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
#[cfg(feature = "postgres")]
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WaitlistSubscriber {
    pub id: Uuid,
    pub email: String,
    pub source: String,
    pub status: String,
    pub unsubscribe_token: Uuid,
    pub created_at: DateTime<Utc>,
    pub unsubscribed_at: Option<DateTime<Utc>>,
}

#[cfg(feature = "postgres")]
impl WaitlistSubscriber {
    /// Subscribe an email. If already subscribed, re-activates silently.
    pub async fn subscribe(pool: &PgPool, email: &str, source: &str) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO waitlist_subscribers (email, source)
            VALUES ($1, $2)
            ON CONFLICT (email) DO UPDATE SET
                status = 'subscribed',
                source = EXCLUDED.source,
                unsubscribed_at = NULL
            RETURNING *
            "#,
        )
        .bind(email)
        .bind(source)
        .fetch_one(pool)
        .await
    }

    /// Unsubscribe by token (public, no auth required).
    pub async fn unsubscribe_by_token(pool: &PgPool, token: Uuid) -> sqlx::Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE waitlist_subscribers
            SET status = 'unsubscribed', unsubscribed_at = NOW()
            WHERE unsubscribe_token = $1 AND status = 'subscribed'
            "#,
        )
        .bind(token)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Count active subscribers.
    pub async fn count_active(pool: &PgPool) -> sqlx::Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM waitlist_subscribers WHERE status = 'subscribed'")
                .fetch_one(pool)
                .await?;
        Ok(row.0)
    }
}
