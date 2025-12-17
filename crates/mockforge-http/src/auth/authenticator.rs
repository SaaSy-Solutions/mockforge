//! Authentication methods and logic
//!
//! This module contains the core authentication logic for different
//! authentication schemes: JWT, Basic Auth, OAuth2, and API keys.

use base64::Engine;
use chrono;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde_json::Value;
use tracing::debug;

use super::state::AuthState;
use super::types::{AuthClaims, AuthResult};

/// Authenticate a request using various methods
///
/// Tries authentication methods in priority order:
/// 1. JWT (Bearer token)
/// 2. Basic Auth
/// 3. OAuth2 token introspection
/// 4. API Key
///
/// Returns success on first successful auth, or continues to try other methods.
pub async fn authenticate_request(
    state: &AuthState,
    auth_header: &Option<String>,
    api_key_header: &Option<String>,
    api_key_query: &Option<String>,
) -> AuthResult {
    let mut last_failure: Option<AuthResult> = None;

    // Try JWT/Bearer token first
    if let Some(header) = auth_header {
        if header.starts_with("Bearer ") {
            if let Some(result) = authenticate_jwt(state, header).await {
                if matches!(result, AuthResult::Success(_)) {
                    return result;
                }
                last_failure = Some(result);
            }
        } else if header.starts_with("Basic ") {
            if let Some(result) = authenticate_basic(state, header) {
                if matches!(result, AuthResult::Success(_)) {
                    return result;
                }
                last_failure = Some(result);
            }
        }
    }

    // Try OAuth2 token introspection
    if let Some(header) = auth_header {
        if header.starts_with("Bearer ") {
            if let Some(result) = authenticate_oauth2(state, header).await {
                if matches!(result, AuthResult::Success(_)) {
                    return result;
                }
                last_failure = Some(result);
            }
        }
    }

    // Try API key authentication
    if let Some(api_key) = api_key_header.as_ref().or(api_key_query.as_ref()) {
        if let Some(result) = authenticate_api_key(state, api_key) {
            if matches!(result, AuthResult::Success(_)) {
                return result;
            }
            last_failure = Some(result);
        }
    }

    // Return last failure if any auth was attempted, otherwise None
    last_failure.unwrap_or(AuthResult::None)
}

