//! Pillars: [Reality]
//!
//! MockForge Chaos Engineering
//!
//! Provides chaos engineering capabilities including traffic shaping, fault injection,
//! and resilience pattern testing.

pub mod ab_testing;
pub mod advanced_analytics;
pub mod advanced_orchestration;
pub mod alerts;
pub mod analytics;
pub mod api;
pub mod auto_remediation;
#[cfg(feature = "enterprise")]
pub mod chaos_mesh;
#[cfg(feature = "enterprise")]
pub mod collaboration;
pub mod config;
#[cfg(feature = "enterprise")]
pub mod dashboard;
#[cfg(feature = "enterprise")]
pub mod distributed_coordinator;
pub mod failure_designer;
pub mod fault;
#[cfg(feature = "enterprise")]
pub mod gitops;
pub mod incident_replay;
#[cfg(feature = "enterprise")]
pub mod integrations;
pub mod latency;
pub mod latency_metrics;
pub mod metrics;
pub mod middleware;
#[cfg(feature = "ml")]
pub mod ml_anomaly_detector;
#[cfg(feature = "ml")]
pub mod ml_assertion_generator;
#[cfg(feature = "ml")]
pub mod ml_parameter_optimizer;
#[cfg(feature = "ml")]
pub mod multi_armed_bandit;
#[cfg(feature = "enterprise")]
pub mod multi_cluster;
#[cfg(feature = "enterprise")]
pub mod multi_tenancy;
#[cfg(feature = "enterprise")]
pub mod observability_api;
pub mod plugins;
#[cfg(feature = "ml")]
pub mod predictive_remediation;
pub mod protocols;
pub mod rate_limit;
pub mod recommendations;
#[cfg(feature = "ml")]
pub mod reinforcement_learning;
pub mod resilience;
pub mod resilience_api;
pub mod scenario_orchestrator;
pub mod scenario_recorder;
pub mod scenario_replay;
pub mod scenario_scheduler;
pub mod scenarios;
#[cfg(feature = "enterprise")]
pub mod template_marketplace;
#[cfg(feature = "enterprise")]
pub mod trace_collector;
pub mod traffic_shaping;
#[cfg(feature = "enterprise")]
pub mod version_control;

// Modules extracted from mockforge-core (Phase 6a)
pub mod core_chaos_utilities;
pub mod core_failure_injection;
pub mod core_network_profiles;
pub mod core_traffic_shaping;

