//! Response plugin template

pub const RESPONSE_TEMPLATE: &str = r#"//! {{plugin_name}} - Response Plugin
//!
//! This plugin generates custom HTTP responses in MockForge.

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
impl ResponsePlugin for Plugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            name: "{{plugin_name}}".to_string(),
            version: "0.1.0".to_string(),
            description: "Custom response generation plugin".to_string(),
        }
    }

    async fn initialize(&mut self, config: serde_json::Value) -> PluginResult<()> {
        self.config = Some(config);
        Ok(())
    }

    async fn can_handle(&self, context: &PluginContext, request: &ResponseRequest) -> PluginResult<bool> {
        // TODO: Implement logic to determine if this plugin can handle the request

        // Example: Check if request path matches certain pattern
        if request.path.starts_with("/api/") {
            return Ok(true);
        }

        // Example: Check for specific headers
        if request.headers.contains_key("X-Custom-Plugin") {
            return Ok(true);
        }

        // Example: Check HTTP method
        if request.method == "POST" && request.path == "/special" {
            return Ok(true);
        }

        Ok(false)
    }

    async fn generate_response(
        &self,
        context: &PluginContext,
        request: &ResponseRequest,
    ) -> PluginResult<ResponseData> {
        // TODO: Implement your response generation logic here

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("X-Plugin-Name".to_string(), "{{plugin_name}}".to_string());
        headers.insert("X-Plugin-Version".to_string(), "0.1.0".to_string());

        // Example: Different responses based on path
        let (status, body) = if request.path.starts_with("/api/users") {
            let response_body = json!({
                "users": [
                    {"id": 1, "name": "Alice"},
                    {"id": 2, "name": "Bob"},
                ]
            });
            (200, serde_json::to_vec(&response_body).unwrap())
        } else if request.path == "/api/status" {
            let response_body = json!({
                "status": "ok",
                "plugin": "{{plugin_name}}",
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            (200, serde_json::to_vec(&response_body).unwrap())
        } else {
            let response_body = json!({
                "message": "Default response from {{plugin_name}}",
                "path": request.path,
                "method": request.method
            });
            (200, serde_json::to_vec(&response_body).unwrap())
        };

        Ok(ResponseData {
            status,
            headers,
            body,
        })
    }

    async fn validate_config(&self, config: &serde_json::Value) -> PluginResult<()> {
        if !config.is_object() {
            return Err(PluginError::ConfigError(
                "Configuration must be an object".to_string()
            ));
        }
        Ok(())
    }

    async fn cleanup(&mut self) -> PluginResult<()> {
        self.config = None;
        Ok(())
    }
}

// Export the plugin
export_plugin!(Plugin);

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_request(method: &str, path: &str) -> ResponseRequest {
        ResponseRequest {
            method: method.to_string(),
            path: path.to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: vec![],
        }
    }

    #[tokio::test]
    async fn test_can_handle_api_path() {
        let plugin = Plugin::new();
        let mut harness = TestHarness::new();
        let context = harness.create_context("{{plugin_id}}", "test-request");

        let request = create_test_request("GET", "/api/users");
        let result = plugin.can_handle(&context, &request).await;

        assert_plugin_ok!(result);
        if let Ok(can_handle) = result {
            assert!(can_handle);
        }
    }

    #[tokio::test]
    async fn test_can_handle_non_api_path() {
        let plugin = Plugin::new();
        let mut harness = TestHarness::new();
        let context = harness.create_context("{{plugin_id}}", "test-request");

        let request = create_test_request("GET", "/home");
        let result = plugin.can_handle(&context, &request).await;

        assert_plugin_ok!(result);
        if let Ok(can_handle) = result {
            assert!(!can_handle);
        }
    }

    #[tokio::test]
    async fn test_generate_response() {
        let plugin = Plugin::new();
        let mut harness = TestHarness::new();
        let context = harness.create_context("{{plugin_id}}", "test-request");

        let request = create_test_request("GET", "/api/users");
        let result = plugin.generate_response(&context, &request).await;

        assert_plugin_ok!(result);
        if let Ok(response) = result {
            assert_eq!(response.status, 200);
            assert!(response.headers.contains_key("Content-Type"));
            assert!(response.headers.contains_key("X-Plugin-Name"));
            assert!(!response.body.is_empty());
        }
    }

    #[tokio::test]
    async fn test_generate_status_response() {
        let plugin = Plugin::new();
        let mut harness = TestHarness::new();
        let context = harness.create_context("{{plugin_id}}", "test-request");

        let request = create_test_request("GET", "/api/status");
        let result = plugin.generate_response(&context, &request).await;

        assert_plugin_ok!(result);
        if let Ok(response) = result {
            assert_eq!(response.status, 200);

            let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
            assert_eq!(body["status"], "ok");
            assert_eq!(body["plugin"], "{{plugin_name}}");
        }
    }
}
"#;
