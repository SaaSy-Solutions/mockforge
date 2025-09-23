//! # Basic Authentication Plugin for MockForge
//!
//! This plugin provides HTTP Basic Authentication support for MockForge.
//! It allows protecting API endpoints with username/password authentication.
//!
//! ## Features
//!
//! - HTTP Basic Authentication validation
//! - Configurable user credentials
//! - Custom authentication realm
//! - Secure password handling

use mockforge_plugin_core::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicAuthConfig {
    /// Authentication realm name
    pub realm: String,
    /// List of users with their passwords
    pub users: Vec<UserCredentials>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCredentials {
    pub username: String,
    pub password: String,
}

/// Basic Authentication Plugin
pub struct BasicAuthPlugin {
    config: BasicAuthConfig,
    user_store: HashMap<String, String>,
}

impl BasicAuthPlugin {
    /// Create a new basic auth plugin
    pub fn new(config: BasicAuthConfig) -> Self {
        let user_store = config.users
            .iter()
            .map(|user| (user.username.clone(), user.password.clone()))
            .collect();

        Self {
            config,
            user_store,
        }
    }

    /// Validate HTTP Basic Authentication credentials
    fn validate_basic_auth(&self, auth_header: &str) -> Result<AuthClaims, AuthPluginError> {
        // Check if it's a Basic auth header
        if !auth_header.starts_with("Basic ") {
            return Err(AuthPluginError::InvalidCredentials("Not a Basic auth header".to_string()));
        }

        // Extract and decode the base64 credentials
        let encoded = &auth_header[6..]; // Remove "Basic " prefix
        let decoded = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            encoded
        ).map_err(|_| AuthPluginError::InvalidCredentials("Invalid base64 encoding".to_string()))?;

        let credentials = String::from_utf8(decoded)
            .map_err(|_| AuthPluginError::InvalidCredentials("Invalid UTF-8 in credentials".to_string()))?;

        // Parse username:password
        let parts: Vec<&str> = credentials.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(AuthPluginError::InvalidCredentials("Invalid credentials format".to_string()));
        }

        let username = parts[0];
        let password = parts[1];

        // Validate against user store
        match self.user_store.get(username) {
            Some(stored_password) if stored_password == password => {
                // Create auth claims
                let mut claims = HashMap::new();
                claims.insert("username".to_string(), username.to_string());
                claims.insert("realm".to_string(), self.config.realm.clone());
                claims.insert("auth_type".to_string(), "basic".to_string());

                Ok(AuthClaims {
                    subject: username.to_string(),
                    issuer: "mockforge-basic-auth".to_string(),
                    audience: vec!["mockforge".to_string()],
                    expires_at: None,
                    issued_at: chrono::Utc::now(),
                    claims,
                })
            }
            _ => Err(AuthPluginError::InvalidCredentials("Invalid username or password".to_string())),
        }
    }
}

impl AuthPlugin for BasicAuthPlugin {
    fn authenticate(
        &self,
        context: &PluginContext,
        _config: &AuthPluginConfig,
    ) -> PluginResult<AuthClaims> {
        // Extract Authorization header
        let auth_header = match context.headers.get("authorization") {
            Some(header) => header,
            None => {
                return PluginResult::failure(
                    "Missing Authorization header".to_string(),
                    0,
                );
            }
        };

        match self.validate_basic_auth(auth_header) {
            Ok(claims) => PluginResult::success(claims, 0),
            Err(e) => PluginResult::failure(e.to_string(), 0),
        }
    }

    fn validate_token(
        &self,
        _token: &str,
        _config: &AuthPluginConfig,
    ) -> PluginResult<AuthClaims> {
        // Basic auth doesn't use tokens, but we could implement session tokens here
        PluginResult::failure(
            "Token validation not supported by basic auth plugin".to_string(),
            0,
        )
    }

    fn get_capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            network: NetworkCapabilities {
                allow_http_outbound: false,
                allowed_hosts: vec![],
            },
            filesystem: FilesystemCapabilities {
                allow_read: false,
                allow_write: false,
                allowed_paths: vec![],
            },
            resources: PluginResources {
                max_memory_bytes: 10 * 1024 * 1024, // 10MB
                max_cpu_time_ms: 100, // 100ms per request
            },
            custom: HashMap::new(),
        }
    }

    fn health_check(&self) -> PluginHealth {
        PluginHealth::healthy(
            "Basic auth plugin is healthy".to_string(),
            PluginMetrics::default(),
        )
    }
}

/// Plugin factory function (called by the plugin loader)
#[no_mangle]
pub extern "C" fn create_auth_plugin(config_json: *const u8, config_len: usize) -> *mut BasicAuthPlugin {
    // Parse configuration from JSON
    let config_bytes = unsafe {
        std::slice::from_raw_parts(config_json, config_len)
    };

    let config_str = match std::str::from_utf8(config_bytes) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let config: BasicAuthConfig = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };

    // Create and return the plugin
    let plugin = Box::new(BasicAuthPlugin::new(config));
    Box::into_raw(plugin)
}

/// Plugin cleanup function
#[no_mangle]
pub extern "C" fn destroy_auth_plugin(plugin: *mut BasicAuthPlugin) {
    if !plugin.is_null() {
        unsafe {
            let _ = Box::from_raw(plugin);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_auth_validation() {
        let config = BasicAuthConfig {
            realm: "Test".to_string(),
            users: vec![
                UserCredentials {
                    username: "admin".to_string(),
                    password: "secret".to_string(),
                },
            ],
        };

        let plugin = BasicAuthPlugin::new(config);

        // Test valid credentials
        let auth_header = "Basic YWRtaW46c2VjcmV0"; // admin:secret in base64
        let result = plugin.validate_basic_auth(auth_header);
        assert!(result.is_ok());

        let claims = result.unwrap();
        assert_eq!(claims.subject, "admin");
        assert_eq!(claims.claims.get("realm"), Some(&"Test".to_string()));

        // Test invalid credentials
        let invalid_header = "Basic aW52YWxpZDppbnZhbGlk"; // invalid:invalid
        let result = plugin.validate_basic_auth(invalid_header);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_auth_header() {
        let config = BasicAuthConfig {
            realm: "Test".to_string(),
            users: vec![],
        };

        let plugin = BasicAuthPlugin::new(config);
        let context = PluginContext::new("GET".to_string(), "/test".to_string(), HashMap::new(), None);

        let result = plugin.authenticate(&context, &AuthPluginConfig::default());
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("Missing Authorization header"));
    }

    #[test]
    fn test_capabilities() {
        let config = BasicAuthConfig {
            realm: "Test".to_string(),
            users: vec![],
        };

        let plugin = BasicAuthPlugin::new(config);
        let capabilities = plugin.get_capabilities();

        assert!(!capabilities.network.allow_http_outbound);
        assert!(!capabilities.filesystem.allow_read);
        assert_eq!(capabilities.resources.max_memory_bytes, 10 * 1024 * 1024);
    }
}
