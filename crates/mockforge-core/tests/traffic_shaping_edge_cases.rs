//! Edge case tests for traffic shaping module
//!
//! These tests cover error paths, edge cases, and boundary conditions
//! for traffic shaping functionality.

use mockforge_core::traffic_shaping::{BandwidthConfig, BurstLossConfig, BurstLossOverride};
use std::collections::HashMap;

/// Test BandwidthConfig default
#[test]
fn test_bandwidth_config_default() {
    let config = BandwidthConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.max_bytes_per_sec, 0);
    assert_eq!(config.burst_capacity_bytes, 1024 * 1024);
    assert!(config.tag_overrides.is_empty());
}

/// Test BandwidthConfig new
#[test]
fn test_bandwidth_config_new() {
    let config = BandwidthConfig::new(1000, 500);
    assert!(config.enabled);
    assert_eq!(config.max_bytes_per_sec, 1000);
    assert_eq!(config.burst_capacity_bytes, 500);
    assert!(config.tag_overrides.is_empty());
}

/// Test BandwidthConfig with tag override
#[test]
fn test_bandwidth_config_tag_override() {
    let config = BandwidthConfig::new(1000, 500)
        .with_tag_override("premium".to_string(), 5000)
        .with_tag_override("basic".to_string(), 100);
    
    assert_eq!(config.tag_overrides.len(), 2);
    assert_eq!(config.tag_overrides.get("premium"), Some(&5000));
    assert_eq!(config.tag_overrides.get("basic"), Some(&100));
}

/// Test BandwidthConfig get_effective_limit with no tags
#[test]
fn test_bandwidth_config_get_effective_limit_no_tags() {
    let config = BandwidthConfig::new(1000, 500);
    assert_eq!(config.get_effective_limit(&[]), 1000);
}

/// Test BandwidthConfig get_effective_limit with matching tag
#[test]
fn test_bandwidth_config_get_effective_limit_with_tag() {
    let config = BandwidthConfig::new(1000, 500)
        .with_tag_override("premium".to_string(), 5000);
    
    assert_eq!(config.get_effective_limit(&["premium".to_string()]), 5000);
    assert_eq!(config.get_effective_limit(&["basic".to_string()]), 1000);
}

/// Test BandwidthConfig get_effective_limit with multiple tags
#[test]
fn test_bandwidth_config_get_effective_limit_multiple_tags() {
    let config = BandwidthConfig::new(1000, 500)
        .with_tag_override("premium".to_string(), 5000)
        .with_tag_override("vip".to_string(), 10000);
    
    // Should use first matching tag
    assert_eq!(config.get_effective_limit(&["premium".to_string(), "vip".to_string()]), 5000);
    assert_eq!(config.get_effective_limit(&["vip".to_string(), "premium".to_string()]), 10000);
}

/// Test BandwidthConfig get_effective_limit with zero limit
#[test]
fn test_bandwidth_config_get_effective_limit_zero() {
    let config = BandwidthConfig::new(0, 500);
    assert_eq!(config.get_effective_limit(&[]), 0);
}

/// Test BurstLossConfig default
#[test]
fn test_burst_loss_config_default() {
    let config = BurstLossConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.burst_probability, 0.1);
    assert_eq!(config.burst_duration_ms, 5000);
    assert_eq!(config.loss_rate_during_burst, 0.5);
    assert_eq!(config.recovery_time_ms, 30000);
    assert!(config.tag_overrides.is_empty());
}

/// Test BurstLossConfig new
#[test]
fn test_burst_loss_config_new() {
    let config = BurstLossConfig::new(0.2, 10000, 0.7, 60000);
    assert!(config.enabled);
    assert_eq!(config.burst_probability, 0.2);
    assert_eq!(config.burst_duration_ms, 10000);
    assert_eq!(config.loss_rate_during_burst, 0.7);
    assert_eq!(config.recovery_time_ms, 60000);
}

/// Test BurstLossConfig new with clamping
#[test]
fn test_burst_loss_config_new_clamping() {
    // Test values outside valid range get clamped
    let config = BurstLossConfig::new(-0.5, 10000, 1.5, 60000);
    assert_eq!(config.burst_probability, 0.0); // Clamped to 0.0
    assert_eq!(config.loss_rate_during_burst, 1.0); // Clamped to 1.0
}

