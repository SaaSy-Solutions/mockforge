//! Authentication middleware for MockForge HTTP server
//!
//! This module provides comprehensive authentication middleware that automatically
//! validates requests against configured authentication schemes including:
//! - Bearer tokens (including JWT)
//! - Basic authentication
//! - API keys
//! - OAuth2 with token introspection

// Re-export types from mockforge-core for convenience
pub use mockforge_core::config::{
    ApiKeyConfig, AuthConfig, BasicAuthConfig, JwtConfig, OAuth2Config,
};

// Sub-modules
pub mod middleware;
pub mod state;
pub mod types;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
pub mod authenticator;
pub mod oauth2;

// Re-export main types and functions for convenience
pub use authenticator::{authenticate_jwt, authenticate_request};
pub use middleware::auth_middleware;
pub use oauth2::create_oauth2_client;
pub use state::AuthState;
pub use types::{AuthClaims, AuthResult};

#[cfg(test)]
mod tests {
    use super::*;
    use authenticator::{authenticate_api_key, authenticate_basic};
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_authenticate_basic_success() {
        let mut credentials = HashMap::new();
        credentials.insert("admin".to_string(), "password123".to_string());

        let config = AuthConfig {
            basic_auth: Some(BasicAuthConfig { credentials }),
            ..Default::default()
        };

        let state = AuthState {
            config,
            spec: None,
            oauth2_client: None,
            introspection_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let auth_header = "Basic YWRtaW46cGFzc3dvcmQxMjM="; // admin:password123 in base64
        let result = authenticate_basic(&state, auth_header);

        match result {
            Some(AuthResult::Success(claims)) => {
                assert_eq!(claims.username, Some("admin".to_string()));
            }
            _ => panic!("Expected successful authentication"),
        }
    }

    #[test]
    fn test_authenticate_basic_invalid_credentials() {
        let mut credentials = HashMap::new();
        credentials.insert("admin".to_string(), "password123".to_string());

        let config = AuthConfig {
            basic_auth: Some(BasicAuthConfig { credentials }),
            ..Default::default()
        };

        let state = AuthState {
            config,
            spec: None,
            oauth2_client: None,
            introspection_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let auth_header = "Basic d3Jvbmd1c2VyOndyb25ncGFzcw=="; // wronguser:wrongpass in base64
        let result = authenticate_basic(&state, auth_header);

        match result {
            Some(AuthResult::Failure(_)) => {} // Expected
            _ => panic!("Expected authentication failure"),
        }
    }

    #[test]
    fn test_authenticate_basic_invalid_format() {
        let config = AuthConfig {
            basic_auth: Some(BasicAuthConfig {
                credentials: HashMap::new(),
            }),
            ..Default::default()
        };

        let state = AuthState {
            config,
            spec: None,
            oauth2_client: None,
            introspection_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let auth_header = "Basic invalidbase64";
        let result = authenticate_basic(&state, auth_header);

        match result {
            Some(AuthResult::Failure(_)) => {} // Expected
            _ => panic!("Expected authentication failure"),
        }
    }

    #[test]
    fn test_authenticate_api_key_success() {
        let config = AuthConfig {
            api_key: Some(ApiKeyConfig {
                header_name: "X-API-Key".to_string(),
                query_name: None,
                keys: vec!["valid-key-123".to_string()],
            }),
            ..Default::default()
        };

        let state = AuthState {
            config,
            spec: None,
            oauth2_client: None,
            introspection_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let result = authenticate_api_key(&state, "valid-key-123");

        match result {
            Some(AuthResult::Success(claims)) => {
                assert_eq!(
                    claims.custom.get("api_key"),
                    Some(&serde_json::Value::String("valid-key-123".to_string()))
                );
            }
            _ => panic!("Expected successful authentication"),
        }
    }

    #[test]
    fn test_authenticate_api_key_invalid() {
        let config = AuthConfig {
            api_key: Some(ApiKeyConfig {
                header_name: "X-API-Key".to_string(),
                query_name: None,
                keys: vec!["valid-key-123".to_string()],
            }),
            ..Default::default()
        };

        let state = AuthState {
            config,
            spec: None,
            oauth2_client: None,
            introspection_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let result = authenticate_api_key(&state, "invalid-key");

        match result {
            Some(AuthResult::Failure(_)) => {} // Expected
            _ => panic!("Expected authentication failure"),
        }
    }

    #[tokio::test]
    async fn test_authenticate_jwt_hs256_success() {
        let secret = "my-secret-key";
        let config = AuthConfig {
            jwt: Some(JwtConfig {
                secret: Some(secret.to_string()),
                rsa_public_key: None,
                ecdsa_public_key: None,
                issuer: Some("test-issuer".to_string()),
                audience: Some("test-audience".to_string()),
                algorithms: vec!["HS256".to_string()],
            }),
            ..Default::default()
        };

        let state = AuthState {
            config,
            spec: None,
            oauth2_client: None,
            introspection_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Create a test JWT
        let header = Header::new(Algorithm::HS256);
        let claims = json!({
            "sub": "user123",
            "iss": "test-issuer",
            "aud": "test-audience",
            "exp": 2000000000, // Future timestamp
            "iat": 1000000000,
            "username": "testuser"
        });

        let token = encode(&header, &claims, &EncodingKey::from_secret(secret.as_bytes()))
            .expect("Failed to create test JWT");

        let auth_header = format!("Bearer {}", token);
        let result = authenticate_jwt(&state, &auth_header).await;

        match result {
            Some(AuthResult::Success(claims)) => {
                assert_eq!(claims.sub, Some("user123".to_string()));
                assert_eq!(claims.iss, Some("test-issuer".to_string()));
                assert_eq!(claims.aud, Some("test-audience".to_string()));
                assert_eq!(claims.username, Some("testuser".to_string()));
            }
            _ => panic!("Expected successful authentication: {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_authenticate_jwt_expired() {
        let secret = "my-secret-key";
        let config = AuthConfig {
            jwt: Some(JwtConfig {
                secret: Some(secret.to_string()),
                rsa_public_key: None,
                ecdsa_public_key: None,
                issuer: None,
                audience: None,
                algorithms: vec!["HS256".to_string()],
            }),
            ..Default::default()
        };

        let state = AuthState {
            config,
            spec: None,
            oauth2_client: None,
            introspection_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Create an expired JWT
        let header = Header::new(Algorithm::HS256);
        let claims = json!({
            "sub": "user123",
            "exp": 1000000000, // Past timestamp
            "iat": 900000000
        });

        let token = encode(&header, &claims, &EncodingKey::from_secret(secret.as_bytes()))
            .expect("Failed to create test JWT");

        let auth_header = format!("Bearer {}", token);
        let result = authenticate_jwt(&state, &auth_header).await;

        match result {
            Some(AuthResult::Failure(_)) => {} // Expected
            _ => panic!("Expected authentication failure for expired token"),
        }
    }

    #[tokio::test]
    async fn test_authenticate_jwt_invalid_signature() {
        let config = AuthConfig {
            jwt: Some(JwtConfig {
                secret: Some("correct-secret".to_string()),
                rsa_public_key: None,
                ecdsa_public_key: None,
                issuer: None,
                audience: None,
                algorithms: vec!["HS256".to_string()],
            }),
            ..Default::default()
        };

        let state = AuthState {
            config,
            spec: None,
            oauth2_client: None,
            introspection_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Create JWT with wrong secret
        let header = Header::new(Algorithm::HS256);
        let claims = json!({
            "sub": "user123",
            "exp": 2000000000
        });

        let token = encode(&header, &claims, &EncodingKey::from_secret("wrong-secret".as_bytes()))
            .expect("Failed to create test JWT");

        let auth_header = format!("Bearer {}", token);
        let result = authenticate_jwt(&state, &auth_header).await;

        match result {
            Some(AuthResult::Failure(_)) => {} // Expected
            _ => panic!("Expected authentication failure for invalid signature"),
        }
    }

    #[tokio::test]
    async fn test_authenticate_request_no_auth_when_optional() {
        let config = AuthConfig {
            require_auth: false,
            ..Default::default()
        };

        let state = AuthState {
            config,
            spec: None,
            oauth2_client: None,
            introspection_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let result = authenticate_request(&state, &None, &None, &None).await;

        match result {
            AuthResult::None => {} // Expected when auth is optional
            _ => panic!("Expected no authentication required"),
        }
    }

    #[tokio::test]
    async fn test_authenticate_request_no_auth_when_required() {
        let config = AuthConfig {
            require_auth: true,
            ..Default::default()
        };

        let state = AuthState {
            config,
            spec: None,
            oauth2_client: None,
            introspection_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let result = authenticate_request(&state, &None, &None, &None).await;

        match result {
            AuthResult::None => {} // This will be handled by the middleware
            _ => panic!("Expected no authentication provided"),
        }
    }

    #[tokio::test]
    async fn test_authenticate_request_with_valid_api_key() {
        let config = AuthConfig {
            api_key: Some(ApiKeyConfig {
                header_name: "X-API-Key".to_string(),
                query_name: None,
                keys: vec!["valid-key".to_string()],
            }),
            ..Default::default()
        };

        let state = AuthState {
            config,
            spec: None,
            oauth2_client: None,
            introspection_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        let api_key_header = Some("valid-key".to_string());
        let result = authenticate_request(&state, &None, &api_key_header, &None).await;

        match result {
            AuthResult::Success(_) => {} // Expected
            _ => panic!("Expected successful authentication with API key"),
        }
    }

    #[test]
    fn test_create_oauth2_client_success() {
        let config = OAuth2Config {
            client_id: "test-client".to_string(),
            client_secret: "test-secret".to_string(),
            introspection_url: "https://example.com/introspect".to_string(),
            auth_url: Some("https://example.com/auth".to_string()),
            token_url: Some("https://example.com/token".to_string()),
            token_type_hint: Some("access_token".to_string()),
        };

        let result = create_oauth2_client(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_oauth2_client_invalid_url() {
        let config = OAuth2Config {
            client_id: "test-client".to_string(),
            client_secret: "test-secret".to_string(),
            introspection_url: "https://example.com/introspect".to_string(),
            auth_url: Some("not-a-valid-url".to_string()),
            token_url: None,
            token_type_hint: None,
        };

        let result = create_oauth2_client(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert!(!config.require_auth);
        assert!(config.jwt.is_none());
        assert!(config.oauth2.is_none());
        assert!(config.basic_auth.is_none());
        assert!(config.api_key.is_some()); // Default API key config is created
    }

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
}