/// Authenticate using JWT
pub async fn authenticate_jwt(state: &AuthState, auth_header: &str) -> Option<AuthResult> {
    let jwt_config = state.config.jwt.as_ref()?;

    // Extract token from header
    let token = auth_header.strip_prefix("Bearer ")?;

    // Try to decode header to determine algorithm
    let header = match decode_header(token) {
        Ok(h) => h,
        Err(e) => {
            debug!("Failed to decode JWT header: {}", e);
            return Some(AuthResult::Failure("Invalid JWT format".to_string()));
        }
    };

    // Check if algorithm is supported
    let alg_str = match header.alg {
        Algorithm::HS256 => "HS256",
        Algorithm::HS384 => "HS384",
        Algorithm::HS512 => "HS512",
        Algorithm::RS256 => "RS256",
        Algorithm::RS384 => "RS384",
        Algorithm::RS512 => "RS512",
        Algorithm::ES256 => "ES256",
        Algorithm::ES384 => "ES384",
        Algorithm::PS256 => "PS256",
        Algorithm::PS384 => "PS384",
        Algorithm::PS512 => "PS512",
        _ => {
            debug!("Unsupported JWT algorithm: {:?}", header.alg);
            return Some(AuthResult::Failure("Unsupported JWT algorithm".to_string()));
        }
    };

    if !jwt_config.algorithms.is_empty() && !jwt_config.algorithms.contains(&alg_str.to_string()) {
        return Some(AuthResult::Failure(format!("Unsupported algorithm: {}", alg_str)));
    }

    // Create validation
    let mut validation = Validation::new(header.alg);
    if let Some(iss) = &jwt_config.issuer {
        validation.set_issuer(&[iss]);
    }
    if let Some(aud) = &jwt_config.audience {
        validation.set_audience(&[aud]);
    }

    // Create decoding key based on algorithm
    let decoding_key = match header.alg {
        Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
            let secret = jwt_config
                .secret
                .as_ref()
                .ok_or_else(|| AuthResult::Failure("JWT secret not configured".to_string()))
                .ok()?;
            DecodingKey::from_secret(secret.as_bytes())
        }
        Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => {
            let key = jwt_config
                .rsa_public_key
                .as_ref()
                .ok_or_else(|| AuthResult::Failure("RSA public key not configured".to_string()))
                .ok()?;
            DecodingKey::from_rsa_pem(key.as_bytes())
                .map_err(|e| {
                    debug!("Failed to parse RSA key: {}", e);
                    AuthResult::Failure("Invalid RSA key configuration".to_string())
                })
                .ok()?
        }
        Algorithm::ES256 | Algorithm::ES384 => {
            let key = jwt_config
                .ecdsa_public_key
                .as_ref()
                .ok_or_else(|| AuthResult::Failure("ECDSA public key not configured".to_string()))
                .ok()?;
            DecodingKey::from_ec_pem(key.as_bytes())
                .map_err(|e| {
                    debug!("Failed to parse ECDSA key: {}", e);
                    AuthResult::Failure("Invalid ECDSA key configuration".to_string())
                })
                .ok()?
        }
        _ => {
            return Some(AuthResult::Failure("Unsupported algorithm".to_string()));
        }
    };

    // Decode and validate token
    match decode::<Value>(token, &decoding_key, &validation) {
        Ok(token_data) => {
            let claims = token_data.claims;
            let mut auth_claims = AuthClaims::new();

            // Extract standard claims
            if let Some(sub) = claims.get("sub").and_then(|v| v.as_str()) {
                auth_claims.sub = Some(sub.to_string());
            }
            if let Some(iss) = claims.get("iss").and_then(|v| v.as_str()) {
                auth_claims.iss = Some(iss.to_string());
            }
            if let Some(aud) = claims.get("aud").and_then(|v| v.as_str()) {
                auth_claims.aud = Some(aud.to_string());
            }
            if let Some(exp) = claims.get("exp").and_then(|v| v.as_i64()) {
                auth_claims.exp = Some(exp);
            }
            if let Some(iat) = claims.get("iat").and_then(|v| v.as_i64()) {
                auth_claims.iat = Some(iat);
            }
            if let Some(username) = claims
                .get("username")
                .or_else(|| claims.get("preferred_username"))
                .and_then(|v| v.as_str())
            {
                auth_claims.username = Some(username.to_string());
            }

            // Extract roles
            if let Some(roles) = claims.get("roles").and_then(|v| v.as_array()) {
                for role in roles {
                    if let Some(role_str) = role.as_str() {
                        auth_claims.roles.push(role_str.to_string());
                    }
                }
            }

            // Store custom claims
            for (key, value) in claims.as_object()? {
                if ![
                    "sub",
                    "iss",
                    "aud",
                    "exp",
                    "iat",
                    "username",
                    "preferred_username",
                    "roles",
                ]
                .contains(&key.as_str())
                {
                    auth_claims.custom.insert(key.clone(), value.clone());
                }
            }

            Some(AuthResult::Success(auth_claims))
        }
        Err(e) => {
            debug!("JWT validation failed: {}", e);
            Some(AuthResult::Failure(format!("Invalid JWT token: {}", e)))
        }
    }
}

/// Authenticate using Basic Auth
pub fn authenticate_basic(state: &AuthState, auth_header: &str) -> Option<AuthResult> {
    let basic_config = state.config.basic_auth.as_ref()?;

    // Extract credentials from header
    let encoded = auth_header.strip_prefix("Basic ")?;
    let decoded = match base64::engine::general_purpose::STANDARD.decode(encoded) {
        Ok(d) => d,
        Err(_) => return Some(AuthResult::Failure("Invalid base64 in Basic auth".to_string())),
    };
    let credentials = match String::from_utf8(decoded) {
        Ok(c) => c,
        Err(_) => {
            return Some(AuthResult::Failure("Invalid UTF-8 in Basic auth credentials".to_string()))
        }
    };
    let parts: Vec<&str> = credentials.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Some(AuthResult::Failure("Invalid Basic auth format".to_string()));
    }

    let username = parts[0];
    let password = parts[1];

    // Check credentials
    if let Some(expected_password) = basic_config.credentials.get(username) {
        if expected_password == password {
            let mut claims = AuthClaims::new();
            claims.username = Some(username.to_string());
            return Some(AuthResult::Success(claims));
        }
    }

    Some(AuthResult::Failure("Invalid credentials".to_string()))
}

