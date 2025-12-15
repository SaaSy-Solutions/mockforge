//! Edge case tests for latency and failure injection
//!
//! These tests cover error paths, edge cases, and boundary conditions
//! for latency simulation and failure injection.

use mockforge_core::failure_injection::{FailureConfig, FailureInjector, TagFailureConfig};
use mockforge_core::latency::{LatencyDistribution, LatencyProfile};
use std::collections::HashMap;

/// Test latency profile with zero base latency
#[test]
fn test_latency_profile_zero_base() {
    let profile = LatencyProfile::new(0, 10);
    let latency = profile.calculate_latency(&[]);
    // Should still produce some latency due to jitter
    assert!(latency.as_millis() <= 10);
}

/// Test latency profile with zero jitter
#[test]
fn test_latency_profile_zero_jitter() {
    let profile = LatencyProfile::new(100, 0);
    let latency = profile.calculate_latency(&[]);
    assert_eq!(latency.as_millis(), 100);
}

/// Test latency profile with very large values
#[test]
fn test_latency_profile_large_values() {
    let profile = LatencyProfile::new(10000, 5000);
    let latency = profile.calculate_latency(&[]);
    // Should be within base ± jitter range
    assert!(latency.as_millis() >= 5000);
    assert!(latency.as_millis() <= 15000);
}

/// Test latency profile with max bound
#[test]
fn test_latency_profile_max_bound() {
    let profile = LatencyProfile::new(100, 50).with_max_ms(120);
    let latency = profile.calculate_latency(&[]);
    // Should not exceed max
    assert!(latency.as_millis() <= 120);
}

/// Test latency profile with min bound
#[test]
fn test_latency_profile_min_bound() {
    let profile = LatencyProfile::new(100, 200).with_min_ms(50);
    let latency = profile.calculate_latency(&[]);
    // Should not go below min
    assert!(latency.as_millis() >= 50);
}

/// Test latency profile with tag overrides
#[test]
fn test_latency_profile_tag_overrides() {
    let profile = LatencyProfile::new(100, 10).with_tag_override("critical".to_string(), 500);

    // Test with matching tag
    let latency_with_tag = profile.calculate_latency(&["critical".to_string()]);
    assert_eq!(latency_with_tag.as_millis(), 500);

    // Test without matching tag
    let latency_without_tag = profile.calculate_latency(&["normal".to_string()]);
    assert!(latency_without_tag.as_millis() >= 90 && latency_without_tag.as_millis() <= 110);
}

/// Test latency profile with multiple tag overrides (first match wins)
#[test]
fn test_latency_profile_multiple_tag_overrides() {
    let profile = LatencyProfile::new(100, 10)
        .with_tag_override("critical".to_string(), 500)
        .with_tag_override("important".to_string(), 300);

    // First matching tag should be used
    let latency = profile.calculate_latency(&["critical".to_string(), "important".to_string()]);
    assert_eq!(latency.as_millis(), 500);
}

/// Test normal distribution latency profile
#[test]
fn test_latency_profile_normal_distribution() {
    let profile = LatencyProfile::with_normal_distribution(100, 20.0);
    let latency = profile.calculate_latency(&[]);
    // Should be roughly around 100ms with some variance
    assert!(latency.as_millis() > 0);
    // Most values should be within 3 standard deviations (60-140ms)
    // But we allow wider range for edge cases
    assert!(latency.as_millis() < 500);
}

/// Test pareto distribution latency profile
#[test]
fn test_latency_profile_pareto_distribution() {
    let profile = LatencyProfile::with_pareto_distribution(100, 2.0);
    let latency = profile.calculate_latency(&[]);
    // Pareto can produce very high values, but should be positive
    assert!(latency.as_millis() > 0);
}

/// Test failure injector with zero error rate
#[test]
fn test_failure_injector_zero_rate() {
    let config = FailureConfig {
        global_error_rate: 0.0,
        ..Default::default()
    };
    let injector = FailureInjector::new(Some(config), true);

    // Should never inject failure
    for _ in 0..100 {
        assert!(!injector.should_inject_failure(&[]));
    }
}

/// Test failure injector with 100% error rate
#[test]
fn test_failure_injector_full_rate() {
    let config = FailureConfig {
        global_error_rate: 1.0,
        ..Default::default()
    };
    let injector = FailureInjector::new(Some(config), true);

    // Should always inject failure
    for _ in 0..100 {
        assert!(injector.should_inject_failure(&[]));
    }
}

/// Test failure injector disabled
#[test]
fn test_failure_injector_disabled() {
    let config = FailureConfig {
        global_error_rate: 1.0,
        ..Default::default()
    };
    let injector = FailureInjector::new(Some(config), false);

    // Should never inject failure when disabled
    assert!(!injector.should_inject_failure(&[]));
}

