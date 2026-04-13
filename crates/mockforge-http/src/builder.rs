//! Composable HTTP router builder for MockForge.
//!
//! `HttpRouterBuilder` replaces the combinatorial explosion of `build_router_*`
//! functions with a single fluent builder that lets callers opt-in to only the
//! features they need.
//!
//! # Example
//!
//! ```rust,no_run
//! use mockforge_http::HttpRouterBuilder;
//!
//! # async fn example() {
//! let router = HttpRouterBuilder::new()
//!     .spec_path("api-spec.json".to_string())
//!     .with_multi_tenant(mockforge_core::MultiTenantConfig::default())
//!     .build()
//!     .await;
//! # }
//! ```

use axum::Router;
use mockforge_core::config::{DeceptiveDeployConfig, HttpCorsConfig, RouteConfig};
use mockforge_chaos::core_failure_injection::FailureConfig;
use mockforge_core::intelligent_behavior::MockAI;
use mockforge_core::openapi::response::AiGenerator;
use mockforge_core::openapi_routes::ValidationOptions;
use mockforge_proxy::config::ProxyConfig;
use mockforge_core::request_chaining::ChainConfig;
use mockforge_chaos::core_traffic_shaping::TrafficShaper;
use mockforge_core::MultiTenantConfig;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::health::HealthManager;

/// A composable builder for the MockForge HTTP router.
///
/// Replaces the family of `build_router_*` free functions with a single builder
/// that accepts optional configuration for each cross-cutting concern.
///
/// Every `.with_*()` method is optional. Calling `.build()` with no options
/// produces a minimal router equivalent to the old `build_router(None, None, None)`.
pub struct HttpRouterBuilder {
    // --- core ---
    spec_path: Option<String>,
    validation_options: Option<ValidationOptions>,
    failure_config: Option<FailureConfig>,

    // --- multi-tenant ---
    multi_tenant_config: Option<MultiTenantConfig>,

    // --- routes ---
    route_configs: Option<Vec<RouteConfig>>,

    // --- CORS ---
    cors_config: Option<HttpCorsConfig>,

    // --- AI ---
    ai_generator: Option<Arc<dyn AiGenerator + Send + Sync>>,
    mockai: Option<Arc<RwLock<MockAI>>>,

    // --- protocol registries ---
    smtp_registry: Option<Arc<dyn std::any::Any + Send + Sync>>,
    mqtt_broker: Option<Arc<dyn std::any::Any + Send + Sync>>,

    // --- traffic shaping ---
    traffic_shaper: Option<TrafficShaper>,
    traffic_shaping_enabled: bool,

    // --- chains ---
    chain_config: Option<ChainConfig>,

    // --- health ---
    health_manager: Option<Arc<HealthManager>>,

    // --- deceptive deploy ---
    deceptive_deploy_config: Option<DeceptiveDeployConfig>,

    // --- proxy ---
    proxy_config: Option<ProxyConfig>,
}

impl Default for HttpRouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpRouterBuilder {
    /// Create a new builder with all options defaulting to `None`/disabled.
    pub fn new() -> Self {
        Self {
            spec_path: None,
            validation_options: None,
            failure_config: None,
            multi_tenant_config: None,
            route_configs: None,
            cors_config: None,
            ai_generator: None,
            mockai: None,
            smtp_registry: None,
            mqtt_broker: None,
            traffic_shaper: None,
            traffic_shaping_enabled: false,
            chain_config: None,
            health_manager: None,
            deceptive_deploy_config: None,
            proxy_config: None,
        }
    }

    /// Set the path to the OpenAPI specification file.
    pub fn spec_path(mut self, path: String) -> Self {
        self.spec_path = Some(path);
        self
    }

    /// Set the path to the OpenAPI specification file (optional variant).
    pub fn spec_path_opt(mut self, path: Option<String>) -> Self {
        self.spec_path = path;
        self
    }

    /// Set validation options for request/response validation.
    pub fn validation_options(mut self, options: ValidationOptions) -> Self {
        self.validation_options = Some(options);
        self
    }

    /// Set validation options (optional variant).
    pub fn validation_options_opt(mut self, options: Option<ValidationOptions>) -> Self {
        self.validation_options = options;
        self
    }

    /// Set failure injection configuration.
    pub fn failure_config(mut self, config: FailureConfig) -> Self {
        self.failure_config = Some(config);
        self
    }

    /// Enable multi-tenant workspace support.
    pub fn with_multi_tenant(mut self, config: MultiTenantConfig) -> Self {
        self.multi_tenant_config = Some(config);
        self
    }

    /// Enable multi-tenant workspace support (optional variant).
    pub fn with_multi_tenant_opt(mut self, config: Option<MultiTenantConfig>) -> Self {
        self.multi_tenant_config = config;
        self
    }

    /// Set custom route configurations.
    pub fn route_configs(mut self, configs: Vec<RouteConfig>) -> Self {
        self.route_configs = Some(configs);
        self
    }

    /// Set custom route configurations (optional variant).
    pub fn route_configs_opt(mut self, configs: Option<Vec<RouteConfig>>) -> Self {
        self.route_configs = configs;
        self
    }

    /// Set CORS configuration.
    pub fn cors_config(mut self, config: HttpCorsConfig) -> Self {
        self.cors_config = Some(config);
        self
    }

