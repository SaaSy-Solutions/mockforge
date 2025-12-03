//! Reality Continuum Engine
//!
//! Manages blend ratios for gradually transitioning from mock to real data sources.
//! Supports time-based progression, manual configuration, and per-route/group/global settings.

use super::blender::ResponseBlender;
use super::config::{ContinuumConfig, ContinuumRule, TransitionMode};
use super::schedule::TimeSchedule;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Reality Continuum Engine
///
/// Manages blend ratios for routes, groups, and global settings.
/// Calculates current blend ratios based on transition mode and time.
#[derive(Debug, Clone)]
pub struct RealityContinuumEngine {
    /// Configuration
    config: Arc<RwLock<ContinuumConfig>>,
    /// Response blender
    blender: ResponseBlender,
    /// Optional virtual clock for time-based progression
    virtual_clock: Option<Arc<crate::time_travel::VirtualClock>>,
    /// Manual blend ratio overrides (path -> ratio)
    manual_overrides: Arc<RwLock<HashMap<String, f64>>>,
}

impl RealityContinuumEngine {
    /// Create a new continuum engine with the given configuration
    pub fn new(config: ContinuumConfig) -> Self {
        let blender = ResponseBlender::new(config.merge_strategy);
        Self {
            config: Arc::new(RwLock::new(config)),
            blender,
            virtual_clock: None,
            manual_overrides: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new continuum engine with virtual clock integration
    pub fn with_virtual_clock(
        config: ContinuumConfig,
        virtual_clock: Arc<crate::time_travel::VirtualClock>,
    ) -> Self {
        let blender = ResponseBlender::new(config.merge_strategy);
        Self {
            config: Arc::new(RwLock::new(config)),
            blender,
            virtual_clock: Some(virtual_clock),
            manual_overrides: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set the virtual clock (can be called after construction)
    pub fn set_virtual_clock(&mut self, virtual_clock: Arc<crate::time_travel::VirtualClock>) {
        self.virtual_clock = Some(virtual_clock);
    }

    /// Get the current blend ratio for a path
    ///
    /// Checks in order:
    /// 1. Manual overrides
    /// 2. Route-specific rules
    /// 3. Group-level overrides
    /// 4. Time-based schedule (if enabled)
    /// 5. Default ratio
    pub async fn get_blend_ratio(&self, path: &str) -> f64 {
        // Check manual overrides first
        {
            let overrides = self.manual_overrides.read().await;
            if let Some(&ratio) = overrides.get(path) {
                debug!("Using manual override for {}: {}", path, ratio);
                return ratio;
            }
        }

        let config = self.config.read().await;

        // Check route-specific rules, but prioritize group-level overrides
        for rule in &config.routes {
            if rule.matches_path(path) {
                // If this route has a group and the group has a ratio, use the group ratio
                if let Some(ref group) = rule.group {
                    if let Some(&group_ratio) = config.groups.get(group) {
                        debug!(
                            "Using group override for {} (group {}): {}",
                            path, group, group_ratio
                        );
                        return group_ratio;
                    }
                }
                // Otherwise, use the route ratio
                debug!("Using route rule for {}: {}", path, rule.ratio);
                return rule.ratio;
            }
        }

        // Check time-based schedule if enabled
        if config.transition_mode == TransitionMode::TimeBased
            || config.transition_mode == TransitionMode::Scheduled
        {
            if let Some(ref schedule) = config.time_schedule {
                let current_time = self.get_current_time().await;
                let ratio = schedule.calculate_ratio(current_time);
                debug!("Using time-based ratio for {}: {} (time: {})", path, ratio, current_time);
                return ratio;
            }
        }

        // Return default ratio
        debug!("Using default ratio for {}: {}", path, config.default_ratio);
        config.default_ratio
    }

    /// Set a manual blend ratio override for a path
    pub async fn set_blend_ratio(&self, path: &str, ratio: f64) {
        let ratio = ratio.clamp(0.0, 1.0);
        let mut overrides = self.manual_overrides.write().await;
        overrides.insert(path.to_string(), ratio);
        info!("Set manual blend ratio for {}: {}", path, ratio);
    }

    /// Remove a manual blend ratio override
    pub async fn remove_blend_ratio(&self, path: &str) {
        let mut overrides = self.manual_overrides.write().await;
        if overrides.remove(path).is_some() {
            info!("Removed manual blend ratio override for {}", path);
        }
    }

    /// Set blend ratio for a group
    pub async fn set_group_ratio(&self, group: &str, ratio: f64) {
        let ratio = ratio.clamp(0.0, 1.0);
        let mut config = self.config.write().await;
        config.groups.insert(group.to_string(), ratio);
        info!("Set group blend ratio for {}: {}", group, ratio);
    }

    /// Update blend ratios based on current time
    ///
    /// This should be called periodically when using time-based progression.
    pub async fn update_from_time(&self, _time: DateTime<Utc>) {
        // The blend ratio calculation happens on-demand in get_blend_ratio,
        // so this method is mainly for logging/observability purposes
        debug!("Continuum engine updated from time: {}", _time);
    }

    /// Get the response blender
    pub fn blender(&self) -> &ResponseBlender {
        &self.blender
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> ContinuumConfig {
        self.config.read().await.clone()
    }

    /// Update the configuration
    pub async fn update_config(&self, config: ContinuumConfig) {
        let mut current_config = self.config.write().await;
        *current_config = config;
        info!("Continuum configuration updated");
    }

    /// Check if continuum is enabled
    pub async fn is_enabled(&self) -> bool {
        self.config.read().await.enabled
    }

    /// Enable or disable continuum
    pub async fn set_enabled(&self, enabled: bool) {
        let mut config = self.config.write().await;
        config.enabled = enabled;
        if enabled {
            info!("Reality Continuum enabled");
        } else {
            info!("Reality Continuum disabled");
        }
    }

    /// Get the time schedule
    pub async fn get_time_schedule(&self) -> Option<TimeSchedule> {
        self.config.read().await.time_schedule.clone()
    }

    /// Update the time schedule
    pub async fn set_time_schedule(&self, schedule: TimeSchedule) {
        let mut config = self.config.write().await;
        config.time_schedule = Some(schedule);
        config.transition_mode = TransitionMode::TimeBased;
        info!("Time schedule updated");
    }

    /// Get current time (virtual or real)
    async fn get_current_time(&self) -> DateTime<Utc> {
        if let Some(ref clock) = self.virtual_clock {
            clock.now()
        } else {
            Utc::now()
        }
    }

    /// Advance the blend ratio manually (for testing/debugging)
    ///
    /// This increments the default ratio by a small amount.
    pub async fn advance_ratio(&self, increment: f64) {
        let mut config = self.config.write().await;
        let new_ratio = (config.default_ratio + increment).clamp(0.0, 1.0);
        config.default_ratio = new_ratio;
        info!("Advanced default blend ratio to {}", new_ratio);
    }

    /// Get all manual overrides
    pub async fn get_manual_overrides(&self) -> HashMap<String, f64> {
        self.manual_overrides.read().await.clone()
    }

    /// Clear all manual overrides
    pub async fn clear_manual_overrides(&self) {
        let mut overrides = self.manual_overrides.write().await;
        overrides.clear();
        info!("Cleared all manual blend ratio overrides");
    }
}

impl Default for RealityContinuumEngine {
    fn default() -> Self {
        Self::new(ContinuumConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_blend_ratio_default() {
        let engine = RealityContinuumEngine::new(ContinuumConfig::default());
        let ratio = engine.get_blend_ratio("/api/test").await;
        assert_eq!(ratio, 0.0); // Default is 0.0 (100% mock)
    }

    #[tokio::test]
    async fn test_set_get_blend_ratio() {
        let engine = RealityContinuumEngine::new(ContinuumConfig::default());
        engine.set_blend_ratio("/api/test", 0.75).await;
        let ratio = engine.get_blend_ratio("/api/test").await;
        assert_eq!(ratio, 0.75);
    }

    #[tokio::test]
    async fn test_route_rule_matching() {
        let mut config = ContinuumConfig::default();
        config.routes.push(ContinuumRule::new("/api/users/*".to_string(), 0.5));
        let engine = RealityContinuumEngine::new(config);

        let ratio = engine.get_blend_ratio("/api/users/123").await;
        assert_eq!(ratio, 0.5);
    }

    #[tokio::test]
    async fn test_group_ratio() {
        let mut config = ContinuumConfig::default();
        config.groups.insert("api-v1".to_string(), 0.3);
        config.routes.push(
            ContinuumRule::new("/api/users/*".to_string(), 0.5).with_group("api-v1".to_string()),
        );
        let engine = RealityContinuumEngine::new(config);

        // Group ratio should override route ratio
        let ratio = engine.get_blend_ratio("/api/users/123").await;
        assert_eq!(ratio, 0.3);
    }

    #[tokio::test]
    async fn test_time_based_ratio() {
        let start = Utc::now();
        let end = start + chrono::Duration::days(30);
        let schedule = TimeSchedule::new(start, end, 0.0, 1.0);

        let mut config = ContinuumConfig::default();
        config.transition_mode = TransitionMode::TimeBased;
        config.time_schedule = Some(schedule);
        let engine = RealityContinuumEngine::new(config);

        // At start time, should return start_ratio
        let ratio = engine.get_blend_ratio("/api/test").await;
        // Should be close to 0.0 (start_ratio)
        assert!(ratio < 0.1);
    }

    #[tokio::test]
    async fn test_remove_blend_ratio() {
        let engine = RealityContinuumEngine::new(ContinuumConfig::default());
        engine.set_blend_ratio("/api/test", 0.75).await;
        assert_eq!(engine.get_blend_ratio("/api/test").await, 0.75);

        engine.remove_blend_ratio("/api/test").await;
        assert_eq!(engine.get_blend_ratio("/api/test").await, 0.0); // Back to default
    }

    #[tokio::test]
    async fn test_group_ratio_override() {
        let mut config = ContinuumConfig::default();
        config.groups.insert("api-v1".to_string(), 0.8);
        config.routes.push(
            ContinuumRule::new("/api/users/*".to_string(), 0.5).with_group("api-v1".to_string()),
        );
        let engine = RealityContinuumEngine::new(config);

        // Group ratio should override route ratio
        let ratio = engine.get_blend_ratio("/api/users/123").await;
        assert_eq!(ratio, 0.8);
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let engine = RealityContinuumEngine::new(ContinuumConfig::default());
        assert!(!engine.is_enabled().await);

        engine.set_enabled(true).await;
        assert!(engine.is_enabled().await);

        engine.set_enabled(false).await;
        assert!(!engine.is_enabled().await);
    }

    #[tokio::test]
    async fn test_advance_ratio() {
        let engine = RealityContinuumEngine::new(ContinuumConfig::default());
        assert_eq!(engine.get_config().await.default_ratio, 0.0);

        engine.advance_ratio(0.2).await;
        assert_eq!(engine.get_config().await.default_ratio, 0.2);

        engine.advance_ratio(0.5).await;
        assert_eq!(engine.get_config().await.default_ratio, 0.7);

        // Should clamp at 1.0
        engine.advance_ratio(0.5).await;
        assert_eq!(engine.get_config().await.default_ratio, 1.0);
    }

    #[tokio::test]
    async fn test_clear_manual_overrides() {
        let engine = RealityContinuumEngine::new(ContinuumConfig::default());
        engine.set_blend_ratio("/api/test1", 0.5).await;
        engine.set_blend_ratio("/api/test2", 0.7).await;

        let overrides = engine.get_manual_overrides().await;
        assert_eq!(overrides.len(), 2);

        engine.clear_manual_overrides().await;
        let overrides = engine.get_manual_overrides().await;
        assert_eq!(overrides.len(), 0);
    }
}