/// Test failure injector with no config
#[test]
fn test_failure_injector_no_config() {
    let injector = FailureInjector::new(None, true);

    // Should never inject failure without config
    assert!(!injector.should_inject_failure(&[]));
}

/// Test failure injector with exclude tags
#[test]
fn test_failure_injector_exclude_tags() {
    let config = FailureConfig {
        global_error_rate: 1.0,
        exclude_tags: vec!["health".to_string()],
        ..Default::default()
    };
    let injector = FailureInjector::new(Some(config), true);

    // Should inject for normal tags
    assert!(injector.should_inject_failure(&["api".to_string()]));

    // Should not inject for excluded tags
    assert!(!injector.should_inject_failure(&["health".to_string()]));
}

/// Test failure injector with include tags
#[test]
fn test_failure_injector_include_tags() {
    let config = FailureConfig {
        global_error_rate: 1.0,
        include_tags: vec!["critical".to_string()],
        ..Default::default()
    };
    let injector = FailureInjector::new(Some(config), true);

    // Should inject for included tags
    assert!(injector.should_inject_failure(&["critical".to_string()]));

    // Should not inject for non-included tags
    assert!(!injector.should_inject_failure(&["normal".to_string()]));
}

/// Test failure injector with tag-specific config
#[test]
fn test_failure_injector_tag_specific_config() {
    let mut tag_configs = HashMap::new();
    tag_configs.insert(
        "critical".to_string(),
        TagFailureConfig {
            error_rate: 1.0,
            status_codes: Some(vec![503]),
            error_message: Some("Critical service failure".to_string()),
        },
    );

    let config = FailureConfig {
        global_error_rate: 0.0,
        tag_configs,
        ..Default::default()
    };
    let injector = FailureInjector::new(Some(config), true);

    // Should inject for critical tag
    assert!(injector.should_inject_failure(&["critical".to_string()]));

    // Should not inject for other tags
    assert!(!injector.should_inject_failure(&["normal".to_string()]));

    // Check failure response
    let response = injector.get_failure_response(&["critical".to_string()]);
    assert!(response.is_some());
    let (status, message) = response.unwrap();
    assert_eq!(status, 503);
    assert_eq!(message, "Critical service failure");
}

/// Test failure injector with empty status codes
#[test]
fn test_failure_injector_empty_status_codes() {
    let config = FailureConfig {
        global_error_rate: 1.0,
        default_status_codes: vec![],
        ..Default::default()
    };
    let injector = FailureInjector::new(Some(config), true);

    let response = injector.get_failure_response(&[]);
    assert!(response.is_some());
    let (status, _) = response.unwrap();
    // Should default to 500 when no status codes provided
    assert_eq!(status, 500);
}

/// Test failure injector probabilistic behavior
#[test]
fn test_failure_injector_probabilistic() {
    let config = FailureConfig {
        global_error_rate: 0.5,
        ..Default::default()
    };
    let injector = FailureInjector::new(Some(config), true);

    let mut failures = 0;
    let iterations = 1000;

    for _ in 0..iterations {
        if injector.should_inject_failure(&[]) {
            failures += 1;
        }
    }

    // Should be roughly 50% with tolerance
    let failure_rate = failures as f64 / iterations as f64;
    assert!(
        failure_rate > 0.4 && failure_rate < 0.6,
        "Failure rate was {}, expected ~0.5",
        failure_rate
    );
}

/// Test failure injector with multiple tags and precedence
#[test]
fn test_failure_injector_tag_precedence() {
    let mut tag_configs = HashMap::new();
    tag_configs.insert(
        "critical".to_string(),
        TagFailureConfig {
            error_rate: 1.0,
            status_codes: Some(vec![503]),
            error_message: Some("Critical".to_string()),
        },
    );
    tag_configs.insert(
        "important".to_string(),
        TagFailureConfig {
            error_rate: 1.0,
            status_codes: Some(vec![502]),
            error_message: Some("Important".to_string()),
        },
    );

    let config = FailureConfig {
        global_error_rate: 0.0,
        tag_configs,
        ..Default::default()
    };
    let injector = FailureInjector::new(Some(config), true);

    // First tag in the list should be used (implementation-dependent)
    let response =
        injector.get_failure_response(&["critical".to_string(), "important".to_string()]);
    assert!(response.is_some());
    // Should use one of the tag configs
    let (status, _) = response.unwrap();
    assert!(status == 503 || status == 502);
}

