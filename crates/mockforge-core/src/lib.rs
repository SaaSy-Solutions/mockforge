//! # MockForge Core
//!
//! Core functionality and shared logic for the MockForge mocking framework.
//!
//! This crate provides the foundational building blocks used across all MockForge protocols
//! (HTTP, WebSocket, gRPC, GraphQL). It can be used as a library to programmatically create
//! and manage mock servers, or to build custom mocking solutions.
//!
//! ## Overview
//!
//! MockForge Core includes:
//!
//! - **Routing & Validation**: OpenAPI-based route registration and request validation
//! - **Request/Response Processing**: Template expansion, data generation, and transformation
//! - **Chaos Engineering**: Latency injection, failure simulation, and traffic shaping
//! - **Proxy & Hybrid Mode**: Forward requests to real backends with intelligent fallback
//! - **Request Chaining**: Multi-step request workflows with context passing
//! - **Workspace Management**: Organize and persist mock configurations
//! - **Observability**: Request logging, metrics collection, and tracing
//!
//! ## Quick Start: Embedding MockForge
//!
//! ### Creating a Simple HTTP Mock Server
//!
//! ```rust,no_run
//! use mockforge_core::{
//!     Config, LatencyProfile, OpenApiRouteRegistry, OpenApiSpec, Result, ValidationOptions,
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Load OpenAPI specification
//!     let spec = OpenApiSpec::from_file("api.json").await?;
//!
//!     // Create route registry with validation
//!     let registry = OpenApiRouteRegistry::new_with_options(spec, ValidationOptions::default());
//!
//!     // Configure core features
//!     let config = Config {
//!         latency_enabled: true,
//!         failures_enabled: false,
//!         default_latency: LatencyProfile::with_normal_distribution(400, 120.0),
//!         ..Default::default()
//!     };
//!
//!     // Build your HTTP server with the registry
//!     // (See mockforge-http crate for router building)
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Request Chaining
//!
//! Chain multiple requests together with shared context:
//!
//! ```rust,no_run
//! use mockforge_core::{
//!     ChainConfig, ChainDefinition, ChainLink, ChainRequest, RequestChainRegistry, Result,
//! };
//! use mockforge_core::request_chaining::RequestBody;
//! use serde_json::json;
//! use std::collections::HashMap;
//!
//! # async fn example() -> Result<()> {
//! let registry = RequestChainRegistry::new(ChainConfig::default());
//!
//! // Define a chain: create user → add to group → verify membership
//! let chain = ChainDefinition {
//!     id: "user_onboarding".to_string(),
//!     name: "User Onboarding".to_string(),
//!     description: Some("Create user → add to group".to_string()),
//!     config: ChainConfig {
//!         enabled: true,
//!         ..ChainConfig::default()
//!     },
//!     links: vec![
//!         ChainLink {
//!             request: ChainRequest {
//!                 id: "create_user".to_string(),
//!                 method: "POST".to_string(),
//!                 url: "https://api.example.com/users".to_string(),
//!                 headers: HashMap::new(),
//!                 body: Some(RequestBody::json(json!({"name": "{{faker.name}}"}))),
//!                 depends_on: Vec::new(),
//!                 timeout_secs: None,
//!                 expected_status: None,
//!                 scripting: None,
//!             },
//!             extract: HashMap::from([("user_id".to_string(), "create_user.body.id".to_string())]),
//!             store_as: Some("create_user_response".to_string()),
//!         },
//!         ChainLink {
//!             request: ChainRequest {
//!                 id: "add_to_group".to_string(),
//!                 method: "POST".to_string(),
//!                 url: "https://api.example.com/groups/{{user_id}}/members".to_string(),
//!                 headers: HashMap::new(),
//!                 body: None,
//!                 depends_on: vec!["create_user".to_string()],
//!                 timeout_secs: None,
//!                 expected_status: None,
//!                 scripting: None,
//!             },
//!             extract: HashMap::new(),
//!             store_as: None,
//!         },
//!     ],
//!     variables: HashMap::new(),
//!     tags: vec!["onboarding".to_string()],
//! };
//!
//! registry.store().register_chain(chain).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Latency & Failure Injection
//!
//! Simulate realistic network conditions and errors:
//!
//! ```rust,no_run
//! use mockforge_core::{LatencyProfile, FailureConfig, create_failure_injector};
//!
//! // Configure latency simulation
//! let latency = LatencyProfile::with_normal_distribution(400, 120.0)
//!     .with_min_ms(100)
//!     .with_max_ms(800);
//!
//! // Configure failure injection
//! let failure_config = FailureConfig {
//!     global_error_rate: 0.05, // 5% of requests fail
//!     default_status_codes: vec![500, 502, 503],
//!     ..Default::default()
//! };
//!
//! let injector = create_failure_injector(true, Some(failure_config));
//! ```
//!
//! ## Key Modules
//!
//! ### OpenAPI Support
//! - [`openapi`]: Parse and work with OpenAPI specifications
//! - [`openapi_routes`]: Register routes from OpenAPI specs with validation
//! - [`validation`]: Request/response validation against schemas
//!
//! ### Request Processing
//! - [`routing`]: Route matching and registration
//! - [`templating`]: Template variable expansion ({{uuid}}, {{now}}, etc.)
//! - [`request_chaining`]: Multi-step request workflows
//! - [`overrides`]: Dynamic request/response modifications
//!
//! ### Chaos Engineering
//! - [`latency`]: Latency injection with configurable profiles
//! - [`failure_injection`]: Simulate service failures and errors
//! - [`traffic_shaping`]: Bandwidth limiting and packet loss
//!
//! ### Proxy & Hybrid
//! - [`proxy`]: Forward requests to upstream services
//! - [`ws_proxy`]: WebSocket proxy with message transformation
//!
//! ### Persistence & Import
//! - [`workspace`]: Workspace management for organizing mocks
//! - [`workspace_import`]: Import from Postman, Insomnia, cURL, HAR
//! - [`record_replay`]: Record real requests and replay as fixtures
//!
//! ### Observability
//! - [`request_logger`]: Centralized request logging
//! - [`performance`]: Performance metrics and profiling
//!
//! ## Feature Flags
//!
//! This crate supports several optional features:
//!
//! - `openapi`: OpenAPI specification support (enabled by default)
//! - `validation`: Request/response validation (enabled by default)
//! - `templating`: Template expansion (enabled by default)
//! - `chaos`: Chaos engineering features (enabled by default)
//! - `proxy`: Proxy and hybrid mode (enabled by default)
//! - `workspace`: Workspace management (enabled by default)
//!
//! ## Examples
//!
//! See the [examples directory](https://github.com/SaaSy-Solutions/mockforge/tree/main/examples)
//! for complete working examples.
//!
//! ## Related Crates
//!
//! - [`mockforge-http`](https://docs.rs/mockforge-http): HTTP/REST mock server
//! - [`mockforge-grpc`](https://docs.rs/mockforge-grpc): gRPC mock server
//! - [`mockforge-ws`](https://docs.rs/mockforge-ws): WebSocket mock server
//! - [`mockforge-graphql`](https://docs.rs/mockforge-graphql): GraphQL mock server
//! - [`mockforge-plugin-core`](https://docs.rs/mockforge-plugin-core): Plugin development
//! - [`mockforge-data`](https://docs.rs/mockforge-data): Synthetic data generation
//!
//! ## Documentation
//!
//! - [MockForge Book](https://docs.mockforge.dev/)
//! - [API Reference](https://docs.rs/mockforge-core)
//! - [GitHub Repository](https://github.com/SaaSy-Solutions/mockforge)

