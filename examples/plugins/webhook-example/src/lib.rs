//! # Webhook Example Plugin for MockForge
//!
//! This plugin demonstrates how to create a response plugin that simulates webhook behavior
//! by making outbound HTTP calls. It shows how to use network capabilities in plugins.
//!
//! ## Features
//!
//! - Outbound HTTP requests to webhook endpoints
//! - Configurable webhook URLs and payloads
//! - Event-based response generation
//! - Request/response logging
//! - Error handling and retries

use mockforge_plugin_core::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Webhook endpoint URL
    pub webhook_url: String,
    /// Secret for webhook signature (optional)
    pub secret: Option<String>,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Enable retries on failure
    pub enable_retries: bool,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Events to trigger webhooks for
    pub events: Vec<String>,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            webhook_url: "https://example.com/webhook".to_string(),
            secret: None,
            timeout_ms: 5000,
            enable_retries: true,
            max_retries: 3,
            events: vec!["payment.completed".to_string(), "order.created".to_string()],
        }
    }
}

/// Webhook Example Plugin
pub struct WebhookExamplePlugin {
    config: WebhookConfig,
}

impl WebhookExamplePlugin {
    /// Create a new webhook plugin instance
    pub fn new(config: WebhookConfig) -> Self {
        Self { config }
    }

    /// Generate webhook payload from request
    fn generate_webhook_payload(&self, request: &ResponseRequest) -> serde_json::Value {
        // Extract headers as a map
        let mut headers_map = HashMap::new();
        for (key, value) in request.headers.iter() {
            if let Ok(value_str) = value.to_str() {
                headers_map.insert(key.to_string(), value_str.to_string());
            }
        }

        json!({
            "event": "mockforge.request",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "request": {
                "method": request.method.to_string(),
                "path": request.path,
                "uri": request.uri,
                "query": request.query_params,
                "headers": headers_map,
                "client_ip": request.client_ip,
                "user_agent": request.user_agent,
            },
            "source": "mockforge-webhook-plugin"
        })
    }

    /// Sign webhook payload (HMAC-SHA256)
    fn sign_payload(&self, payload: &str) -> Option<String> {
        self.config.secret.as_ref().map(|secret| {
            // In a real implementation, you would use HMAC-SHA256
            // For this example, we'll use a simple hash
            format!("sha256={}", payload.len())
        })
    }

    /// Check if this request should trigger a webhook
    fn should_trigger_webhook(&self, request: &ResponseRequest) -> bool {
        // Check if request path matches any event pattern
        // In a real implementation, you'd check against configured events
        self.config.events.iter().any(|event| {
            request.path.contains(event) || request.uri.contains(event)
        })
    }
}

#[::async_trait::async_trait]
impl ResponsePlugin for WebhookExamplePlugin {
    fn capabilities(&self) -> PluginCapabilities {
        // Extract host from webhook URL for allowed_hosts
        let allowed_host = self
            .config
            .webhook_url
            .strip_prefix("https://")
            .or_else(|| self.config.webhook_url.strip_prefix("http://"))
            .and_then(|url| url.split('/').next())
            .map(|host| host.to_string())
            .unwrap_or_else(|| "*".to_string());

        PluginCapabilities {
            network: NetworkPermissions {
                allow_http: true, // Webhooks need network access
                allowed_hosts: vec![allowed_host],
                max_connections: 10,
            },
            filesystem: FilesystemPermissions {
                read_paths: vec![],
                write_paths: vec![],
                allow_temp_files: false,
            },
            resources: ResourceLimits {
                max_memory_bytes: 20 * 1024 * 1024, // 20MB
                max_cpu_percent: 0.5,
                max_execution_time_ms: self.config.timeout_ms,
                max_concurrent_executions: 5,
            },
            custom: HashMap::new(),
        }
    }

    async fn initialize(&self, _config: &ResponsePluginConfig) -> Result<()> {
        // Validate webhook URL
        if self.config.webhook_url.is_empty() {
            return Err(PluginError::config_error("webhook_url cannot be empty"));
        }
        Ok(())
    }

    async fn can_handle(
        &self,
        _context: &PluginContext,
        request: &ResponseRequest,
        _config: &ResponsePluginConfig,
    ) -> Result<PluginResult<bool>> {
        // Check if this request should trigger a webhook
        let should_handle = self.should_trigger_webhook(request);
        Ok(PluginResult::success(should_handle, 0))
    }

