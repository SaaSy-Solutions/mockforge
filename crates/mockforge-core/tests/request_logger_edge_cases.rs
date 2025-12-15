//! Edge case tests for request logger module
//!
//! These tests cover error paths, edge cases, and boundary conditions
//! for request logging functionality.

use mockforge_core::request_logger::{DataSourceBreakdown, RealityContinuumType, RealityTraceMetadata};

/// Test RealityContinuumType from_blend_ratio
#[test]
fn test_reality_continuum_type_from_blend_ratio() {
    assert_eq!(RealityContinuumType::from_blend_ratio(-1.0), RealityContinuumType::Synthetic);
    assert_eq!(RealityContinuumType::from_blend_ratio(0.0), RealityContinuumType::Synthetic);
    assert_eq!(RealityContinuumType::from_blend_ratio(0.5), RealityContinuumType::Blended);
    assert_eq!(RealityContinuumType::from_blend_ratio(1.0), RealityContinuumType::Live);
    assert_eq!(RealityContinuumType::from_blend_ratio(2.0), RealityContinuumType::Live);
}

/// Test RealityContinuumType name
#[test]
fn test_reality_continuum_type_name() {
    assert_eq!(RealityContinuumType::Synthetic.name(), "Synthetic");
    assert_eq!(RealityContinuumType::Blended.name(), "Blended");
    assert_eq!(RealityContinuumType::Live.name(), "Live");
}

/// Test DataSourceBreakdown default
#[test]
fn test_data_source_breakdown_default() {
    let breakdown = DataSourceBreakdown::default();
    assert_eq!(breakdown.recorded_percent, 0.0);
    assert_eq!(breakdown.generator_percent, 100.0);
    assert_eq!(breakdown.upstream_percent, 0.0);
}

/// Test DataSourceBreakdown from_blend_ratio with zero blend
#[test]
fn test_data_source_breakdown_from_blend_ratio_zero() {
    let breakdown = DataSourceBreakdown::from_blend_ratio(0.0, 0.0);
    assert_eq!(breakdown.recorded_percent, 0.0);
    assert_eq!(breakdown.generator_percent, 100.0);
    assert_eq!(breakdown.upstream_percent, 0.0);
}

/// Test DataSourceBreakdown from_blend_ratio with full blend
#[test]
fn test_data_source_breakdown_from_blend_ratio_full() {
    let breakdown = DataSourceBreakdown::from_blend_ratio(1.0, 0.0);
    assert_eq!(breakdown.recorded_percent, 0.0);
    assert_eq!(breakdown.generator_percent, 0.0);
    assert_eq!(breakdown.upstream_percent, 100.0);
}

/// Test DataSourceBreakdown from_blend_ratio with recorded data
#[test]
fn test_data_source_breakdown_from_blend_ratio_with_recorded() {
    let breakdown = DataSourceBreakdown::from_blend_ratio(0.5, 0.2);
    assert_eq!(breakdown.recorded_percent, 20.0);
    assert_eq!(breakdown.generator_percent, 40.0); // (1 - 0.5) * (1 - 0.2) * 100
    assert_eq!(breakdown.upstream_percent, 40.0); // 0.5 * (1 - 0.2) * 100
}

/// Test DataSourceBreakdown from_blend_ratio with full recorded
#[test]
fn test_data_source_breakdown_from_blend_ratio_full_recorded() {
    let breakdown = DataSourceBreakdown::from_blend_ratio(0.5, 1.0);
    assert_eq!(breakdown.recorded_percent, 100.0);
    assert_eq!(breakdown.generator_percent, 0.0);
    assert_eq!(breakdown.upstream_percent, 0.0);
}

/// Test DataSourceBreakdown normalize
#[test]
fn test_data_source_breakdown_normalize() {
    let mut breakdown = DataSourceBreakdown {
        recorded_percent: 50.0,
        generator_percent: 50.0,
        upstream_percent: 50.0,
    };
    
    breakdown.normalize();
    
    // Should sum to 100.0
    let total = breakdown.recorded_percent + breakdown.generator_percent + breakdown.upstream_percent;
    assert!((total - 100.0).abs() < 0.01);
}

