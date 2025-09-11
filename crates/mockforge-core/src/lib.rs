//! # MockForge Core
//!
//! Shared logic for MockForge including routing, validation, latency injection, and proxy functionality.

pub mod config;
pub mod error;
pub mod failure_injection;
pub mod latency;
pub mod openapi;
pub mod openapi_routes;
pub mod overrides;
pub mod priority_handler;
pub mod proxy;
pub mod record_replay;
pub mod request_fingerprint;
pub mod routing;
pub mod server_utils;
pub mod templating;
pub mod validation;
pub mod ws_proxy;

pub use config::{
    apply_env_overrides, load_config, load_config_with_fallback, save_config, ServerConfig,
};
pub use error::{Error, Result};
pub use failure_injection::{create_failure_injector, FailureInjector, FailureConfig, TagFailureConfig};
pub use latency::LatencyProfile;
pub use priority_handler::{PriorityHttpHandler, PriorityResponse, MockGenerator, MockResponse, SimpleMockGenerator};
pub use record_replay::{RecordedRequest, ReplayHandler, RecordHandler, RecordReplayHandler, list_fixtures, clean_old_fixtures, list_ready_fixtures, list_smoke_endpoints};
pub use request_fingerprint::{RequestFingerprint, ResponsePriority, ResponseSource, RequestHandlerResult};
pub use openapi::{OpenApiOperation, OpenApiRoute, OpenApiSchema, OpenApiSpec};
pub use openapi_routes::{
    create_registry_from_file, create_registry_from_json, OpenApiRouteRegistry,
};
pub use overrides::{OverrideRule, OverrideMode, Overrides, PatchOp};
pub use proxy::{ProxyConfig, ProxyHandler, ProxyResponse};
pub use routing::RouteRegistry;
pub use server_utils::errors::{json_error, json_success};
pub use server_utils::{create_socket_addr, localhost_socket_addr, wildcard_socket_addr};
pub use validation::Validator;
pub use ws_proxy::{WsProxyConfig, WsProxyHandler, WsProxyRule};


/// Core configuration for MockForge
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Config {
    /// Enable latency simulation
    pub latency_enabled: bool,
    /// Enable failure simulation
    pub failures_enabled: bool,
    /// Failure injection configuration
    pub failure_config: Option<FailureConfig>,
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
            failure_config: None,
            proxy: None,
            default_latency: LatencyProfile::default(),
        }
    }
}
