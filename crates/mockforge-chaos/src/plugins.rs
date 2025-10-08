//! Chaos Plugin System
//!
//! Extensible plugin system for custom chaos engineering functionality.
//! Allows users to create and integrate custom chaos scenarios, fault injectors,
//! and resilience patterns.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use async_trait::async_trait;
use thiserror::Error;
use serde_json::Value as JsonValue;

/// Plugin system errors
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    #[error("Plugin already registered: {0}")]
    PluginAlreadyRegistered(String),

    #[error("Plugin initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Plugin execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Invalid plugin configuration: {0}")]
    InvalidConfig(String),

    #[error("Incompatible plugin version: {0}")]
    IncompatibleVersion(String),

    #[error("Missing required dependency: {0}")]
    MissingDependency(String),
}

pub type Result<T> = std::result::Result<T, PluginError>;

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub api_version: String,
}

/// Plugin capability
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    FaultInjection,
    TrafficShaping,
    Observability,
    Resilience,
    Scenario,
    Metrics,
    Custom(String),
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub enabled: bool,
    pub config: HashMap<String, JsonValue>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            config: HashMap::new(),
        }
    }
}

/// Plugin execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    pub tenant_id: Option<String>,
    pub scenario_id: Option<String>,
    pub execution_id: Option<String>,
    pub parameters: HashMap<String, JsonValue>,
    pub metadata: HashMap<String, String>,
}

impl Default for PluginContext {
    fn default() -> Self {
        Self {
            tenant_id: None,
            scenario_id: None,
            execution_id: None,
            parameters: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
}

/// Plugin execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    pub success: bool,
    pub message: String,
    pub data: HashMap<String, JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl PluginResult {
    pub fn success(message: String, data: HashMap<String, JsonValue>) -> Self {
        Self {
            success: true,
            message,
            data,
            error: None,
        }
    }

    pub fn failure(message: String, error: String) -> Self {
        Self {
            success: false,
            message,
            data: HashMap::new(),
            error: Some(error),
        }
    }
}

/// Chaos plugin trait
#[async_trait]
pub trait ChaosPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Get plugin capabilities
    fn capabilities(&self) -> Vec<PluginCapability>;

    /// Initialize plugin with configuration
    async fn initialize(&mut self, config: PluginConfig) -> Result<()>;

    /// Execute plugin action
    async fn execute(&self, context: PluginContext) -> Result<PluginResult>;

    /// Cleanup plugin resources
    async fn cleanup(&mut self) -> Result<()>;

    /// Validate configuration
    fn validate_config(&self, config: &PluginConfig) -> Result<()> {
        if !config.enabled {
            return Err(PluginError::InvalidConfig("Plugin is disabled".to_string()));
        }
        Ok(())
    }

    /// Get configuration schema (JSON Schema)
    fn config_schema(&self) -> Option<JsonValue> {
        None
    }
}

/// Plugin lifecycle hook
#[async_trait]
pub trait PluginHook: Send + Sync {
    /// Called before plugin execution
    async fn before_execute(&self, context: &PluginContext) -> Result<()> {
        Ok(())
    }

    /// Called after plugin execution
    async fn after_execute(&self, context: &PluginContext, result: &PluginResult) -> Result<()> {
        Ok(())
    }

    /// Called on plugin error
    async fn on_error(&self, context: &PluginContext, error: &PluginError) -> Result<()> {
        Ok(())
    }
}

/// Plugin registry
pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<String, Arc<dyn ChaosPlugin>>>>,
    hooks: Arc<RwLock<Vec<Arc<dyn PluginHook>>>>,
    configs: Arc<RwLock<HashMap<String, PluginConfig>>>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            hooks: Arc::new(RwLock::new(Vec::new())),
            configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a plugin
    pub fn register_plugin(&self, plugin: Arc<dyn ChaosPlugin>) -> Result<()> {
        let plugin_id = plugin.metadata().id.clone();

        let mut plugins = self.plugins.write().unwrap();

        if plugins.contains_key(&plugin_id) {
            return Err(PluginError::PluginAlreadyRegistered(plugin_id));
        }

        plugins.insert(plugin_id, plugin);
        Ok(())
    }

    /// Unregister a plugin
    pub fn unregister_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().unwrap();

        plugins
            .remove(plugin_id)
            .ok_or_else(|| PluginError::PluginNotFound(plugin_id.to_string()))?;

