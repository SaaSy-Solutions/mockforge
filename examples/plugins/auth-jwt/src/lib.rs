//! JWT Authentication Plugin for MockForge
//!
//! This plugin provides JWT-based authentication with configurable
//! token validation, claims extraction, and user role management.

use mockforge_plugin_core::*;
use serde::{Deserialize, Serialize};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use std::collections::HashMap;

/// JWT Authentication Plugin
#[derive(Debug)]
pub struct JwtAuthPlugin {
    config: JwtConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// Public key or secret for token verification
    pub verification_key: String,
    /// Supported algorithms (defaults to HS256, RS256)
    pub algorithms: Vec<String>,
    /// Required issuer (optional)
    pub required_issuer: Option<String>,
    /// Required audience (optional)
    pub required_audience: Option<String>,
    /// Clock skew tolerance in seconds
    pub clock_skew_seconds: u64,
    /// Custom claims to extract
    pub extract_claims: Vec<String>,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            verification_key: "your-secret-key".to_string(),
            algorithms: vec!["HS256".to_string(), "RS256".to_string()],
            required_issuer: None,
            required_audience: None,
            clock_skew_seconds: 300, // 5 minutes
            extract_claims: vec![
                "sub".to_string(),
                "email".to_string(),
                "roles".to_string(),
                "permissions".to_string(),
            ],
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    /// Subject (user ID)
    sub: Option<String>,
    /// Issuer
    iss: Option<String>,
    /// Audience
    aud: Option<String>,
    /// Expiration time
    exp: Option<u64>,
    /// Issued at
    iat: Option<u64>,
    /// Not before
    nbf: Option<u64>,
    /// Email
    email: Option<String>,
    /// Roles
    roles: Option<Vec<String>>,
    /// Permissions
    permissions: Option<Vec<String>>,
    /// Custom claims
    #[serde(flatten)]
    custom: HashMap<String, serde_json::Value>,
}

impl JwtAuthPlugin {
    pub fn new() -> Self {
        Self {
            config: JwtConfig::default(),
        }
    }

    /// Validate JWT token and extract claims
    fn validate_token(&self, token: &str) -> PluginResult<JwtClaims> {
        // Decode header to determine algorithm
        let header = decode_header(token).map_err(|e| {
            PluginError::invalid_input(format!("Invalid JWT header: {}", e))
        })?;

        let algorithm = match header.alg {
            Algorithm::HS256 => "HS256",
            Algorithm::RS256 => "RS256",
            Algorithm::ES256 => "ES256",
            _ => return PluginResult::failure(
                format!("Unsupported algorithm: {:?}", header.alg),
                0
            ),
        };

        // Check if algorithm is supported
        if !self.config.algorithms.contains(&algorithm.to_string()) {
            return PluginResult::failure(
                format!("Algorithm not supported: {}", algorithm),
                0
            );
        }

        // Create validation rules
        let mut validation = Validation::new(header.alg);
        validation.leeway = self.config.clock_skew_seconds;

        if let Some(iss) = &self.config.required_issuer {
            validation.set_issuer(&[iss]);
        }

        if let Some(aud) = &self.config.required_audience {
            validation.set_audience(&[aud]);
        }

        // Create decoding key
        let key = if algorithm.starts_with("HS") {
            DecodingKey::from_secret(self.config.verification_key.as_bytes())
        } else {
            DecodingKey::from_rsa_pem(self.config.verification_key.as_bytes())
                .map_err(|e| PluginError::config(format!("Invalid RSA key: {}", e)))?
        };

        // Decode and validate token
        let token_data = decode::<JwtClaims>(token, &key, &validation)
            .map_err(|e| PluginError::invalid_input(format!("Token validation failed: {}", e)))?;

        PluginResult::success(token_data.claims)
    }

    /// Extract user information from claims
    fn extract_user_info(&self, claims: &JwtClaims) -> HashMap<String, serde_json::Value> {
        let mut user_info = HashMap::new();

        // Extract standard claims
        if let Some(sub) = &claims.sub {
            user_info.insert("user_id".to_string(), serde_json::json!(sub));
        }

        if let Some(email) = &claims.email {
            user_info.insert("email".to_string(), serde_json::json!(email));
        }

        if let Some(roles) = &claims.roles {
            user_info.insert("roles".to_string(), serde_json::json!(roles));
        }

        if let Some(permissions) = &claims.permissions {
            user_info.insert("permissions".to_string(), serde_json::json!(permissions));
        }

        // Extract custom claims
        for claim_name in &self.config.extract_claims {
            if let Some(value) = claims.custom.get(claim_name) {
                user_info.insert(claim_name.clone(), value.clone());
            }
        }

        user_info
    }
}

#[async_trait::async_trait]
impl AuthPlugin for JwtAuthPlugin {
    async fn authenticate(
        &self,
        _context: &PluginContext,
        credentials: &AuthCredentials,
    ) -> PluginResult<AuthResult> {
        // Extract Bearer token
        let token = match credentials {
            AuthCredentials::Bearer(token) => token,
            _ => {
                return PluginResult::failure(
                    "JWT authentication requires Bearer token".to_string(),
                    0
                );
            }
        };

        // Validate token
        let claims = match self.validate_token(token) {
            PluginResult { success: true, data: Some(claims), .. } => claims,
            PluginResult { success: false, error: Some(err), .. } => {
                return PluginResult::failure(format!("Token validation failed: {}", err), 0);
            }
            _ => {
                return PluginResult::failure("Token validation failed".to_string(), 0);
            }
        };

        // Extract user ID
        let user_id = claims.sub.ok_or_else(|| {
            PluginError::invalid_input("Token missing subject claim")
        })?;

        // Extract additional user information
        let claims_data = self.extract_user_info(&claims);

        PluginResult::success(AuthResult::Authenticated {
            user_id,
            claims: claims_data,
        })
    }

    fn get_capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            network: NetworkCapabilities::default(), // No network access needed
            filesystem: FilesystemCapabilities::default(), // No filesystem access needed
            resources: PluginResources {
                max_memory_bytes: 16 * 1024 * 1024, // 16MB
                max_cpu_time_ms: 1000, // 1 second
            },
        }
    }
}

mockforge_plugin_core::export_plugin!(JwtAuthPlugin);