pub use ab_testing::{
    ABTest, ABTestConfig, ABTestStats, ABTestStatus, ABTestingEngine, MetricComparison, MetricType,
    SingleMetricComparison, SuccessCriteria, TestConclusion, TestVariant, VariantMetrics,
    VariantResults,
};
pub use advanced_analytics::{
    AdvancedAnalyticsEngine, Anomaly, AnomalyType as AnalyticsAnomalyType, CorrelationAnalysis,
    DataPoint, HealthFactor, HealthScore, PredictiveInsight, TrendAnalysis, TrendDirection,
};
pub use advanced_orchestration::{
    AdvancedOrchestratedScenario, AdvancedScenarioStep, Assertion, Condition, ConditionalStep,
    ExecutionContext, ExecutionReport, Hook, HookAction, HookType, OrchestrationLibrary,
    RetryConfig, StepResult,
};
pub use alerts::{
    Alert, AlertHandler, AlertManager, AlertRule, AlertRuleType, AlertSeverity, AlertType,
};
pub use analytics::{ChaosAnalytics, ChaosImpact, MetricsBucket, TimeBucket};
pub use api::{create_chaos_api_router, ProfileManager};
pub use auto_remediation::{
    ApprovalRequest, EffectivenessMetrics, RemediationAction, RemediationConfig, RemediationEngine,
    RemediationResult, RemediationStats, RemediationStatus, RiskAssessment as AutoRiskAssessment,
    RiskLevel, RollbackData, SafetyCheck, SystemMetrics,
};
#[cfg(feature = "enterprise")]
pub use chaos_mesh::{
    ChaosMeshClient, ChaosMeshExperiment, ExperimentSpec, ExperimentStatus, ExperimentType,
    NetworkChaosAction, NetworkDelay, NetworkLoss, PodChaosAction, PodSelector, StressConfig,
};
#[cfg(feature = "enterprise")]
pub use collaboration::{
    ChangeType, CollaborationChange, CollaborationManager, CollaborationMessage,
    CollaborationSession, CollaborationUser, CursorPosition,
};
pub use config::{
    BulkheadConfig, ChaosConfig, CircuitBreakerConfig, CorruptionType, ErrorPattern,
    FaultInjectionConfig, LatencyConfig, NetworkProfile, RateLimitConfig, TrafficShapingConfig,
};
#[cfg(feature = "enterprise")]
pub use dashboard::{DashboardManager, DashboardQuery, DashboardStats, DashboardUpdate};
#[cfg(feature = "enterprise")]
pub use distributed_coordinator::{
    CoordinationMode, DistributedCoordinator, DistributedTask, ExecutionMetrics, LeaderState, Node,
    NodeExecutionState, NodeStatus, TaskStatus,
};
pub use failure_designer::{
    ConditionOperator, ConditionType, FailureCondition, FailureDesignRule, FailureDesigner,
    FailureTarget, FailureType,
};
pub use fault::{FaultInjector, FaultType};
#[cfg(feature = "enterprise")]
pub use gitops::{GitOpsAuth, GitOpsConfig, GitOpsManager, SyncState, SyncStatus};
pub use incident_replay::{
    IncidentEvent, IncidentEventType, IncidentFormatAdapter, IncidentReplayGenerator,
    IncidentTimeline,
};
#[cfg(feature = "enterprise")]
pub use integrations::{
    GrafanaConfig, GrafanaIntegration, IntegrationConfig, IntegrationManager, JiraConfig,
    JiraIntegration, Notification, NotificationResults, NotificationSeverity, PagerDutyConfig,
    PagerDutyIntegration, SlackConfig, SlackNotifier, TeamsConfig, TeamsNotifier,
};
pub use latency::LatencyInjector;
pub use latency_metrics::{LatencyMetricsTracker, LatencySample, LatencyStats};
pub use metrics::{registry as metrics_registry, ChaosMetrics, CHAOS_METRICS};
pub use middleware::{chaos_middleware, ChaosMiddleware};
#[cfg(feature = "ml")]
pub use ml_anomaly_detector::{
    Anomaly as MLAnomaly, AnomalyDetector, AnomalyDetectorConfig, AnomalySeverity,
    AnomalyType as MLAnomalyType, MetricBaseline, TimeSeriesPoint,
};
#[cfg(feature = "ml")]
pub use ml_assertion_generator::{
    AssertionGenerator, AssertionGeneratorConfig, AssertionOperator, AssertionType,
    ExecutionDataPoint, GeneratedAssertion, MetricStats,
};
#[cfg(feature = "ml")]
pub use ml_parameter_optimizer::{
    ExpectedImpact, OptimizationObjective, OptimizationRecommendation, OptimizerConfig,
    OrchestrationRun, ParameterBounds, ParameterOptimizer, RunMetrics,
};
#[cfg(feature = "ml")]
pub use multi_armed_bandit::{
    Arm, ArmReport, BanditReport, BanditStrategy, MultiArmedBandit, ThompsonSampling,
    TrafficAllocator, UCB1,
};
#[cfg(feature = "enterprise")]
pub use multi_cluster::{
    ClusterTarget, ExecutionStatus, MultiClusterOrchestration, MultiClusterOrchestrator,
    MultiClusterStatus, SyncMode,
};
#[cfg(feature = "enterprise")]
pub use multi_tenancy::{
    MultiTenancyError, ResourceQuota, ResourceUsage, Tenant, TenantManager, TenantPermissions,
    TenantPlan,
};
#[cfg(feature = "enterprise")]
pub use observability_api::{create_observability_router, ObservabilityState};
pub use plugins::{
    ChaosPlugin, CustomFaultPlugin, MetricsPlugin, PluginCapability, PluginConfig, PluginContext,
    PluginError, PluginHook, PluginMetadata, PluginRegistry, PluginResult,
};
#[cfg(feature = "ml")]
pub use predictive_remediation::{
    AnomalyDetector as PredictiveAnomalyDetector, DataPoint as PredictiveDataPoint,
    FailurePrediction, MetricTrend, MetricType as PredictiveMetricType,
    PredictiveRemediationEngine, TimeSeries, TrendAnalyzer,
    TrendDirection as PredictiveTrendDirection, TrendReport,
};
pub use protocols::{graphql::GraphQLChaos, grpc::GrpcChaos, websocket::WebSocketChaos};
pub use rate_limit::RateLimiter;
pub use recommendations::{
    Confidence, EngineConfig, Recommendation, RecommendationCategory, RecommendationEngine,
    RecommendationSeverity,
};
#[cfg(feature = "ml")]
pub use reinforcement_learning::{
    AdaptiveRiskAssessor, QLearningConfig, RLAgent, RemediationAction as RLRemediationAction,
    RiskAssessment, SystemState,
};
pub use resilience::{
    Bulkhead, BulkheadError, BulkheadGuard, BulkheadManager, BulkheadMetrics, BulkheadStats,
    CircuitBreaker, CircuitBreakerManager, CircuitBreakerMetrics, CircuitState, CircuitStats,
    DynamicThresholdAdjuster, FallbackHandler, HealthCheckIntegration, JsonFallbackHandler,
    RetryConfig as ResilienceRetryConfig, RetryPolicy,
};
pub use resilience_api::{
    create_resilience_router, BulkheadStateResponse, CircuitBreakerStateResponse,
    ResilienceApiState,
};
pub use scenario_orchestrator::{OrchestratedScenario, ScenarioOrchestrator, ScenarioStep};
pub use scenario_recorder::{ChaosEvent, ChaosEventType, RecordedScenario, ScenarioRecorder};
pub use scenario_replay::{ReplayOptions, ReplaySpeed, ScenarioReplayEngine};
pub use scenario_scheduler::{ScenarioScheduler, ScheduleType, ScheduledScenario};
pub use scenarios::{ChaosScenario, PredefinedScenarios, ScenarioEngine};
#[cfg(feature = "enterprise")]
pub use template_marketplace::{
    CompatibilityInfo, OrchestrationTemplate, TemplateCategory, TemplateMarketplace,
    TemplateReview, TemplateSearchFilters, TemplateSortBy, TemplateStats,
};
pub use traffic_shaping::TrafficShaper;