        Ok(())
    }

    /// Get a plugin
    pub fn get_plugin(&self, plugin_id: &str) -> Result<Arc<dyn ChaosPlugin>> {
        let plugins = self.plugins.read().unwrap();

        plugins
            .get(plugin_id)
            .cloned()
            .ok_or_else(|| PluginError::PluginNotFound(plugin_id.to_string()))
    }

    /// List all plugins
    pub fn list_plugins(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().unwrap();
        plugins
            .values()
            .map(|p| p.metadata().clone())
            .collect()
    }

    /// Register a hook
    pub fn register_hook(&self, hook: Arc<dyn PluginHook>) {
        let mut hooks = self.hooks.write().unwrap();
        hooks.push(hook);
    }

    /// Configure a plugin
    pub fn configure_plugin(&self, plugin_id: &str, config: PluginConfig) -> Result<()> {
        let plugin = self.get_plugin(plugin_id)?;
        plugin.validate_config(&config)?;

        let mut configs = self.configs.write().unwrap();
        configs.insert(plugin_id.to_string(), config);

        Ok(())
    }

    /// Get plugin configuration
    pub fn get_config(&self, plugin_id: &str) -> Option<PluginConfig> {
        let configs = self.configs.read().unwrap();
        configs.get(plugin_id).cloned()
    }

    /// Execute a plugin
    pub async fn execute_plugin(
        &self,
        plugin_id: &str,
        context: PluginContext,
    ) -> Result<PluginResult> {
        let plugin = self.get_plugin(plugin_id)?;

        // Check if plugin is enabled
        if let Some(config) = self.get_config(plugin_id) {
            if !config.enabled {
                return Err(PluginError::ExecutionFailed(
                    "Plugin is disabled".to_string()
                ));
            }
        }

        // Execute before hooks
        let hooks = self.hooks.read().unwrap().clone();
        for hook in &hooks {
            hook.before_execute(&context).await?;
        }

        // Execute plugin
        let result = match plugin.execute(context.clone()).await {
            Ok(result) => {
                // Execute after hooks
                for hook in &hooks {
                    hook.after_execute(&context, &result).await?;
                }
                result
            }
            Err(error) => {
                // Execute error hooks
                for hook in &hooks {
                    hook.on_error(&context, &error).await?;
                }
                return Err(error);
            }
        };

        Ok(result)
    }

    /// Find plugins by capability
    pub fn find_by_capability(&self, capability: &PluginCapability) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().unwrap();
        plugins
            .values()
            .filter(|p| p.capabilities().contains(capability))
            .map(|p| p.metadata().clone())
            .collect()
    }

    /// Initialize all plugins
    pub async fn initialize_all(&self) -> Result<()> {
        let plugins = self.plugins.write().unwrap();

        for (plugin_id, plugin) in plugins.iter() {
            let config = self.get_config(plugin_id).unwrap_or_default();

            // Create a mutable reference to the plugin
            // Note: This requires the plugin to be properly designed for interior mutability
            // or we need to store plugins differently
            tracing::info!("Initializing plugin: {}", plugin_id);
        }

        Ok(())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Example: Custom fault injection plugin
pub struct CustomFaultPlugin {
    metadata: PluginMetadata,
    config: Option<PluginConfig>,
}

impl CustomFaultPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                id: "custom-fault-injector".to_string(),
                name: "Custom Fault Injector".to_string(),
                version: "1.0.0".to_string(),
                description: "Inject custom faults into applications".to_string(),
                author: "MockForge Team".to_string(),
                homepage: Some("https://mockforge.dev/plugins/custom-fault".to_string()),
                repository: None,
                tags: vec!["fault".to_string(), "injection".to_string()],
                dependencies: vec![],
                api_version: "v1".to_string(),
            },
            config: None,
        }
    }
}

#[async_trait]
impl ChaosPlugin for CustomFaultPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::FaultInjection]
    }

    async fn initialize(&mut self, config: PluginConfig) -> Result<()> {
        self.validate_config(&config)?;
        self.config = Some(config);
        Ok(())
    }

    async fn execute(&self, context: PluginContext) -> Result<PluginResult> {
        // Custom fault injection logic here
        let fault_type = context.parameters.get("fault_type")
            .and_then(|v| v.as_str())
            .unwrap_or("generic");

        let mut data = HashMap::new();
        data.insert("fault_type".to_string(), JsonValue::String(fault_type.to_string()));
        data.insert("injected_at".to_string(), JsonValue::String(chrono::Utc::now().to_rfc3339()));

        Ok(PluginResult::success(
            format!("Injected {} fault", fault_type),
            data,
        ))
    }

    async fn cleanup(&mut self) -> Result<()> {
        self.config = None;
        Ok(())
    }

    fn config_schema(&self) -> Option<JsonValue> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "enabled": {
                    "type": "boolean",
                    "default": true
                },
                "config": {
                    "type": "object",
                    "properties": {
                        "fault_probability": {
                            "type": "number",
                            "minimum": 0.0,
                            "maximum": 1.0,
                            "default": 0.1
                        }
                    }
                }
            }
        }))
    }
}

