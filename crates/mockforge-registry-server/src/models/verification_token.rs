//! Email verification token model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use base64::Engine as _;

/// Email verification token
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct VerificationToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl VerificationToken {
    /// Create a new verification token
    pub async fn create(
        pool: &sqlx::PgPool,
        user_id: Uuid,
    ) -> sqlx::Result<Self> {
        // Generate random token (must be done before any await to ensure Send)
        use rand::Rng;
        let token_bytes: [u8; 32] = {
            let mut rng = rand::thread_rng();
            rng.gen()
        };
        use base64::engine::general_purpose;
        let token = general_purpose::URL_SAFE_NO_PAD
            .encode(&token_bytes);

        // Token expires in 24 hours
        let expires_at = Utc::now() + chrono::Duration::hours(24);

        // Invalidate any existing tokens for this user
        sqlx::query("UPDATE verification_tokens SET used_at = NOW() WHERE user_id = $1 AND used_at IS NULL")
            .bind(user_id)
            .execute(pool)
            .await?;

        // Create new token
        let verification_token = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO verification_tokens (user_id, token, expires_at)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&token)
        .bind(expires_at)
        .fetch_one(pool)
        .await?;

        Ok(verification_token)
    }

    /// Find token by token string
    pub async fn find_by_token(
        pool: &sqlx::PgPool,
        token: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM verification_tokens WHERE token = $1 AND used_at IS NULL"
        )
        .bind(token)
        .fetch_optional(pool)
        .await
    }

    /// Mark token as used
    pub async fn mark_as_used(
        pool: &sqlx::PgPool,
        token_id: Uuid,
    ) -> sqlx::Result<()> {
        sqlx::query("UPDATE verification_tokens SET used_at = NOW() WHERE id = $1")
            .bind(token_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Check if token is valid (not expired and not used)
    pub fn is_valid(&self) -> bool {
        self.used_at.is_none() && self.expires_at > Utc::now()
    }
}