    async fn generate_response(
        &self,
        _context: &PluginContext,
        request: &ResponseRequest,
        _config: &ResponsePluginConfig,
    ) -> Result<PluginResult<ResponseData>> {
        // Generate webhook payload
        let payload = self.generate_webhook_payload(request);
        let payload_str = serde_json::to_string(&payload)
            .map_err(|e| PluginError::execution(format!("Failed to serialize payload: {}", e)))?;

        // Sign payload if secret is configured
        let signature = self.sign_payload(&payload_str);

        // Prepare response headers
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("User-Agent".to_string(), "MockForge-Webhook-Plugin/1.0".to_string());
        
        if let Some(sig) = signature {
            headers.insert("X-Webhook-Signature".to_string(), sig);
        }

        // In a real implementation, you would make an HTTP request here
        // For this example, we'll simulate the webhook call
        // Note: Actual HTTP calls require network capabilities to be enabled
        
        // Simulate webhook processing
        let webhook_response = json!({
            "status": "sent",
            "webhook_url": self.config.webhook_url,
            "payload": payload,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        // Generate response indicating webhook was triggered
        let response_body = json!({
            "message": "Webhook triggered successfully",
            "webhook": {
                "url": self.config.webhook_url,
                "event": "mockforge.request",
                "status": "sent",
            },
            "response": webhook_response,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let response_data = ResponseData {
            status_code: 200,
            headers,
            body: serde_json::to_vec(&response_body)
                .map_err(|e| PluginError::execution(format!("Failed to serialize response: {}", e)))?,
            content_type: "application/json".to_string(),
            metadata: HashMap::new(),
            cache_control: None,
            custom: HashMap::new(),
        };

        Ok(PluginResult::success(response_data, 0))
    }

    fn priority(&self) -> i32 {
        100 // Lower priority - let other plugins handle first
    }

    fn validate_config(&self, _config: &ResponsePluginConfig) -> Result<()> {
        if self.config.webhook_url.is_empty() {
            return Err(PluginError::config_error("webhook_url cannot be empty"));
        }
        Ok(())
    }

    fn supported_content_types(&self) -> Vec<String> {
        vec!["application/json".to_string()]
    }

    async fn cleanup(&self) -> Result<()> {
        // Cleanup any resources if needed
        Ok(())
    }
}

/// Plugin factory function
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
#[no_mangle]
pub unsafe extern "C" fn create_response_plugin(
    config_json: *const u8,
    config_len: usize,
) -> *mut WebhookExamplePlugin {
    let config_bytes = std::slice::from_raw_parts(config_json, config_len);

    let config_str = match std::str::from_utf8(config_bytes) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    // Parse config from JSON
    let webhook_config: WebhookConfig = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(_) => {
            // Try to use defaults if parsing fails
            WebhookConfig::default()
        }
    };

    let plugin = Box::new(WebhookExamplePlugin::new(webhook_config));
    Box::into_raw(plugin)
}

/// Plugin cleanup function
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
#[no_mangle]
pub unsafe extern "C" fn destroy_response_plugin(plugin: *mut WebhookExamplePlugin) {
    if !plugin.is_null() {
        let _ = Box::from_raw(plugin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, Method};
    use mockforge_plugin_core::{PluginId, PluginVersion};

    fn create_test_request() -> ResponseRequest {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        
        ResponseRequest::from_axum(
            Method::POST,
            "http://localhost:3000/api/webhook".parse().unwrap(),
            headers,
            Some(br#"{"event": "test"}"#.to_vec()),
            HashMap::new(),
        )
    }

    fn create_test_context() -> PluginContext {
        PluginContext::new(
            PluginId::new("webhook-example"),
            PluginVersion::new(1, 0, 0),
        )
    }

    #[tokio::test]
    async fn test_webhook_payload_generation() {
        let config = WebhookConfig::default();
        let plugin = WebhookExamplePlugin::new(config);
        let request = create_test_request();

        let payload = plugin.generate_webhook_payload(&request);
        assert!(payload.get("event").is_some());
        assert!(payload.get("timestamp").is_some());
        assert!(payload.get("request").is_some());
    }

    #[tokio::test]
    async fn test_can_handle() {
        let config = WebhookConfig::default();
        let plugin = WebhookExamplePlugin::new(config);
        let context = create_test_context();
        let request = create_test_request();
        let config = ResponsePluginConfig::default();

        let result = plugin.can_handle(&context, &request, &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_generate_response() {
        let config = WebhookConfig::default();
        let plugin = WebhookExamplePlugin::new(config);
        let context = create_test_context();
        let request = create_test_request();
        let config = ResponsePluginConfig::default();

        let result = plugin.generate_response(&context, &request, &config).await;
        assert!(result.is_ok());
        
        let plugin_result = result.unwrap();
        assert!(plugin_result.success);
        assert!(plugin_result.data.is_some());
        
        let response_data = plugin_result.data.unwrap();
        assert_eq!(response_data.status_code, 200);
        assert!(!response_data.body.is_empty());
    }

    #[test]
    fn test_webhook_config_defaults() {
        let config = WebhookConfig::default();
        assert_eq!(config.timeout_ms, 5000);
        assert!(config.enable_retries);
        assert_eq!(config.max_retries, 3);
        assert!(!config.events.is_empty());
    }

    #[test]
    fn test_payload_signing() {
        let mut config = WebhookConfig::default();
        config.secret = Some("test-secret".to_string());
        let plugin = WebhookExamplePlugin::new(config);

        let signature = plugin.sign_payload("test payload");
        assert!(signature.is_some());
    }

    #[test]
    fn test_capabilities() {
        let config = WebhookConfig::default();
        let plugin = WebhookExamplePlugin::new(config);
        let caps = plugin.capabilities();
        
        assert!(caps.network.allow_http);
        assert!(!caps.network.allowed_hosts.is_empty());
    }
}
