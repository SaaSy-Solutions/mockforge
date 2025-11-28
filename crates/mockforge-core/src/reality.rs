//! Pillars: [Reality]
//!
//! Reality Slider - Unified control for mock environment realism
//!
//! This module provides a unified control mechanism that transitions mock environments
//! between "stubbed simplicity" (level 1) and "production chaos" (level 5) by
//! automatically coordinating chaos engineering, latency injection, and MockAI behaviors.

use crate::chaos_utilities::ChaosConfig;
use crate::intelligent_behavior::config::IntelligentBehaviorConfig;
use crate::latency::{LatencyDistribution, LatencyProfile};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Reality level for mock environments (1-5)
///
/// Each level represents a different degree of realism, from simple static mocks
/// to full production-like chaos behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum RealityLevel {
    /// Level 1: Static Stubs - Simple, instant responses with no chaos
    StaticStubs = 1,
    /// Level 2: Light Simulation - Minimal latency, basic intelligence
    LightSimulation = 2,
    /// Level 3: Moderate Realism - Some chaos, moderate latency, full intelligence
    #[default]
    ModerateRealism = 3,
    /// Level 4: High Realism - Increased chaos, realistic latency, session state
    HighRealism = 4,
    /// Level 5: Production Chaos - Maximum chaos, production-like latency, full features
    ProductionChaos = 5,
}

impl RealityLevel {
    /// Get the numeric value (1-5)
    pub fn value(&self) -> u8 {
        *self as u8
    }

    /// Get a human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            RealityLevel::StaticStubs => "Static Stubs",
            RealityLevel::LightSimulation => "Light Simulation",
            RealityLevel::ModerateRealism => "Moderate Realism",
            RealityLevel::HighRealism => "High Realism",
            RealityLevel::ProductionChaos => "Production Chaos",
        }
    }

    /// Get a short description
    pub fn description(&self) -> &'static str {
        match self {
            RealityLevel::StaticStubs => "Simple, instant responses with no chaos",
            RealityLevel::LightSimulation => "Minimal latency, basic intelligence",
            RealityLevel::ModerateRealism => "Some chaos, moderate latency, full intelligence",
            RealityLevel::HighRealism => "Increased chaos, realistic latency, session state",
            RealityLevel::ProductionChaos => {
                "Maximum chaos, production-like latency, full features"
            }
        }
    }

    /// Create from numeric value (1-5)
    pub fn from_value(value: u8) -> Option<Self> {
        match value {
            1 => Some(RealityLevel::StaticStubs),
            2 => Some(RealityLevel::LightSimulation),
            3 => Some(RealityLevel::ModerateRealism),
            4 => Some(RealityLevel::HighRealism),
            5 => Some(RealityLevel::ProductionChaos),
            _ => None,
        }
    }

    /// Get all available levels
    pub fn all() -> Vec<Self> {
        vec![
            RealityLevel::StaticStubs,
            RealityLevel::LightSimulation,
            RealityLevel::ModerateRealism,
            RealityLevel::HighRealism,
            RealityLevel::ProductionChaos,
        ]
    }
}

/// Reality configuration that maps a level to specific subsystem settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealityConfig {
    /// Current reality level
    pub level: RealityLevel,
    /// Chaos configuration for this level
    pub chaos: ChaosConfig,
    /// Latency profile for this level
    pub latency: LatencyProfile,
    /// MockAI configuration for this level
    pub mockai: IntelligentBehaviorConfig,
}

impl RealityConfig {
    /// Create configuration for a specific reality level
    pub fn for_level(level: RealityLevel) -> Self {
        match level {
            RealityLevel::StaticStubs => Self::level_1_static_stubs(),
            RealityLevel::LightSimulation => Self::level_2_light_simulation(),
            RealityLevel::ModerateRealism => Self::level_3_moderate_realism(),
            RealityLevel::HighRealism => Self::level_4_high_realism(),
            RealityLevel::ProductionChaos => Self::level_5_production_chaos(),
        }
    }

