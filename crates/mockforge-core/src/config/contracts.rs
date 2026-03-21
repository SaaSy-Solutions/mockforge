//! Contract, incident, and behavioral configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Incident management configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct IncidentConfig {
    /// Storage configuration
    pub storage: IncidentStorageConfig,
    /// External integrations configuration
    pub external_integrations: crate::incidents::integrations::ExternalIntegrationConfig,
    /// Webhook configurations
    pub webhooks: Vec<crate::incidents::integrations::WebhookConfig>,
}

/// Incident storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct IncidentStorageConfig {
    /// Use in-memory cache (default: true)
    pub use_cache: bool,
    /// Use database persistence (default: true)
    pub use_database: bool,
    /// Retention period for resolved incidents (days)
    pub retention_days: u32,
}

impl Default for IncidentStorageConfig {
    fn default() -> Self {
        Self {
            use_cache: true,
            use_database: true,
            retention_days: 90,
        }
    }
}

/// Consumer contracts configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ConsumerContractsConfig {
    /// Whether consumer contracts are enabled
    pub enabled: bool,
    /// Auto-register consumers from requests
    pub auto_register: bool,
    /// Track field usage
    pub track_usage: bool,
}

/// Contracts configuration for fitness rules and contract management
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ContractsConfig {
    /// Fitness rules for contract validation
    pub fitness_rules: Vec<FitnessRuleConfig>,
}

/// Behavioral Economics Engine configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct BehavioralEconomicsConfig {
    /// Whether the behavioral economics engine is enabled
    pub enabled: bool,
    /// List of behavior rules
    #[serde(default)]
    pub rules: Vec<crate::behavioral_economics::BehaviorRule>,
    /// Global sensitivity for behavioral changes (0.0 - 1.0)
    /// A higher sensitivity means mocks react more strongly to conditions.
    #[serde(default = "default_behavioral_sensitivity")]
    pub global_sensitivity: f64,
    /// Interval in milliseconds for re-evaluating time-based conditions
    #[serde(default = "default_evaluation_interval_ms")]
    pub evaluation_interval_ms: u64,
}

fn default_behavioral_sensitivity() -> f64 {
    0.5
}

fn default_evaluation_interval_ms() -> u64 {
    1000 // 1 second
}

/// Drift Learning configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct DriftLearningConfig {
    /// Enable or disable drift learning
    pub enabled: bool,
    /// Learning mode (behavioral, statistical, hybrid)
    #[serde(default)]
    pub mode: DriftLearningMode,
    /// How quickly mocks adapt to new patterns (0.0 - 1.0)
    #[serde(default = "default_learning_sensitivity")]
    pub sensitivity: f64,
    /// How quickly old patterns are forgotten (0.0 - 1.0)
    #[serde(default = "default_learning_decay")]
    pub decay: f64,
    /// Minimum number of samples required to learn a pattern
    #[serde(default = "default_min_samples")]
    pub min_samples: u64,
    /// Enable persona-specific behavior adaptation
    #[serde(default)]
    pub persona_adaptation: bool,
    /// Opt-in configuration for specific personas to learn
    #[serde(default)]
    pub persona_learning: HashMap<String, bool>, // persona_id -> enabled
    /// Opt-in configuration for specific endpoints to learn
    #[serde(default)]
    pub endpoint_learning: HashMap<String, bool>, // endpoint_pattern -> enabled
}

/// Drift learning mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum DriftLearningMode {
    /// Behavioral learning - adapts to behavior patterns
    #[default]
    Behavioral,
    /// Statistical learning - adapts to statistical patterns
    Statistical,
    /// Hybrid - combines behavioral and statistical
    Hybrid,
}

fn default_learning_sensitivity() -> f64 {
    0.2
}

fn default_learning_decay() -> f64 {
    0.05
}

fn default_min_samples() -> u64 {
    10
}

/// Configuration for a fitness rule (YAML config format)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FitnessRuleConfig {
    /// Human-readable name for the fitness rule
    pub name: String,
    /// Scope where this rule applies (endpoint pattern, service name, or "global")
    pub scope: String,
    /// Type of fitness rule
    #[serde(rename = "type")]
    pub rule_type: FitnessRuleType,
    /// Maximum percent increase for response size (for response_size_delta type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_percent_increase: Option<f64>,
    /// Maximum number of fields (for field_count type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fields: Option<u32>,
    /// Maximum schema depth (for schema_complexity type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
}

/// Type of fitness rule (YAML config format)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum FitnessRuleType {
    /// Response size must not increase by more than max_percent_increase
    ResponseSizeDelta,
    /// No new required fields allowed
    NoNewRequiredFields,
    /// Field count must not exceed max_fields
    FieldCount,
    /// Schema complexity (depth) must not exceed max_depth
    SchemaComplexity,
}

/// Behavioral cloning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct BehavioralCloningConfig {
    /// Whether behavioral cloning is enabled
    pub enabled: bool,
    /// Path to recorder database (defaults to ./recordings.db)
    pub database_path: Option<String>,
    /// Enable middleware to apply learned behavior
    pub enable_middleware: bool,
    /// Minimum frequency threshold for sequence learning (0.0 to 1.0)
    pub min_sequence_frequency: f64,
    /// Minimum requests per trace for sequence discovery
    pub min_requests_per_trace: Option<i32>,
    /// Flow recording configuration
    #[serde(default)]
    pub flow_recording: FlowRecordingConfig,
    /// Scenario replay configuration
    #[serde(default)]
    pub scenario_replay: ScenarioReplayConfig,
}

/// Flow recording configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct FlowRecordingConfig {
    /// Whether flow recording is enabled
    pub enabled: bool,
    /// How to group requests into flows (trace_id, session_id, ip_time_window)
    pub group_by: String,
    /// Time window in seconds for IP-based grouping
    pub time_window_seconds: u64,
}

impl Default for FlowRecordingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            group_by: "trace_id".to_string(),
            time_window_seconds: 300, // 5 minutes
        }
    }
}

/// Scenario replay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ScenarioReplayConfig {
    /// Whether scenario replay is enabled
    pub enabled: bool,
    /// Default replay mode (strict or flex)
    pub default_mode: String,
    /// List of scenario IDs to activate on startup
    pub active_scenarios: Vec<String>,
}

impl Default for ScenarioReplayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_mode: "strict".to_string(),
            active_scenarios: Vec::new(),
        }
    }
}

impl Default for BehavioralCloningConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            database_path: None,
            enable_middleware: false,
            min_sequence_frequency: 0.1, // 10% default
            min_requests_per_trace: None,
            flow_recording: FlowRecordingConfig::default(),
            scenario_replay: ScenarioReplayConfig::default(),
        }
    }
}