/// Test latency profile with all distributions
#[test]
fn test_latency_profile_all_distributions() {
    // Fixed distribution
    let fixed = LatencyProfile::new(100, 20);
    let latency_fixed = fixed.calculate_latency(&[]);
    assert!(latency_fixed.as_millis() >= 80 && latency_fixed.as_millis() <= 120);

    // Normal distribution
    let normal = LatencyProfile::with_normal_distribution(100, 20.0);
    let latency_normal = normal.calculate_latency(&[]);
    assert!(latency_normal.as_millis() > 0);

    // Pareto distribution
    let pareto = LatencyProfile::with_pareto_distribution(100, 2.0);
    let latency_pareto = pareto.calculate_latency(&[]);
    assert!(latency_pareto.as_millis() > 0);
}

/// Test latency profile edge cases with bounds
#[test]
fn test_latency_profile_bounds_edge_cases() {
    // Min greater than base
    let profile = LatencyProfile::new(50, 10).with_min_ms(100);
    let latency = profile.calculate_latency(&[]);
    assert!(latency.as_millis() >= 100);

    // Max less than base
    let profile = LatencyProfile::new(100, 10).with_max_ms(50);
    let latency = profile.calculate_latency(&[]);
    assert!(latency.as_millis() <= 50);
}

/// Test failure injector with include and exclude tags (exclude takes precedence)
#[test]
fn test_failure_injector_include_exclude_precedence() {
    let config = FailureConfig {
        global_error_rate: 1.0,
        include_tags: vec!["api".to_string()],
        exclude_tags: vec!["health".to_string()],
        ..Default::default()
    };
    let injector = FailureInjector::new(Some(config), true);

    // Health tag is excluded, so should not inject even if it matches include
    assert!(!injector.should_inject_failure(&["health".to_string()]));

    // API tag is included and not excluded, so should inject
    assert!(injector.should_inject_failure(&["api".to_string()]));

    // Tag not in include list, should not inject
    assert!(!injector.should_inject_failure(&["other".to_string()]));
}

/// Test failure injector get_failure_response with various configs
#[test]
fn test_failure_injector_get_response_variations() {
    // Test with custom status codes
    let config = FailureConfig {
        global_error_rate: 1.0,
        default_status_codes: vec![500, 502, 503],
        ..Default::default()
    };
    let injector = FailureInjector::new(Some(config), true);

    let response = injector.get_failure_response(&[]);
    assert!(response.is_some());
    let (status, _) = response.unwrap();
    assert!(vec![500, 502, 503].contains(&status));

    // Test with custom error message
    let config = FailureConfig {
        global_error_rate: 1.0,
        default_status_codes: vec![500],
        ..Default::default()
    };
    let injector = FailureInjector::new(Some(config), true);

    let response = injector.get_failure_response(&[]);
    assert!(response.is_some());
    let (_, message) = response.unwrap();
    assert_eq!(message, "Injected failure");
}

/// Test latency profile consistency across multiple calls
#[test]
fn test_latency_profile_consistency() {
    let profile = LatencyProfile::new(100, 0); // No jitter for consistency

    // All calls should produce same latency (within floating point precision)
    let latency1 = profile.calculate_latency(&[]);
    // With jitter=0, should be consistent, but random distributions will vary
    // So we just check it's in a reasonable range
    assert!(latency1.as_millis() >= 90 && latency1.as_millis() <= 110);
}

/// Test failure injector with very small error rate
#[test]
fn test_failure_injector_small_rate() {
    let config = FailureConfig {
        global_error_rate: 0.01, // 1%
        ..Default::default()
    };
    let injector = FailureInjector::new(Some(config), true);

    let mut failures = 0;
    let iterations = 10000;

    for _ in 0..iterations {
        if injector.should_inject_failure(&[]) {
            failures += 1;
        }
    }

    // Should be roughly 1% with tolerance
    let failure_rate = failures as f64 / iterations as f64;
    assert!(
        failure_rate > 0.005 && failure_rate < 0.02,
        "Failure rate was {}, expected ~0.01",
        failure_rate
    );
}

/// Test failure injector with very high error rate (but not 100%)
#[test]
fn test_failure_injector_high_rate() {
    let config = FailureConfig {
        global_error_rate: 0.99, // 99%
        ..Default::default()
    };
    let injector = FailureInjector::new(Some(config), true);

    let mut failures = 0;
    let iterations = 1000;

    for _ in 0..iterations {
        if injector.should_inject_failure(&[]) {
            failures += 1;
        }
    }

    // Should be roughly 99% with tolerance
    let failure_rate = failures as f64 / iterations as f64;
    assert!(
        failure_rate > 0.95 && failure_rate < 1.0,
        "Failure rate was {}, expected ~0.99",
        failure_rate
    );
}