    /// Level 1: Static Stubs
    ///
    /// - Chaos: Disabled
    /// - Latency: 0ms (instant)
    /// - MockAI: Disabled (static responses only)
    fn level_1_static_stubs() -> Self {
        Self {
            level: RealityLevel::StaticStubs,
            chaos: ChaosConfig {
                enabled: false,
                error_rate: 0.0,
                delay_rate: 0.0,
                min_delay_ms: 0,
                max_delay_ms: 0,
                status_codes: vec![],
                inject_timeouts: false,
                timeout_ms: 0,
            },
            latency: LatencyProfile {
                base_ms: 0,
                jitter_ms: 0,
                distribution: LatencyDistribution::Fixed,
                std_dev_ms: None,
                pareto_shape: None,
                min_ms: 0,
                max_ms: Some(0),
                tag_overrides: Default::default(),
            },
            mockai: IntelligentBehaviorConfig {
                enabled: false,
                ..Default::default()
            },
        }
    }

    /// Level 2: Light Simulation
    ///
    /// - Chaos: Disabled
    /// - Latency: 10-50ms (minimal)
    /// - MockAI: Enabled (basic intelligence)
    fn level_2_light_simulation() -> Self {
        Self {
            level: RealityLevel::LightSimulation,
            chaos: ChaosConfig {
                enabled: false,
                error_rate: 0.0,
                delay_rate: 0.0,
                min_delay_ms: 0,
                max_delay_ms: 0,
                status_codes: vec![],
                inject_timeouts: false,
                timeout_ms: 0,
            },
            latency: LatencyProfile {
                base_ms: 30,
                jitter_ms: 20,
                distribution: LatencyDistribution::Fixed,
                std_dev_ms: None,
                pareto_shape: None,
                min_ms: 10,
                max_ms: Some(50),
                tag_overrides: Default::default(),
            },
            mockai: IntelligentBehaviorConfig {
                enabled: true,
                ..Default::default()
            },
        }
    }

    /// Level 3: Moderate Realism
    ///
    /// - Chaos: Enabled (5% error rate, 10% delay rate)
    /// - Latency: 50-200ms (moderate)
    /// - MockAI: Enabled (full intelligence)
    fn level_3_moderate_realism() -> Self {
        Self {
            level: RealityLevel::ModerateRealism,
            chaos: ChaosConfig {
                enabled: true,
                error_rate: 0.05,
                delay_rate: 0.10,
                min_delay_ms: 50,
                max_delay_ms: 200,
                status_codes: vec![500, 502, 503],
                inject_timeouts: false,
                timeout_ms: 0,
            },
            latency: LatencyProfile {
                base_ms: 125,
                jitter_ms: 75,
                distribution: LatencyDistribution::Normal,
                std_dev_ms: Some(30.0),
                pareto_shape: None,
                min_ms: 50,
                max_ms: Some(200),
                tag_overrides: Default::default(),
            },
            mockai: IntelligentBehaviorConfig {
                enabled: true,
                ..Default::default()
            },
        }
    }

