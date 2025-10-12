//! Authentication plugin interface
//!
//! This module defines the AuthPlugin trait and related types for implementing
//! custom authentication methods in MockForge. Authentication plugins can handle
//! various authentication schemes like SAML, LDAP, custom OAuth flows, etc.

use crate::{PluginCapabilities, PluginContext, PluginResult, Result};
use axum::http::{HeaderMap, Method, Uri};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Authentication plugin trait
///
/// Implement this trait to create custom authentication methods for MockForge.
/// Authentication plugins are called during the request processing pipeline
/// to validate incoming requests.
#[async_trait::async_trait]
pub trait AuthPlugin: Send + Sync {
    /// Get plugin capabilities (permissions and limits)
    fn capabilities(&self) -> PluginCapabilities;

    /// Initialize the plugin with configuration
    async fn initialize(&self, config: &AuthPluginConfig) -> Result<()>;

    /// Authenticate a request
    ///
    /// This method is called for each incoming request that requires authentication.
    /// The plugin should examine the request headers, body, and other context
    /// to determine if the request is authenticated.
    ///
    /// # Arguments
    /// * `context` - Plugin execution context
    /// * `request` - HTTP request information
    /// * `config` - Plugin configuration
    ///
    /// # Returns
    /// Authentication result indicating success/failure and user claims
    async fn authenticate(
        &self,
        context: &PluginContext,
        request: &AuthRequest,
        config: &AuthPluginConfig,
    ) -> Result<PluginResult<AuthResponse>>;

    /// Validate plugin configuration
    fn validate_config(&self, config: &AuthPluginConfig) -> Result<()>;

    /// Get supported authentication schemes
    fn supported_schemes(&self) -> Vec<String>;

    /// Cleanup plugin resources
    async fn cleanup(&self) -> Result<()>;
}

/// Authentication plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthPluginConfig {
    /// Plugin-specific configuration
    pub config: HashMap<String, serde_json::Value>,
    /// Enable/disable the plugin
    pub enabled: bool,
    /// Plugin priority (lower numbers = higher priority)
    pub priority: i32,
    /// Custom settings
    pub settings: HashMap<String, serde_json::Value>,
}

impl Default for AuthPluginConfig {
    fn default() -> Self {
        Self {
            config: HashMap::new(),
            enabled: true,
            priority: 100,
            settings: HashMap::new(),
        }
    }
}

/// Authentication request information
#[derive(Debug, Clone)]
pub struct AuthRequest {
    /// HTTP method
    pub method: Method,
    /// Request URI
    pub uri: Uri,
    /// Request headers
    pub headers: HeaderMap,
    /// Request body (if available)
    pub body: Option<Vec<u8>>,
    /// Query parameters
    pub query_params: HashMap<String, String>,
    /// Client IP address
    pub client_ip: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Timestamp when request was received
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl AuthRequest {
    /// Create from axum request components
    pub fn from_axum(method: Method, uri: Uri, headers: HeaderMap, body: Option<Vec<u8>>) -> Self {
        let query_params = uri
            .query()
            .map(|q| url::form_urlencoded::parse(q.as_bytes()).into_owned().collect())
            .unwrap_or_default();

        let client_ip = headers
            .get("x-forwarded-for")
            .or_else(|| headers.get("x-real-ip"))
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let user_agent =
            headers.get("user-agent").and_then(|h| h.to_str().ok()).map(|s| s.to_string());

        Self {
            method,
            uri,
            headers,
            body,
            query_params,
            client_ip,
            user_agent,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Get authorization header value
    pub fn authorization_header(&self) -> Option<&str> {
        self.headers.get("authorization").and_then(|h| h.to_str().ok())
    }

    /// Get bearer token from authorization header
    pub fn bearer_token(&self) -> Option<&str> {
        self.authorization_header().and_then(|auth| auth.strip_prefix("Bearer "))
    }

    /// Get basic auth credentials from authorization header
    pub fn basic_credentials(&self) -> Option<(String, String)> {
        self.authorization_header()
            .and_then(|auth| auth.strip_prefix("Basic "))
            .and_then(|encoded| {
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded).ok()
            })
            .and_then(|decoded| String::from_utf8(decoded).ok())
            .and_then(|creds| {
                let parts: Vec<&str> = creds.splitn(2, ':').collect();
                if parts.len() == 2 {
                    Some((parts[0].to_string(), parts[1].to_string()))
                } else {
                    None
                }
            })
    }

    /// Get custom header value
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|h| h.to_str().ok())
    }