#![allow(deprecated)]

pub mod ab_testing;
pub mod ai_contract_diff;
pub mod ai_response;
/// AI Studio - Unified AI Copilot for all AI-powered features
pub mod ai_studio;
/// Behavioral cloning of backends - learn from recorded traffic to create realistic mock behavior
pub mod behavioral_cloning;
pub mod behavioral_economics;
pub mod cache;
pub mod chain_execution;
pub mod chaos_utilities;
pub mod codegen;
/// Collection export utilities for exporting mock data in various formats
pub mod collection_export;
pub mod conditions;
pub mod config;
/// Cross-protocol consistency engine for unified state across all protocols
pub mod consistency;
/// Consumer-driven contracts for tracking usage and detecting consumer-specific breaking changes
pub mod consumer_contracts;
/// Contract validation for ensuring API contracts match specifications
pub mod contract_drift;
/// Contract validation for ensuring API contracts match specifications
pub mod contract_validation;
/// Contract webhooks for notifying external systems about contract changes
pub mod contract_webhooks;
pub mod custom_fixture;
/// Data source abstraction for loading test data from multiple sources
pub mod data_source;
/// Deceptive canary mode for routing team traffic to deceptive deploys
pub mod deceptive_canary;
/// Docker Compose integration for containerized mock deployments
pub mod docker_compose;
/// GitOps integration for drift budget violations
pub mod drift_gitops;
pub mod encryption;
pub mod error;
pub mod failure_analysis;
pub mod failure_injection;
pub mod fidelity;
pub mod generate_config;
pub mod generative_schema;
pub mod git_watch;
pub mod graph;
pub mod import;
pub mod incidents;
pub mod intelligent_behavior;
pub mod latency;
pub mod lifecycle;
pub mod multi_tenant;
pub mod network_profiles;
pub mod openapi;
pub mod openapi_routes;
pub mod output_control;
pub mod overrides;
pub mod performance;
/// Pillar usage tracking utilities
pub mod pillar_tracking;
/// Pillar metadata system for compile-time pillar tagging
pub mod pillars;
pub mod pr_generation;
pub mod priority_handler;
pub mod protocol_abstraction;
pub mod proxy;
pub mod reality;
pub mod reality_continuum;
pub mod record_replay;
pub mod request_capture;
pub mod request_chaining;
pub mod request_fingerprint;
pub mod request_logger;
pub mod request_scripting;
// Route chaos has been moved to mockforge-route-chaos crate to avoid Send issues
// Import directly from mockforge-route-chaos crate instead of re-exporting here
// to avoid circular dependency (mockforge-route-chaos depends on mockforge-core for config types)
pub mod persona_lifecycle_time;
pub mod routing;
/// Runtime validation for SDKs (request/response validation at runtime)
pub mod runtime_validation;
/// Scenario Studio - Visual editor for co-editing business flows
pub mod scenario_studio;
pub mod scenarios;
pub mod schema_diff;
pub mod security;
pub mod server_utils;
/// Time travel and snapshot functionality for saving and restoring system states
pub mod snapshots;
pub mod spec_parser;
pub mod stateful_handler;
pub mod sync_watcher;
/// Template expansion utilities (Send-safe, isolated from templating module)
pub mod template_expansion;
/// Template library system for shared templates, versioning, and marketplace
pub mod template_library;
pub mod templating;
pub mod time_travel;
pub mod time_travel_handler;
pub mod traffic_shaping;
pub mod validation;
pub mod verification;
pub mod voice;
pub mod workspace;
pub mod workspace_import;
pub mod workspace_persistence;
pub mod ws_proxy;

