//! Integration tests for Reality Continuum feature
//!
//! Tests the full flow of blending mock and real responses with time-based progression.

use chrono::{DateTime, Duration, Utc};
use mockforge_core::{
    ContinuumConfig, ContinuumRule, MergeStrategy, RealityContinuumEngine, ResponseBlender,
    TimeSchedule, TransitionCurve, TransitionMode, VirtualClock,
};
use std::sync::Arc;

#[tokio::test]
async fn test_continuum_with_virtual_clock() {
    // Create virtual clock
    let clock = Arc::new(VirtualClock::new());
    let start_time = Utc::now();
    clock.enable_and_set(start_time);

    // Create time schedule
    let end_time = start_time + Duration::days(30);
    let schedule =
        TimeSchedule::with_curve(start_time, end_time, 0.0, 1.0, TransitionCurve::Linear);

    // Create continuum config
    let mut config = ContinuumConfig::default();
    config.enabled = true;
    config.transition_mode = TransitionMode::TimeBased;
    config.time_schedule = Some(schedule);
    config.default_ratio = 0.0;

    // Create engine with virtual clock
    let engine = RealityContinuumEngine::with_virtual_clock(config, clock.clone());

    // At start time, ratio should be 0.0
    let ratio_start = engine.get_blend_ratio("/api/test").await;
    assert!(ratio_start < 0.1);

    // Advance time to midpoint
    clock.advance(Duration::days(15));
    let ratio_mid = engine.get_blend_ratio("/api/test").await;
    assert!((ratio_mid - 0.5).abs() < 0.1); // Should be approximately 0.5

    // Advance time to end
    clock.advance(Duration::days(15));
    let ratio_end = engine.get_blend_ratio("/api/test").await;
    assert!(ratio_end > 0.9); // Should be close to 1.0
}

#[tokio::test]
async fn test_continuum_response_blending() {
    let blender = ResponseBlender::default();
    let mock = serde_json::json!({
        "id": 1,
        "name": "Mock User",
        "email": "mock@example.com",
        "status": "pending"
    });
    let real = serde_json::json!({
        "id": 2,
        "name": "Real User",
        "email": "real@example.com",
        "status": "active",
        "verified": true
    });

    // Blend at 0.5 ratio
    let blended = blender.blend_responses(&mock, &real, 0.5);
    assert!(blended.is_object());
    assert!(blended.get("id").is_some());
    assert!(blended.get("name").is_some());
    assert!(blended.get("email").is_some());
}

#[tokio::test]
async fn test_continuum_route_priority() {
    let mut config = ContinuumConfig::default();
    config.enabled = true;
    config.default_ratio = 0.0;

    // Add route-specific rule
    config.routes.push(ContinuumRule::new("/api/users/*".to_string(), 0.7));

    let engine = RealityContinuumEngine::new(config);

    // Route-specific ratio should override default
    let user_ratio = engine.get_blend_ratio("/api/users/123").await;
    assert_eq!(user_ratio, 0.7);

    // Other routes should use default
    let other_ratio = engine.get_blend_ratio("/api/orders/456").await;
    assert_eq!(other_ratio, 0.0);
}

#[tokio::test]
async fn test_continuum_manual_override() {
    let config = ContinuumConfig::default();
    let engine = RealityContinuumEngine::new(config);

    // Set manual override
    engine.set_blend_ratio("/api/special", 0.9).await;
    let ratio = engine.get_blend_ratio("/api/special").await;
    assert_eq!(ratio, 0.9);

    // Remove override
    engine.remove_blend_ratio("/api/special").await;
    let ratio_after = engine.get_blend_ratio("/api/special").await;
    assert_eq!(ratio_after, 0.0); // Back to default
}

#[tokio::test]
async fn test_continuum_time_schedule_curves() {
    let start = Utc::now();
    let end = start + Duration::days(30);

    // Test linear curve
    let linear = TimeSchedule::with_curve(start, end, 0.0, 1.0, TransitionCurve::Linear);
    let midpoint = start + Duration::days(15);
    let linear_ratio = linear.calculate_ratio(midpoint);
    assert!((linear_ratio - 0.5).abs() < 0.01);

    // Test exponential curve (should be less than linear at midpoint)
    let exponential = TimeSchedule::with_curve(start, end, 0.0, 1.0, TransitionCurve::Exponential);
    let exp_ratio = exponential.calculate_ratio(midpoint);
    assert!(exp_ratio < linear_ratio);

    // Test sigmoid curve (should be close to 0.5 at midpoint)
    let sigmoid = TimeSchedule::with_curve(start, end, 0.0, 1.0, TransitionCurve::Sigmoid);
    let sig_ratio = sigmoid.calculate_ratio(midpoint);
    assert!((sig_ratio - 0.5).abs() < 0.1);
}
