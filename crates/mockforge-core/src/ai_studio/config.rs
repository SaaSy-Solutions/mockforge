//! Configuration for AI Studio
//!
//! This module defines configuration structures for the AI Studio, including
//! deterministic mode settings, budget controls, and feature toggles.

use serde::{Deserialize, Serialize};

/// AI Studio configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AiStudioConfig {
    /// Deterministic mode configuration
    #[serde(default)]
    pub deterministic_mode: DeterministicModeConfig,

    /// Budget and cost controls
    #[serde(default)]
    pub budgets: BudgetConfig,

    /// Feature toggles
    #[serde(default)]
    pub features: FeatureConfig,
}

/// Deterministic mode configuration
///
/// When enabled, AI-generated artifacts are automatically frozen to
/// deterministic YAML/JSON files for version control and reproducible testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DeterministicModeConfig {
    /// Whether deterministic mode is enabled
    #[serde(default = "default_false")]
    pub enabled: bool,

    /// Automatically freeze AI-generated artifacts after creation
    #[serde(default = "default_true")]
    pub auto_freeze: bool,

    /// Format for frozen artifacts (yaml or json)
    #[serde(default = "default_freeze_format")]
    pub freeze_format: String,

    /// Directory to store frozen artifacts
    #[serde(default = "default_freeze_directory")]
    pub freeze_directory: String,
}

impl Default for DeterministicModeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_freeze: true,
            freeze_format: "yaml".to_string(),
            freeze_directory: ".mockforge/frozen".to_string(),
        }
    }
}

/// Budget configuration for AI usage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BudgetConfig {
    /// Maximum tokens per workspace
    #[serde(default = "default_max_tokens")]
    pub max_tokens_per_workspace: u64,

    /// Maximum AI calls per day
    #[serde(default = "default_max_calls")]
    pub max_ai_calls_per_day: u64,

    /// Rate limit per minute
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u64,

    /// Budget alerts threshold (percentage)
    #[serde(default = "default_alert_threshold")]
    pub alert_threshold: f64,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            max_tokens_per_workspace: 100_000,
            max_ai_calls_per_day: 1_000,
            rate_limit_per_minute: 10,
            alert_threshold: 0.8, // Alert at 80% usage
        }
    }
}

/// Feature configuration for enabling/disabling specific AI features
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FeatureConfig {
    /// Enable mock generation from natural language
    #[serde(default = "default_true")]
    pub mock_generation: bool,

    /// Enable AI contract diff analysis
    #[serde(default = "default_true")]
    pub contract_diff: bool,

    /// Enable persona generation
    #[serde(default = "default_true")]
    pub persona_generation: bool,

    /// Enable free-form generation (general chat)
    #[serde(default = "default_true")]
    pub free_form_generation: bool,

    /// Enable AI-guided debugging
    #[serde(default = "default_true")]
    pub debug_analysis: bool,
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            mock_generation: true,
            contract_diff: true,
            persona_generation: true,
            free_form_generation: true,
            debug_analysis: true,
        }
    }
}

// Helper functions for default values

fn default_false() -> bool {
    false
}

fn default_true() -> bool {
    true
}

fn default_freeze_format() -> String {
    "yaml".to_string()
}

fn default_freeze_directory() -> String {
    ".mockforge/frozen".to_string()
}

fn default_max_tokens() -> u64 {
    100_000
}

fn default_max_calls() -> u64 {
    1_000
}

fn default_rate_limit() -> u64 {
    10
}

fn default_alert_threshold() -> f64 {
    0.8
}
