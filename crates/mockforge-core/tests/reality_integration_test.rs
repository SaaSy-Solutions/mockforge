//! Integration tests for Reality Slider functionality
//!
//! These tests verify that the RealityEngine correctly coordinates
//! chaos, latency, and MockAI subsystems based on the selected reality level.

use mockforge_core::reality::{
    RealityConfig, RealityEngine, RealityLevel, RealityPreset, PresetMetadata,
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_reality_level_creation() {
    // Test that all reality levels can be created
    for level_value in 1..=5 {
        let level = RealityLevel::from_value(level_value);
        assert!(level.is_some(), "Level {} should be valid", level_value);

        let level = level.unwrap();
        assert_eq!(level.value(), level_value);
        assert!(!level.name().is_empty());
        assert!(!level.description().is_empty());
    }

    // Test invalid levels
    assert!(RealityLevel::from_value(0).is_none());
    assert!(RealityLevel::from_value(6).is_none());
    assert!(RealityLevel::from_value(99).is_none());
}

#[tokio::test]
async fn test_reality_engine_initialization() {
    let engine = RealityEngine::new();

    // Default should be Level 3 (Moderate Realism)
    assert_eq!(engine.get_level().await, RealityLevel::ModerateRealism);

    let config = engine.get_config().await;
    assert!(config.chaos.error_rate > 0.0);
    assert!(config.latency.base_ms > 0);
    assert!(config.mockai.enabled);
}

#[tokio::test]
async fn test_reality_level_changes() {
    let engine = Arc::new(RwLock::new(RealityEngine::new()));

    // Test Level 1: Static Stubs
    {
        let engine = engine.read().await;
        engine.set_level(RealityLevel::StaticStubs).await;
        drop(engine);

        let engine = engine.read().await;
        assert_eq!(engine.get_level().await, RealityLevel::StaticStubs);

        let config = engine.get_config().await;
        assert_eq!(config.chaos.error_rate, 0.0);
        assert_eq!(config.latency.base_ms, 0);
        assert!(!config.mockai.enabled);
    }

    // Test Level 5: Production Chaos
    {
        let engine = engine.read().await;
        engine.set_level(RealityLevel::ProductionChaos).await;
        drop(engine);

        let engine = engine.read().await;
        assert_eq!(engine.get_level().await, RealityLevel::ProductionChaos);

        let config = engine.get_config().await;
        assert!(config.chaos.error_rate > 0.1);
        assert!(config.latency.base_ms > 100);
        assert!(config.mockai.enabled);
    }
}

#[tokio::test]
async fn test_reality_config_progression() {
    let engine = Arc::new(RwLock::new(RealityEngine::new()));

    let mut prev_error_rate = 0.0;
    let mut prev_latency = 0;

    for level_value in 1..=5 {
        let level = RealityLevel::from_value(level_value).unwrap();

        {
            let engine = engine.read().await;
            engine.set_level(level).await;
            drop(engine);

            let engine = engine.read().await;
            let config = engine.get_config().await;

            // Error rate should increase or stay the same as level increases
            assert!(
                config.chaos.error_rate >= prev_error_rate,
                "Error rate should increase with level. Level {}: {}, Previous: {}",
                level_value,
                config.chaos.error_rate,
                prev_error_rate
            );

            // Latency should increase or stay the same as level increases
            assert!(
                config.latency.base_ms >= prev_latency,
                "Latency should increase with level. Level {}: {}, Previous: {}",
                level_value,
                config.latency.base_ms,
                prev_latency
            );

            prev_error_rate = config.chaos.error_rate;
            prev_latency = config.latency.base_ms;
        }
    }
}

#[tokio::test]
async fn test_reality_preset_creation() {
    let engine = Arc::new(RwLock::new(RealityEngine::new()));

    {
        let engine = engine.read().await;
        engine.set_level(RealityLevel::HighRealism).await;
        drop(engine);
    }

    {
        let engine = engine.read().await;
        let preset = engine.create_preset(
            "test-preset".to_string(),
            Some("Test preset description".to_string()),
        ).await;

        assert_eq!(preset.name, "test-preset");
        assert_eq!(
            preset.description,
            Some("Test preset description".to_string())
        );
        assert_eq!(preset.config.level, RealityLevel::HighRealism);
        assert!(preset.metadata.is_some());
    }
}

#[tokio::test]
async fn test_reality_preset_application() {
    let engine = Arc::new(RwLock::new(RealityEngine::new()));

    // Create a preset at Level 5
    let preset = {
        let mut engine = engine.write().await;
        engine.set_level(RealityLevel::ProductionChaos);
        drop(engine);
        let engine = engine.read().await;
        engine.create_preset("chaos-preset".to_string(), None).await
    };

    // Reset to Level 1
    {
        let engine = engine.read().await;
        engine.set_level(RealityLevel::StaticStubs).await;
        drop(engine);

        let engine = engine.read().await;
        assert_eq!(engine.get_level().await, RealityLevel::StaticStubs);
    }

    // Apply preset
    {
        let engine = engine.read().await;
        engine.apply_preset(preset.clone()).await;
        drop(engine);

        let engine = engine.read().await;
        let config = engine.get_config().await;
        assert!(config.chaos.error_rate > 0.1);
        assert!(config.latency.base_ms > 100);
        assert!(config.mockai.enabled);
    }
}

#[tokio::test]
async fn test_reality_config_default() {
    let config = RealityConfig::default();

    // Default should be Level 3 characteristics
    assert!(config.chaos.error_rate > 0.0);
    assert!(config.latency.base_ms > 0);
    assert!(config.mockai.enabled);
}

#[tokio::test]
async fn test_reality_preset_metadata() {
    let metadata = PresetMetadata {
        created_at: Some(chrono::Utc::now().to_rfc3339()),
        author: Some("test-author".to_string()),
        tags: vec!["test".to_string()],
        version: Some("1.0".to_string()),
    };

    assert_eq!(metadata.author, Some("test-author".to_string()));
    assert_eq!(metadata.version, Some("1.0".to_string()));
    assert!(!metadata.tags.is_empty());
}

#[tokio::test]
async fn test_reality_level_names() {
    assert_eq!(RealityLevel::StaticStubs.name(), "Static Stubs");
    assert_eq!(RealityLevel::LightSimulation.name(), "Light Simulation");
    assert_eq!(RealityLevel::ModerateRealism.name(), "Moderate Realism");
    assert_eq!(RealityLevel::HighRealism.name(), "High Realism");
    assert_eq!(RealityLevel::ProductionChaos.name(), "Production Chaos");
}

#[tokio::test]
async fn test_reality_level_descriptions() {
    // All levels should have non-empty descriptions
    for level_value in 1..=5 {
        let level = RealityLevel::from_value(level_value).unwrap();
        assert!(!level.description().is_empty());
    }
}

#[tokio::test]
async fn test_reality_engine_concurrent_access() {
    let engine = Arc::new(RwLock::new(RealityEngine::new()));

    // Test concurrent reads
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let engine = engine.clone();
            tokio::spawn(async move {
                let engine = engine.read().await;
                let _config = engine.get_config().await;
            })
        })
        .collect();

    futures::future::join_all(handles).await;

    // Test concurrent writes (should not panic)
    let handles: Vec<_> = (1..=5)
        .map(|level_value| {
            let engine = engine.clone();
            tokio::spawn(async move {
                let level = RealityLevel::from_value(level_value).unwrap();
                let engine = engine.read().await;
                engine.set_level(level).await;
            })
        })
        .collect();

    futures::future::join_all(handles).await;
}

