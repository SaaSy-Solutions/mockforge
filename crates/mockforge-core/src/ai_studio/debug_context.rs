//! Debug context types for AI-guided debugging
//!
//! This module defines the context structures that collect information from
//! various subsystems (Reality, Contracts, Scenarios, Personas, Chaos) to
//! provide comprehensive debugging context.

use crate::reality::{RealityConfig, RealityLevel};
use crate::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Unified debug context combining all subsystem contexts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugContext {
    /// Reality subsystem context
    pub reality: RealityContext,
    /// Contract subsystem context
    pub contract: ContractContext,
    /// Scenario subsystem context
    pub scenario: ScenarioContext,
    /// Persona subsystem context
    pub persona: PersonaContext,
    /// Chaos subsystem context
    pub chaos: ChaosContext,
    /// Timestamp when context was collected
    pub collected_at: DateTime<Utc>,
}

impl DebugContext {
    /// Create a new empty debug context
    pub fn new() -> Self {
        Self {
            reality: RealityContext::default(),
            contract: ContractContext::default(),
            scenario: ScenarioContext::default(),
            persona: PersonaContext::default(),
            chaos: ChaosContext::default(),
            collected_at: Utc::now(),
        }
    }
}

impl Default for DebugContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Reality subsystem context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealityContext {
    /// Current reality level (1-5)
    pub level: Option<RealityLevel>,
    /// Reality level name
    pub level_name: Option<String>,
    /// Whether chaos is enabled
    pub chaos_enabled: bool,
    /// Chaos error rate (0.0-1.0)
    pub chaos_error_rate: f64,
    /// Chaos delay rate (0.0-1.0)
    pub chaos_delay_rate: f64,
    /// Base latency in milliseconds
    pub latency_base_ms: u64,
    /// Latency jitter in milliseconds
    pub latency_jitter_ms: u64,
    /// Whether MockAI is enabled
    pub mockai_enabled: bool,
    /// Full reality configuration (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_config: Option<RealityConfig>,
}

impl Default for RealityContext {
    fn default() -> Self {
        Self {
            level: None,
            level_name: None,
            chaos_enabled: false,
            chaos_error_rate: 0.0,
            chaos_delay_rate: 0.0,
            latency_base_ms: 0,
            latency_jitter_ms: 0,
            mockai_enabled: false,
            full_config: None,
        }
    }
}

impl RealityContext {
    /// Create from RealityConfig
    pub fn from_config(config: &RealityConfig) -> Self {
        Self {
            level: Some(config.level),
            level_name: Some(config.level.name().to_string()),
            chaos_enabled: config.chaos.enabled,
            chaos_error_rate: config.chaos.error_rate,
            chaos_delay_rate: config.chaos.delay_rate,
            latency_base_ms: config.latency.base_ms,
            latency_jitter_ms: config.latency.jitter_ms,
            mockai_enabled: config.mockai.enabled,
            full_config: Some(config.clone()),
        }
    }
}

/// Contract subsystem context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractContext {
    /// Whether contract validation is enabled
    pub validation_enabled: bool,
    /// Contract validation results (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_result: Option<ContractValidationResult>,
    /// Contract enforcement mode (strict, lenient, disabled)
    pub enforcement_mode: String,
    /// Recent contract drift history
    pub drift_history: Vec<DriftHistoryEntry>,
    /// Active contract paths/specs
    pub active_contracts: Vec<String>,
    /// Contract validation errors (if any)
    pub validation_errors: Vec<String>,
}

impl Default for ContractContext {
    fn default() -> Self {
        Self {
            validation_enabled: false,
            validation_result: None,
            enforcement_mode: "disabled".to_string(),
            drift_history: Vec::new(),
            active_contracts: Vec::new(),
            validation_errors: Vec::new(),
        }
    }
}

/// Contract validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractValidationResult {
    /// Whether validation passed
    pub passed: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Contract details
    pub contract_details: Value,
}

/// Contract drift history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftHistoryEntry {
    /// Timestamp of drift detection
    pub detected_at: DateTime<Utc>,
    /// Endpoint/method where drift was detected
    pub endpoint: String,
    /// Type of drift (breaking, non-breaking, etc.)
    pub drift_type: String,
    /// Description of the drift
    pub description: String,
    /// Details (JSON)
    pub details: Value,
}

/// Scenario subsystem context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioContext {
    /// Active scenario name/ID (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_scenario: Option<String>,
    /// Current state machine state (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_state: Option<String>,
    /// Available state transitions
    pub available_transitions: Vec<String>,
    /// Scenario configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario_config: Option<Value>,
}

impl Default for ScenarioContext {
    fn default() -> Self {
        Self {
            active_scenario: None,
            current_state: None,
            available_transitions: Vec::new(),
            scenario_config: None,
        }
    }
}

/// Persona subsystem context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaContext {
    /// Active persona ID (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_persona_id: Option<String>,
    /// Active persona name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_persona_name: Option<String>,
    /// Persona traits
    pub traits: HashMap<String, String>,
    /// Persona domain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Persona backstory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backstory: Option<String>,
    /// Persona relationships
    pub relationships: HashMap<String, Vec<String>>,
    /// Persona lifecycle state (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifecycle_state: Option<String>,
}

impl Default for PersonaContext {
    fn default() -> Self {
        Self {
            active_persona_id: None,
            active_persona_name: None,
            traits: HashMap::new(),
            domain: None,
            backstory: None,
            relationships: HashMap::new(),
            lifecycle_state: None,
        }
    }
}

/// Chaos subsystem context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosContext {
    /// Whether chaos is enabled
    pub enabled: bool,
    /// Active chaos rules
    pub active_rules: Vec<ChaosRule>,
    /// Failure injection configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_injection: Option<FailureInjectionConfig>,
    /// Chaos tags/patterns
    pub tags: Vec<String>,
}

impl Default for ChaosContext {
    fn default() -> Self {
        Self {
            enabled: false,
            active_rules: Vec::new(),
            failure_injection: None,
            tags: Vec::new(),
        }
    }
}

/// Chaos rule information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosRule {
    /// Rule name/identifier
    pub name: String,
    /// Rule type (error, delay, timeout, etc.)
    pub rule_type: String,
    /// Whether rule is enabled
    pub enabled: bool,
    /// Rule configuration
    pub config: Value,
}

/// Failure injection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureInjectionConfig {
    /// Whether failure injection is enabled
    pub enabled: bool,
    /// HTTP error probability (0.0-1.0)
    pub http_error_probability: f64,
    /// Whether timeout errors are injected
    pub timeout_errors: bool,
    /// Timeout duration in milliseconds
    pub timeout_ms: u64,
    /// Tag-based failure configs
    pub tag_configs: HashMap<String, Value>,
}