/// Authenticate using OAuth2 token introspection
async fn authenticate_oauth2(state: &AuthState, auth_header: &str) -> Option<AuthResult> {
    let oauth2_config = state.config.oauth2.as_ref()?;

    // Extract token
    let token = auth_header.strip_prefix("Bearer ")?;

    // Check cache first
    {
        let cache = state.introspection_cache.read().await;
        if let Some(cached) = cache.get(token) {
            let now = chrono::Utc::now().timestamp();
            if cached.expires_at > now {
                return Some(cached.result.clone());
            }
        }
    }

    // Perform token introspection
    let client = reqwest::Client::new();
    let response = match client
        .post(&oauth2_config.introspection_url)
        .basic_auth(&oauth2_config.client_id, Some(&oauth2_config.client_secret))
        .form(&[
            ("token", token),
            (
                "token_type_hint",
                oauth2_config.token_type_hint.as_deref().unwrap_or("access_token"),
            ),
        ])
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            debug!("Network error during OAuth2 introspection: {}", e);
            return Some(AuthResult::NetworkError(format!(
                "Failed to connect to introspection endpoint: {}",
                e
            )));
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        debug!("OAuth2 introspection server error: {}", status);
        return Some(AuthResult::ServerError(format!(
            "Introspection endpoint returned {}: {}",
            status,
            status.canonical_reason().unwrap_or("Unknown error")
        )));
    }

    let introspection_result: Value = match response.json().await {
        Ok(json) => json,
        Err(e) => {
            debug!("Failed to parse introspection response: {}", e);
            return Some(AuthResult::ServerError(format!(
                "Invalid JSON response from introspection endpoint: {}",
                e
            )));
        }
    };

    // Check if token is active
    let active = introspection_result.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
    if !active {
        let cached_result = AuthResult::TokenInvalid("Token is not active".to_string());
        // Cache inactive tokens for a shorter time to avoid repeated checks
        let expires_at = chrono::Utc::now().timestamp() + 300; // 5 minutes
        let cached = super::state::CachedIntrospection {
            result: cached_result.clone(),
            expires_at,
        };
        let mut cache = state.introspection_cache.write().await;
        cache.insert(token.to_string(), cached);
        return Some(cached_result);
    }

    // Check if token is expired
    if let Some(exp) = introspection_result.get("exp").and_then(|v| v.as_i64()) {
        let now = chrono::Utc::now().timestamp();
        if exp <= now {
            let cached_result = AuthResult::TokenExpired;
            // Cache expired tokens for a short time
            let expires_at = chrono::Utc::now().timestamp() + 60; // 1 minute
            let cached = super::state::CachedIntrospection {
                result: cached_result.clone(),
                expires_at,
            };
            let mut cache = state.introspection_cache.write().await;
            cache.insert(token.to_string(), cached);
            return Some(cached_result);
        }
    }

    // Extract claims from introspection response
    let mut claims = AuthClaims::new();
    if let Some(sub) = introspection_result.get("sub").and_then(|v| v.as_str()) {
        claims.sub = Some(sub.to_string());
    }
    if let Some(username) = introspection_result.get("username").and_then(|v| v.as_str()) {
        claims.username = Some(username.to_string());
    }
    if let Some(exp) = introspection_result.get("exp").and_then(|v| v.as_i64()) {
        claims.exp = Some(exp);
    }

    // Cache successful result - use token expiration or default to 1 hour
    let expires_at = claims.exp.unwrap_or(chrono::Utc::now().timestamp() + 3600);
    let cached_result = AuthResult::Success(claims);
    let cached = super::state::CachedIntrospection {
        result: cached_result.clone(),
        expires_at,
    };
    let mut cache = state.introspection_cache.write().await;
    cache.insert(token.to_string(), cached);

    Some(cached_result)
}

