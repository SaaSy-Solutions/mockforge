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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Find user by email
    pub async fn find_by_email(pool: &sqlx::PgPool, email: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM users WHERE email = $1"
        )
        .bind(email)
        .fetch_optional(pool)
        .await
    }

    /// Find user by username
    pub async fn find_by_username(pool: &sqlx::PgPool, username: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM users WHERE username = $1"
        )
        .bind(username)
        .fetch_optional(pool)
        .await
    }

    /// Find user by ID
    pub async fn find_by_id(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM users WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(pool)
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
            "#
        )
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .fetch_one(pool)
        .await
    }

    /// Set API token
    pub async fn set_api_token(pool: &sqlx::PgPool, user_id: Uuid, token: &str) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE users SET api_token = $1 WHERE id = $2"
        )
        .bind(token)
        .bind(user_id)
        .execute(pool)
        .await?;
        Ok(())
    }
}
