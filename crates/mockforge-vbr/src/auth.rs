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
