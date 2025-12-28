//! Authentication and JWT handling
//!
//! # Security Features
//!
//! - JWT tokens include `aud` (audience) and `iss` (issuer) claims
//! - Token verification validates these claims to prevent token misuse
//! - Access and refresh tokens are distinguished by type claim

use anyhow::Result;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Default issuer for JWT tokens
const DEFAULT_JWT_ISSUER: &str = "mockforge-registry";

/// Default audience for JWT tokens
const DEFAULT_JWT_AUDIENCE: &str = "mockforge-api";

/// Cache the JWT issuer value
static JWT_ISSUER: OnceLock<String> = OnceLock::new();

/// Cache the JWT audience value
static JWT_AUDIENCE: OnceLock<String> = OnceLock::new();

/// Get the JWT issuer (from environment or default)
fn get_jwt_issuer() -> &'static str {
    JWT_ISSUER.get_or_init(|| {
        std::env::var("JWT_ISSUER").unwrap_or_else(|_| DEFAULT_JWT_ISSUER.to_string())
    })
}

/// Get the JWT audience (from environment or default)
fn get_jwt_audience() -> &'static str {
    JWT_AUDIENCE.get_or_init(|| {
        std::env::var("JWT_AUDIENCE").unwrap_or_else(|_| DEFAULT_JWT_AUDIENCE.to_string())
    })
}

/// Token type for distinguishing access vs refresh tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    Access,
    Refresh,
}

/// Access token expiration: 1 hour
pub const ACCESS_TOKEN_EXPIRY_HOURS: i64 = 1;

/// Refresh token expiration: 7 days
pub const REFRESH_TOKEN_EXPIRY_DAYS: i64 = 7;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub exp: usize,  // expiry timestamp
    pub iat: usize,  // issued at timestamp
    pub iss: String, // issuer - identifies who issued the token
    pub aud: String, // audience - identifies intended recipients
    #[serde(default = "default_token_type")]
    pub token_type: TokenType, // access or refresh
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>, // unique token ID (for refresh token revocation)
}

fn default_token_type() -> TokenType {
    TokenType::Access
}

/// Token pair returned on login
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub access_token_expires_at: i64,
    pub refresh_token_expires_at: i64,
}

/// Create an access token (short-lived, 1 hour)
pub fn create_access_token(user_id: &str, secret: &str) -> Result<String> {
    let now = Utc::now();
    let expiration = now
        .checked_add_signed(Duration::hours(ACCESS_TOKEN_EXPIRY_HOURS))
        .ok_or_else(|| anyhow::anyhow!("Failed to calculate token expiration"))?
        .timestamp();

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration as usize,
        iat: now.timestamp() as usize,
        iss: get_jwt_issuer().to_string(),
        aud: get_jwt_audience().to_string(),
        token_type: TokenType::Access,
        jti: None,
    };

    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))?;
    Ok(token)
}

/// Create a refresh token (longer-lived, 7 days)
/// The jti (JWT ID) can be stored in the database for revocation
pub fn create_refresh_token(user_id: &str, secret: &str) -> Result<(String, String, i64)> {
    let now = Utc::now();
    let expiration = now
        .checked_add_signed(Duration::days(REFRESH_TOKEN_EXPIRY_DAYS))
        .ok_or_else(|| anyhow::anyhow!("Failed to calculate refresh token expiration"))?
        .timestamp();

    // Generate unique token ID for revocation tracking
    let jti = uuid::Uuid::new_v4().to_string();

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration as usize,
        iat: now.timestamp() as usize,
        iss: get_jwt_issuer().to_string(),
        aud: get_jwt_audience().to_string(),
        token_type: TokenType::Refresh,
        jti: Some(jti.clone()),
    };

    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))?;
    Ok((token, jti, expiration))
}

/// Create both access and refresh tokens
pub fn create_token_pair(user_id: &str, secret: &str) -> Result<(TokenPair, String)> {
    let access_token = create_access_token(user_id, secret)?;
    let (refresh_token, jti, refresh_exp) = create_refresh_token(user_id, secret)?;

    let now = Utc::now();
    let access_exp = now
        .checked_add_signed(Duration::hours(ACCESS_TOKEN_EXPIRY_HOURS))
        .ok_or_else(|| anyhow::anyhow!("Failed to calculate access token expiration"))?
        .timestamp();

    Ok((
        TokenPair {
            access_token,
            refresh_token,
            access_token_expires_at: access_exp,
            refresh_token_expires_at: refresh_exp,
        },
        jti,
    ))
}

/// Legacy function for backwards compatibility - creates short-lived access token
pub fn create_token(user_id: &str, secret: &str) -> Result<String> {
    create_access_token(user_id, secret)
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims> {
    let mut validation = Validation::default();

    // Configure audience validation
    validation.set_audience(&[get_jwt_audience()]);

    // Configure issuer validation
    validation.set_issuer(&[get_jwt_issuer()]);

    let token_data =
        decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &validation)?;

    Ok(token_data.claims)
}