/// Test BurstLossConfig with tag override
#[test]
fn test_burst_loss_config_tag_override() {
    let override_config = BurstLossOverride {
        burst_probability: 0.3,
        burst_duration_ms: 20000,
        loss_rate_during_burst: 0.8,
        recovery_time_ms: 120000,
    };
    
    let config = BurstLossConfig::new(0.1, 5000, 0.5, 30000)
        .with_tag_override("critical".to_string(), override_config.clone());
    
    assert_eq!(config.tag_overrides.len(), 1);
    let stored = config.tag_overrides.get("critical").unwrap();
    assert_eq!(stored.burst_probability, override_config.burst_probability);
    assert_eq!(stored.burst_duration_ms, override_config.burst_duration_ms);
    assert_eq!(stored.loss_rate_during_burst, override_config.loss_rate_during_burst);
    assert_eq!(stored.recovery_time_ms, override_config.recovery_time_ms);
}

/// Test BurstLossConfig effective_config with no tags
#[test]
fn test_burst_loss_config_effective_config_no_tags() {
    let config = BurstLossConfig::new(0.1, 5000, 0.5, 30000);
    let effective = config.effective_config(&[]);
    
    assert_eq!(effective.burst_probability, 0.1);
    assert_eq!(effective.burst_duration_ms, 5000);
    assert_eq!(effective.loss_rate_during_burst, 0.5);
    assert_eq!(effective.recovery_time_ms, 30000);
}

/// Test BurstLossConfig effective_config with matching tag
#[test]
fn test_burst_loss_config_effective_config_with_tag() {
    let override_config = BurstLossOverride {
        burst_probability: 0.3,
        burst_duration_ms: 20000,
        loss_rate_during_burst: 0.8,
        recovery_time_ms: 120000,
    };
    
    let config = BurstLossConfig::new(0.1, 5000, 0.5, 30000)
        .with_tag_override("critical".to_string(), override_config);
    
    let effective = config.effective_config(&["critical".to_string()]);
    
    assert_eq!(effective.burst_probability, 0.3);
    assert_eq!(effective.burst_duration_ms, 20000);
    assert_eq!(effective.loss_rate_during_burst, 0.8);
    assert_eq!(effective.recovery_time_ms, 120000);
}

/// Test BurstLossConfig effective_config with multiple tags
#[test]
fn test_burst_loss_config_effective_config_multiple_tags() {
    let override_config1 = BurstLossOverride {
        burst_probability: 0.3,
        burst_duration_ms: 20000,
        loss_rate_during_burst: 0.8,
        recovery_time_ms: 120000,
    };
    
    let override_config2 = BurstLossOverride {
        burst_probability: 0.5,
        burst_duration_ms: 30000,
        loss_rate_during_burst: 0.9,
        recovery_time_ms: 180000,
    };
    
    let config = BurstLossConfig::new(0.1, 5000, 0.5, 30000)
        .with_tag_override("tag1".to_string(), override_config1)
        .with_tag_override("tag2".to_string(), override_config2);
    
    // Should use first matching tag
    let effective = config.effective_config(&["tag1".to_string(), "tag2".to_string()]);
    assert_eq!(effective.burst_probability, 0.3);
}

/// Test BurstLossConfig with zero probability
#[test]
fn test_burst_loss_config_zero_probability() {
    let config = BurstLossConfig::new(0.0, 5000, 0.5, 30000);
    assert_eq!(config.burst_probability, 0.0);
}

/// Test BurstLossConfig with maximum probability
#[test]
fn test_burst_loss_config_max_probability() {
    let config = BurstLossConfig::new(1.0, 5000, 0.5, 30000);
    assert_eq!(config.burst_probability, 1.0);
}

/// Test BurstLossConfig with zero loss rate
#[test]
fn test_burst_loss_config_zero_loss_rate() {
    let config = BurstLossConfig::new(0.1, 5000, 0.0, 30000);
    assert_eq!(config.loss_rate_during_burst, 0.0);
}

/// Test BurstLossConfig with maximum loss rate
#[test]
fn test_burst_loss_config_max_loss_rate() {
    let config = BurstLossConfig::new(0.1, 5000, 1.0, 30000);
    assert_eq!(config.loss_rate_during_burst, 1.0);
}

