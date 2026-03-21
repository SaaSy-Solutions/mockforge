//! Authentication configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Authentication configuration for HTTP requests
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AuthConfig {
    /// JWT configuration
    pub jwt: Option<JwtConfig>,
    /// OAuth2 configuration
    pub oauth2: Option<OAuth2Config>,
    /// Basic auth configuration
    pub basic_auth: Option<BasicAuthConfig>,
    /// API key configuration
    pub api_key: Option<ApiKeyConfig>,
    /// Whether to require authentication for all requests
    pub require_auth: bool,
}

/// JWT authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct JwtConfig {
    /// JWT secret key for HMAC algorithms
    pub secret: Option<String>,
    /// RSA public key PEM for RSA algorithms
    pub rsa_public_key: Option<String>,
    /// ECDSA public key PEM for ECDSA algorithms
    pub ecdsa_public_key: Option<String>,
    /// Expected issuer
    pub issuer: Option<String>,
    /// Expected audience
    pub audience: Option<String>,
    /// Supported algorithms (defaults to HS256, RS256, ES256)
    pub algorithms: Vec<String>,
}

/// OAuth2 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct OAuth2Config {
    /// OAuth2 client ID
    pub client_id: String,
    /// OAuth2 client secret
    pub client_secret: String,
    /// Token introspection URL
    pub introspection_url: String,
    /// Authorization server URL
    pub auth_url: Option<String>,
    /// Token URL
    pub token_url: Option<String>,
    /// Expected token type
    pub token_type_hint: Option<String>,
}

/// Basic authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BasicAuthConfig {
    /// Username/password pairs
    pub credentials: HashMap<String, String>,
}

/// API key configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ApiKeyConfig {
    /// Expected header name (default: X-API-Key)
    pub header_name: String,
    /// Expected query parameter name
    pub query_name: Option<String>,
    /// Valid API keys
    pub keys: Vec<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt: None,
            oauth2: None,
            basic_auth: None,
            api_key: Some(ApiKeyConfig {
                header_name: "X-API-Key".to_string(),
                query_name: None,
                keys: vec![],
            }),
            require_auth: false,
        }
    }
}