/// Verify a token and ensure it's specifically a refresh token
/// Returns the claims and the JTI (for revocation checking)
pub fn verify_refresh_token(token: &str, secret: &str) -> Result<(Claims, String)> {
    let claims = verify_token(token, secret)?;

    if claims.token_type != TokenType::Refresh {
        anyhow::bail!("Expected refresh token, got access token");
    }

    let jti = claims.jti.clone().ok_or_else(|| anyhow::anyhow!("Refresh token missing JTI"))?;

    Ok((claims, jti))
}

/// Verify a token and ensure it's specifically an access token
pub fn verify_access_token(token: &str, secret: &str) -> Result<Claims> {
    let claims = verify_token(token, secret)?;

    if claims.token_type != TokenType::Access {
        anyhow::bail!("Expected access token, got refresh token");
    }

    Ok(claims)
}

pub fn hash_password(password: &str) -> Result<String> {
    let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
    Ok(hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let valid = bcrypt::verify(password, hash)?;
    Ok(valid)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-key-for-jwt-signing-minimum-32-chars";

    #[test]
    fn test_create_token() {
        let user_id = "user-123";
        let token = create_token(user_id, TEST_SECRET).unwrap();

        // Token should not be empty
        assert!(!token.is_empty());

        // Token should have 3 parts separated by dots (JWT format)
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_verify_token_valid() {
        let user_id = "user-456";
        let token = create_token(user_id, TEST_SECRET).unwrap();

        // Verify the token
        let claims = verify_token(&token, TEST_SECRET).unwrap();

        // Check the claims
        assert_eq!(claims.sub, user_id);
        assert!(claims.exp > claims.iat);

        // Token should be valid for approximately 1 hour (access token)
        let duration = claims.exp - claims.iat;
        // Should be approximately 1 hour in seconds (with some tolerance)
        assert!(duration >= 59 * 60, "Token should be valid for at least 59 minutes");
        assert!(duration <= 61 * 60, "Token should be valid for at most 61 minutes");

        // Token should be an access token
        assert_eq!(claims.token_type, TokenType::Access);
    }

    #[test]
    fn test_access_token() {
        let user_id = "user-access";
        let token = create_access_token(user_id, TEST_SECRET).unwrap();
        let claims = verify_access_token(&token, TEST_SECRET).unwrap();

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.token_type, TokenType::Access);
        assert!(claims.jti.is_none());
    }

    #[test]
    fn test_refresh_token() {
        let user_id = "user-refresh";
        let (token, jti, _expires) = create_refresh_token(user_id, TEST_SECRET).unwrap();
        let (claims, verified_jti) = verify_refresh_token(&token, TEST_SECRET).unwrap();

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.token_type, TokenType::Refresh);
        assert_eq!(verified_jti, jti);

        // Refresh token should be valid for approximately 7 days
        let duration = claims.exp - claims.iat;
        assert!(
            duration >= 6 * 24 * 60 * 60,
            "Refresh token should be valid for at least 6 days"
        );
        assert!(duration <= 8 * 24 * 60 * 60, "Refresh token should be valid for at most 8 days");
    }

    #[test]
    fn test_token_pair() {
        let user_id = "user-pair";
        let (pair, jti) = create_token_pair(user_id, TEST_SECRET).unwrap();

        // Verify access token
        let access_claims = verify_access_token(&pair.access_token, TEST_SECRET).unwrap();
        assert_eq!(access_claims.sub, user_id);
        assert_eq!(access_claims.token_type, TokenType::Access);

        // Verify refresh token
        let (refresh_claims, verified_jti) =
            verify_refresh_token(&pair.refresh_token, TEST_SECRET).unwrap();
        assert_eq!(refresh_claims.sub, user_id);
        assert_eq!(refresh_claims.token_type, TokenType::Refresh);
        assert_eq!(verified_jti, jti);

        // Access token should expire before refresh token
        assert!(pair.access_token_expires_at < pair.refresh_token_expires_at);
    }

    #[test]
    fn test_refresh_token_rejected_as_access() {
        let user_id = "user-reject";
        let (token, _, _) = create_refresh_token(user_id, TEST_SECRET).unwrap();

        // Should fail when trying to verify as access token
        let result = verify_access_token(&token, TEST_SECRET);
        assert!(result.is_err());
    }

    #[test]
    fn test_access_token_rejected_as_refresh() {
        let user_id = "user-reject2";
        let token = create_access_token(user_id, TEST_SECRET).unwrap();

        // Should fail when trying to verify as refresh token
        let result = verify_refresh_token(&token, TEST_SECRET);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_token_wrong_secret() {
        let user_id = "user-789";
        let token = create_token(user_id, TEST_SECRET).unwrap();

        // Try to verify with wrong secret
        let wrong_secret = "wrong-secret-key-for-jwt-signing";
        let result = verify_token(&token, wrong_secret);

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_token_invalid_format() {
        let invalid_token = "invalid.token.format";
        let result = verify_token(invalid_token, TEST_SECRET);

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_token_malformed() {
        let malformed_token = "not-a-jwt-token";
        let result = verify_token(malformed_token, TEST_SECRET);

        assert!(result.is_err());
    }

    #[test]
    fn test_hash_password() {
        let password = "my-secure-password";
        let hash = hash_password(password).unwrap();

        // Hash should not be empty
        assert!(!hash.is_empty());

        // Hash should start with bcrypt prefix
        assert!(hash.starts_with("$2"));

        // Hash should be different from original password
        assert_ne!(hash, password);
    }

    #[test]
    fn test_hash_password_different_hashes() {
        let password = "same-password";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        // Same password should produce different hashes (due to salt)
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_verify_password_valid() {
        let password = "correct-password";
        let hash = hash_password(password).unwrap();

        // Verify with correct password
        let valid = verify_password(password, &hash).unwrap();
        assert!(valid);
    }

    #[test]
    fn test_verify_password_invalid() {
        let password = "correct-password";
        let hash = hash_password(password).unwrap();

        // Verify with wrong password
        let valid = verify_password("wrong-password", &hash).unwrap();
        assert!(!valid);
    }

    #[test]
    fn test_verify_password_empty() {
        let password = "password";
        let hash = hash_password(password).unwrap();

        // Verify with empty password
        let valid = verify_password("", &hash).unwrap();
        assert!(!valid);
    }

    #[test]
    fn test_verify_password_invalid_hash() {
        let password = "password";
        let invalid_hash = "not-a-valid-bcrypt-hash";

        // Should return error for invalid hash format
        let result = verify_password(password, invalid_hash);
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_password_empty() {
        let password = "";
        let hash = hash_password(password).unwrap();

        // Should still produce a valid hash for empty password
        assert!(!hash.is_empty());
        assert!(hash.starts_with("$2"));
    }

    #[test]
    fn test_hash_password_special_chars() {
        let password = "p@ssw0rd!#$%^&*()";
        let hash = hash_password(password).unwrap();

        // Should handle special characters
        assert!(!hash.is_empty());

        // Should verify correctly
        let valid = verify_password(password, &hash).unwrap();
        assert!(valid);
    }

    #[test]
    fn test_hash_password_unicode() {
        let password = "пароль密码🔒";
        let hash = hash_password(password).unwrap();

        // Should handle unicode
        assert!(!hash.is_empty());

        // Should verify correctly
        let valid = verify_password(password, &hash).unwrap();
        assert!(valid);
    }

    #[test]
    fn test_claims_serialization() {
        let claims = Claims {
            sub: "user-123".to_string(),
            exp: 1234567890,
            iat: 1234567800,
            iss: "mockforge-registry".to_string(),
            aud: "mockforge-api".to_string(),
            token_type: TokenType::Access,
            jti: None,
        };

        // Should serialize to JSON
        let json = serde_json::to_string(&claims).unwrap();
        assert!(json.contains("user-123"));
        assert!(json.contains("1234567890"));
        assert!(json.contains("access")); // token_type
        assert!(json.contains("mockforge-registry")); // issuer
        assert!(json.contains("mockforge-api")); // audience

        // Should deserialize from JSON
        let deserialized: Claims = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.sub, claims.sub);
        assert_eq!(deserialized.exp, claims.exp);
        assert_eq!(deserialized.iat, claims.iat);
        assert_eq!(deserialized.iss, claims.iss);
        assert_eq!(deserialized.aud, claims.aud);
        assert_eq!(deserialized.token_type, TokenType::Access);
    }

    #[test]
    fn test_token_contains_user_id() {
        let user_id = "unique-user-id-12345";
        let token = create_token(user_id, TEST_SECRET).unwrap();
        let claims = verify_token(&token, TEST_SECRET).unwrap();

        assert_eq!(claims.sub, user_id);
    }

    #[test]
    fn test_multiple_tokens_same_user() {
        let user_id = "user-123";
        let token1 = create_token(user_id, TEST_SECRET).unwrap();

        // Wait at least 1 second to ensure different iat (JWT timestamps have second resolution)
        std::thread::sleep(std::time::Duration::from_millis(1100));

        let token2 = create_token(user_id, TEST_SECRET).unwrap();

        // Tokens should be different (different iat)
        assert_ne!(token1, token2);

        // But both should verify correctly
        let claims1 = verify_token(&token1, TEST_SECRET).unwrap();
        let claims2 = verify_token(&token2, TEST_SECRET).unwrap();

        assert_eq!(claims1.sub, user_id);
        assert_eq!(claims2.sub, user_id);
    }

    #[test]
    fn test_token_includes_issuer_and_audience() {
        let user_id = "user-iss-aud";
        let token = create_access_token(user_id, TEST_SECRET).unwrap();
        let claims = verify_token(&token, TEST_SECRET).unwrap();

        // Check issuer and audience are set
        assert!(!claims.iss.is_empty());
        assert!(!claims.aud.is_empty());
    }

    #[test]
    fn test_refresh_token_includes_issuer_and_audience() {
        let user_id = "user-refresh-iss";
        let (token, _, _) = create_refresh_token(user_id, TEST_SECRET).unwrap();
        let (claims, _) = verify_refresh_token(&token, TEST_SECRET).unwrap();

        assert!(!claims.iss.is_empty());
        assert!(!claims.aud.is_empty());
    }
}
