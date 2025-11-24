//! OpenID Connect (OIDC) simulation support
//!
//! This module provides OIDC-compliant endpoints for simulating identity providers,
//! including discovery documents and JSON Web Key Set (JWKS) endpoints.

use axum::response::Json;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use mockforge_core::Error;

/// OIDC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    /// Whether OIDC is enabled
    pub enabled: bool,
    /// Issuer identifier (e.g., "https://mockforge.example.com")
    pub issuer: String,
    /// JWKS configuration
    pub jwks: JwksConfig,
    /// Default claims to include in tokens
    pub claims: ClaimsConfig,
    /// Multi-tenant configuration
    pub multi_tenant: Option<MultiTenantConfig>,
}

/// JWKS (JSON Web Key Set) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwksConfig {
    /// List of signing keys
    pub keys: Vec<JwkKey>,
}

/// JSON Web Key configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwkKey {
    /// Key ID
    pub kid: String,
    /// Algorithm (RS256, ES256, HS256, etc.)
    pub alg: String,
    /// Public key (PEM format for RSA/ECDSA, base64 for HMAC)
    pub public_key: String,
    /// Private key (PEM format for RSA/ECDSA, base64 for HMAC) - optional, used for signing
    #[serde(skip_serializing)]
    pub private_key: Option<String>,
    /// Key type (RSA, EC, oct)
    pub kty: String,
    /// Key use (sig, enc)
    #[serde(default = "default_key_use")]
    pub use_: String,
}

fn default_key_use() -> String {
    "sig".to_string()
}

/// Claims configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimsConfig {
    /// Default claims to include in all tokens
    pub default: Vec<String>,
    /// Custom claim templates
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

/// Multi-tenant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiTenantConfig {
    /// Whether multi-tenant mode is enabled
    pub enabled: bool,
    /// Claim name for organization ID
    pub org_id_claim: String,
    /// Claim name for tenant ID
    pub tenant_id_claim: Option<String>,
}

impl Default for OidcConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            issuer: "https://mockforge.example.com".to_string(),
            jwks: JwksConfig { keys: vec![] },
            claims: ClaimsConfig {
                default: vec!["sub".to_string(), "iss".to_string(), "exp".to_string()],
                custom: HashMap::new(),
            },
            multi_tenant: None,
        }
    }
}

/// OIDC discovery document response
#[derive(Debug, Serialize)]
pub struct OidcDiscoveryDocument {
    /// Issuer identifier
    pub issuer: String,
    /// Authorization endpoint
    pub authorization_endpoint: String,
    /// Token endpoint
    pub token_endpoint: String,
    /// UserInfo endpoint
    pub userinfo_endpoint: String,
    /// JWKS URI
    pub jwks_uri: String,
    /// Supported response types
    pub response_types_supported: Vec<String>,
    /// Supported subject types
    pub subject_types_supported: Vec<String>,
    /// Supported ID token signing algorithms
    pub id_token_signing_alg_values_supported: Vec<String>,
    /// Supported scopes
    pub scopes_supported: Vec<String>,
    /// Supported claims
    pub claims_supported: Vec<String>,
    /// Supported grant types
    pub grant_types_supported: Vec<String>,
}

/// JWKS response
#[derive(Debug, Serialize)]
pub struct JwksResponse {
    /// Array of JSON Web Keys
    pub keys: Vec<JwkPublicKey>,
}

/// Public JSON Web Key (for JWKS endpoint - no private key)
#[derive(Debug, Serialize)]
pub struct JwkPublicKey {
    /// Key ID
    pub kid: String,
    /// Key type
    pub kty: String,
    /// Algorithm
    pub alg: String,
    /// Key use
    #[serde(rename = "use")]
    pub use_: String,
    /// RSA modulus (for RSA keys)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<String>,
    /// RSA exponent (for RSA keys)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub e: Option<String>,
    /// EC curve (for EC keys)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crv: Option<String>,
    /// EC X coordinate (for EC keys)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<String>,
    /// EC Y coordinate (for EC keys)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<String>,
}

/// OIDC state stored in AuthState
#[derive(Clone)]
pub struct OidcState {
    /// OIDC configuration
    pub config: OidcConfig,
    /// Active signing keys (indexed by kid)
    pub signing_keys: Arc<RwLock<HashMap<String, EncodingKey>>>,
}