#[tokio::test]
async fn test_reality_config_chaos_settings() {
    let engine = Arc::new(RwLock::new(RealityEngine::new()));

    // Level 1 should have no chaos
    {
        let engine = engine.read().await;
        engine.set_level(RealityLevel::StaticStubs).await;
        drop(engine);

        let engine = engine.read().await;
        let config = engine.get_config().await;
        assert_eq!(config.chaos.error_rate, 0.0);
        assert_eq!(config.chaos.delay_rate, 0.0);
    }

    // Level 5 should have high chaos
    {
        let engine = engine.read().await;
        engine.set_level(RealityLevel::ProductionChaos).await;
        drop(engine);

        let engine = engine.read().await;
        let config = engine.get_config().await;
        assert!(config.chaos.error_rate > 0.1);
        assert!(config.chaos.delay_rate > 0.2);
    }
}

#[tokio::test]
async fn test_reality_config_latency_settings() {
    let engine = Arc::new(RwLock::new(RealityEngine::new()));

    // Level 1 should have no latency
    {
        let engine = engine.read().await;
        engine.set_level(RealityLevel::StaticStubs).await;
        drop(engine);

        let engine = engine.read().await;
        let config = engine.get_config().await;
        assert_eq!(config.latency.base_ms, 0);
        assert_eq!(config.latency.jitter_ms, 0);
    }

    // Level 5 should have high latency
    {
        let engine = engine.read().await;
        engine.set_level(RealityLevel::ProductionChaos).await;
        drop(engine);

        let engine = engine.read().await;
        let config = engine.get_config().await;
        assert!(config.latency.base_ms > 100);
        assert!(config.latency.jitter_ms > 0);
    }
}

#[tokio::test]
async fn test_reality_config_mockai_settings() {
    let engine = Arc::new(RwLock::new(RealityEngine::new()));

    // Level 1 should have MockAI disabled
    {
        let engine = engine.read().await;
        engine.set_level(RealityLevel::StaticStubs).await;
        drop(engine);

        let engine = engine.read().await;
        let config = engine.get_config().await;
        assert!(!config.mockai.enabled);
    }

    // Levels 2-5 should have MockAI enabled
    for level_value in 2..=5 {
        let level = RealityLevel::from_value(level_value).unwrap();
        let engine = engine.read().await;
        engine.set_level(level).await;
        drop(engine);

        let engine = engine.read().await;
        let config = engine.get_config().await;
        assert!(
            config.mockai.enabled,
            "Level {} should have MockAI enabled",
            level_value
        );
    }
}
