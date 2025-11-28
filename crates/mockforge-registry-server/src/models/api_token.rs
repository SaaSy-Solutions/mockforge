//! API Token (Personal Access Token) model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// API Token scopes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenScope {
    ReadPackages,
    PublishPackages,
    DeployMocks,
    AdminOrg,
    ReadUsage,
    ManageBilling,
}

impl TokenScope {
    pub fn to_string(&self) -> String {
        match self {
            TokenScope::ReadPackages => "read:packages".to_string(),
            TokenScope::PublishPackages => "publish:packages".to_string(),
            TokenScope::DeployMocks => "deploy:mocks".to_string(),
            TokenScope::AdminOrg => "admin:org".to_string(),
            TokenScope::ReadUsage => "read:usage".to_string(),
            TokenScope::ManageBilling => "manage:billing".to_string(),
        }
    }

    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "read:packages" => Some(TokenScope::ReadPackages),
            "publish:packages" => Some(TokenScope::PublishPackages),
            "deploy:mocks" => Some(TokenScope::DeployMocks),
            "admin:org" => Some(TokenScope::AdminOrg),
            "read:usage" => Some(TokenScope::ReadUsage),
            "manage:billing" => Some(TokenScope::ManageBilling),
            _ => None,
        }
    }
}

/// API Token model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ApiToken {
    pub id: Uuid,
    pub org_id: Uuid,
    pub user_id: Option<Uuid>,
    pub name: String,
    pub token_prefix: String, // First 8 chars for display
    pub hashed_token: String,
    pub scopes: Vec<String>, // Stored as TEXT[]
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ApiToken {
    /// Create a new API token
    /// Returns the full token (only shown once) and the ApiToken record
    pub async fn create(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        user_id: Option<Uuid>,
        name: &str,
        scopes: &[TokenScope],
        expires_at: Option<DateTime<Utc>>,
    ) -> sqlx::Result<(String, Self)> {
        // Generate token: mfx_<random_base64>
        use rand::Rng;
        use base64::{Engine as _, engine::general_purpose};
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        // Use base64 crate version 0.21 API
        let token_suffix = general_purpose::STANDARD.encode(&random_bytes);
        let full_token = format!("mfx_{}", token_suffix);
        let token_prefix = full_token.chars().take(12).collect::<String>(); // "mfx_" + 8 chars

        // Hash the token
        let hashed_token = bcrypt::hash(&full_token, bcrypt::DEFAULT_COST)
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to hash token: {}", e).into()))?;

        // Convert scopes to strings
        let scope_strings: Vec<String> = scopes.iter().map(|s| s.to_string()).collect();

        let token = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO api_tokens (org_id, user_id, name, token_prefix, hashed_token, scopes, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(name)
        .bind(&token_prefix)
        .bind(&hashed_token)
        .bind(&scope_strings)
        .bind(expires_at)
        .fetch_one(pool)
        .await?;

        Ok((full_token, token))
    }

    /// Find token by prefix (for listing)
    pub async fn find_by_prefix(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        prefix: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM api_tokens WHERE org_id = $1 AND token_prefix = $2",
        )
        .bind(org_id)
        .bind(prefix)
        .fetch_optional(pool)
        .await
    }

    /// Find token by ID
    pub async fn find_by_id(pool: &sqlx::PgPool, token_id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM api_tokens WHERE id = $1")
            .bind(token_id)
            .fetch_optional(pool)
            .await
    }

    /// Get all tokens for an organization
    pub async fn find_by_org(pool: &sqlx::PgPool, org_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM api_tokens WHERE org_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    /// Verify token and return the token record if valid
    pub async fn verify_token(
        pool: &sqlx::PgPool,
        token: &str,
    ) -> sqlx::Result<Option<Self>> {
        // Token format: mfx_<base64>
        if !token.starts_with("mfx_") {
            return Ok(None);
        }

        // Find all tokens with matching prefix (brute force check, but tokens are hashed)
        // For better performance, we could store a hash of the prefix, but for MVP this is fine
        let prefix = token.chars().take(12).collect::<String>();

        // Get all tokens with this prefix (should be very few)
        let candidates = sqlx::query_as::<_, Self>(
            "SELECT * FROM api_tokens WHERE token_prefix = $1 AND (expires_at IS NULL OR expires_at > NOW())",
        )
        .bind(&prefix)
        .fetch_all(pool)
        .await?;

        // Check each candidate
        for candidate in candidates {
            if bcrypt::verify(token, &candidate.hashed_token)
                .unwrap_or(false)
            {
                // Update last_used_at
                sqlx::query("UPDATE api_tokens SET last_used_at = NOW() WHERE id = $1")
                    .bind(candidate.id)
                    .execute(pool)
                    .await?;

                return Ok(Some(candidate));
            }
        }

        Ok(None)
    }

    /// Check if token has a specific scope
    pub fn has_scope(&self, scope: &TokenScope) -> bool {
        let scope_str = scope.to_string();
        self.scopes.contains(&scope_str)
    }

    /// Delete token
    pub async fn delete(pool: &sqlx::PgPool, token_id: Uuid) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM api_tokens WHERE id = $1")
            .bind(token_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Rotate a token (create new token with same scopes, optionally delete old)
    /// Returns the new full token (only shown once) and the new ApiToken record
    pub async fn rotate(
        pool: &sqlx::PgPool,
        token_id: Uuid,
        new_name: Option<&str>,
        delete_old: bool,
    ) -> sqlx::Result<(String, Self, Option<Self>)> {
        // Get old token
        let old_token = Self::find_by_id(pool, token_id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        // Create new token with same scopes
        let scopes: Vec<TokenScope> = old_token.scopes
            .iter()
            .filter_map(|s| TokenScope::from_string(s))
            .collect();

        let new_name = new_name.unwrap_or(&old_token.name);
        let (new_full_token, new_token) = Self::create(
            pool,
            old_token.org_id,
            old_token.user_id,
            new_name,
            &scopes,
            old_token.expires_at,
        )
        .await?;

        // Optionally delete old token
        let deleted_token = if delete_old {
            let deleted = old_token.clone();
            Self::delete(pool, token_id).await?;
            Some(deleted)
        } else {
            None
        };

        Ok((new_full_token, new_token, deleted_token))
    }

    /// Find tokens that need rotation (older than N days)
    pub async fn find_tokens_needing_rotation(
        pool: &sqlx::PgPool,
        org_id: Option<Uuid>,
        days_old: i64,
    ) -> sqlx::Result<Vec<Self>> {
        let cutoff = Utc::now() - chrono::Duration::days(days_old);

        let query = if let Some(org_id) = org_id {
            sqlx::query_as::<_, Self>(
                r#"
                SELECT * FROM api_tokens
                WHERE org_id = $1
                  AND created_at < $2
                  AND (expires_at IS NULL OR expires_at > NOW())
                ORDER BY created_at ASC
                "#
            )
            .bind(org_id)
            .bind(cutoff)
        } else {
            sqlx::query_as::<_, Self>(
                r#"
                SELECT * FROM api_tokens
                WHERE created_at < $1
                  AND (expires_at IS NULL OR expires_at > NOW())
                ORDER BY created_at ASC
                "#
            )
            .bind(cutoff)
        };

        query.fetch_all(pool).await
    }

    /// Check if token needs rotation (older than N days)
    pub fn needs_rotation(&self, days_old: i64) -> bool {
        let cutoff = Utc::now() - chrono::Duration::days(days_old);
        self.created_at < cutoff && (self.expires_at.is_none() || self.expires_at.unwrap() > Utc::now())
    }

    /// Get age of token in days
    pub fn age_days(&self) -> i64 {
        let duration = Utc::now() - self.created_at;
        duration.num_days()
    }
}
