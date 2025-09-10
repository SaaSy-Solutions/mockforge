//! # MockForge Core
//!
//! Shared logic for MockForge including routing, validation, latency injection, and proxy functionality.

pub mod routing;
pub mod validation;
pub mod latency;
pub mod proxy;
pub mod error;
pub mod config;
pub mod server_utils;
pub mod openapi;
pub mod openapi_routes;

pub use error::{Error, Result};
pub use routing::RouteRegistry;
pub use validation::Validator;
pub use latency::LatencyProfile;
pub use proxy::ProxyConfig;
pub use config::{ServerConfig, load_config, save_config, load_config_with_fallback, apply_env_overrides};
pub use server_utils::{create_socket_addr, localhost_socket_addr, wildcard_socket_addr};
pub use server_utils::errors::{json_error, json_success};
pub use openapi::{OpenApiSpec, OpenApiRoute, OpenApiOperation, OpenApiSchema};
pub use openapi_routes::{OpenApiRouteRegistry, create_registry_from_file, create_registry_from_json};

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