/// Test DataSourceBreakdown normalize with zero total
#[test]
fn test_data_source_breakdown_normalize_zero_total() {
    let mut breakdown = DataSourceBreakdown {
        recorded_percent: 0.0,
        generator_percent: 0.0,
        upstream_percent: 0.0,
    };
    
    breakdown.normalize();
    
    // Should remain zero
    assert_eq!(breakdown.recorded_percent, 0.0);
    assert_eq!(breakdown.generator_percent, 0.0);
    assert_eq!(breakdown.upstream_percent, 0.0);
}

/// Test DataSourceBreakdown normalize with large values
#[test]
fn test_data_source_breakdown_normalize_large_values() {
    let mut breakdown = DataSourceBreakdown {
        recorded_percent: 200.0,
        generator_percent: 300.0,
        upstream_percent: 500.0,
    };
    
    breakdown.normalize();
    
    // Should sum to 100.0
    let total = breakdown.recorded_percent + breakdown.generator_percent + breakdown.upstream_percent;
    assert!((total - 100.0).abs() < 0.01);
}

/// Test RealityTraceMetadata default
#[test]
fn test_reality_trace_metadata_default() {
    let metadata = RealityTraceMetadata::default();
    assert!(metadata.reality_level.is_none());
    assert_eq!(metadata.reality_continuum_type, RealityContinuumType::Synthetic);
    assert_eq!(metadata.blend_ratio, 0.0);
    assert_eq!(metadata.data_source_breakdown.recorded_percent, 0.0);
    assert_eq!(metadata.data_source_breakdown.generator_percent, 100.0);
    assert_eq!(metadata.data_source_breakdown.upstream_percent, 0.0);
    assert!(metadata.active_persona_id.is_none());
    assert!(metadata.active_scenario.is_none());
    assert!(metadata.active_chaos_profiles.is_empty());
    assert!(metadata.active_latency_profiles.is_empty());
}

/// Test RealityContinuumType equality
#[test]
fn test_reality_continuum_type_equality() {
    assert_eq!(RealityContinuumType::Synthetic, RealityContinuumType::Synthetic);
    assert_eq!(RealityContinuumType::Blended, RealityContinuumType::Blended);
    assert_eq!(RealityContinuumType::Live, RealityContinuumType::Live);
    assert_ne!(RealityContinuumType::Synthetic, RealityContinuumType::Blended);
    assert_ne!(RealityContinuumType::Blended, RealityContinuumType::Live);
    assert_ne!(RealityContinuumType::Synthetic, RealityContinuumType::Live);
}

/// Test DataSourceBreakdown from_blend_ratio edge cases
#[test]
fn test_data_source_breakdown_from_blend_ratio_edge_cases() {
    // Negative blend ratio (should still work)
    let breakdown = DataSourceBreakdown::from_blend_ratio(-0.5, 0.0);
    assert_eq!(breakdown.upstream_percent, -50.0);
    
    // Blend ratio > 1.0
    let breakdown2 = DataSourceBreakdown::from_blend_ratio(1.5, 0.0);
    assert_eq!(breakdown2.upstream_percent, 150.0);
    
    // Negative recorded ratio
    let breakdown3 = DataSourceBreakdown::from_blend_ratio(0.5, -0.2);
    assert_eq!(breakdown3.recorded_percent, -20.0);
    
    // Recorded ratio > 1.0
    let breakdown4 = DataSourceBreakdown::from_blend_ratio(0.5, 1.5);
    assert_eq!(breakdown4.recorded_percent, 150.0);
}

/// Test DataSourceBreakdown normalize preserves ratios
#[test]
fn test_data_source_breakdown_normalize_preserves_ratios() {
    let mut breakdown = DataSourceBreakdown {
        recorded_percent: 25.0,
        generator_percent: 25.0,
        upstream_percent: 50.0,
    };
    
    let original_ratios = (
        breakdown.recorded_percent / 100.0,
        breakdown.generator_percent / 100.0,
        breakdown.upstream_percent / 100.0,
    );
    
    breakdown.normalize();
    
    let normalized_ratios = (
        breakdown.recorded_percent / 100.0,
        breakdown.generator_percent / 100.0,
        breakdown.upstream_percent / 100.0,
    );
    
    // Ratios should be preserved (approximately)
    assert!((original_ratios.0 - normalized_ratios.0).abs() < 0.01);
    assert!((original_ratios.1 - normalized_ratios.1).abs() < 0.01);
    assert!((original_ratios.2 - normalized_ratios.2).abs() < 0.01);
}

