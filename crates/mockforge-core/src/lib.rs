//! # MockForge Core
//!
//! Shared logic for MockForge including routing, validation, latency injection, and proxy functionality.

pub mod cache;
pub mod chain_execution;
pub mod collection_export;
pub mod conditions;
pub mod config;
pub mod contract_validation;
pub mod docker_compose;
pub mod encryption;
pub mod error;
pub mod failure_injection;
pub mod import;
pub mod latency;
pub mod openapi;
pub mod openapi_routes;
pub mod overrides;
pub mod performance;
pub mod priority_handler;
pub mod protocol_abstraction;
pub mod proxy;
pub mod record_replay;
pub mod request_chaining;
pub mod request_fingerprint;
pub mod request_logger;
pub mod request_scripting;
pub mod routing;
pub mod schema_diff;
pub mod server_utils;
pub mod sync_watcher;
pub mod templating;
pub mod traffic_shaping;
pub mod validation;
pub mod workspace;
pub mod workspace_import;
pub mod workspace_persistence;
pub mod ws_proxy;

pub use chain_execution::{ChainExecutionEngine, ChainExecutionResult, ChainExecutionStatus};
pub use conditions::{evaluate_condition, ConditionContext, ConditionError};
pub use config::{
    apply_env_overrides, load_config, load_config_with_fallback, save_config, ApiKeyConfig,
    AuthConfig, ServerConfig,
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
    create_registry_from_file, create_registry_from_json, OpenApiRouteRegistry, ValidationOptions,
};
pub use overrides::{OverrideMode, OverrideRule, Overrides, PatchOp};
pub use priority_handler::{
    MockGenerator, MockResponse, PriorityHttpHandler, PriorityResponse, SimpleMockGenerator,
};
pub use protocol_abstraction::{
    MiddlewareChain, Protocol, ProtocolMiddleware, ProtocolRequest, ProtocolResponse,
    RequestMatcher, ResponseStatus, SpecOperation, SpecRegistry,
    ValidationError as ProtocolValidationError, ValidationResult as ProtocolValidationResult,
};
pub use proxy::{ProxyConfig, ProxyHandler, ProxyResponse};
pub use record_replay::{
    clean_old_fixtures, list_fixtures, list_ready_fixtures, list_smoke_endpoints, RecordHandler,
    RecordReplayHandler, RecordedRequest, ReplayHandler,
};
pub use request_chaining::{
    ChainConfig, ChainContext, ChainDefinition, ChainExecutionContext, ChainLink, ChainRequest,
    ChainResponse, ChainStore, ChainTemplatingContext, RequestChainRegistry,
};
pub use request_fingerprint::{
    RequestFingerprint, RequestHandlerResult, ResponsePriority, ResponseSource,
};
pub use request_logger::{
    create_grpc_log_entry, create_http_log_entry, create_websocket_log_entry, get_global_logger,
    init_global_logger, log_request_global, CentralizedRequestLogger, RequestLogEntry,
};
pub use routing::{HttpMethod, Route, RouteRegistry};
pub use schema_diff::{to_enhanced_422_json, validation_diff, ValidationError};
pub use server_utils::errors::{json_error, json_success};
pub use server_utils::{create_socket_addr, localhost_socket_addr, wildcard_socket_addr};
pub use sync_watcher::{FileChange, SyncEvent, SyncService, SyncWatcher};
pub use templating::{expand_str, expand_tokens};
pub use traffic_shaping::{BandwidthConfig, BurstLossConfig, TrafficShaper, TrafficShapingConfig};
pub use uuid::Uuid;
pub use validation::{validate_openapi_operation_security, validate_openapi_security, Validator};
pub use workspace::{EntityId, Folder, MockRequest, Workspace, WorkspaceConfig, WorkspaceRegistry};
pub use workspace_import::{
    create_workspace_from_curl, create_workspace_from_har, create_workspace_from_insomnia,
    create_workspace_from_postman, import_postman_to_existing_workspace,
    import_postman_to_workspace, WorkspaceImportConfig, WorkspaceImportResult,
};
pub use workspace_persistence::WorkspacePersistence;
pub use ws_proxy::{WsProxyConfig, WsProxyHandler, WsProxyRule};

/// Core configuration for MockForge
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Config {
    /// Enable latency simulation
    pub latency_enabled: bool,
    /// Enable failure simulation
    pub failures_enabled: bool,
    /// Enable response overrides
    pub overrides_enabled: bool,
    /// Enable traffic shaping (bandwidth + burst loss)
    pub traffic_shaping_enabled: bool,
    /// Failure injection configuration
    pub failure_config: Option<FailureConfig>,
    /// Proxy configuration
    pub proxy: Option<ProxyConfig>,
    /// Default latency profile
    pub default_latency: LatencyProfile,
    /// Traffic shaping configuration
    pub traffic_shaping: TrafficShapingConfig,
}

/// Default configuration
impl Default for Config {
    fn default() -> Self {
        Self {
            latency_enabled: true,
            failures_enabled: false,
            overrides_enabled: true,
            traffic_shaping_enabled: false,
            failure_config: None,
            proxy: None,
            default_latency: LatencyProfile::default(),
            traffic_shaping: TrafficShapingConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.latency_enabled);
        assert!(!config.failures_enabled);
        assert!(config.overrides_enabled);
        assert!(!config.traffic_shaping_enabled);
        assert!(config.failure_config.is_none());
        assert!(config.proxy.is_none());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("latency_enabled"));
        assert!(json.contains("failures_enabled"));
    }

    #[test]
    fn test_config_deserialization() {
        // Use default config and modify
        let mut config = Config::default();
        config.latency_enabled = false;
        config.failures_enabled = true;

        // Serialize and deserialize
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();

        assert!(!deserialized.latency_enabled);
        assert!(deserialized.failures_enabled);
        assert!(deserialized.overrides_enabled);
    }

    #[test]
    fn test_config_with_custom_values() {
        let mut config = Config::default();
        config.latency_enabled = false;
        config.failures_enabled = true;

        assert!(!config.latency_enabled);
        assert!(config.failures_enabled);
    }
}