/// Authenticate using API key
pub fn authenticate_api_key(state: &AuthState, api_key: &str) -> Option<AuthResult> {
    let api_key_config = state.config.api_key.as_ref()?;

    if api_key_config.keys.contains(&api_key.to_string()) {
        let mut claims = AuthClaims::new();
        claims.custom.insert("api_key".to_string(), Value::String(api_key.to_string()));
        Some(AuthResult::Success(claims))
    } else {
        Some(AuthResult::Failure("Invalid API key".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::AuthState;
    use super::*;
    use base64::Engine;
    use mockforge_core::config::{ApiKeyConfig, AuthConfig, BasicAuthConfig, JwtConfig};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn create_auth_state(config: AuthConfig) -> AuthState {
        AuthState {
            config,
            spec: None,
            oauth2_client: None,
            introspection_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn create_test_auth_state_with_jwt() -> AuthState {
        let jwt_config = JwtConfig {
            secret: Some("test-secret-key-for-jwt-authentication".to_string()),
            rsa_public_key: None,
            ecdsa_public_key: None,
            algorithms: vec!["HS256".to_string()],
            issuer: Some("test-issuer".to_string()),
            audience: Some("test-audience".to_string()),
        };

        let auth_config = AuthConfig {
            jwt: Some(jwt_config),
            basic_auth: None,
            oauth2: None,
            api_key: None,
            require_auth: false,
        };

        create_auth_state(auth_config)
    }

    fn create_test_auth_state_with_basic() -> AuthState {
        let mut credentials = HashMap::new();
        credentials.insert("testuser".to_string(), "testpass".to_string());
        credentials.insert("admin".to_string(), "admin123".to_string());

        let basic_config = BasicAuthConfig { credentials };

        let auth_config = AuthConfig {
            jwt: None,
            basic_auth: Some(basic_config),
            oauth2: None,
            api_key: None,
            require_auth: false,
        };

        create_auth_state(auth_config)
    }

    fn create_test_auth_state_with_api_key() -> AuthState {
        let api_key_config = ApiKeyConfig {
            header_name: "X-API-Key".to_string(),
            query_name: None,
            keys: vec![
                "valid-api-key-123".to_string(),
                "another-valid-key-456".to_string(),
            ],
        };

        let auth_config = AuthConfig {
            jwt: None,
            basic_auth: None,
            oauth2: None,
            api_key: Some(api_key_config),
            require_auth: false,
        };

        create_auth_state(auth_config)
    }

    #[tokio::test]
    async fn test_authenticate_request_no_auth() {
        let state = create_auth_state(AuthConfig {
            jwt: None,
            basic_auth: None,
            oauth2: None,
            api_key: None,
            require_auth: false,
        });

        let result = authenticate_request(&state, &None, &None, &None).await;
        assert!(matches!(result, AuthResult::None));
    }

    #[tokio::test]
    async fn test_authenticate_jwt_valid() {
        use jsonwebtoken::{encode, EncodingKey, Header};
        use serde_json::json;

        let state = create_test_auth_state_with_jwt();

        // Create a valid JWT token
        let mut claims = serde_json::Map::new();
        claims.insert("sub".to_string(), json!("user123"));
        claims.insert("iss".to_string(), json!("test-issuer"));
        claims.insert("aud".to_string(), json!("test-audience"));
        claims.insert(
            "exp".to_string(),
            json!((chrono::Utc::now() + chrono::Duration::hours(1)).timestamp()),
        );

        let secret = "test-secret-key-for-jwt-authentication";
        let token =
            encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
                .unwrap();

        let auth_header = format!("Bearer {}", token);
        let result = authenticate_jwt(&state, &auth_header).await;

        assert!(result.is_some());
        match result.unwrap() {
            AuthResult::Success(claims) => {
                assert_eq!(claims.sub, Some("user123".to_string()));
                assert_eq!(claims.iss, Some("test-issuer".to_string()));
                assert_eq!(claims.aud, Some("test-audience".to_string()));
            }
            _ => panic!("Expected successful authentication"),
        }
    }

    #[tokio::test]
    async fn test_authenticate_jwt_invalid_format() {
        let state = create_test_auth_state_with_jwt();
        let auth_header = "Bearer invalid-token-format";
        let result = authenticate_jwt(&state, &auth_header).await;

        assert!(result.is_some());
        assert!(matches!(result.unwrap(), AuthResult::Failure(_)));
    }

    #[tokio::test]
    async fn test_authenticate_jwt_expired() {
        use jsonwebtoken::{encode, EncodingKey, Header};
        use serde_json::json;

        let state = create_test_auth_state_with_jwt();

        // Create an expired JWT token
        let mut claims = serde_json::Map::new();
        claims.insert("sub".to_string(), json!("user123"));
        claims.insert("iss".to_string(), json!("test-issuer"));
        claims.insert("aud".to_string(), json!("test-audience"));
        claims.insert(
            "exp".to_string(),
            json!((chrono::Utc::now() - chrono::Duration::hours(1)).timestamp()),
        );

        let secret = "test-secret-key-for-jwt-authentication";
        let token =
            encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
                .unwrap();

        let auth_header = format!("Bearer {}", token);
        let result = authenticate_jwt(&state, &auth_header).await;

        assert!(result.is_some());
        assert!(matches!(result.unwrap(), AuthResult::Failure(_)));
    }

    #[tokio::test]
    async fn test_authenticate_jwt_wrong_issuer() {
        use jsonwebtoken::{encode, EncodingKey, Header};
        use serde_json::json;

        let state = create_test_auth_state_with_jwt();

        // Create a token with wrong issuer
        let mut claims = serde_json::Map::new();
        claims.insert("sub".to_string(), json!("user123"));
        claims.insert("iss".to_string(), json!("wrong-issuer"));
        claims.insert("aud".to_string(), json!("test-audience"));
        claims.insert(
            "exp".to_string(),
            json!((chrono::Utc::now() + chrono::Duration::hours(1)).timestamp()),
        );

        let secret = "test-secret-key-for-jwt-authentication";
        let token =
            encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
                .unwrap();

        let auth_header = format!("Bearer {}", token);
        let result = authenticate_jwt(&state, &auth_header).await;

        assert!(result.is_some());
        assert!(matches!(result.unwrap(), AuthResult::Failure(_)));
    }

    #[tokio::test]
    async fn test_authenticate_jwt_with_roles() {
        use jsonwebtoken::{encode, EncodingKey, Header};
        use serde_json::json;

        let state = create_test_auth_state_with_jwt();

        let mut claims = serde_json::Map::new();
        claims.insert("sub".to_string(), json!("user123"));
        claims.insert("iss".to_string(), json!("test-issuer"));
        claims.insert("aud".to_string(), json!("test-audience"));
        claims.insert(
            "exp".to_string(),
            json!((chrono::Utc::now() + chrono::Duration::hours(1)).timestamp()),
        );
        claims.insert("roles".to_string(), json!(["admin", "user"]));
        claims.insert("username".to_string(), json!("testuser"));

        let secret = "test-secret-key-for-jwt-authentication";
        let token =
            encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
                .unwrap();

        let auth_header = format!("Bearer {}", token);
        let result = authenticate_jwt(&state, &auth_header).await;

        assert!(result.is_some());
        match result.unwrap() {
            AuthResult::Success(claims) => {
                assert_eq!(claims.username, Some("testuser".to_string()));
                assert_eq!(claims.roles, vec!["admin", "user"]);
            }
            _ => panic!("Expected successful authentication"),
        }
    }

    #[tokio::test]
    async fn test_authenticate_jwt_no_config() {
        let state = create_auth_state(AuthConfig {
            jwt: None,
            basic_auth: None,
            oauth2: None,
            api_key: None,
            require_auth: false,
        });

        let auth_header = "Bearer some-token";
        let result = authenticate_jwt(&state, &auth_header).await;

        assert!(result.is_none());
    }

    #[test]
    fn test_authenticate_basic_valid() {
        let state = create_test_auth_state_with_basic();

        // Encode "testuser:testpass" in base64
        let credentials = base64::engine::general_purpose::STANDARD.encode(b"testuser:testpass");
        let auth_header = format!("Basic {}", credentials);

        let result = authenticate_basic(&state, &auth_header);

        assert!(result.is_some());
        match result.unwrap() {
            AuthResult::Success(claims) => {
                assert_eq!(claims.username, Some("testuser".to_string()));
            }
            _ => panic!("Expected successful authentication"),
        }
    }

    #[test]
    fn test_authenticate_basic_invalid_credentials() {
        let state = create_test_auth_state_with_basic();

        let credentials = base64::engine::general_purpose::STANDARD.encode(b"testuser:wrongpass");
        let auth_header = format!("Basic {}", credentials);

        let result = authenticate_basic(&state, &auth_header);

        assert!(result.is_some());
        assert!(matches!(result.unwrap(), AuthResult::Failure(_)));
    }

    #[test]
    fn test_authenticate_basic_invalid_format() {
        let state = create_test_auth_state_with_basic();

        // Invalid base64
        let auth_header = "Basic invalid-base64!!!";

        let result = authenticate_basic(&state, &auth_header);

        assert!(result.is_some());
        assert!(matches!(result.unwrap(), AuthResult::Failure(_)));
    }

    #[test]
    fn test_authenticate_basic_missing_colon() {
        let state = create_test_auth_state_with_basic();

        // Encode credentials without colon
        let credentials =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"testuser");
        let auth_header = format!("Basic {}", credentials);

        let result = authenticate_basic(&state, &auth_header);

        assert!(result.is_some());
        assert!(matches!(result.unwrap(), AuthResult::Failure(_)));
    }

    #[test]
    fn test_authenticate_basic_no_config() {
        let state = create_auth_state(AuthConfig {
            jwt: None,
            basic_auth: None,
            oauth2: None,
            api_key: None,
            require_auth: false,
        });

        let credentials = base64::engine::general_purpose::STANDARD.encode(b"user:pass");
        let auth_header = format!("Basic {}", credentials);

        let result = authenticate_basic(&state, &auth_header);

        assert!(result.is_none());
    }

    #[test]
    fn test_authenticate_api_key_valid() {
        let state = create_test_auth_state_with_api_key();

        let result = authenticate_api_key(&state, "valid-api-key-123");

        assert!(result.is_some());
        match result.unwrap() {
            AuthResult::Success(claims) => {
                assert_eq!(
                    claims.custom.get("api_key").and_then(|v| v.as_str()),
                    Some("valid-api-key-123")
                );
            }
            _ => panic!("Expected successful authentication"),
        }
    }

    #[test]
    fn test_authenticate_api_key_invalid() {
        let state = create_test_auth_state_with_api_key();

        let result = authenticate_api_key(&state, "invalid-key");

        assert!(result.is_some());
        assert!(matches!(result.unwrap(), AuthResult::Failure(_)));
    }

    #[test]
    fn test_authenticate_api_key_no_config() {
        let state = create_auth_state(AuthConfig {
            jwt: None,
            basic_auth: None,
            oauth2: None,
            api_key: None,
            require_auth: false,
        });

        let result = authenticate_api_key(&state, "some-key");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_authenticate_request_with_bearer_token() {
        use jsonwebtoken::{encode, EncodingKey, Header};
        use serde_json::json;

        let state = create_test_auth_state_with_jwt();

        let mut claims = serde_json::Map::new();
        claims.insert("sub".to_string(), json!("user123"));
        claims.insert("iss".to_string(), json!("test-issuer"));
        claims.insert("aud".to_string(), json!("test-audience"));
        claims.insert(
            "exp".to_string(),
            json!((chrono::Utc::now() + chrono::Duration::hours(1)).timestamp()),
        );

        let secret = "test-secret-key-for-jwt-authentication";
        let token =
            encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
                .unwrap();

        let auth_header = Some(format!("Bearer {}", token));
        let result = authenticate_request(&state, &auth_header, &None, &None).await;

        assert!(matches!(result, AuthResult::Success(_)));
    }

    #[tokio::test]
    async fn test_authenticate_request_with_basic_auth() {
        let state = create_test_auth_state_with_basic();

        let credentials = base64::engine::general_purpose::STANDARD.encode(b"testuser:testpass");
        let auth_header = Some(format!("Basic {}", credentials));

        let result = authenticate_request(&state, &auth_header, &None, &None).await;

        assert!(matches!(result, AuthResult::Success(_)));
    }

    #[tokio::test]
    async fn test_authenticate_request_with_api_key_header() {
        let state = create_test_auth_state_with_api_key();

        let result =
            authenticate_request(&state, &None, &Some("valid-api-key-123".to_string()), &None)
                .await;

        assert!(matches!(result, AuthResult::Success(_)));
    }

    #[tokio::test]
    async fn test_authenticate_request_with_api_key_query() {
        let state = create_test_auth_state_with_api_key();

        let result =
            authenticate_request(&state, &None, &None, &Some("another-valid-key-456".to_string()))
                .await;

        assert!(matches!(result, AuthResult::Success(_)));
    }

    #[tokio::test]
    async fn test_authenticate_request_priority() {
        // JWT should be tried first, then basic auth, then API key
        let jwt_config = JwtConfig {
            secret: Some("test-secret".to_string()),
            rsa_public_key: None,
            ecdsa_public_key: None,
            algorithms: vec!["HS256".to_string()],
            issuer: None,
            audience: None,
        };

        let api_key_config = ApiKeyConfig {
            header_name: "X-API-Key".to_string(),
            query_name: None,
            keys: vec!["valid-key".to_string()],
        };

        let state = create_auth_state(AuthConfig {
            jwt: Some(jwt_config),
            basic_auth: None,
            oauth2: None,
            api_key: Some(api_key_config),
            require_auth: false,
        });

        // Provide both invalid JWT and valid API key
        let auth_header = Some("Bearer invalid-token".to_string());
        let api_key = Some("valid-key".to_string());

        let result = authenticate_request(&state, &auth_header, &api_key, &None).await;

        // Should try JWT first and fail, then try API key and succeed
        assert!(matches!(result, AuthResult::Success(_)));
    }

    #[tokio::test]
    async fn test_authenticate_jwt_with_custom_claims() {
        use jsonwebtoken::{encode, EncodingKey, Header};
        use serde_json::json;

        let state = create_test_auth_state_with_jwt();

        let mut claims = serde_json::Map::new();
        claims.insert("sub".to_string(), json!("user123"));
        claims.insert("iss".to_string(), json!("test-issuer"));
        claims.insert("aud".to_string(), json!("test-audience"));
        claims.insert(
            "exp".to_string(),
            json!((chrono::Utc::now() + chrono::Duration::hours(1)).timestamp()),
        );
        claims.insert("custom_field".to_string(), json!("custom_value"));
        claims.insert("department".to_string(), json!("engineering"));

        let secret = "test-secret-key-for-jwt-authentication";
        let token =
            encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
                .unwrap();

        let auth_header = format!("Bearer {}", token);
        let result = authenticate_jwt(&state, &auth_header).await;

        assert!(result.is_some());
        match result.unwrap() {
            AuthResult::Success(claims) => {
                assert_eq!(
                    claims.custom.get("custom_field").and_then(|v| v.as_str()),
                    Some("custom_value")
                );
                assert_eq!(
                    claims.custom.get("department").and_then(|v| v.as_str()),
                    Some("engineering")
                );
            }
            _ => panic!("Expected successful authentication"),
        }
    }

    #[tokio::test]
    async fn test_authenticate_jwt_with_preferred_username() {
        use jsonwebtoken::{encode, EncodingKey, Header};
        use serde_json::json;

        let state = create_test_auth_state_with_jwt();

        let mut claims = serde_json::Map::new();
        claims.insert("sub".to_string(), json!("user123"));
        claims.insert("iss".to_string(), json!("test-issuer"));
        claims.insert("aud".to_string(), json!("test-audience"));
        claims.insert(
            "exp".to_string(),
            json!((chrono::Utc::now() + chrono::Duration::hours(1)).timestamp()),
        );
        claims.insert("preferred_username".to_string(), json!("preferred_user"));

        let secret = "test-secret-key-for-jwt-authentication";
        let token =
            encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
                .unwrap();

        let auth_header = format!("Bearer {}", token);
        let result = authenticate_jwt(&state, &auth_header).await;

        assert!(result.is_some());
        match result.unwrap() {
            AuthResult::Success(claims) => {
                assert_eq!(claims.username, Some("preferred_user".to_string()));
            }
            _ => panic!("Expected successful authentication"),
        }
    }

    #[test]
    fn test_authenticate_basic_multiple_users() {
        let state = create_test_auth_state_with_basic();

        // Test first user
        let creds1 = base64::engine::general_purpose::STANDARD.encode(b"testuser:testpass");
        let result1 = authenticate_basic(&state, &format!("Basic {}", creds1));
        assert!(matches!(result1.unwrap(), AuthResult::Success(_)));

        // Test second user
        let creds2 = base64::engine::general_purpose::STANDARD.encode(b"admin:admin123");
        let result2 = authenticate_basic(&state, &format!("Basic {}", creds2));
        assert!(matches!(result2.unwrap(), AuthResult::Success(_)));
    }

    #[test]
    fn test_authenticate_basic_user_not_found() {
        let state = create_test_auth_state_with_basic();

        let credentials = base64::engine::general_purpose::STANDARD.encode(b"nonexistent:password");
        let auth_header = format!("Basic {}", credentials);

        let result = authenticate_basic(&state, &auth_header);

        assert!(result.is_some());
        assert!(matches!(result.unwrap(), AuthResult::Failure(_)));
    }
}
