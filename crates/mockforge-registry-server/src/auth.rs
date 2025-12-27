//! Authentication and JWT handling

use anyhow::Result;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub exp: usize,  // expiry timestamp
    pub iat: usize,  // issued at timestamp
}

pub fn create_token(user_id: &str, secret: &str) -> Result<String> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(30))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration as usize,
        iat: Utc::now().timestamp() as usize,
    };

    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))?;

    Ok(token)
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims> {
    let validation = Validation::default();
    let token_data =
        decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &validation)?;

    Ok(token_data.claims)
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

        // Token should be valid for approximately 30 days
        let duration = claims.exp - claims.iat;
        // Should be approximately 30 days in seconds (with some tolerance)
        assert!(duration > 29 * 24 * 60 * 60);
        assert!(duration < 31 * 24 * 60 * 60);
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
        };

        // Should serialize to JSON
        let json = serde_json::to_string(&claims).unwrap();
        assert!(json.contains("user-123"));
        assert!(json.contains("1234567890"));

        // Should deserialize from JSON
        let deserialized: Claims = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.sub, claims.sub);
        assert_eq!(deserialized.exp, claims.exp);
        assert_eq!(deserialized.iat, claims.iat);
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
}
