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
        use base64::{engine::general_purpose, Engine as _};
        let token_suffix = {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
            general_purpose::STANDARD.encode(&random_bytes)
        };
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
    pub async fn verify_token(pool: &sqlx::PgPool, token: &str) -> sqlx::Result<Option<Self>> {
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
            if bcrypt::verify(token, &candidate.hashed_token).unwrap_or(false) {
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
        let scopes: Vec<TokenScope> =
            old_token.scopes.iter().filter_map(|s| TokenScope::from_string(s)).collect();

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
                "#,
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
                "#,
            )
            .bind(cutoff)
        };

        query.fetch_all(pool).await
    }

    /// Check if token needs rotation (older than N days)
    pub fn needs_rotation(&self, days_old: i64) -> bool {
        let cutoff = Utc::now() - chrono::Duration::days(days_old);
        self.created_at < cutoff && self.expires_at.map_or(true, |t| t > Utc::now())
    }

    /// Get age of token in days
    pub fn age_days(&self) -> i64 {
        let duration = Utc::now() - self.created_at;
        duration.num_days()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_scope_to_string() {
        assert_eq!(TokenScope::ReadPackages.to_string(), "read:packages");
        assert_eq!(TokenScope::PublishPackages.to_string(), "publish:packages");
        assert_eq!(TokenScope::DeployMocks.to_string(), "deploy:mocks");
        assert_eq!(TokenScope::AdminOrg.to_string(), "admin:org");
        assert_eq!(TokenScope::ReadUsage.to_string(), "read:usage");
        assert_eq!(TokenScope::ManageBilling.to_string(), "manage:billing");
    }

    #[test]
    fn test_token_scope_from_string() {
        assert_eq!(TokenScope::from_string("read:packages"), Some(TokenScope::ReadPackages));
        assert_eq!(TokenScope::from_string("publish:packages"), Some(TokenScope::PublishPackages));
        assert_eq!(TokenScope::from_string("deploy:mocks"), Some(TokenScope::DeployMocks));
        assert_eq!(TokenScope::from_string("admin:org"), Some(TokenScope::AdminOrg));
        assert_eq!(TokenScope::from_string("read:usage"), Some(TokenScope::ReadUsage));
        assert_eq!(TokenScope::from_string("manage:billing"), Some(TokenScope::ManageBilling));

        // Invalid scope
        assert_eq!(TokenScope::from_string("invalid"), None);
        assert_eq!(TokenScope::from_string(""), None);
    }

    #[test]
    fn test_token_scope_round_trip() {
        let scopes = vec![
            TokenScope::ReadPackages,
            TokenScope::PublishPackages,
            TokenScope::DeployMocks,
            TokenScope::AdminOrg,
            TokenScope::ReadUsage,
            TokenScope::ManageBilling,
        ];

        for scope in scopes {
            let string = scope.to_string();
            let parsed = TokenScope::from_string(&string);
            assert_eq!(Some(scope), parsed);
        }
    }

    #[test]
    fn test_token_scope_serialization() {
        let scope = TokenScope::ReadPackages;
        let json = serde_json::to_string(&scope).unwrap();
        assert_eq!(json, "\"readpackages\"");

        let scope = TokenScope::PublishPackages;
        let json = serde_json::to_string(&scope).unwrap();
        assert_eq!(json, "\"publishpackages\"");
    }

    #[test]
    fn test_token_scope_deserialization() {
        let scope: TokenScope = serde_json::from_str("\"readpackages\"").unwrap();
        assert_eq!(scope, TokenScope::ReadPackages);

        let scope: TokenScope = serde_json::from_str("\"publishpackages\"").unwrap();
        assert_eq!(scope, TokenScope::PublishPackages);
    }

    #[test]
    fn test_token_scope_equality() {
        assert_eq!(TokenScope::ReadPackages, TokenScope::ReadPackages);
        assert_ne!(TokenScope::ReadPackages, TokenScope::PublishPackages);
    }

    #[test]
    fn test_token_scope_clone() {
        let scope = TokenScope::AdminOrg;
        let cloned = scope.clone();
        assert_eq!(scope, cloned);
    }

    #[test]
    fn test_api_token_has_scope() {
        let token = ApiToken {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            user_id: None,
            name: "Test Token".to_string(),
            token_prefix: "mfx_12345678".to_string(),
            hashed_token: "hash".to_string(),
            scopes: vec!["read:packages".to_string(), "publish:packages".to_string()],
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(token.has_scope(&TokenScope::ReadPackages));
        assert!(token.has_scope(&TokenScope::PublishPackages));
        assert!(!token.has_scope(&TokenScope::DeployMocks));
        assert!(!token.has_scope(&TokenScope::AdminOrg));
    }

    #[test]
    fn test_api_token_needs_rotation() {
        let old_token = ApiToken {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            user_id: None,
            name: "Old Token".to_string(),
            token_prefix: "mfx_old12345".to_string(),
            hashed_token: "hash".to_string(),
            scopes: vec!["read:packages".to_string()],
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now() - chrono::Duration::days(100),
            updated_at: Utc::now(),
        };

        assert!(old_token.needs_rotation(90));
        assert!(old_token.needs_rotation(50));
        assert!(!old_token.needs_rotation(200));
    }

    #[test]
    fn test_api_token_needs_rotation_expired() {
        let expired_token = ApiToken {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            user_id: None,
            name: "Expired Token".to_string(),
            token_prefix: "mfx_exp12345".to_string(),
            hashed_token: "hash".to_string(),
            scopes: vec!["read:packages".to_string()],
            last_used_at: None,
            expires_at: Some(Utc::now() - chrono::Duration::days(10)),
            created_at: Utc::now() - chrono::Duration::days(100),
            updated_at: Utc::now(),
        };

        // Expired tokens should not need rotation (they're already invalid)
        assert!(!expired_token.needs_rotation(90));
    }

    #[test]
    fn test_api_token_age_days() {
        let token = ApiToken {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            user_id: None,
            name: "Test Token".to_string(),
            token_prefix: "mfx_12345678".to_string(),
            hashed_token: "hash".to_string(),
            scopes: vec!["read:packages".to_string()],
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now() - chrono::Duration::days(42),
            updated_at: Utc::now(),
        };

        let age = token.age_days();
        assert!(age >= 41 && age <= 43); // Allow some tolerance
    }

    #[test]
    fn test_api_token_age_days_new() {
        let token = ApiToken {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            user_id: None,
            name: "New Token".to_string(),
            token_prefix: "mfx_new12345".to_string(),
            hashed_token: "hash".to_string(),
            scopes: vec!["read:packages".to_string()],
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let age = token.age_days();
        assert_eq!(age, 0);
    }

    #[test]
    fn test_api_token_serialization() {
        let token = ApiToken {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            user_id: Some(Uuid::new_v4()),
            name: "Test Token".to_string(),
            token_prefix: "mfx_12345678".to_string(),
            hashed_token: "hash".to_string(),
            scopes: vec!["read:packages".to_string()],
            last_used_at: Some(Utc::now()),
            expires_at: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&token).unwrap();
        assert!(json.contains("Test Token"));
        assert!(json.contains("mfx_12345678"));
        assert!(json.contains("read:packages"));
    }

    #[test]
    fn test_api_token_clone() {
        let token = ApiToken {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            user_id: None,
            name: "Test Token".to_string(),
            token_prefix: "mfx_12345678".to_string(),
            hashed_token: "hash".to_string(),
            scopes: vec!["read:packages".to_string()],
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let cloned = token.clone();
        assert_eq!(token.id, cloned.id);
        assert_eq!(token.name, cloned.name);
        assert_eq!(token.token_prefix, cloned.token_prefix);
        assert_eq!(token.scopes, cloned.scopes);
    }

    #[test]
    fn test_api_token_has_scope_empty() {
        let token = ApiToken {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            user_id: None,
            name: "No Scopes Token".to_string(),
            token_prefix: "mfx_noscopes".to_string(),
            hashed_token: "hash".to_string(),
            scopes: vec![],
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(!token.has_scope(&TokenScope::ReadPackages));
        assert!(!token.has_scope(&TokenScope::AdminOrg));
    }

    #[test]
    fn test_api_token_token_prefix_format() {
        let prefix = "mfx_12345678";
        assert!(prefix.starts_with("mfx_"));
        assert_eq!(prefix.len(), 12);
    }

    #[test]
    fn test_api_token_with_user() {
        let user_id = Uuid::new_v4();
        let token = ApiToken {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            user_id: Some(user_id),
            name: "User Token".to_string(),
            token_prefix: "mfx_user1234".to_string(),
            hashed_token: "hash".to_string(),
            scopes: vec!["read:packages".to_string()],
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(token.user_id, Some(user_id));
    }

    #[test]
    fn test_api_token_without_user() {
        let token = ApiToken {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            user_id: None,
            name: "Org Token".to_string(),
            token_prefix: "mfx_org12345".to_string(),
            hashed_token: "hash".to_string(),
            scopes: vec!["read:packages".to_string()],
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(token.user_id, None);
    }
}
