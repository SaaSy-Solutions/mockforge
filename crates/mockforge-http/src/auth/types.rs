//! Authentication types and data structures
//!
//! This module defines the core types used in authentication:
//! - AuthResult: Result of authentication attempts
//! - AuthClaims: JWT claims extracted from tokens

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Authentication result
#[derive(Debug, Clone)]
pub enum AuthResult {
    /// Authentication successful
    Success(AuthClaims),
    /// Authentication failed with reason
    Failure(String),
    /// No authentication provided (when auth is optional)
    None,
}

/// Authentication claims extracted from tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthClaims {
    /// Subject (user ID)
    pub sub: Option<String>,
    /// Issuer
    pub iss: Option<String>,
    /// Audience
    pub aud: Option<String>,
    /// Expiration time
    pub exp: Option<i64>,
    /// Issued at time
    pub iat: Option<i64>,
    /// Username
    pub username: Option<String>,
    /// Roles/permissions
    pub roles: Vec<String>,
    /// Custom claims
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for AuthClaims {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthClaims {
    pub fn new() -> Self {
        Self {
            sub: None,
            iss: None,
            aud: None,
            exp: None,
            iat: None,
            username: None,
            roles: Vec::new(),
            custom: HashMap::new(),
        }
    }
}
