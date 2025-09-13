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
pub mod request_logger;
pub mod routing;
pub mod server_utils;
pub mod templating;
pub mod validation;
pub mod ws_proxy;

pub use config::{
    apply_env_overrides, load_config, load_config_with_fallback, save_config, ServerConfig,
};
pub use error::{Error, Result};
pub use failure_injection::{
    create_failure_injector, FailureConfig, FailureInjector, TagFailureConfig,
};
pub use latency::LatencyProfile;
pub use openapi::{
    OpenApiOperation, OpenApiRoute, OpenApiSchema, OpenApiSecurityRequirement, OpenApiSpec,
};
pub use openapi_routes::{
    create_registry_from_file, create_registry_from_json, OpenApiRouteRegistry,
};
pub use overrides::{OverrideMode, OverrideRule, Overrides, PatchOp};
pub use priority_handler::{
    MockGenerator, MockResponse, PriorityHttpHandler, PriorityResponse, SimpleMockGenerator,
};
pub use proxy::{ProxyConfig, ProxyHandler, ProxyResponse};
pub use record_replay::{
    clean_old_fixtures, list_fixtures, list_ready_fixtures, list_smoke_endpoints, RecordHandler,
    RecordReplayHandler, RecordedRequest, ReplayHandler,
};
pub use request_fingerprint::{
    RequestFingerprint, RequestHandlerResult, ResponsePriority, ResponseSource,
};
pub use request_logger::{
    create_grpc_log_entry, create_http_log_entry, create_websocket_log_entry, get_global_logger,
    init_global_logger, log_request_global, CentralizedRequestLogger, RequestLogEntry,
};
pub use routing::RouteRegistry;
pub use server_utils::errors::{json_error, json_success};
pub use server_utils::{create_socket_addr, localhost_socket_addr, wildcard_socket_addr};
pub use validation::{validate_openapi_operation_security, validate_openapi_security, Validator};
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
