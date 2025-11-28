//! Tests for hot-reload functionality of Reality Slider subsystems
//!
//! These tests verify that LatencyInjector and MockAI can be updated
//! at runtime without requiring recreation.

use mockforge_core::intelligent_behavior::config::IntelligentBehaviorConfig;
use mockforge_core::intelligent_behavior::MockAI;
use mockforge_core::latency::{FaultConfig, LatencyInjector, LatencyProfile};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_latency_injector_update_profile() {
    // Create initial injector
    let mut injector = LatencyInjector::new(LatencyProfile::new(50, 20), FaultConfig::default());

    // Verify initial state
    assert!(injector.is_enabled());

    // Update profile
    let new_profile = LatencyProfile::new(100, 30);
    injector.update_profile(new_profile);

    // Verify injector still works
    assert!(injector.is_enabled());
}

#[tokio::test]
async fn test_latency_injector_update_profile_async() {
    // Create injector wrapped in Arc<RwLock>
    let injector = Arc::new(RwLock::new(LatencyInjector::new(
        LatencyProfile::new(50, 20),
        FaultConfig::default(),
    )));

    // Update profile using async method
    let new_profile = LatencyProfile::new(200, 50);
    LatencyInjector::update_profile_async(&injector, new_profile).await.unwrap();

    // Verify injector still works
    assert!(injector.read().await.is_enabled());
}

#[tokio::test]
async fn test_mockai_update_config() {
    // Create initial MockAI
    let mut mockai = MockAI::new(IntelligentBehaviorConfig::default());

    // Verify initial config
    let initial_config = mockai.get_config();
    assert_eq!(initial_config.enabled, false);

    // Create new config with enabled
    let mut new_config = IntelligentBehaviorConfig::default();
    new_config.enabled = true;
    new_config.behavior_model.llm_provider = "test".to_string();
    new_config.behavior_model.model = "test-model".to_string();

    // Update config
    mockai.update_config(new_config.clone());

    // Verify config was updated
    let updated_config = mockai.get_config();
    assert_eq!(updated_config.enabled, true);
    assert_eq!(updated_config.behavior_model.llm_provider, "test");
    assert_eq!(updated_config.behavior_model.model, "test-model");
}

#[tokio::test]
async fn test_mockai_update_config_async() {
    // Create MockAI wrapped in Arc<RwLock>
    let mockai = Arc::new(RwLock::new(MockAI::new(IntelligentBehaviorConfig::default())));

    // Create new config
    let mut new_config = IntelligentBehaviorConfig::default();
    new_config.enabled = true;
    new_config.behavior_model.llm_provider = "test".to_string();

    // Update config using async method
    MockAI::update_config_async(&mockai, new_config.clone()).await.unwrap();

    // Verify config was updated
    let mockai_guard = mockai.read().await;
    let updated_config = mockai_guard.get_config();
    assert_eq!(updated_config.enabled, true);
    assert_eq!(updated_config.behavior_model.llm_provider, "test");
}

#[tokio::test]
async fn test_mockai_preserves_rules_on_config_update() {
    // Create MockAI with some rules
    let mut mockai = MockAI::new(IntelligentBehaviorConfig::default());

    // Get initial rules (should be empty/default)
    let initial_rules = mockai.rules().clone();

    // Update config
    let mut new_config = IntelligentBehaviorConfig::default();
    new_config.enabled = true;
    mockai.update_config(new_config);

    // Verify rules are preserved
    let rules_after_update = mockai.rules();
    // Rules should be preserved - verify the method doesn't panic and returns a valid reference
    // Since BehaviorRules doesn't implement PartialEq, we can't directly compare,
    // but we can verify that rules still exist and are accessible after config update
    let initial_prompt = initial_rules.system_prompt.clone();
    let updated_prompt = rules_after_update.system_prompt.clone();
    // The rules should still be accessible (not dropped)
    assert_eq!(initial_prompt, updated_prompt);
}