pub use ab_testing::{
    apply_variant_to_response, select_variant, ABTestConfig, ABTestReport,
    ABTestingMiddlewareState, MockVariant, VariantAllocation, VariantAnalytics, VariantComparison,
    VariantManager, VariantSelectionStrategy,
};
pub use behavioral_cloning::{
    AmplificationScope, BehavioralSequence, EdgeAmplificationConfig, EdgeAmplifier,
    EndpointProbabilityModel, ErrorPattern, LatencyDistribution, PayloadVariation,
    ProbabilisticModel, SequenceLearner, SequenceStep,
};
pub use chain_execution::{ChainExecutionEngine, ChainExecutionResult, ChainExecutionStatus};
pub use chaos_utilities::{ChaosConfig, ChaosEngine, ChaosResult, ChaosStatistics};
pub use conditions::{evaluate_condition, ConditionContext, ConditionError};
pub use config::{
    apply_env_overrides, load_config, load_config_with_fallback, save_config, ApiKeyConfig,
    AuthConfig, ServerConfig,
};
pub use consistency::{
    ConsistencyEngine, EntityState, ProtocolState, SessionInfo, StateChangeEvent, UnifiedState,
};
pub use custom_fixture::{CustomFixture, CustomFixtureLoader};
pub use data_source::{
    DataSource, DataSourceConfig, DataSourceContent, DataSourceFactory, DataSourceManager,
    DataSourceType, GitDataSource, HttpDataSource, LocalDataSource,
};
pub use deceptive_canary::{
    CanaryRoutingStrategy, CanaryStats, DeceptiveCanaryConfig, DeceptiveCanaryRouter,
    TeamIdentifiers,
};
pub use error::{Error, Result};
pub use failure_analysis::{
    ContributingFactor, FailureContext, FailureContextCollector, FailureNarrative,
    FailureNarrativeGenerator, NarrativeFrame,
};
pub use failure_injection::{
    create_failure_injector, FailureConfig, FailureInjector, TagFailureConfig,
};
pub use fidelity::{FidelityCalculator, FidelityScore, SampleComparator, SchemaComparator};
pub use generate_config::{
    discover_config_file, load_generate_config, load_generate_config_with_fallback,
    save_generate_config, BarrelType, GenerateConfig, GenerateOptions, InputConfig, OutputConfig,
    PluginConfig,
};
pub use git_watch::{GitWatchConfig, GitWatchService};
pub use graph::{
    builder::GraphBuilder, relationships, ClusterType, EdgeType, GraphCluster, GraphData,
    GraphEdge, GraphNode, NodeType, Protocol as GraphProtocol,
};
pub use latency::LatencyProfile;
pub use lifecycle::{
    LifecycleHook, LifecycleHookRegistry, MockLifecycleEvent, RequestContext, ResponseContext,
    ServerLifecycleEvent,
};
pub use multi_tenant::{
    MultiTenantConfig, MultiTenantWorkspaceRegistry, RoutingStrategy, TenantWorkspace,
    WorkspaceContext, WorkspaceRouter, WorkspaceStats,
};
pub use network_profiles::{NetworkProfile, NetworkProfileCatalog};
pub use openapi::{
    OpenApiOperation, OpenApiRoute, OpenApiSchema, OpenApiSecurityRequirement, OpenApiSpec,
};
pub use openapi_routes::{
    create_registry_from_file, create_registry_from_json, OpenApiRouteRegistry, ValidationOptions,
};
pub use output_control::{
    apply_banner, apply_extension, apply_file_naming_template, build_file_naming_context,
    process_generated_file, BarrelGenerator, FileNamingContext, GeneratedFile,
};
pub use overrides::{OverrideMode, OverrideRule, Overrides, PatchOp};
pub use pillars::{Pillar, PillarMetadata};
pub use priority_handler::{
    MockGenerator, MockResponse, PriorityHttpHandler, PriorityResponse, SimpleMockGenerator,
};
pub use protocol_abstraction::{
    MessagePattern, MiddlewareChain, Protocol, ProtocolMiddleware, ProtocolRequest,
    ProtocolResponse, RequestMatcher, ResponseStatus, SpecOperation, SpecRegistry,
    ValidationError as ProtocolValidationError, ValidationResult as ProtocolValidationResult,
};
pub use proxy::{ProxyConfig, ProxyHandler, ProxyResponse};
pub use reality::{PresetMetadata, RealityConfig, RealityEngine, RealityLevel, RealityPreset};
pub use reality_continuum::{
    ContinuumConfig, ContinuumRule, MergeStrategy, RealityContinuumEngine, ResponseBlender,
    TimeSchedule, TransitionCurve, TransitionMode,
};
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
    create_http_log_entry_with_query,
    create_grpc_log_entry, create_http_log_entry, create_websocket_log_entry, get_global_logger,
    init_global_logger, log_request_global, CentralizedRequestLogger, RequestLogEntry,
};
// Route chaos types moved to mockforge-route-chaos crate
// Import directly: use mockforge_route_chaos::{RouteChaosInjector, RouteFaultResponse, RouteMatcher};
pub use routing::{HttpMethod, Route, RouteRegistry};
pub use runtime_validation::{
    RuntimeValidationError, RuntimeValidationResult, RuntimeValidatorConfig, SchemaMetadata,
};
pub use scenario_studio::{
    ConditionOperator, FlowCondition, FlowConnection, FlowDefinition, FlowExecutionResult,
    FlowExecutor, FlowPosition, FlowStep, FlowStepResult, FlowType, FlowVariant, StepType,
};
pub use scenarios::types::StepResult;
pub use scenarios::{
    ScenarioDefinition, ScenarioExecutor, ScenarioParameter, ScenarioRegistry, ScenarioResult,
    ScenarioStep,
};
pub use schema_diff::{to_enhanced_422_json, validation_diff, ValidationError};
pub use server_utils::errors::{json_error, json_success};
pub use server_utils::{create_socket_addr, localhost_socket_addr, wildcard_socket_addr};
pub use snapshots::{SnapshotComponents, SnapshotManager, SnapshotManifest, SnapshotMetadata};
pub use spec_parser::{GraphQLValidator, OpenApiValidator, SpecFormat};
pub use stateful_handler::{
    ResourceIdExtract, StateInfo, StateResponse, StatefulConfig, StatefulResponse,
    StatefulResponseHandler, TransitionTrigger,
};
pub use sync_watcher::{FileChange, SyncEvent, SyncService, SyncWatcher};
pub use template_library::{
    TemplateLibrary, TemplateLibraryEntry, TemplateLibraryManager, TemplateMarketplace,
    TemplateMetadata, TemplateVersion,
};
pub use templating::{expand_str, expand_tokens};
pub use time_travel::{
    cron::{CronJob, CronJobAction, CronScheduler},
    get_global_clock, is_time_travel_enabled, now as time_travel_now, register_global_clock,
    unregister_global_clock, RepeatConfig, ResponseScheduler, ScheduledResponse, TimeScenario,
    TimeTravelConfig, TimeTravelManager, TimeTravelStatus, VirtualClock,
};
pub use time_travel_handler::{
    time_travel_middleware, ScheduledResponseWrapper, TimeTravelHandler,
};
pub use traffic_shaping::{BandwidthConfig, BurstLossConfig, TrafficShaper, TrafficShapingConfig};
pub use uuid::Uuid;
pub use validation::{validate_openapi_operation_security, validate_openapi_security, Validator};
pub use verification::{
    matches_verification_pattern, verify_at_least, verify_never, verify_requests, verify_sequence,
    VerificationCount, VerificationRequest, VerificationResult,
};
pub use voice::{
    ConversationContext, ConversationManager, ConversationState, GeneratedWorkspaceScenario,
    HookTranspiler, ParsedCommand, ParsedWorkspaceScenario, VoiceCommandParser, VoiceSpecGenerator,
    WorkspaceConfigSummary, WorkspaceScenarioGenerator,
};
pub use workspace::promotion_trait::PromotionService;
pub use workspace::{EntityId, Folder, MockRequest, Workspace, WorkspaceConfig, WorkspaceRegistry};
pub use workspace_import::{
    create_workspace_from_curl, create_workspace_from_har, create_workspace_from_insomnia,
    create_workspace_from_postman, import_postman_to_existing_workspace,
    import_postman_to_workspace, WorkspaceImportConfig, WorkspaceImportResult,
};
pub use workspace_persistence::WorkspacePersistence;
pub use ws_proxy::{WsProxyConfig, WsProxyHandler, WsProxyRule};
// Note: ValidationError and ValidationResult from spec_parser conflict with schema_diff::ValidationError
// Use qualified paths: spec_parser::ValidationError, spec_parser::ValidationResult

