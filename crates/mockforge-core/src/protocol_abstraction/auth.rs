//! Unified authentication middleware for all protocols

use super::{Protocol, ProtocolMiddleware, ProtocolRequest, ProtocolResponse};
use crate::config::AuthConfig;
use crate::Result;
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// JWT Claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: Option<usize>,
    pub iat: Option<usize>,
    pub aud: Option<String>,
    pub iss: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Authentication result
#[derive(Debug, Clone)]
pub enum AuthResult {
    /// Authentication successful
    Success(Claims),
    /// Authentication failed
    Failure(String),
    /// Network error during auth
    NetworkError(String),
}

/// Unified authentication middleware
pub struct AuthMiddleware {
    /// Middleware name
    name: String,
    /// Authentication configuration
    config: Arc<AuthConfig>,
    /// Token introspection cache
    introspection_cache: Arc<RwLock<HashMap<String, CachedToken>>>,
}

/// Cached token information
#[derive(Debug, Clone)]
struct CachedToken {
    claims: Claims,
    expires_at: std::time::Instant,
}

impl AuthMiddleware {
    /// Create a new auth middleware
    pub fn new(config: AuthConfig) -> Self {
        Self {
            name: "AuthMiddleware".to_string(),
            config: Arc::new(config),
            introspection_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Extract auth token from request metadata
    fn extract_token(&self, request: &ProtocolRequest) -> Option<String> {
        // Try Authorization header first (works for HTTP, GraphQL, WebSocket)
        if let Some(auth_header) = request.metadata.get("authorization") {
            // Handle "Bearer <token>" format
            if auth_header.starts_with("Bearer ") {
                return Some(auth_header[7..].to_string());
            }
            return Some(auth_header.clone());
        }

        // Try API key header
        if let Some(api_key_config) = &self.config.api_key {
            if let Some(api_key) = request.metadata.get(&api_key_config.header_name) {
                return Some(api_key.clone());
            }
        }

        // For gRPC, try metadata
        if request.protocol == Protocol::Grpc {
            if let Some(token) = request.metadata.get("grpc-metadata-authorization") {
                if token.starts_with("Bearer ") {
                    return Some(token[7..].to_string());
                }
                return Some(token.clone());
            }
        }

        None
    }

    /// Validate JWT token
    async fn validate_jwt(&self, token: &str) -> AuthResult {
        // Check cache first
        if let Some(cached) = self.introspection_cache.read().await.get(token) {
            if cached.expires_at > std::time::Instant::now() {
                return AuthResult::Success(cached.claims.clone());
            }
        }

        // Get JWT configuration
        let jwt_config = match &self.config.jwt {
            Some(config) => config,
            None => return AuthResult::Failure("JWT not configured".to_string()),
        };

        // Decode header to get algorithm
        let header = match decode_header(token) {
            Ok(h) => h,
            Err(e) => return AuthResult::Failure(format!("Invalid token header: {}", e)),
        };

        // Create validation
        let mut validation = Validation::new(header.alg);
        if let Some(audience) = &jwt_config.audience {
            validation.set_audience(&[audience]);
        }
        if let Some(issuer) = &jwt_config.issuer {
            validation.set_issuer(&[issuer]);
        }

        // Get secret
        let secret = match &jwt_config.secret {
            Some(s) => s,
            None => return AuthResult::Failure("JWT secret not configured".to_string()),
        };

        // Decode token
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());
        match decode::<Claims>(token, &decoding_key, &validation) {
            Ok(token_data) => {
                let claims = token_data.claims;

                // Cache the token
                let expires_at = if let Some(exp) = claims.exp {
                    let exp_instant = std::time::UNIX_EPOCH + std::time::Duration::from_secs(exp as u64);
                    std::time::Instant::now() + exp_instant.elapsed().unwrap_or(std::time::Duration::from_secs(300))
                } else {
                    std::time::Instant::now() + std::time::Duration::from_secs(300)
                };

                self.introspection_cache.write().await.insert(
                    token.to_string(),
                    CachedToken {
                        claims: claims.clone(),
                        expires_at,
                    },
                );

                AuthResult::Success(claims)
            }
            Err(e) => AuthResult::Failure(format!("Token validation failed: {}", e)),
        }
    }

    /// Validate API key
    async fn validate_api_key(&self, key: &str) -> AuthResult {
        let api_key_config = match &self.config.api_key {
            Some(config) => config,
            None => return AuthResult::Failure("API key not configured".to_string()),
        };

        // Check if the key is valid
        if api_key_config.keys.contains(&key.to_string()) {
            AuthResult::Success(Claims {
                sub: "api_key_user".to_string(),
                exp: None,
                iat: None,
                aud: None,
                iss: Some("mockforge".to_string()),
                extra: {
                    let mut extra = HashMap::new();
                    extra.insert("auth_type".to_string(), serde_json::json!("api_key"));
                    extra
                },
            })
        } else {
            AuthResult::Failure("Invalid API key".to_string())
        }
    }

    /// Perform authentication
    async fn authenticate(&self, request: &ProtocolRequest) -> AuthResult {
        // Extract token
        let token = match self.extract_token(request) {
            Some(t) => t,
            None => {
                // If no token and auth is not required, allow
                if !self.config.require_auth {
                    return AuthResult::Success(Claims {
                        sub: "anonymous".to_string(),
                        exp: None,
                        iat: None,
                        aud: None,
                        iss: Some("mockforge".to_string()),
                        extra: HashMap::new(),
                    });
                }
                return AuthResult::Failure("No authentication token provided".to_string());
            }
        };

        // Try JWT validation first
        if self.config.jwt.is_some() {
            let result = self.validate_jwt(&token).await;
            if matches!(result, AuthResult::Success(_)) {
                return result;
            }
        }

        // Try API key validation
        if self.config.api_key.is_some() {
            let result = self.validate_api_key(&token).await;
            if matches!(result, AuthResult::Success(_)) {
                return result;
            }
        }

        AuthResult::Failure("Authentication failed".to_string())
    }
}

#[async_trait::async_trait]
impl ProtocolMiddleware for AuthMiddleware {
    fn name(&self) -> &str {
        &self.name
    }

