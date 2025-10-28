//! Authentication and authorization

use crate::error::{CollabError, Result};
use crate::models::User;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT claims for authentication tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Username
    pub username: String,
    /// Expiration time
    pub exp: i64,
    /// Issued at
    pub iat: i64,
}

impl Claims {
    /// Create new claims for a user
    pub fn new(user_id: Uuid, username: String, expires_in: Duration) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id.to_string(),
            username,
            exp: (now + expires_in).timestamp(),
            iat: now.timestamp(),
        }
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }
}

/// Authentication token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    /// The JWT token string
    pub access_token: String,
    /// Token type (always "Bearer")
    pub token_type: String,
    /// Expiration time
    pub expires_at: DateTime<Utc>,
}

/// User credentials for login
#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    /// Username or email
    pub username: String,
    /// Password
    pub password: String,
}

/// Active user session
#[derive(Debug, Clone)]
pub struct Session {
    /// User ID
    pub user_id: Uuid,
    /// Username
    pub username: String,
    /// Session expiration
    pub expires_at: DateTime<Utc>,
}

/// Authentication service
pub struct AuthService {
    /// JWT secret for signing tokens
    jwt_secret: String,
    /// Token expiration duration (default: 24 hours)
    token_expiration: Duration,
}

impl AuthService {
    /// Create a new authentication service
    pub fn new(jwt_secret: String) -> Self {
        Self {
            jwt_secret,
            token_expiration: Duration::hours(24),
        }
    }

    /// Set custom token expiration
    pub fn with_expiration(mut self, expiration: Duration) -> Self {
        self.token_expiration = expiration;
        self
    }

    /// Hash a password using Argon2
    pub fn hash_password(&self, password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| CollabError::Internal(format!("Password hashing failed: {}", e)))?
            .to_string();

        Ok(password_hash)
    }

    /// Verify a password against a hash
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| CollabError::Internal(format!("Invalid password hash: {}", e)))?;

        let argon2 = Argon2::default();

        Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }

    /// Generate a JWT token for a user
    pub fn generate_token(&self, user: &User) -> Result<Token> {
        let claims = Claims::new(user.id, user.username.clone(), self.token_expiration);
        let expires_at = Utc::now() + self.token_expiration;

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| CollabError::Internal(format!("Token generation failed: {}", e)))?;

        Ok(Token {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_at,
        })
    }

    /// Verify and decode a JWT token
    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| CollabError::AuthenticationFailed(format!("Invalid token: {}", e)))?;

        if token_data.claims.is_expired() {
            return Err(CollabError::AuthenticationFailed("Token expired".to_string()));
        }

        Ok(token_data.claims)
    }

    /// Create a session from a token
    pub fn create_session(&self, token: &str) -> Result<Session> {
        let claims = self.verify_token(token)?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| CollabError::Internal(format!("Invalid user ID in token: {}", e)))?;

        Ok(Session {
            user_id,
            username: claims.username,
            expires_at: DateTime::from_timestamp(claims.exp, 0)
                .ok_or_else(|| CollabError::Internal("Invalid timestamp".to_string()))?,
        })
    }

    /// Generate a random invitation token
    pub fn generate_invitation_token(&self) -> String {
        use blake3::hash;
        let random_data =
            format!("{}{}", Uuid::new_v4(), Utc::now().timestamp_nanos_opt().unwrap_or(0));
        hash(random_data.as_bytes()).to_hex().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let auth = AuthService::new("test_secret".to_string());
        let password = "test_password_123";

        let hash = auth.hash_password(password).unwrap();
        assert!(auth.verify_password(password, &hash).unwrap());
        assert!(!auth.verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_token_generation() {
        let auth = AuthService::new("test_secret".to_string());
        let user =
            User::new("testuser".to_string(), "test@example.com".to_string(), "hash".to_string());

        let token = auth.generate_token(&user).unwrap();
        assert_eq!(token.token_type, "Bearer");
        assert!(!token.access_token.is_empty());
    }

    #[test]
    fn test_token_verification() {
        let auth = AuthService::new("test_secret".to_string());
        let user =
            User::new("testuser".to_string(), "test@example.com".to_string(), "hash".to_string());

        let token = auth.generate_token(&user).unwrap();
        let claims = auth.verify_token(&token.access_token).unwrap();

        assert_eq!(claims.username, "testuser");
        assert!(!claims.is_expired());
    }

    #[test]
    fn test_session_creation() {
        let auth = AuthService::new("test_secret".to_string());
        let user =
            User::new("testuser".to_string(), "test@example.com".to_string(), "hash".to_string());

        let token = auth.generate_token(&user).unwrap();
        let session = auth.create_session(&token.access_token).unwrap();

        assert_eq!(session.username, "testuser");
    }

    #[test]
    fn test_invitation_token_generation() {
        let auth = AuthService::new("test_secret".to_string());
        let token1 = auth.generate_invitation_token();
        let token2 = auth.generate_invitation_token();

        assert!(!token1.is_empty());
        assert!(!token2.is_empty());
        assert_ne!(token1, token2); // Should be unique
    }
}