/// Core configuration for MockForge
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
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
    /// Random chaos configuration
    pub chaos_random: Option<ChaosConfig>,
    /// Maximum number of request logs to keep in memory (default: 1000)
    /// Helps prevent unbounded memory growth from request logging
    pub max_request_logs: usize,
    /// Time travel configuration for temporal testing
    pub time_travel: TimeTravelConfig,
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
            chaos_random: None,
            max_request_logs: 1000, // Default: keep last 1000 requests
            time_travel: TimeTravelConfig::default(),
        }
    }
}

impl Config {
    /// Create a ChaosEngine from the chaos_random configuration if enabled
    pub fn create_chaos_engine(&self) -> Option<ChaosEngine> {
        self.chaos_random.as_ref().map(|config| ChaosEngine::new(config.clone()))
    }

    /// Check if random chaos mode is enabled
    pub fn is_chaos_random_enabled(&self) -> bool {
        self.chaos_random.as_ref().map(|c| c.enabled).unwrap_or(false)
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
        let config = Config {
            latency_enabled: false,
            failures_enabled: true,
            ..Default::default()
        };

        // Serialize and deserialize
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();

        assert!(!deserialized.latency_enabled);
        assert!(deserialized.failures_enabled);
        assert!(deserialized.overrides_enabled);
    }

    #[test]
    fn test_config_with_custom_values() {
        let config = Config {
            latency_enabled: false,
            failures_enabled: true,
            ..Default::default()
        };

        assert!(!config.latency_enabled);
        assert!(config.failures_enabled);
    }
}
