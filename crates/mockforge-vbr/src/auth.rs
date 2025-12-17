//! Authentication emulation
//!
//! This module provides virtual user management, JWT token generation/validation,
//! and session-based authentication for the VBR engine.

use crate::{Error, Result};
use chrono::Duration;
use mockforge_core::time_travel_now;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Virtual user for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualUser {
    /// User ID
    pub id: Uuid,
    /// Username
    pub username: String,
    /// Email
    pub email: String,
    /// Password hash (for virtual users)
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    /// User roles/permissions
    #[serde(default)]
    pub roles: Vec<String>,
}

/// JWT claims for VBR authentication
#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    /// Subject (user ID)
    sub: String,
    /// Username
    username: String,
    /// Email
    email: String,
    /// Expiration time (Unix timestamp)
    exp: usize,
    /// Issued at (Unix timestamp)
    iat: usize,
    /// Roles
    #[serde(default)]
    roles: Vec<String>,
}

impl JwtClaims {
    /// Check if token is expired
    ///
    /// Automatically uses virtual clock if time travel is enabled,
    /// otherwise uses real time.
    fn is_expired(&self) -> bool {
        let now = time_travel_now().timestamp() as usize;
        now >= self.exp
    }
}

/// Authentication service for VBR
pub struct VbrAuthService {
    /// JWT secret (required for token generation)
    jwt_secret: String,
    /// Token expiration in seconds
    token_expiration: u64,
    /// Virtual users (in-memory storage for demo)
    /// In production, this would be stored in the virtual database
    users: std::sync::Arc<tokio::sync::RwLock<HashMap<String, VirtualUser>>>,
}