    /// Level 4: High Realism
    ///
    /// - Chaos: Enabled (10% error rate, 20% delay rate)
    /// - Latency: 100-500ms (realistic)
    /// - MockAI: Enabled (full intelligence + session state)
    fn level_4_high_realism() -> Self {
        Self {
            level: RealityLevel::HighRealism,
            chaos: ChaosConfig {
                enabled: true,
                error_rate: 0.10,
                delay_rate: 0.20,
                min_delay_ms: 100,
                max_delay_ms: 500,
                status_codes: vec![500, 502, 503, 504],
                inject_timeouts: false,
                timeout_ms: 0,
            },
            latency: LatencyProfile {
                base_ms: 300,
                jitter_ms: 200,
                distribution: LatencyDistribution::Normal,
                std_dev_ms: Some(80.0),
                pareto_shape: None,
                min_ms: 100,
                max_ms: Some(500),
                tag_overrides: Default::default(),
            },
            mockai: IntelligentBehaviorConfig {
                enabled: true,
                performance: crate::intelligent_behavior::config::PerformanceConfig {
                    max_history_length: 100,
                    session_timeout_seconds: 3600,
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    /// Level 5: Production Chaos
    ///
    /// - Chaos: Enabled (15% error rate, 30% delay rate, timeouts enabled)
    /// - Latency: 200-2000ms (production-like, heavy-tailed)
    /// - MockAI: Enabled (full intelligence + session state + mutations)
    fn level_5_production_chaos() -> Self {
        Self {
            level: RealityLevel::ProductionChaos,
            chaos: ChaosConfig {
                enabled: true,
                error_rate: 0.15,
                delay_rate: 0.30,
                min_delay_ms: 200,
                max_delay_ms: 2000,
                status_codes: vec![500, 502, 503, 504, 408],
                inject_timeouts: true,
                timeout_ms: 5000,
            },
            latency: LatencyProfile {
                base_ms: 1100,
                jitter_ms: 900,
                distribution: LatencyDistribution::Pareto,
                std_dev_ms: None,
                pareto_shape: Some(2.0),
                min_ms: 200,
                max_ms: Some(2000),
                tag_overrides: Default::default(),
            },
            mockai: IntelligentBehaviorConfig {
                enabled: true,
                performance: crate::intelligent_behavior::config::PerformanceConfig {
                    max_history_length: 200,
                    session_timeout_seconds: 7200,
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }
}

impl Default for RealityConfig {
    fn default() -> Self {
        Self::for_level(RealityLevel::default())
    }
}

/// Reality preset for export/import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealityPreset {
    /// Preset name
    pub name: String,
    /// Preset description
    pub description: Option<String>,
    /// Reality configuration
    pub config: RealityConfig,
    /// Metadata
    pub metadata: Option<PresetMetadata>,
}

/// Preset metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetMetadata {
    /// Created timestamp
    pub created_at: Option<String>,
    /// Author
    pub author: Option<String>,
    /// Tags
    pub tags: Vec<String>,
    /// Version
    pub version: Option<String>,
}

impl Default for PresetMetadata {
    fn default() -> Self {
        Self {
            created_at: None,
            author: None,
            tags: vec![],
            version: Some("1.0".to_string()),
        }
    }
}

/// Reality engine that coordinates chaos, latency, and MockAI subsystems
///
/// This engine applies the appropriate settings to each subsystem based on
/// the current reality level. It acts as a coordinator and doesn't own the
/// subsystems directly, but provides configuration that can be applied to them.
#[derive(Debug, Clone)]
pub struct RealityEngine {
    /// Current reality configuration
    config: Arc<RwLock<RealityConfig>>,
}

impl RealityEngine {
    /// Create a new reality engine with default level
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(RealityConfig::default())),
        }
    }

    /// Create a new reality engine with a specific level
    pub fn with_level(level: RealityLevel) -> Self {
        Self {
            config: Arc::new(RwLock::new(RealityConfig::for_level(level))),
        }
    }

    /// Get the current reality level
    pub async fn get_level(&self) -> RealityLevel {
        self.config.read().await.level
    }

    /// Set the reality level and update configuration
    pub async fn set_level(&self, level: RealityLevel) {
        let mut config = self.config.write().await;
        *config = RealityConfig::for_level(level);
    }

    /// Get the current reality configuration
    pub async fn get_config(&self) -> RealityConfig {
        self.config.read().await.clone()
    }

    /// Get chaos configuration for current level
    pub async fn get_chaos_config(&self) -> ChaosConfig {
        self.config.read().await.chaos.clone()
    }

    /// Get latency profile for current level
    pub async fn get_latency_profile(&self) -> LatencyProfile {
        self.config.read().await.latency.clone()
    }

    /// Get MockAI configuration for current level
    pub async fn get_mockai_config(&self) -> IntelligentBehaviorConfig {
        self.config.read().await.mockai.clone()
    }

    /// Create a preset from current configuration
    pub async fn create_preset(&self, name: String, description: Option<String>) -> RealityPreset {
        let config = self.config.read().await.clone();
        RealityPreset {
            name,
            description,
            config,
            metadata: Some(PresetMetadata {
                created_at: Some(chrono::Utc::now().to_rfc3339()),
                author: None,
                tags: vec![],
                version: Some("1.0".to_string()),
            }),
        }
    }

    /// Apply a preset configuration
    pub async fn apply_preset(&self, preset: RealityPreset) {
        let mut config = self.config.write().await;
        *config = preset.config;
    }

    /// Apply reality configuration to a ServerConfig
    ///
    /// This method updates the provided ServerConfig with chaos, latency, and MockAI
    /// settings from the current reality level. This should be called when initializing
    /// the server or when the reality level changes.
    pub async fn apply_to_config(&self, config: &mut crate::config::ServerConfig) {
        let reality_config = self.get_config().await;

        // Apply chaos configuration
        if config.reality.enabled {
            // Update chaos config if it exists in observability
            if let Some(ref mut chaos_eng) = config.observability.chaos {
                chaos_eng.enabled = reality_config.chaos.enabled;
                if let Some(ref mut fault) = chaos_eng.fault_injection {
                    fault.enabled = reality_config.chaos.enabled;
                    fault.http_error_probability = reality_config.chaos.error_rate;
                    fault.timeout_errors = reality_config.chaos.inject_timeouts;
                    fault.timeout_ms = reality_config.chaos.timeout_ms;
                }
                if let Some(ref mut latency) = chaos_eng.latency {
                    latency.enabled = reality_config.latency.base_ms > 0;
                    latency.fixed_delay_ms = Some(reality_config.latency.base_ms);
                    latency.jitter_percent = if reality_config.latency.jitter_ms > 0 {
                        (reality_config.latency.jitter_ms as f64
                            / reality_config.latency.base_ms as f64)
                            .min(1.0)
                    } else {
                        0.0
                    };
                }
            }
        }

        // Apply latency configuration
        if config.reality.enabled {
            config.core.default_latency = reality_config.latency.clone();
            config.core.latency_enabled = reality_config.latency.base_ms > 0;
        }

        // Apply MockAI configuration
        if config.reality.enabled {
            config.mockai.enabled = reality_config.mockai.enabled;
            config.mockai.intelligent_behavior = reality_config.mockai.clone();
        }
    }
}

impl Default for RealityEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reality_level_values() {
        assert_eq!(RealityLevel::StaticStubs.value(), 1);
        assert_eq!(RealityLevel::LightSimulation.value(), 2);
        assert_eq!(RealityLevel::ModerateRealism.value(), 3);
        assert_eq!(RealityLevel::HighRealism.value(), 4);
        assert_eq!(RealityLevel::ProductionChaos.value(), 5);
    }

    #[test]
    fn test_reality_level_from_value() {
        assert_eq!(RealityLevel::from_value(1), Some(RealityLevel::StaticStubs));
        assert_eq!(RealityLevel::from_value(3), Some(RealityLevel::ModerateRealism));
        assert_eq!(RealityLevel::from_value(5), Some(RealityLevel::ProductionChaos));
        assert_eq!(RealityLevel::from_value(0), None);
        assert_eq!(RealityLevel::from_value(6), None);
    }

    #[test]
    fn test_level_1_config() {
        let config = RealityConfig::for_level(RealityLevel::StaticStubs);
        assert!(!config.chaos.enabled);
        assert_eq!(config.latency.base_ms, 0);
        assert!(!config.mockai.enabled);
    }

    #[test]
    fn test_level_5_config() {
        let config = RealityConfig::for_level(RealityLevel::ProductionChaos);
        assert!(config.chaos.enabled);
        assert!(config.chaos.inject_timeouts);
        assert_eq!(config.chaos.error_rate, 0.15);
        assert!(config.latency.base_ms >= 200);
        assert!(config.mockai.enabled);
    }

    #[tokio::test]
    async fn test_reality_engine() {
        let engine = RealityEngine::with_level(RealityLevel::StaticStubs);
        assert_eq!(engine.get_level().await, RealityLevel::StaticStubs);

        engine.set_level(RealityLevel::ProductionChaos).await;
        assert_eq!(engine.get_level().await, RealityLevel::ProductionChaos);

        let chaos_config = engine.get_chaos_config().await;
        assert!(chaos_config.enabled);
    }

    #[tokio::test]
    async fn test_preset_creation() {
        let engine = RealityEngine::with_level(RealityLevel::ModerateRealism);
        let preset = engine
            .create_preset("test-preset".to_string(), Some("Test description".to_string()))
            .await;

        assert_eq!(preset.name, "test-preset");
        assert_eq!(preset.config.level, RealityLevel::ModerateRealism);
        assert!(preset.metadata.is_some());
    }
}