    /// Set CORS configuration (optional variant).
    pub fn cors_config_opt(mut self, config: Option<HttpCorsConfig>) -> Self {
        self.cors_config = config;
        self
    }

    /// Set an AI response generator.
    pub fn with_ai(mut self, generator: Arc<dyn AiGenerator + Send + Sync>) -> Self {
        self.ai_generator = Some(generator);
        self
    }

    /// Set the MockAI instance for intelligent response generation.
    pub fn with_mockai(mut self, mockai: Arc<RwLock<MockAI>>) -> Self {
        self.mockai = Some(mockai);
        self
    }

    /// Set the MockAI instance (optional variant).
    pub fn with_mockai_opt(mut self, mockai: Option<Arc<RwLock<MockAI>>>) -> Self {
        self.mockai = mockai;
        self
    }

    /// Set the SMTP registry for email mock support.
    pub fn smtp_registry(mut self, registry: Arc<dyn std::any::Any + Send + Sync>) -> Self {
        self.smtp_registry = Some(registry);
        self
    }

    /// Set the SMTP registry (optional variant).
    pub fn smtp_registry_opt(
        mut self,
        registry: Option<Arc<dyn std::any::Any + Send + Sync>>,
    ) -> Self {
        self.smtp_registry = registry;
        self
    }

    /// Set the MQTT broker for MQTT mock support.
    pub fn mqtt_broker(mut self, broker: Arc<dyn std::any::Any + Send + Sync>) -> Self {
        self.mqtt_broker = Some(broker);
        self
    }

    /// Set the MQTT broker (optional variant).
    pub fn mqtt_broker_opt(mut self, broker: Option<Arc<dyn std::any::Any + Send + Sync>>) -> Self {
        self.mqtt_broker = broker;
        self
    }

    /// Enable traffic shaping with the given shaper.
    pub fn with_traffic_shaping(mut self, shaper: TrafficShaper) -> Self {
        self.traffic_shaper = Some(shaper);
        self.traffic_shaping_enabled = true;
        self
    }

    /// Enable traffic shaping (optional variant, with explicit enabled flag).
    pub fn with_traffic_shaping_opt(
        mut self,
        shaper: Option<TrafficShaper>,
        enabled: bool,
    ) -> Self {
        self.traffic_shaper = shaper;
        self.traffic_shaping_enabled = enabled;
        self
    }

    /// Set request chaining configuration.
    pub fn with_chains(mut self, config: ChainConfig) -> Self {
        self.chain_config = Some(config);
        self
    }

    /// Set the health manager for health check endpoints.
    pub fn health_manager(mut self, manager: Arc<HealthManager>) -> Self {
        self.health_manager = Some(manager);
        self
    }

    /// Set the health manager (optional variant).
    pub fn health_manager_opt(mut self, manager: Option<Arc<HealthManager>>) -> Self {
        self.health_manager = manager;
        self
    }

    /// Enable deceptive deploy mode with production-like configuration.
    pub fn with_deceptive_deploy(mut self, config: DeceptiveDeployConfig) -> Self {
        self.deceptive_deploy_config = Some(config);
        self
    }

    /// Enable deceptive deploy mode (optional variant).
    pub fn with_deceptive_deploy_opt(mut self, config: Option<DeceptiveDeployConfig>) -> Self {
        self.deceptive_deploy_config = config;
        self
    }

    /// Set proxy configuration.
    pub fn proxy_config(mut self, config: ProxyConfig) -> Self {
        self.proxy_config = Some(config);
        self
    }

    /// Build the HTTP router with all configured options.
    ///
    /// This delegates to `build_router_with_chains_and_multi_tenant` internally,
    /// which is the most feature-complete router builder.
    #[allow(deprecated)]
    pub async fn build(self) -> Router {
        crate::build_router_with_chains_and_multi_tenant(
            self.spec_path,
            self.validation_options,
            self.chain_config,
            self.multi_tenant_config,
            self.route_configs,
            self.cors_config,
            self.ai_generator,
            self.smtp_registry,
            self.mqtt_broker,
            self.traffic_shaper,
            self.traffic_shaping_enabled,
            self.health_manager,
            self.mockai,
            self.deceptive_deploy_config,
            self.proxy_config,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_builder_default() {
        // Minimal builder produces a working router
        let _router = HttpRouterBuilder::new().build().await;
    }

    #[tokio::test]
    async fn test_builder_with_spec_path() {
        let _router = HttpRouterBuilder::new()
            .spec_path("/nonexistent/spec.yaml".to_string())
            .build()
            .await;
    }

    #[tokio::test]
    async fn test_builder_default_impl() {
        let _router = HttpRouterBuilder::default().build().await;
    }

    #[tokio::test]
    async fn test_builder_chain_multiple_options() {
        let _router = HttpRouterBuilder::new()
            .spec_path_opt(None)
            .validation_options_opt(None)
            .with_multi_tenant_opt(None)
            .route_configs_opt(None)
            .cors_config_opt(None)
            .with_mockai_opt(None)
            .smtp_registry_opt(None)
            .mqtt_broker_opt(None)
            .with_traffic_shaping_opt(None, false)
            .health_manager_opt(None)
            .with_deceptive_deploy_opt(None)
            .build()
            .await;
    }
}