impl OidcState {
    /// Create new OIDC state from configuration
    pub fn new(config: OidcConfig) -> Result<Self, Error> {
        let mut signing_keys = HashMap::new();

        // Load signing keys
        for key in &config.jwks.keys {
            if let Some(ref private_key) = key.private_key {
                let encoding_key = match key.alg.as_str() {
                    "RS256" | "RS384" | "RS512" => {
                        EncodingKey::from_rsa_pem(private_key.as_bytes()).map_err(|e| {
                            Error::generic(format!("Failed to load RSA key {}: {}", key.kid, e))
                        })?
                    }
                    "ES256" | "ES384" | "ES512" => EncodingKey::from_ec_pem(private_key.as_bytes())
                        .map_err(|e| {
                            Error::generic(format!("Failed to load EC key {}: {}", key.kid, e))
                        })?,
                    "HS256" | "HS384" | "HS512" => EncodingKey::from_secret(private_key.as_bytes()),
                    _ => {
                        return Err(Error::generic(format!("Unsupported algorithm: {}", key.alg)));
                    }
                };
                signing_keys.insert(key.kid.clone(), encoding_key);
            }
        }

        Ok(Self {
            config,
            signing_keys: Arc::new(RwLock::new(signing_keys)),
        })
    }

    /// Create OIDC state with default configuration for mock server
    ///
    /// This creates a basic OIDC configuration suitable for testing and development.
    /// For production use, load configuration from config files or environment variables.
    pub fn default_mock() -> Result<Self, Error> {
        use std::env;

        // Get issuer from environment or use default
        let issuer = env::var("MOCKFORGE_OIDC_ISSUER").unwrap_or_else(|_| {
            env::var("MOCKFORGE_BASE_URL")
                .unwrap_or_else(|_| "https://mockforge.example.com".to_string())
        });

        // Create default HS256 key for signing (suitable for development/testing)
        let default_secret = env::var("MOCKFORGE_OIDC_SECRET")
            .unwrap_or_else(|_| "mockforge-default-secret-key-change-in-production".to_string());

        let default_key = JwkKey {
            kid: "default".to_string(),
            alg: "HS256".to_string(),
            public_key: default_secret.clone(),
            private_key: Some(default_secret),
            kty: "oct".to_string(),
            use_: "sig".to_string(),
        };

        let config = OidcConfig {
            enabled: true,
            issuer,
            jwks: JwksConfig {
                keys: vec![default_key],
            },
            claims: ClaimsConfig {
                default: vec!["sub".to_string(), "iss".to_string(), "exp".to_string()],
                custom: HashMap::new(),
            },
            multi_tenant: None,
        };

        Self::new(config)
    }
}

/// Helper function to load OIDC state from configuration
///
/// Attempts to load OIDC configuration from:
/// 1. Environment variables (MOCKFORGE_OIDC_CONFIG, MOCKFORGE_OIDC_ISSUER, etc.)
/// 2. Config file (if available)
/// 3. Default mock configuration
///
/// Returns None if OIDC is not configured or disabled.
pub fn load_oidc_state() -> Option<OidcState> {
    use std::env;

    // Check if OIDC is explicitly disabled
    if let Ok(disabled) = env::var("MOCKFORGE_OIDC_ENABLED") {
        if disabled == "false" || disabled == "0" {
            return None;
        }
    }

    // Try to load from environment variable (JSON config)
    if let Ok(config_json) = env::var("MOCKFORGE_OIDC_CONFIG") {
        if let Ok(config) = serde_json::from_str::<OidcConfig>(&config_json) {
            if config.enabled {
                return OidcState::new(config).ok();
            }
            return None;
        }
    }

    // Try to load from config file (future enhancement)
    // For now, use default mock configuration if OIDC is not explicitly disabled
    OidcState::default_mock().ok()
}

