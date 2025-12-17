//! User model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub api_token: Option<String>,
    pub is_verified: bool,
    pub is_admin: bool,
    pub two_factor_enabled: bool,
    #[serde(skip_serializing)]
    pub two_factor_secret: Option<String>, // Base32-encoded TOTP secret
    #[serde(skip_serializing)]
    pub two_factor_backup_codes: Option<Vec<String>>, // Array of hashed backup codes
    pub two_factor_verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Find user by email
    pub async fn find_by_email(pool: &sqlx::PgPool, email: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(pool)
            .await
    }

    /// Find user by username
    pub async fn find_by_username(
        pool: &sqlx::PgPool,
        username: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(pool)
            .await
    }

    /// Find user by ID
    pub async fn find_by_id(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Find multiple users by IDs (batch lookup to avoid N+1 queries)
    pub async fn find_by_ids(pool: &sqlx::PgPool, ids: &[Uuid]) -> sqlx::Result<Vec<Self>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        sqlx::query_as::<_, Self>("SELECT * FROM users WHERE id = ANY($1)")
            .bind(ids)
            .fetch_all(pool)
            .await
    }

    /// Create a new user
    pub async fn create(
        pool: &sqlx::PgPool,
        username: &str,
        email: &str,
        password_hash: &str,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO users (username, email, password_hash)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .fetch_one(pool)
        .await
    }

    /// Set API token
    pub async fn set_api_token(
        pool: &sqlx::PgPool,
        user_id: Uuid,
        token: &str,
    ) -> sqlx::Result<()> {
        sqlx::query("UPDATE users SET api_token = $1 WHERE id = $2")
            .bind(token)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Enable 2FA for a user
    pub async fn enable_2fa(
        pool: &sqlx::PgPool,
        user_id: Uuid,
        secret: &str,
        backup_codes: &[String], // Hashed backup codes
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET two_factor_enabled = TRUE,
                two_factor_secret = $1,
                two_factor_backup_codes = $2,
                two_factor_verified_at = NOW(),
                updated_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(secret)
        .bind(backup_codes)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Disable 2FA for a user
    pub async fn disable_2fa(pool: &sqlx::PgPool, user_id: Uuid) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET two_factor_enabled = FALSE,
                two_factor_secret = NULL,
                two_factor_backup_codes = NULL,
                two_factor_verified_at = NULL,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update 2FA verified timestamp
    pub async fn update_2fa_verified(pool: &sqlx::PgPool, user_id: Uuid) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE users SET two_factor_verified_at = NOW(), updated_at = NOW() WHERE id = $1",
        )
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Remove a used backup code
    pub async fn remove_backup_code(
        pool: &sqlx::PgPool,
        user_id: Uuid,
        code_index: usize,
    ) -> sqlx::Result<()> {
        // Get current backup codes
        let user =
            Self::find_by_id(pool, user_id).await?.ok_or_else(|| sqlx::Error::RowNotFound)?;

        if let Some(mut codes) = user.two_factor_backup_codes {
            if code_index < codes.len() {
                codes.remove(code_index);
                sqlx::query(
                    "UPDATE users SET two_factor_backup_codes = $1, updated_at = NOW() WHERE id = $2",
                )
                .bind(&codes)
                .bind(user_id)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }
}