// Re-exports from modules extracted from mockforge-core (Phase 6a)
pub use core_chaos_utilities::{
    ChaosConfig as CoreChaosConfig, ChaosEngine, ChaosResult, ChaosStatistics,
};
pub use core_failure_injection::{
    create_failure_injector, FailureConfig, FailureInjector, TagFailureConfig,
};
pub use core_network_profiles::{NetworkProfile as CoreNetworkProfile, NetworkProfileCatalog};
pub use core_traffic_shaping::{
    BandwidthConfig, BurstLossConfig, TrafficShaper as CoreTrafficShaper,
    TrafficShapingConfig as CoreTrafficShapingConfig,
};

#[cfg(feature = "enterprise")]
pub use version_control::{
    Branch, Commit, Diff, DiffChange, DiffChangeType, DiffStats, VersionControlRepository,
};

use thiserror::Error;

/// Chaos engineering errors
#[derive(Error, Debug)]
pub enum ChaosError {
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Connection throttled")]
    ConnectionThrottled,

    #[error("Injected fault: {0}")]
    InjectedFault(String),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Scenario error: {0}")]
    Scenario(String),

    #[error("Circuit breaker open")]
    CircuitBreakerOpen,

    #[error("Bulkhead rejected: {0}")]
    BulkheadRejected(String),
}

pub type Result<T> = std::result::Result<T, ChaosError>;