/// Get OIDC discovery document
pub async fn get_oidc_discovery() -> Json<OidcDiscoveryDocument> {
    // Get base URL from environment variable or use default
    // In production, this would be loaded from configuration
    let base_url = std::env::var("MOCKFORGE_BASE_URL")
        .unwrap_or_else(|_| "https://mockforge.example.com".to_string());

    let discovery = OidcDiscoveryDocument {
        issuer: base_url.clone(),
        authorization_endpoint: format!("{}/oauth2/authorize", base_url),
        token_endpoint: format!("{}/oauth2/token", base_url),
        userinfo_endpoint: format!("{}/oauth2/userinfo", base_url),
        jwks_uri: format!("{}/.well-known/jwks.json", base_url),
        response_types_supported: vec![
            "code".to_string(),
            "id_token".to_string(),
            "token id_token".to_string(),
        ],
        subject_types_supported: vec!["public".to_string()],
        id_token_signing_alg_values_supported: vec![
            "RS256".to_string(),
            "ES256".to_string(),
            "HS256".to_string(),
        ],
        scopes_supported: vec![
            "openid".to_string(),
            "profile".to_string(),
            "email".to_string(),
            "address".to_string(),
            "phone".to_string(),
        ],
        claims_supported: vec![
            "sub".to_string(),
            "iss".to_string(),
            "aud".to_string(),
            "exp".to_string(),
            "iat".to_string(),
            "auth_time".to_string(),
            "nonce".to_string(),
            "email".to_string(),
            "email_verified".to_string(),
            "name".to_string(),
            "given_name".to_string(),
            "family_name".to_string(),
        ],
        grant_types_supported: vec![
            "authorization_code".to_string(),
            "implicit".to_string(),
            "refresh_token".to_string(),
            "client_credentials".to_string(),
        ],
    };

    Json(discovery)
}

/// Get JWKS (JSON Web Key Set)
pub async fn get_jwks() -> Json<JwksResponse> {
    // Return empty JWKS by default
    // Use get_jwks_from_state() when OIDC state is available from request context
    let jwks = JwksResponse { keys: vec![] };

    Json(jwks)
}

/// Get JWKS from OIDC state
pub fn get_jwks_from_state(oidc_state: &OidcState) -> Result<JwksResponse, Error> {
    use crate::auth::jwks_converter::convert_jwk_key_simple;

    let mut public_keys = Vec::new();

    for key in &oidc_state.config.jwks.keys {
        match convert_jwk_key_simple(key) {
            Ok(jwk) => public_keys.push(jwk),
            Err(e) => {
                tracing::warn!("Failed to convert key {} to JWK format: {}", key.kid, e);
                // Continue with other keys
            }
        }
    }

    Ok(JwksResponse { keys: public_keys })
}

/// Generate a signed JWT token with configurable claims
///
/// # Arguments
/// * `claims` - Map of claim names to values
/// * `kid` - Optional key ID for the signing key
/// * `algorithm` - Signing algorithm (RS256, ES256, HS256, etc.)
/// * `encoding_key` - Encoding key for signing
/// * `expires_in_seconds` - Optional expiration time in seconds from now
/// * `issuer` - Optional issuer claim
/// * `audience` - Optional audience claim
pub fn generate_signed_jwt(
    mut claims: HashMap<String, serde_json::Value>,
    kid: Option<String>,
    algorithm: Algorithm,
    encoding_key: &EncodingKey,
    expires_in_seconds: Option<i64>,
    issuer: Option<String>,
    audience: Option<String>,
) -> Result<String, Error> {
    use chrono::Utc;

    let mut header = Header::new(algorithm);
    if let Some(kid) = kid {
        header.kid = Some(kid);
    }

    // Add standard claims
    let now = Utc::now();
    claims.insert("iat".to_string(), json!(now.timestamp()));

    if let Some(exp_seconds) = expires_in_seconds {
        let exp = now + chrono::Duration::seconds(exp_seconds);
        claims.insert("exp".to_string(), json!(exp.timestamp()));
    }

    if let Some(iss) = issuer {
        claims.insert("iss".to_string(), json!(iss));
    }

    if let Some(aud) = audience {
        claims.insert("aud".to_string(), json!(aud));
    }

    let token = jsonwebtoken::encode(&header, &claims, encoding_key)
        .map_err(|e| Error::generic(format!("Failed to sign JWT: {}", e)))?;

    Ok(token)
}

/// Tenant context for multi-tenant token generation
#[derive(Debug, Clone)]
pub struct TenantContext {
    /// Organization ID
    pub org_id: Option<String>,
    /// Tenant ID
    pub tenant_id: Option<String>,
}

