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
pub mod chaos_mesh;
pub mod collaboration;
pub mod config;
pub mod dashboard;
pub mod distributed_coordinator;
pub mod fault;
pub mod gitops;
pub mod integrations;
pub mod latency;
pub mod metrics;
pub mod multi_armed_bandit;
pub mod multi_cluster;
pub mod multi_tenancy;
pub mod middleware;
pub mod ml_anomaly_detector;
pub mod ml_assertion_generator;
pub mod ml_parameter_optimizer;
pub mod observability_api;
pub mod plugins;
pub mod predictive_remediation;
pub mod protocols;
pub mod rate_limit;
pub mod recommendations;
pub mod reinforcement_learning;
pub mod resilience;
pub mod resilience_api;
pub mod scenario_orchestrator;
pub mod scenario_recorder;
pub mod scenario_replay;
pub mod scenario_scheduler;
pub mod scenarios;
 pub mod template_marketplace;
 pub mod trace_collector;
 pub mod traffic_shaping;
 pub mod version_control;

pub use ab_testing::{
    ABTestingEngine, ABTestConfig, ABTest, ABTestStatus, TestVariant, SuccessCriteria,
    MetricType, VariantResults, VariantMetrics, TestConclusion, MetricComparison,
    SingleMetricComparison, ABTestStats,
};
pub use advanced_analytics::{
    AdvancedAnalyticsEngine, Anomaly, AnomalyType as AnalyticsAnomalyType, PredictiveInsight,
    TrendAnalysis, TrendDirection, CorrelationAnalysis, HealthScore, HealthFactor, DataPoint,
};
pub use advanced_orchestration::{
    AdvancedOrchestratedScenario, AdvancedScenarioStep, Assertion, Condition, ConditionalStep,
    ExecutionContext, ExecutionReport, Hook, HookAction, HookType, OrchestrationLibrary,
    RetryConfig, StepResult,
};
pub use alerts::{Alert, AlertHandler, AlertManager, AlertRule, AlertRuleType, AlertSeverity, AlertType};
pub use analytics::{ChaosAnalytics, ChaosImpact, MetricsBucket, TimeBucket};
pub use distributed_coordinator::{
    DistributedCoordinator, DistributedTask, Node, NodeStatus, LeaderState, CoordinationMode,
    TaskStatus, NodeExecutionState, ExecutionMetrics,
};
pub use api::create_chaos_api_router;
pub use auto_remediation::{
    RemediationEngine, RemediationConfig, RemediationAction, RemediationStatus,
    RemediationResult, EffectivenessMetrics, SystemMetrics, ApprovalRequest,
    RiskAssessment as AutoRiskAssessment, RiskLevel, SafetyCheck, RemediationStats, RollbackData,
};
pub use chaos_mesh::{
    ChaosMeshClient, ChaosMeshExperiment, ExperimentType, ExperimentSpec, ExperimentStatus,
    PodSelector, PodChaosAction, NetworkChaosAction, NetworkDelay, NetworkLoss, StressConfig,
};
pub use collaboration::{
    CollaborationManager, CollaborationSession, CollaborationUser, CollaborationChange,
    CollaborationMessage, ChangeType, CursorPosition,
};
pub use config::{
    BulkheadConfig, ChaosConfig, CircuitBreakerConfig, FaultInjectionConfig, LatencyConfig,
    RateLimitConfig, TrafficShapingConfig,
};
pub use dashboard::{DashboardManager, DashboardQuery, DashboardStats, DashboardUpdate};
pub use fault::{FaultInjector, FaultType};
pub use gitops::{GitOpsManager, GitOpsConfig, GitOpsAuth, SyncStatus, SyncState};
pub use latency::LatencyInjector;
pub use multi_cluster::{
    MultiClusterOrchestrator, MultiClusterOrchestration, ClusterTarget,
    SyncMode, MultiClusterStatus, ExecutionStatus,
};
pub use metrics::{ChaosMetrics, CHAOS_METRICS, registry as metrics_registry};
pub use middleware::{chaos_middleware, ChaosMiddleware};
pub use multi_tenancy::{
    TenantManager, Tenant, TenantPlan, ResourceQuota, ResourceUsage, TenantPermissions,
    MultiTenancyError,
};
pub use ml_anomaly_detector::{
    AnomalyDetector, AnomalyDetectorConfig, Anomaly as MLAnomaly, AnomalySeverity, AnomalyType as MLAnomalyType,
    MetricBaseline, TimeSeriesPoint,
};
pub use ml_assertion_generator::{
    AssertionGenerator, AssertionGeneratorConfig, GeneratedAssertion, AssertionType,
    AssertionOperator, ExecutionDataPoint, MetricStats,
};
pub use ml_parameter_optimizer::{
    ParameterOptimizer, OptimizerConfig, OptimizationRecommendation, OptimizationObjective,
    OrchestrationRun, RunMetrics, ExpectedImpact, ParameterBounds,
};
pub use observability_api::{create_observability_router, ObservabilityState};
pub use plugins::{
    PluginRegistry, ChaosPlugin, PluginHook, PluginMetadata, PluginConfig, PluginContext,
    PluginResult, PluginCapability, CustomFaultPlugin, MetricsPlugin, PluginError,
};
pub use protocols::{
    grpc::GrpcChaos,
    graphql::GraphQLChaos,
    websocket::WebSocketChaos,
};
pub use rate_limit::RateLimiter;
pub use recommendations::{
    Confidence, Recommendation, RecommendationCategory, RecommendationEngine,
    RecommendationSeverity, EngineConfig,
};
pub use resilience::{
    Bulkhead, BulkheadError, BulkheadGuard, BulkheadStats, BulkheadManager,
    CircuitBreaker, CircuitState, CircuitStats, CircuitBreakerManager,
    RetryPolicy, RetryConfig as ResilienceRetryConfig,
    FallbackHandler, JsonFallbackHandler,
    CircuitBreakerMetrics, BulkheadMetrics,
    DynamicThresholdAdjuster, HealthCheckIntegration,
};
pub use resilience_api::{
    create_resilience_router, ResilienceApiState,
    CircuitBreakerStateResponse, BulkheadStateResponse,
};
pub use scenario_orchestrator::{OrchestratedScenario, ScenarioOrchestrator, ScenarioStep};
pub use scenario_recorder::{ChaosEvent, ChaosEventType, RecordedScenario, ScenarioRecorder};
pub use scenario_replay::{ReplayOptions, ReplaySpeed, ScenarioReplayEngine};
pub use scenario_scheduler::{ScheduleType, ScheduledScenario, ScenarioScheduler};
pub use scenarios::{ChaosScenario, PredefinedScenarios, ScenarioEngine};
pub use template_marketplace::{
    TemplateMarketplace, OrchestrationTemplate, TemplateCategory, TemplateStats,
    TemplateReview, TemplateSearchFilters, TemplateSortBy, CompatibilityInfo,
};
pub use traffic_shaping::TrafficShaper;
pub use version_control::{
    VersionControlRepository, Commit, Branch, Diff, DiffChange, DiffChangeType, DiffStats,
};
pub use reinforcement_learning::{
    RLAgent, QLearningConfig, SystemState, RemediationAction as RLRemediationAction,
    AdaptiveRiskAssessor, RiskAssessment,
};
pub use multi_armed_bandit::{
    MultiArmedBandit, Arm, BanditStrategy, ThompsonSampling, UCB1, BanditReport, ArmReport,
    TrafficAllocator,
};
pub use integrations::{
    IntegrationConfig, IntegrationManager, SlackConfig, TeamsConfig, JiraConfig,
    PagerDutyConfig, GrafanaConfig, Notification, NotificationSeverity, NotificationResults,
    SlackNotifier, TeamsNotifier, JiraIntegration, PagerDutyIntegration, GrafanaIntegration,
};
pub use predictive_remediation::{
    PredictiveRemediationEngine, MetricType as PredictiveMetricType, TimeSeries, DataPoint as PredictiveDataPoint,
    AnomalyDetector as PredictiveAnomalyDetector, FailurePrediction, TrendAnalyzer, TrendReport, MetricTrend, TrendDirection as PredictiveTrendDirection,
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
