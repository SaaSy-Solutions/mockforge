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
    /// Network error during authentication
    NetworkError(String),
    /// Authentication server error
    ServerError(String),
    /// Token is expired
    TokenExpired,
    /// Token is invalid
    TokenInvalid(String),
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
    #[serde(default)]
    pub roles: Vec<String>,
    /// Custom claims
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for AuthClaims {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthClaims {
    /// Create a new empty authentication claims structure
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_auth_claims_new() {
        let claims = AuthClaims::new();
        assert!(claims.sub.is_none());
        assert!(claims.iss.is_none());
        assert!(claims.aud.is_none());
        assert!(claims.exp.is_none());
        assert!(claims.iat.is_none());
        assert!(claims.username.is_none());
        assert!(claims.roles.is_empty());
        assert!(claims.custom.is_empty());
    }

    #[test]
    fn test_auth_claims_default() {
        let claims = AuthClaims::default();
        assert!(claims.sub.is_none());
        assert!(claims.roles.is_empty());
        assert!(claims.custom.is_empty());
    }

    #[test]
    fn test_auth_claims_with_values() {
        let mut claims = AuthClaims::new();
        claims.sub = Some("user123".to_string());
        claims.iss = Some("test-issuer".to_string());
        claims.aud = Some("test-audience".to_string());
        claims.exp = Some(1234567890);
        claims.iat = Some(1234567800);
        claims.username = Some("testuser".to_string());
        claims.roles = vec!["admin".to_string(), "user".to_string()];
        claims.custom.insert("email".to_string(), json!("test@example.com"));

        assert_eq!(claims.sub, Some("user123".to_string()));
        assert_eq!(claims.iss, Some("test-issuer".to_string()));
        assert_eq!(claims.aud, Some("test-audience".to_string()));
        assert_eq!(claims.exp, Some(1234567890));
        assert_eq!(claims.iat, Some(1234567800));
        assert_eq!(claims.username, Some("testuser".to_string()));
        assert_eq!(claims.roles.len(), 2);
        assert_eq!(claims.roles[0], "admin");
        assert_eq!(claims.roles[1], "user");
        assert_eq!(claims.custom.get("email").unwrap(), &json!("test@example.com"));
    }

    #[test]
    fn test_auth_claims_serialization() {
        let mut claims = AuthClaims::new();
        claims.sub = Some("user123".to_string());
        claims.roles = vec!["admin".to_string()];
        claims.custom.insert("department".to_string(), json!("engineering"));

        let serialized = serde_json::to_value(&claims).unwrap();
        assert_eq!(serialized["sub"], "user123");
        assert_eq!(serialized["roles"][0], "admin");
        assert_eq!(serialized["custom"]["department"], "engineering");
    }

    #[test]
    fn test_auth_claims_deserialization() {
        let json_data = json!({
            "sub": "user456",
            "iss": "my-issuer",
            "aud": "my-audience",
            "exp": 9999999999i64,
            "iat": 9999999990i64,
            "username": "myuser",
            "roles": ["viewer", "editor"],
            "custom": {
                "org_id": "org-123",
                "permissions": ["read", "write"]
            }
        });

        let claims: AuthClaims = serde_json::from_value(json_data).unwrap();
        assert_eq!(claims.sub, Some("user456".to_string()));
        assert_eq!(claims.iss, Some("my-issuer".to_string()));
        assert_eq!(claims.aud, Some("my-audience".to_string()));
        assert_eq!(claims.exp, Some(9999999999));
        assert_eq!(claims.iat, Some(9999999990));
        assert_eq!(claims.username, Some("myuser".to_string()));
        assert_eq!(claims.roles, vec!["viewer", "editor"]);
        assert_eq!(claims.custom.get("org_id").unwrap(), &json!("org-123"));
    }

    #[test]
    fn test_auth_claims_clone() {
        let mut claims1 = AuthClaims::new();
        claims1.sub = Some("user789".to_string());
        claims1.roles.push("admin".to_string());

        let claims2 = claims1.clone();
        assert_eq!(claims1.sub, claims2.sub);
        assert_eq!(claims1.roles, claims2.roles);
    }

    #[test]
    fn test_auth_result_success() {
        let claims = AuthClaims::new();
        let result = AuthResult::Success(claims.clone());

        match result {
            AuthResult::Success(c) => {
                assert!(c.sub.is_none());
                assert!(c.roles.is_empty());
            }
            _ => panic!("Expected Success variant"),
        }
    }

    #[test]
    fn test_auth_result_failure() {
        let result = AuthResult::Failure("Invalid credentials".to_string());

        match result {
            AuthResult::Failure(msg) => {
                assert_eq!(msg, "Invalid credentials");
            }
            _ => panic!("Expected Failure variant"),
        }
    }

    #[test]
    fn test_auth_result_network_error() {
        let result = AuthResult::NetworkError("Connection timeout".to_string());

        match result {
            AuthResult::NetworkError(msg) => {
                assert_eq!(msg, "Connection timeout");
            }
            _ => panic!("Expected NetworkError variant"),
        }
    }

    #[test]
    fn test_auth_result_server_error() {
        let result = AuthResult::ServerError("Internal server error".to_string());

        match result {
            AuthResult::ServerError(msg) => {
                assert_eq!(msg, "Internal server error");
            }
            _ => panic!("Expected ServerError variant"),
        }
    }

    #[test]
    fn test_auth_result_token_expired() {
        let result = AuthResult::TokenExpired;

        match result {
            AuthResult::TokenExpired => {
                // Success
            }
            _ => panic!("Expected TokenExpired variant"),
        }
    }

    #[test]
    fn test_auth_result_token_invalid() {
        let result = AuthResult::TokenInvalid("Malformed token".to_string());

        match result {
            AuthResult::TokenInvalid(msg) => {
                assert_eq!(msg, "Malformed token");
            }
            _ => panic!("Expected TokenInvalid variant"),
        }
    }

    #[test]
    fn test_auth_result_none() {
        let result = AuthResult::None;

        match result {
            AuthResult::None => {
                // Success
            }
            _ => panic!("Expected None variant"),
        }
    }

    #[test]
    fn test_auth_result_clone() {
        let result1 = AuthResult::Failure("Test error".to_string());
        let result2 = result1.clone();

        match result2 {
            AuthResult::Failure(msg) => {
                assert_eq!(msg, "Test error");
            }
            _ => panic!("Expected Failure variant"),
        }
    }

    #[test]
    fn test_auth_claims_partial_deserialization() {
        // Test that missing fields are handled correctly
        let json_data = json!({
            "sub": "partial_user",
            "roles": []
        });

        let claims: AuthClaims = serde_json::from_value(json_data).unwrap();
        assert_eq!(claims.sub, Some("partial_user".to_string()));
        assert!(claims.iss.is_none());
        assert!(claims.aud.is_none());
        assert!(claims.exp.is_none());
        assert!(claims.username.is_none());
        assert!(claims.roles.is_empty());
    }

    #[test]
    fn test_auth_claims_custom_nested_values() {
        let mut claims = AuthClaims::new();
        claims.custom.insert(
            "metadata".to_string(),
            json!({
                "tenant": "tenant-001",
                "environment": "production",
                "permissions": {
                    "read": true,
                    "write": false
                }
            }),
        );

        let metadata = claims.custom.get("metadata").unwrap();
        assert_eq!(metadata["tenant"], "tenant-001");
        assert_eq!(metadata["environment"], "production");
        assert_eq!(metadata["permissions"]["read"], true);
        assert_eq!(metadata["permissions"]["write"], false);
    }

    #[test]
    fn test_auth_claims_empty_roles() {
        let claims = AuthClaims::new();
        assert_eq!(claims.roles.len(), 0);
        assert!(claims.roles.is_empty());
    }

    #[test]
    fn test_auth_claims_multiple_roles() {
        let mut claims = AuthClaims::new();
        claims.roles.push("role1".to_string());
        claims.roles.push("role2".to_string());
        claims.roles.push("role3".to_string());

        assert_eq!(claims.roles.len(), 3);
        assert!(claims.roles.contains(&"role1".to_string()));
        assert!(claims.roles.contains(&"role2".to_string()));
        assert!(claims.roles.contains(&"role3".to_string()));
    }

    #[test]
    fn test_auth_claims_custom_empty() {
        let claims = AuthClaims::new();
        assert!(claims.custom.is_empty());
        assert_eq!(claims.custom.len(), 0);
    }

    #[test]
    fn test_auth_claims_custom_multiple_entries() {
        let mut claims = AuthClaims::new();
        claims.custom.insert("key1".to_string(), json!("value1"));
        claims.custom.insert("key2".to_string(), json!(42));
        claims.custom.insert("key3".to_string(), json!(true));

        assert_eq!(claims.custom.len(), 3);
        assert_eq!(claims.custom.get("key1").unwrap(), &json!("value1"));
        assert_eq!(claims.custom.get("key2").unwrap(), &json!(42));
        assert_eq!(claims.custom.get("key3").unwrap(), &json!(true));
    }
}
