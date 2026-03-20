//! ML-powered chaos engineering
//!
//! Provides anomaly detection, reinforcement learning, predictive remediation,
//! multi-armed bandit, and intelligent recommendations for chaos experiments.

pub mod advanced_analytics;
pub mod auto_remediation;
pub mod ml_anomaly_detector;
pub mod ml_assertion_generator;
pub mod ml_parameter_optimizer;
pub mod multi_armed_bandit;
pub mod predictive_remediation;
pub mod recommendations;
pub mod reinforcement_learning;

pub use advanced_analytics::{
    AdvancedAnalyticsEngine, Anomaly, AnomalyType as AnalyticsAnomalyType, CorrelationAnalysis,
    DataPoint, HealthFactor, HealthScore, PredictiveInsight, TrendAnalysis, TrendDirection,
};
pub use auto_remediation::{
    ApprovalRequest, EffectivenessMetrics, RemediationAction, RemediationConfig, RemediationEngine,
    RemediationResult, RemediationStats, RemediationStatus, RiskAssessment as AutoRiskAssessment,
    RiskLevel, RollbackData, SafetyCheck, SystemMetrics,
};
pub use ml_anomaly_detector::{
    Anomaly as MLAnomaly, AnomalyDetector, AnomalyDetectorConfig, AnomalySeverity,
    AnomalyType as MLAnomalyType, MetricBaseline, TimeSeriesPoint,
};
pub use ml_assertion_generator::{
    AssertionGenerator, AssertionGeneratorConfig, AssertionOperator, AssertionType,
    ExecutionDataPoint, GeneratedAssertion, MetricStats,
};
pub use ml_parameter_optimizer::{
    ExpectedImpact, OptimizationObjective, OptimizationRecommendation, OptimizerConfig,
    OrchestrationRun, ParameterBounds, ParameterOptimizer, RunMetrics,
};
pub use multi_armed_bandit::{
    Arm, ArmReport, BanditReport, BanditStrategy, MultiArmedBandit, ThompsonSampling,
    TrafficAllocator, UCB1,
};
pub use predictive_remediation::{
    AnomalyDetector as PredictiveAnomalyDetector, DataPoint as PredictiveDataPoint,
    FailurePrediction, MetricTrend, MetricType as PredictiveMetricType,
    PredictiveRemediationEngine, TimeSeries, TrendAnalyzer,
    TrendDirection as PredictiveTrendDirection, TrendReport,
};
pub use recommendations::{
    Confidence, EngineConfig, Recommendation, RecommendationCategory, RecommendationEngine,
    RecommendationSeverity,
};
pub use reinforcement_learning::{
    AdaptiveRiskAssessor, QLearningConfig, RLAgent, RemediationAction as RLRemediationAction,
    RiskAssessment, SystemState,
};