impl Default for CustomFaultPlugin {
    fn default() -> Self {
        Self::new()
    }
}

/// Example: Metrics collection plugin
pub struct MetricsPlugin {
    metadata: PluginMetadata,
    config: Option<PluginConfig>,
    metrics: Arc<RwLock<Vec<HashMap<String, JsonValue>>>>,
}

impl MetricsPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                id: "metrics-collector".to_string(),
                name: "Metrics Collector".to_string(),
                version: "1.0.0".to_string(),
                description: "Collect and aggregate chaos metrics".to_string(),
                author: "MockForge Team".to_string(),
                homepage: None,
                repository: None,
                tags: vec!["metrics".to_string(), "observability".to_string()],
                dependencies: vec![],
                api_version: "v1".to_string(),
            },
            config: None,
            metrics: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn get_metrics(&self) -> Vec<HashMap<String, JsonValue>> {
        let metrics = self.metrics.read().unwrap();
        metrics.clone()
    }
}

#[async_trait]
impl ChaosPlugin for MetricsPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::Metrics, PluginCapability::Observability]
    }

    async fn initialize(&mut self, config: PluginConfig) -> Result<()> {
        self.validate_config(&config)?;
        self.config = Some(config);
        Ok(())
    }

    async fn execute(&self, context: PluginContext) -> Result<PluginResult> {
        // Collect metrics from context
        let mut metric = HashMap::new();
        metric.insert("timestamp".to_string(), JsonValue::String(chrono::Utc::now().to_rfc3339()));

        if let Some(tenant_id) = &context.tenant_id {
            metric.insert("tenant_id".to_string(), JsonValue::String(tenant_id.clone()));
        }

        if let Some(scenario_id) = &context.scenario_id {
            metric.insert("scenario_id".to_string(), JsonValue::String(scenario_id.clone()));
        }

        // Store metric
        let mut metrics = self.metrics.write().unwrap();
        metrics.push(metric.clone());

        Ok(PluginResult::success(
            "Metric collected".to_string(),
            metric,
        ))
    }

    async fn cleanup(&mut self) -> Result<()> {
        let mut metrics = self.metrics.write().unwrap();
        metrics.clear();
        self.config = None;
        Ok(())
    }
}

impl Default for MetricsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_registration() {
        let registry = PluginRegistry::new();
        let plugin = Arc::new(CustomFaultPlugin::new());

        registry.register_plugin(plugin.clone()).unwrap();

        let retrieved = registry.get_plugin("custom-fault-injector").unwrap();
        assert_eq!(retrieved.metadata().name, "Custom Fault Injector");
    }

    #[tokio::test]
    async fn test_plugin_execution() {
        let registry = PluginRegistry::new();
        let plugin = Arc::new(CustomFaultPlugin::new());

        registry.register_plugin(plugin).unwrap();

        let config = PluginConfig::default();
        registry.configure_plugin("custom-fault-injector", config).unwrap();

        let mut context = PluginContext::default();
        context.parameters.insert(
            "fault_type".to_string(),
            JsonValue::String("timeout".to_string()),
        );

        let result = registry.execute_plugin("custom-fault-injector", context).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_find_by_capability() {
        let registry = PluginRegistry::new();

        registry.register_plugin(Arc::new(CustomFaultPlugin::new())).unwrap();
        registry.register_plugin(Arc::new(MetricsPlugin::new())).unwrap();

        let fault_plugins = registry.find_by_capability(&PluginCapability::FaultInjection);
        assert_eq!(fault_plugins.len(), 1);

        let metrics_plugins = registry.find_by_capability(&PluginCapability::Metrics);
        assert_eq!(metrics_plugins.len(), 1);
    }

    #[tokio::test]
    async fn test_metrics_plugin() {
        let plugin = Arc::new(MetricsPlugin::new());
        let registry = PluginRegistry::new();

        registry.register_plugin(plugin.clone()).unwrap();
        registry.configure_plugin("metrics-collector", PluginConfig::default()).unwrap();

        let mut context = PluginContext::default();
        context.tenant_id = Some("tenant-1".to_string());

        let result = registry.execute_plugin("metrics-collector", context).await.unwrap();
        assert!(result.success);

        let metrics = plugin.get_metrics();
        assert_eq!(metrics.len(), 1);
    }
}