    async fn process_request(&self, request: &mut ProtocolRequest) -> Result<()> {
        // Skip authentication for health checks and admin endpoints
        if request.path.starts_with("/health") || request.path.starts_with("/__mockforge") {
            return Ok(());
        }

        // Perform authentication
        match self.authenticate(request).await {
            AuthResult::Success(claims) => {
                // Add claims to request metadata
                request.metadata.insert("x-auth-sub".to_string(), claims.sub.clone());
                if let Some(iss) = &claims.iss {
                    request.metadata.insert("x-auth-iss".to_string(), iss.clone());
                }
                tracing::debug!(
                    protocol = %request.protocol,
                    user = %claims.sub,
                    "Authentication successful"
                );
                Ok(())
            }
            AuthResult::Failure(reason) => {
                tracing::warn!(
                    protocol = %request.protocol,
                    path = %request.path,
                    reason = %reason,
                    "Authentication failed"
                );
                Err(crate::Error::validation(format!("Authentication failed: {}", reason)))
            }
            AuthResult::NetworkError(reason) => {
                tracing::error!(
                    protocol = %request.protocol,
                    reason = %reason,
                    "Authentication network error"
                );
                Err(crate::Error::validation(format!("Authentication error: {}", reason)))
            }
        }
    }

    async fn process_response(
        &self,
        _request: &ProtocolRequest,
        _response: &mut ProtocolResponse,
    ) -> Result<()> {
        // No post-processing needed for auth
        Ok(())
    }

    fn supports_protocol(&self, _protocol: Protocol) -> bool {
        // Auth middleware supports all protocols
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiKeyConfig;

    #[test]
    fn test_auth_middleware_creation() {
        let config = AuthConfig {
            require_auth: true,
            jwt: None,
            api_key: None,
            oauth2: None,
            basic_auth: None,
        };

        let middleware = AuthMiddleware::new(config);
        assert_eq!(middleware.name(), "AuthMiddleware");
        assert!(middleware.supports_protocol(Protocol::Http));
        assert!(middleware.supports_protocol(Protocol::Grpc));
        assert!(middleware.supports_protocol(Protocol::GraphQL));
    }

    #[test]
    fn test_extract_token_bearer() {
        let config = AuthConfig::default();
        let middleware = AuthMiddleware::new(config);

        let mut metadata = HashMap::new();
        metadata.insert("authorization".to_string(), "Bearer test_token".to_string());

        let request = ProtocolRequest {
            protocol: Protocol::Http,
            operation: "GET".to_string(),
            path: "/test".to_string(),
            metadata,
            body: None,
            client_ip: None,
        };

        let token = middleware.extract_token(&request);
        assert_eq!(token, Some("test_token".to_string()));
    }

    #[test]
    fn test_extract_token_api_key() {
        let config = AuthConfig {
            require_auth: true,
            jwt: None,
            api_key: Some(ApiKeyConfig {
                header_name: "X-API-Key".to_string(),
                query_name: None,
                keys: vec!["test_key".to_string()],
            }),
            oauth2: None,
            basic_auth: None,
        };
        let middleware = AuthMiddleware::new(config);

        let mut metadata = HashMap::new();
        metadata.insert("X-API-Key".to_string(), "test_key".to_string());

        let request = ProtocolRequest {
            protocol: Protocol::Http,
            operation: "GET".to_string(),
            path: "/test".to_string(),
            metadata,
            body: None,
            client_ip: None,
        };

        let token = middleware.extract_token(&request);
        assert_eq!(token, Some("test_key".to_string()));
    }

    #[tokio::test]
    async fn test_validate_api_key_success() {
        let config = AuthConfig {
            require_auth: true,
            jwt: None,
            api_key: Some(ApiKeyConfig {
                header_name: "X-API-Key".to_string(),
                query_name: None,
                keys: vec!["valid_key".to_string()],
            }),
            oauth2: None,
            basic_auth: None,
        };
        let middleware = AuthMiddleware::new(config);

        let result = middleware.validate_api_key("valid_key").await;
        assert!(matches!(result, AuthResult::Success(_)));
    }

    #[tokio::test]
    async fn test_validate_api_key_failure() {
        let config = AuthConfig {
            require_auth: true,
            jwt: None,
            api_key: Some(ApiKeyConfig {
                header_name: "X-API-Key".to_string(),
                query_name: None,
                keys: vec!["valid_key".to_string()],
            }),
            oauth2: None,
            basic_auth: None,
        };
        let middleware = AuthMiddleware::new(config);

        let result = middleware.validate_api_key("invalid_key").await;
        assert!(matches!(result, AuthResult::Failure(_)));
    }
}