/// Test BurstLossConfig with very short duration
#[test]
fn test_burst_loss_config_short_duration() {
    let config = BurstLossConfig::new(0.1, 1, 0.5, 30000);
    assert_eq!(config.burst_duration_ms, 1);
}

/// Test BurstLossConfig with very long duration
#[test]
fn test_burst_loss_config_long_duration() {
    let config = BurstLossConfig::new(0.1, 3600000, 0.5, 30000); // 1 hour
    assert_eq!(config.burst_duration_ms, 3600000);
}

/// Test BurstLossConfig with zero recovery time
#[test]
fn test_burst_loss_config_zero_recovery() {
    let config = BurstLossConfig::new(0.1, 5000, 0.5, 0);
    assert_eq!(config.recovery_time_ms, 0);
}

/// Test BurstLossConfig overwriting tag override
#[test]
fn test_burst_loss_config_overwrite_tag_override() {
    let override1 = BurstLossOverride {
        burst_probability: 0.1,
        burst_duration_ms: 5000,
        loss_rate_during_burst: 0.5,
        recovery_time_ms: 30000,
    };
    
    let override2 = BurstLossOverride {
        burst_probability: 0.2,
        burst_duration_ms: 10000,
        loss_rate_during_burst: 0.6,
        recovery_time_ms: 60000,
    };
    
    let config = BurstLossConfig::new(0.1, 5000, 0.5, 30000)
        .with_tag_override("tag".to_string(), override1)
        .with_tag_override("tag".to_string(), override2);
    
    // Should have the last override
    let effective = config.effective_config(&["tag".to_string()]);
    assert_eq!(effective.burst_probability, 0.2);
}

/// Test BandwidthConfig with multiple tag overrides
#[test]
fn test_bandwidth_config_multiple_tag_overrides() {
    let config = BandwidthConfig::new(1000, 500)
        .with_tag_override("tag1".to_string(), 2000)
        .with_tag_override("tag2".to_string(), 3000)
        .with_tag_override("tag3".to_string(), 4000);
    
    assert_eq!(config.tag_overrides.len(), 3);
    assert_eq!(config.get_effective_limit(&["tag1".to_string()]), 2000);
    assert_eq!(config.get_effective_limit(&["tag2".to_string()]), 3000);
    assert_eq!(config.get_effective_limit(&["tag3".to_string()]), 4000);
}

/// Test BandwidthConfig overwriting tag override
#[test]
fn test_bandwidth_config_overwrite_tag_override() {
    let config = BandwidthConfig::new(1000, 500)
        .with_tag_override("tag".to_string(), 2000)
        .with_tag_override("tag".to_string(), 3000);
    
    assert_eq!(config.tag_overrides.len(), 1);
    assert_eq!(config.get_effective_limit(&["tag".to_string()]), 3000);
}

/// Test BurstLossOverride with edge values
#[test]
fn test_burst_loss_override_edge_values() {
    let override_config = BurstLossOverride {
        burst_probability: 0.0,
        burst_duration_ms: 0,
        loss_rate_during_burst: 0.0,
        recovery_time_ms: 0,
    };
    
    let config = BurstLossConfig::new(0.1, 5000, 0.5, 30000)
        .with_tag_override("edge".to_string(), override_config);
    
    let effective = config.effective_config(&["edge".to_string()]);
    assert_eq!(effective.burst_probability, 0.0);
    assert_eq!(effective.burst_duration_ms, 0);
    assert_eq!(effective.loss_rate_during_burst, 0.0);
    assert_eq!(effective.recovery_time_ms, 0);
}

/// Test BurstLossOverride with maximum values
#[test]
fn test_burst_loss_override_max_values() {
    let override_config = BurstLossOverride {
        burst_probability: 1.0,
        burst_duration_ms: u64::MAX,
        loss_rate_during_burst: 1.0,
        recovery_time_ms: u64::MAX,
    };
    
    let config = BurstLossConfig::new(0.1, 5000, 0.5, 30000)
        .with_tag_override("max".to_string(), override_config);
    
    let effective = config.effective_config(&["max".to_string()]);
    assert_eq!(effective.burst_probability, 1.0);
    assert_eq!(effective.burst_duration_ms, u64::MAX);
    assert_eq!(effective.loss_rate_during_burst, 1.0);
    assert_eq!(effective.recovery_time_ms, u64::MAX);
}

