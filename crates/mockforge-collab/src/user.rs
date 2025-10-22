//! User management service

use crate::auth::AuthService;
use crate::error::{CollabError, Result};
use crate::models::User;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use uuid::Uuid;

/// User service for managing user accounts
pub struct UserService {
    db: Pool<Sqlite>,
    auth: Arc<AuthService>,
}

impl UserService {
    /// Create a new user service
    pub fn new(db: Pool<Sqlite>, auth: Arc<AuthService>) -> Self {
        Self { db, auth }
    }

    /// Create a new user account
    pub async fn create_user(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<User> {
        // Validate input
        if username.is_empty() || email.is_empty() || password.is_empty() {
            return Err(CollabError::InvalidInput(
                "Username, email, and password are required".to_string(),
            ));
        }

        // Check if username already exists
        let existing = sqlx::query!(
            r#"SELECT COUNT(*) as count FROM users WHERE username = ? OR email = ?"#,
            username,
            email
        )
        .fetch_one(&self.db)
        .await?;

        if existing.count > 0 {
            return Err(CollabError::AlreadyExists("Username or email already exists".to_string()));
        }

        // Hash password
        let password_hash = self.auth.hash_password(&password)?;

        // Create user
        let user = User::new(username, email, password_hash);

        // Insert into database
        sqlx::query!(
            r#"
            INSERT INTO users (id, username, email, password_hash, display_name, avatar_url, created_at, updated_at, is_active)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            user.id,
            user.username,
            user.email,
            user.password_hash,
            user.display_name,
            user.avatar_url,
            user.created_at,
            user.updated_at,
            user.is_active
        )
        .execute(&self.db)
        .await?;

        Ok(user)
    }

    /// Authenticate a user and return user if valid
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<User> {
        // Fetch user by username or email
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id as "id: Uuid", username, email, password_hash, display_name, avatar_url,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>",
                   updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
                   is_active as "is_active: bool"
            FROM users
            WHERE (username = ? OR email = ?) AND is_active = TRUE
            "#,
            username,
            username
        )
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| CollabError::AuthenticationFailed("Invalid credentials".to_string()))?;

        // Verify password
        if !self.auth.verify_password(password, &user.password_hash)? {
            return Err(CollabError::AuthenticationFailed("Invalid credentials".to_string()));
        }

        Ok(user)
    }

    /// Get user by ID
    pub async fn get_user(&self, user_id: Uuid) -> Result<User> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id as "id: Uuid", username, email, password_hash, display_name, avatar_url,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>",
                   updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
                   is_active as "is_active: bool"
            FROM users
            WHERE id = ?
            "#,
            user_id
        )
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| CollabError::UserNotFound(user_id.to_string()))?;

        Ok(user)
    }

    /// Get user by username
    pub async fn get_user_by_username(&self, username: &str) -> Result<User> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id as "id: Uuid", username, email, password_hash, display_name, avatar_url,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>",
                   updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
                   is_active as "is_active: bool"
            FROM users
            WHERE username = ?
            "#,
            username
        )
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| CollabError::UserNotFound(username.to_string()))?;

        Ok(user)
    }

    /// Update user profile
    pub async fn update_user(
        &self,
        user_id: Uuid,
        display_name: Option<String>,
        avatar_url: Option<String>,
    ) -> Result<User> {
        let now = chrono::Utc::now();

        sqlx::query!(
            r#"
            UPDATE users
            SET display_name = COALESCE(?, display_name),
                avatar_url = COALESCE(?, avatar_url),
                updated_at = ?
            WHERE id = ?
            "#,
            display_name,
            avatar_url,
            now,
            user_id
        )
        .execute(&self.db)
        .await?;

        self.get_user(user_id).await
    }

    /// Change user password
    pub async fn change_password(
        &self,
        user_id: Uuid,
        old_password: &str,
        new_password: &str,
    ) -> Result<()> {
        // Get user
        let user = self.get_user(user_id).await?;

        // Verify old password
        if !self.auth.verify_password(old_password, &user.password_hash)? {
            return Err(CollabError::AuthenticationFailed("Invalid old password".to_string()));
        }

        // Hash new password
        let new_hash = self.auth.hash_password(new_password)?;

        // Update password
        sqlx::query!(
            r#"UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?"#,
            new_hash,
            chrono::Utc::now(),
            user_id
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Deactivate user account
    pub async fn deactivate_user(&self, user_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"UPDATE users SET is_active = FALSE, updated_at = ? WHERE id = ?"#,
            chrono::Utc::now(),
            user_id
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a database setup
    // They serve as documentation of the API
}