impl VbrAuthService {
    /// Create a new authentication service
    pub fn new(jwt_secret: String, token_expiration_secs: u64) -> Self {
        Self {
            jwt_secret,
            token_expiration: token_expiration_secs,
            users: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Create a default user (for demo/testing)
    pub async fn create_default_user(
        &self,
        username: String,
        password: String,
        email: String,
    ) -> Result<VirtualUser> {
        // Hash password (simple implementation - in production use bcrypt)
        let password_hash = self.hash_password(&password)?;

        let user = VirtualUser {
            id: Uuid::new_v4(),
            username: username.clone(),
            email,
            password_hash: Some(password_hash),
            roles: Vec::new(),
        };

        let mut users = self.users.write().await;
        users.insert(username, user.clone());
        Ok(user)
    }

    /// Hash a password (simple implementation)
    fn hash_password(&self, password: &str) -> Result<String> {
        // Simple hash for demo - in production use bcrypt or argon2
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(self.jwt_secret.as_bytes()); // Salt with JWT secret
        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }

    /// Verify a password
    fn verify_password(&self, password: &str, hash: &str) -> bool {
        match self.hash_password(password) {
            Ok(new_hash) => new_hash == hash,
            Err(_) => false,
        }
    }

    /// Authenticate a user
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<VirtualUser> {
        let users = self.users.read().await;
        let user = users
            .get(username)
            .ok_or_else(|| Error::generic("User not found".to_string()))?;

        // Verify password
        if let Some(ref hash) = user.password_hash {
            if !self.verify_password(password, hash) {
                return Err(Error::generic("Invalid password".to_string()));
            }
        }

        Ok(user.clone())
    }

    /// Generate JWT token for a user
    ///
    /// Automatically uses virtual clock if time travel is enabled,
    /// otherwise uses real time.
    pub fn generate_token(&self, user: &VirtualUser) -> Result<String> {
        let now = time_travel_now();
        let exp = now
            .checked_add_signed(Duration::seconds(self.token_expiration as i64))
            .ok_or_else(|| Error::generic("Invalid expiration time".to_string()))?
            .timestamp() as usize;

        let claims = JwtClaims {
            sub: user.id.to_string(),
            username: user.username.clone(),
            email: user.email.clone(),
            exp,
            iat: now.timestamp() as usize,
            roles: user.roles.clone(),
        };

        // Use jsonwebtoken crate if available, otherwise return error
        #[cfg(feature = "jwt")]
        {
            use jsonwebtoken::{encode, EncodingKey, Header};
            let token = encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
            )
            .map_err(|e| Error::generic(format!("Token generation failed: {}", e)))?;
            Ok(token)
        }

        #[cfg(not(feature = "jwt"))]
        {
            // Fallback: return a simple token format (not secure, for testing only)
            let token_data = serde_json::to_string(&claims)
                .map_err(|e| Error::generic(format!("Serialization failed: {}", e)))?;
            Ok(format!("vbr.{}", base64::encode(&token_data)))
        }
    }

    /// Validate JWT token
    pub fn validate_token(&self, token: &str) -> Result<VirtualUser> {
        #[cfg(feature = "jwt")]
        {
            use jsonwebtoken::{decode, DecodingKey, Validation};
            let validation = Validation::default();
            let token_data = decode::<JwtClaims>(
                token,
                &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
                &validation,
            )
            .map_err(|e| Error::generic(format!("Token validation failed: {}", e)))?;

            if token_data.claims.is_expired() {
                return Err(Error::generic("Token expired".to_string()));
            }

            Ok(VirtualUser {
                id: Uuid::parse_str(&token_data.claims.sub)
                    .map_err(|e| Error::generic(format!("Invalid user ID: {}", e)))?,
                username: token_data.claims.username,
                email: token_data.claims.email,
                password_hash: None,
                roles: token_data.claims.roles,
            })
        }

        #[cfg(not(feature = "jwt"))]
        {
            // Fallback: decode simple token format
            if let Some(token_data) = token.strip_prefix("vbr.") {
                let decoded = base64::decode(token_data)
                    .map_err(|e| Error::generic(format!("Token decode failed: {}", e)))?;
                let claims: JwtClaims = serde_json::from_slice(&decoded)
                    .map_err(|e| Error::generic(format!("Token parse failed: {}", e)))?;

                if claims.is_expired() {
                    return Err(Error::generic("Token expired".to_string()));
                }

                Ok(VirtualUser {
                    id: Uuid::parse_str(&claims.sub)
                        .map_err(|e| Error::generic(format!("Invalid user ID: {}", e)))?,
                    username: claims.username,
                    email: claims.email,
                    password_hash: None,
                    roles: claims.roles,
                })
            } else {
                Err(Error::generic("Invalid token format".to_string()))
            }
        }
    }

    /// Get user by username
    pub async fn get_user(&self, username: &str) -> Option<VirtualUser> {
        let users = self.users.read().await;
        users.get(username).cloned()
    }

    /// List all users
    pub async fn list_users(&self) -> Vec<VirtualUser> {
        let users = self.users.read().await;
        users.values().cloned().collect()
    }
}

// Add base64 encoding for fallback implementation
#[cfg(not(feature = "jwt"))]
mod base64 {
    use base64::{engine::general_purpose, Engine as _};
    pub fn encode(data: &str) -> String {
        general_purpose::STANDARD.encode(data.as_bytes())
    }
    pub fn decode(data: &str) -> Result<Vec<u8>, String> {
        general_purpose::STANDARD
            .decode(data)
            .map_err(|e| format!("Decode error: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // VirtualUser tests
    #[test]
    fn test_virtual_user_clone() {
        let user = VirtualUser {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: Some("hash123".to_string()),
            roles: vec!["admin".to_string()],
        };

        let cloned = user.clone();
        assert_eq!(user.username, cloned.username);
        assert_eq!(user.email, cloned.email);
        assert_eq!(user.roles, cloned.roles);
    }

    #[test]
    fn test_virtual_user_debug() {
        let user = VirtualUser {
            id: Uuid::new_v4(),
            username: "debuguser".to_string(),
            email: "debug@test.com".to_string(),
            password_hash: None,
            roles: Vec::new(),
        };

        let debug = format!("{:?}", user);
        assert!(debug.contains("VirtualUser"));
        assert!(debug.contains("debuguser"));
    }

    #[test]
    fn test_virtual_user_serialize() {
        let user = VirtualUser {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            username: "serialuser".to_string(),
            email: "serial@test.com".to_string(),
            password_hash: Some("secret".to_string()),
            roles: vec!["user".to_string()],
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("serialuser"));
        assert!(json.contains("serial@test.com"));
        // password_hash should be skipped during serialization
        assert!(!json.contains("secret"));
    }

    #[test]
    fn test_virtual_user_deserialize() {
        let json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "username": "deserialuser",
            "email": "deserial@test.com",
            "roles": ["admin", "user"]
        }"#;

        let user: VirtualUser = serde_json::from_str(json).unwrap();
        assert_eq!(user.username, "deserialuser");
        assert_eq!(user.email, "deserial@test.com");
        assert_eq!(user.roles, vec!["admin", "user"]);
    }

    #[test]
    fn test_virtual_user_default_roles() {
        let json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "username": "norolesuser",
            "email": "noroles@test.com"
        }"#;

        let user: VirtualUser = serde_json::from_str(json).unwrap();
        assert!(user.roles.is_empty());
    }

    #[test]
    fn test_virtual_user_with_multiple_roles() {
        let user = VirtualUser {
            id: Uuid::new_v4(),
            username: "multirole".to_string(),
            email: "multi@test.com".to_string(),
            password_hash: None,
            roles: vec![
                "admin".to_string(),
                "moderator".to_string(),
                "user".to_string(),
            ],
        };

        assert_eq!(user.roles.len(), 3);
        assert!(user.roles.contains(&"admin".to_string()));
        assert!(user.roles.contains(&"moderator".to_string()));
    }

    // VbrAuthService tests
    #[test]
    fn test_vbr_auth_service_new() {
        let service = VbrAuthService::new("test_secret".to_string(), 3600);
        assert_eq!(service.jwt_secret, "test_secret");
        assert_eq!(service.token_expiration, 3600);
    }

    #[test]
    fn test_vbr_auth_service_hash_password() {
        let service = VbrAuthService::new("secret".to_string(), 3600);
        let hash1 = service.hash_password("password123").unwrap();
        let hash2 = service.hash_password("password123").unwrap();

        // Same password should produce same hash
        assert_eq!(hash1, hash2);

        // Different passwords should produce different hashes
        let hash3 = service.hash_password("different").unwrap();
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_vbr_auth_service_verify_password() {
        let service = VbrAuthService::new("secret".to_string(), 3600);
        let hash = service.hash_password("mypassword").unwrap();

        assert!(service.verify_password("mypassword", &hash));
        assert!(!service.verify_password("wrongpassword", &hash));
    }

    #[tokio::test]
    async fn test_vbr_auth_service_create_default_user() {
        let service = VbrAuthService::new("secret".to_string(), 3600);

        let user = service
            .create_default_user(
                "newuser".to_string(),
                "password123".to_string(),
                "new@test.com".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(user.username, "newuser");
        assert_eq!(user.email, "new@test.com");
        assert!(user.password_hash.is_some());
    }

    #[tokio::test]
    async fn test_vbr_auth_service_authenticate_success() {
        let service = VbrAuthService::new("secret".to_string(), 3600);

        service
            .create_default_user(
                "authuser".to_string(),
                "correctpass".to_string(),
                "auth@test.com".to_string(),
            )
            .await
            .unwrap();

        let result = service.authenticate("authuser", "correctpass").await;
        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.username, "authuser");
    }

    #[tokio::test]
    async fn test_vbr_auth_service_authenticate_wrong_password() {
        let service = VbrAuthService::new("secret".to_string(), 3600);

        service
            .create_default_user(
                "wrongpassuser".to_string(),
                "correctpass".to_string(),
                "wrong@test.com".to_string(),
            )
            .await
            .unwrap();

        let result = service.authenticate("wrongpassuser", "wrongpass").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_vbr_auth_service_authenticate_user_not_found() {
        let service = VbrAuthService::new("secret".to_string(), 3600);

        let result = service.authenticate("nonexistent", "anypass").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_vbr_auth_service_get_user() {
        let service = VbrAuthService::new("secret".to_string(), 3600);

        service
            .create_default_user(
                "getuser".to_string(),
                "pass".to_string(),
                "get@test.com".to_string(),
            )
            .await
            .unwrap();

        let user = service.get_user("getuser").await;
        assert!(user.is_some());
        assert_eq!(user.unwrap().email, "get@test.com");

        let nonexistent = service.get_user("nonexistent").await;
        assert!(nonexistent.is_none());
    }

    #[tokio::test]
    async fn test_vbr_auth_service_list_users() {
        let service = VbrAuthService::new("secret".to_string(), 3600);

        // Initially empty
        let users = service.list_users().await;
        assert!(users.is_empty());

        // Add users
        service
            .create_default_user(
                "user1".to_string(),
                "pass1".to_string(),
                "u1@test.com".to_string(),
            )
            .await
            .unwrap();

        service
            .create_default_user(
                "user2".to_string(),
                "pass2".to_string(),
                "u2@test.com".to_string(),
            )
            .await
            .unwrap();

        let users = service.list_users().await;
        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn test_vbr_auth_service_generate_token() {
        let service = VbrAuthService::new("my_jwt_secret".to_string(), 3600);

        let user = service
            .create_default_user(
                "tokenuser".to_string(),
                "pass".to_string(),
                "token@test.com".to_string(),
            )
            .await
            .unwrap();

        let token = service.generate_token(&user);
        assert!(token.is_ok());
        let token_str = token.unwrap();
        assert!(!token_str.is_empty());
    }

    #[tokio::test]
    async fn test_vbr_auth_service_validate_token() {
        let service = VbrAuthService::new("jwt_secret_for_test".to_string(), 3600);

        let user = service
            .create_default_user(
                "validateuser".to_string(),
                "pass".to_string(),
                "validate@test.com".to_string(),
            )
            .await
            .unwrap();

        let token = service.generate_token(&user).unwrap();
        let validated_user = service.validate_token(&token);

        assert!(validated_user.is_ok());
        let validated = validated_user.unwrap();
        assert_eq!(validated.username, "validateuser");
        assert_eq!(validated.email, "validate@test.com");
    }

    #[test]
    fn test_vbr_auth_service_validate_invalid_token() {
        let service = VbrAuthService::new("secret".to_string(), 3600);

        let result = service.validate_token("invalid_token");
        assert!(result.is_err());
    }

    #[test]
    fn test_vbr_auth_service_validate_malformed_token() {
        let service = VbrAuthService::new("secret".to_string(), 3600);

        // Token without vbr. prefix (for non-jwt feature)
        let result = service.validate_token("not.vbr.token");
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_different_secrets_produce_different_hashes() {
        let service1 = VbrAuthService::new("secret1".to_string(), 3600);
        let service2 = VbrAuthService::new("secret2".to_string(), 3600);

        let hash1 = service1.hash_password("samepassword").unwrap();
        let hash2 = service2.hash_password("samepassword").unwrap();

        // Same password with different salts (jwt_secret) should produce different hashes
        assert_ne!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_vbr_auth_service_user_with_roles() {
        let service = VbrAuthService::new("secret".to_string(), 3600);

        let user = service
            .create_default_user(
                "roleuser".to_string(),
                "pass".to_string(),
                "role@test.com".to_string(),
            )
            .await
            .unwrap();

        // Default user has no roles
        assert!(user.roles.is_empty());
    }
}
