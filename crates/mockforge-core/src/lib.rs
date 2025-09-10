//! # MockForge Core
//!
//! Shared logic for MockForge including routing, validation, latency injection, and proxy functionality.

pub mod config;
pub mod error;
pub mod latency;
pub mod openapi;
pub mod openapi_routes;
pub mod proxy;
pub mod routing;
pub mod server_utils;
pub mod validation;

pub use config::{
    apply_env_overrides, load_config, load_config_with_fallback, save_config, ServerConfig,
};
pub use error::{Error, Result};
pub use latency::LatencyProfile;
pub use openapi::{OpenApiOperation, OpenApiRoute, OpenApiSchema, OpenApiSpec};
pub use openapi_routes::{
    create_registry_from_file, create_registry_from_json, OpenApiRouteRegistry,
};
pub use proxy::ProxyConfig;
pub use routing::RouteRegistry;
pub use server_utils::errors::{json_error, json_success};
pub use server_utils::{create_socket_addr, localhost_socket_addr, wildcard_socket_addr};
pub use validation::Validator;

/// Core configuration for MockForge
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Config {
    /// Enable latency simulation
    pub latency_enabled: bool,
    /// Enable failure simulation
    pub failures_enabled: bool,
    /// Proxy configuration
    pub proxy: Option<ProxyConfig>,
    /// Default latency profile
    pub default_latency: LatencyProfile,
}

/// Default configuration
impl Default for Config {
    fn default() -> Self {
        Self {
            latency_enabled: true,
            failures_enabled: false,
            proxy: None,
            default_latency: LatencyProfile::default(),
        }
    }
}
