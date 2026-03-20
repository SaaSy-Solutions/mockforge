//! Chaos engineering orchestration
//!
//! Provides scenario management, scheduling, distributed coordination,
//! and experiment management for MockForge chaos engineering.

pub mod config;

pub mod ab_testing;
pub mod advanced_orchestration;
pub mod alerts;
pub mod analytics;
pub mod chaos_mesh;
pub mod collaboration;
pub mod dashboard;
pub mod distributed_coordinator;
pub mod failure_designer;
pub mod gitops;
pub mod incident_replay;
pub mod integrations;
pub mod multi_cluster;
pub mod scenario_orchestrator;
pub mod scenario_recorder;
pub mod scenario_replay;
pub mod scenario_scheduler;
pub mod scenarios;
pub mod template_marketplace;
pub mod trace_collector;
pub mod version_control;

pub use ab_testing::{
    ABTest, ABTestConfig, ABTestStats, ABTestStatus, ABTestingEngine, MetricComparison, MetricType,
    SingleMetricComparison, SuccessCriteria, TestConclusion, TestVariant, VariantMetrics,
    VariantResults,
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
pub use chaos_mesh::{
    ChaosMeshClient, ChaosMeshExperiment, ExperimentSpec, ExperimentStatus, ExperimentType,
    NetworkChaosAction, NetworkDelay, NetworkLoss, PodChaosAction, PodSelector, StressConfig,
};
pub use collaboration::{
    ChangeType, CollaborationChange, CollaborationManager, CollaborationMessage,
    CollaborationSession, CollaborationUser, CursorPosition,
};
pub use dashboard::{DashboardManager, DashboardQuery, DashboardStats, DashboardUpdate};
pub use distributed_coordinator::{
    CoordinationMode, DistributedCoordinator, DistributedTask, ExecutionMetrics, LeaderState, Node,
    NodeExecutionState, NodeStatus, TaskStatus,
};
pub use failure_designer::{
    ConditionOperator, ConditionType, FailureCondition, FailureDesignRule, FailureDesigner,
    FailureTarget, FailureType,
};
pub use gitops::{GitOpsAuth, GitOpsConfig, GitOpsManager, SyncState, SyncStatus};
pub use incident_replay::{
    IncidentEvent, IncidentEventType, IncidentFormatAdapter, IncidentReplayGenerator,
    IncidentTimeline,
};
pub use integrations::{
    GrafanaConfig, GrafanaIntegration, IntegrationConfig, IntegrationManager, JiraConfig,
    JiraIntegration, Notification, NotificationResults, NotificationSeverity, PagerDutyConfig,
    PagerDutyIntegration, SlackConfig, SlackNotifier, TeamsConfig, TeamsNotifier,
};
pub use multi_cluster::{
    ClusterTarget, ExecutionStatus, MultiClusterOrchestration, MultiClusterOrchestrator,
    MultiClusterStatus, SyncMode,
};
pub use scenario_orchestrator::{OrchestratedScenario, ScenarioOrchestrator, ScenarioStep};
pub use scenario_recorder::{ChaosEvent, ChaosEventType, RecordedScenario, ScenarioRecorder};
pub use scenario_replay::{ReplayOptions, ReplaySpeed, ScenarioReplayEngine};
pub use scenario_scheduler::{ScenarioScheduler, ScheduleType, ScheduledScenario};
pub use scenarios::{ChaosScenario, PredefinedScenarios, ScenarioEngine};
pub use template_marketplace::{
    CompatibilityInfo, OrchestrationTemplate, TemplateCategory, TemplateMarketplace,
    TemplateReview, TemplateSearchFilters, TemplateSortBy, TemplateStats,
};
pub use version_control::{
    Branch, Commit, Diff, DiffChange, DiffChangeType, DiffStats, VersionControlRepository,
};

// Re-export config types at the crate root for convenience
pub use config::{
    BulkheadConfig, ChaosConfig, CircuitBreakerConfig, CorruptionType, ErrorPattern,
    FaultInjectionConfig, LatencyConfig, NetworkProfile, RateLimitConfig, TrafficShapingConfig,
};