/// Generate a signed JWT token with default claims from OIDC config
pub fn generate_oidc_token(
    oidc_state: &OidcState,
    subject: String,
    additional_claims: Option<HashMap<String, serde_json::Value>>,
    expires_in_seconds: Option<i64>,
    tenant_context: Option<TenantContext>,
) -> Result<String, Error> {
    use chrono::Utc;
    use jsonwebtoken::Algorithm;

    // Start with default claims
    let mut claims = HashMap::new();
    claims.insert("sub".to_string(), json!(subject));
    claims.insert("iss".to_string(), json!(oidc_state.config.issuer.clone()));

    // Add default claims from config
    for claim_name in &oidc_state.config.claims.default {
        if !claims.contains_key(claim_name) {
            // Add standard claim if not already present
            match claim_name.as_str() {
                "sub" | "iss" => {} // Already added
                "exp" => {
                    let exp_seconds = expires_in_seconds.unwrap_or(3600);
                    let exp = Utc::now() + chrono::Duration::seconds(exp_seconds);
                    claims.insert("exp".to_string(), json!(exp.timestamp()));
                }
                "iat" => {
                    claims.insert("iat".to_string(), json!(Utc::now().timestamp()));
                }
                _ => {
                    // Use custom claim value if available
                    if let Some(value) = oidc_state.config.claims.custom.get(claim_name) {
                        claims.insert(claim_name.clone(), value.clone());
                    }
                }
            }
        }
    }

    // Add custom claims from config
    for (key, value) in &oidc_state.config.claims.custom {
        if !claims.contains_key(key) {
            claims.insert(key.clone(), value.clone());
        }
    }

    // Add multi-tenant claims if enabled
    if let Some(ref mt_config) = oidc_state.config.multi_tenant {
        if mt_config.enabled {
            // Get org_id and tenant_id from tenant context or use defaults
            let org_id = tenant_context
                .as_ref()
                .and_then(|ctx| ctx.org_id.clone())
                .unwrap_or_else(|| "org-default".to_string());
            let tenant_id = tenant_context
                .as_ref()
                .and_then(|ctx| ctx.tenant_id.clone())
                .or_else(|| Some("tenant-default".to_string()));

            claims.insert(mt_config.org_id_claim.clone(), json!(org_id));
            if let Some(ref tenant_claim) = mt_config.tenant_id_claim {
                if let Some(tid) = tenant_id {
                    claims.insert(tenant_claim.clone(), json!(tid));
                }
            }
        }
    }

    // Merge additional claims (override defaults)
    if let Some(additional) = additional_claims {
        for (key, value) in additional {
            claims.insert(key, value);
        }
    }

    // Get signing key (use first available key for now)
    let signing_keys = oidc_state.signing_keys.blocking_read();
    let (kid, encoding_key) = signing_keys
        .iter()
        .next()
        .ok_or_else(|| Error::generic("No signing keys available".to_string()))?;

    // Determine algorithm from key configuration
    // Default to HS256 if algorithm not specified in key config
    let algorithm = oidc_state
        .config
        .jwks
        .keys
        .iter()
        .find(|k| k.kid == *kid)
        .and_then(|k| match k.alg.as_str() {
            "RS256" => Some(Algorithm::RS256),
            "RS384" => Some(Algorithm::RS384),
            "RS512" => Some(Algorithm::RS512),
            "ES256" => Some(Algorithm::ES256),
            "ES384" => Some(Algorithm::ES384),
            "HS256" => Some(Algorithm::HS256),
            "HS384" => Some(Algorithm::HS384),
            "HS512" => Some(Algorithm::HS512),
            _ => None,
        })
        .unwrap_or(Algorithm::HS256);

    generate_signed_jwt(
        claims,
        Some(kid.clone()),
        algorithm,
        encoding_key,
        expires_in_seconds,
        Some(oidc_state.config.issuer.clone()),
        None,
    )
}

/// Create OIDC router with well-known endpoints
pub fn oidc_router() -> axum::Router {
    use axum::{routing::get, Router};

    Router::new()
        .route("/.well-known/openid-configuration", get(get_oidc_discovery))
        .route("/.well-known/jwks.json", get(get_jwks))
}
