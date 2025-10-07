//! Auth plugin template

pub const AUTH_TEMPLATE: &str = r#"//! {{plugin_name}} - Authentication Plugin
//!
//! This plugin provides custom authentication logic for MockForge.

use mockforge_plugin_sdk::prelude::*;

#[derive(Debug)]
pub struct Plugin {
    config: Option<serde_json::Value>,
}

impl Default for Plugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin {
    pub fn new() -> Self {
        Self { config: None }
    }
}

#[async_trait]
impl AuthPlugin for Plugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            name: "{{plugin_name}}".to_string(),
            version: "0.1.0".to_string(),
            description: "Custom authentication plugin".to_string(),
        }
    }

    async fn initialize(&mut self, config: serde_json::Value) -> PluginResult<()> {
        // Store configuration for later use
        self.config = Some(config);
        Ok(())
    }

    async fn authenticate(
        &self,
        context: &PluginContext,
        request: &AuthRequest,
    ) -> PluginResult<AuthResponse> {
        // TODO: Implement your authentication logic here

        // Example: Check for basic auth credentials
        if let Some((username, password)) = request.basic_credentials() {
            if username == "admin" && password == "secret" {
                return Ok(AuthResponse::authenticated(UserIdentity {
                    id: username.clone(),
                    username: Some(username),
                    email: None,
                    roles: vec!["admin".to_string()],
                    metadata: HashMap::new(),
                }));
            } else {
                return Ok(AuthResponse::denied("Invalid credentials"));
            }
        }

        // Example: Check for bearer token
        if let Some(token) = request.bearer_token() {
            if token.starts_with("valid_") {
                return Ok(AuthResponse::authenticated(UserIdentity {
                    id: "token_user".to_string(),
                    username: Some("token_user".to_string()),
                    email: None,
                    roles: vec!["user".to_string()],
                    metadata: HashMap::new(),
                }));
            } else {
                return Ok(AuthResponse::denied("Invalid token"));
            }
        }

        // No valid credentials found
        Ok(AuthResponse::denied("No authentication credentials provided"))
    }

    async fn validate_config(&self, config: &serde_json::Value) -> PluginResult<()> {
        // TODO: Validate your plugin configuration
        // Example: Check for required fields
        if !config.is_object() {
            return Err(PluginError::ConfigError(
                "Configuration must be an object".to_string()
            ));
        }
        Ok(())
    }

    fn supported_schemes(&self) -> Vec<String> {
        // Return list of authentication schemes this plugin supports
        vec![
            "Basic".to_string(),
            "Bearer".to_string(),
        ]
    }

    async fn cleanup(&mut self) -> PluginResult<()> {
        // Cleanup any resources (database connections, file handles, etc.)
        self.config = None;
        Ok(())
    }
}

// Export the plugin
export_plugin!(Plugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_auth_success() {
        let plugin = Plugin::new();
        let mut harness = TestHarness::new();
        let context = harness.create_context("{{plugin_id}}", "test-request");

        let request = MockAuthRequest::with_basic_auth("admin", "secret");
        let result = plugin.authenticate(&context, &request).await;

        assert_plugin_ok!(result);
        if let Ok(auth_response) = result {
            assert!(auth_response.authenticated);
            assert_eq!(auth_response.user.as_ref().unwrap().username, Some("admin".to_string()));
        }
    }

    #[tokio::test]
    async fn test_basic_auth_failure() {
        let plugin = Plugin::new();
        let mut harness = TestHarness::new();
        let context = harness.create_context("{{plugin_id}}", "test-request");

        let request = MockAuthRequest::with_basic_auth("user", "wrong");
        let result = plugin.authenticate(&context, &request).await;

        assert_plugin_ok!(result);
        if let Ok(auth_response) = result {
            assert!(!auth_response.authenticated);
        }
    }

    #[tokio::test]
    async fn test_bearer_token() {
        let plugin = Plugin::new();
        let mut harness = TestHarness::new();
        let context = harness.create_context("{{plugin_id}}", "test-request");

        let request = MockAuthRequest::with_bearer_token("valid_token_123");
        let result = plugin.authenticate(&context, &request).await;

        assert_plugin_ok!(result);
        if let Ok(auth_response) = result {
            assert!(auth_response.authenticated);
        }
    }

    #[tokio::test]
    async fn test_supported_schemes() {
        let plugin = Plugin::new();
        let schemes = plugin.supported_schemes();

        assert!(schemes.contains(&"Basic".to_string()));
        assert!(schemes.contains(&"Bearer".to_string()));
    }

    #[tokio::test]
    async fn test_initialize() {
        let mut plugin = Plugin::new();
        let config = serde_json::json!({"key": "value"});

        let result = plugin.initialize(config).await;
        assert_plugin_ok!(result);
        assert!(plugin.config.is_some());
    }
}
"#;