    /// Get query parameter value
    pub fn query_param(&self, name: &str) -> Option<&str> {
        self.query_params.get(name).map(|s| s.as_str())
    }
}

/// Authentication response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    /// Authentication successful
    pub authenticated: bool,
    /// User identity information
    pub identity: Option<UserIdentity>,
    /// Authentication claims/tokens
    pub claims: HashMap<String, serde_json::Value>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Error message (if authentication failed)
    pub error_message: Option<String>,
}

impl AuthResponse {
    /// Create successful authentication response
    pub fn success(identity: UserIdentity, claims: HashMap<String, serde_json::Value>) -> Self {
        Self {
            authenticated: true,
            identity: Some(identity),
            claims,
            metadata: HashMap::new(),
            error_message: None,
        }
    }

    /// Create failed authentication response
    pub fn failure<S: Into<String>>(error_message: S) -> Self {
        Self {
            authenticated: false,
            identity: None,
            claims: HashMap::new(),
            metadata: HashMap::new(),
            error_message: Some(error_message.into()),
        }
    }

    /// Add metadata
    pub fn with_metadata<S: Into<String>>(mut self, key: S, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Check if authentication was successful
    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    /// Get user identity (if authenticated)
    pub fn identity(&self) -> Option<&UserIdentity> {
        self.identity.as_ref()
    }

    /// Get authentication claims
    pub fn claims(&self) -> &HashMap<String, serde_json::Value> {
        &self.claims
    }

    /// Get error message (if authentication failed)
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
}

/// User identity information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIdentity {
    /// Unique user identifier
    pub user_id: String,
    /// Username/login
    pub username: Option<String>,
    /// Email address
    pub email: Option<String>,
    /// Display name
    pub display_name: Option<String>,
    /// User roles/permissions
    pub roles: Vec<String>,
    /// User groups
    pub groups: Vec<String>,
    /// Additional attributes
    pub attributes: HashMap<String, serde_json::Value>,
}

impl UserIdentity {
    /// Create basic user identity
    pub fn new<S: Into<String>>(user_id: S) -> Self {
        Self {
            user_id: user_id.into(),
            username: None,
            email: None,
            display_name: None,
            roles: Vec::new(),
            groups: Vec::new(),
            attributes: HashMap::new(),
        }
    }

    /// Set username
    pub fn with_username<S: Into<String>>(mut self, username: S) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set email
    pub fn with_email<S: Into<String>>(mut self, email: S) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Set display name
    pub fn with_display_name<S: Into<String>>(mut self, display_name: S) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    /// Add role
    pub fn with_role<S: Into<String>>(mut self, role: S) -> Self {
        self.roles.push(role.into());
        self
    }

    /// Add multiple roles
    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles.extend(roles);
        self
    }

    /// Add group
    pub fn with_group<S: Into<String>>(mut self, group: S) -> Self {
        self.groups.push(group.into());
        self
    }

    /// Add attribute
    pub fn with_attribute<S: Into<String>>(mut self, key: S, value: serde_json::Value) -> Self {
        self.attributes.insert(key.into(), value);
        self
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user is in a specific group
    pub fn in_group(&self, group: &str) -> bool {
        self.groups.iter().any(|g| g == group)
    }
}

/// Authentication plugin registry entry
pub struct AuthPluginEntry {
    /// Plugin ID
    pub plugin_id: crate::PluginId,
    /// Plugin instance
    pub plugin: Box<dyn AuthPlugin>,
    /// Plugin configuration
    pub config: AuthPluginConfig,
    /// Plugin capabilities
    pub capabilities: PluginCapabilities,
}

impl AuthPluginEntry {
    /// Create new plugin entry
    pub fn new(
        plugin_id: crate::PluginId,
        plugin: Box<dyn AuthPlugin>,
        config: AuthPluginConfig,
    ) -> Self {
        let capabilities = plugin.capabilities();
        Self {
            plugin_id,
            plugin,
            config,
            capabilities,
        }
    }

    /// Check if plugin is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get plugin priority
    pub fn priority(&self) -> i32 {
        self.config.priority
    }
}

/// Helper trait for creating authentication plugins
pub trait AuthPluginFactory: Send + Sync {
    /// Create a new authentication plugin instance
    fn create_plugin(&self) -> Result<Box<dyn AuthPlugin>>;
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_module_compiles() {
        // Basic compilation test
    }
}
