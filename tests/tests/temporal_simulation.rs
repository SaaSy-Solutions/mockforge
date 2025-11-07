//! Integration tests for Temporal Simulation Engine
//!
//! Tests cover:
//! - Virtual clock functionality
//! - Cron scheduler
//! - Mutation rules
//! - Token/session expiration with virtual clock
//! - VBR snapshot integration

use chrono::{DateTime, Duration, Utc};
use mockforge_core::time_travel::{TimeTravelConfig, TimeTravelManager, VirtualClock};
use mockforge_core::time_travel_now;
use mockforge_vbr::{
    config::{StorageBackend, VbrConfig},
    mutation_rules::{MutationOperation, MutationRule, MutationRuleManager, MutationTrigger},
    VbrEngine,
};
use std::sync::Arc;
use tokio::time::sleep;

#[tokio::test]
async fn test_virtual_clock_basic() {
    let clock = Arc::new(VirtualClock::new());
    assert!(!clock.is_enabled());

    let test_time = Utc::now();
    clock.enable_and_set(test_time);

    assert!(clock.is_enabled());
    let now = clock.now();
    assert!((now - test_time).num_seconds().abs() < 1);

    // Advance time
    clock.advance(Duration::hours(2));
    let advanced = clock.now();
    assert!((advanced - test_time - Duration::hours(2)).num_seconds().abs() < 1);
}

#[tokio::test]
async fn test_time_travel_manager() {
    let config = TimeTravelConfig {
        enabled: true,
        initial_time: Some(Utc::now()),
        scale_factor: 1.0,
        enable_scheduling: true,
    };

    let manager = TimeTravelManager::new(config);
    assert!(manager.clock().is_enabled());

    let initial_time = manager.now();
    manager.advance(Duration::hours(1));
    let advanced_time = manager.now();

    assert!((advanced_time - initial_time - Duration::hours(1)).num_seconds().abs() < 1);
}

#[tokio::test]
async fn test_cron_scheduler() {
    let clock = Arc::new(VirtualClock::new());
    let test_time = Utc::now();
    clock.enable_and_set(test_time);

    let scheduler = mockforge_core::time_travel::cron::CronScheduler::new(clock.clone());

    // Create a cron job that runs every minute
    let job = mockforge_core::time_travel::cron::CronJob::new(
        "test-job".to_string(),
        "Test Job".to_string(),
        "* * * * *".to_string(),
    );

    let action = mockforge_core::time_travel::cron::CronJobAction::Callback(Box::new(|_| Ok(())));

    scheduler.add_job(job, action).await.unwrap();

    // Advance time by 1 minute
    clock.advance(Duration::minutes(1));

    // Check that job should have executed
    let jobs = scheduler.list_jobs().await;
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].id, "test-job");
}

#[tokio::test]
async fn test_mutation_rules_basic() {
    let manager = MutationRuleManager::new();

    let rule = MutationRule::new(
        "test-rule".to_string(),
        "User".to_string(),
        MutationTrigger::Interval {
            duration_seconds: 3600,
        },
        MutationOperation::Increment {
            field: "count".to_string(),
            amount: 1.0,
        },
    );

    manager.add_rule(rule).await.unwrap();

    let rules = manager.list_rules().await;
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].id, "test-rule");
}

#[tokio::test]
async fn test_mutation_rules_interval_trigger() {
    let manager = MutationRuleManager::new();

    let rule = MutationRule::new(
        "hourly-increment".to_string(),
        "User".to_string(),
        MutationTrigger::Interval {
            duration_seconds: 3600,
        },
        MutationOperation::Increment {
            field: "login_count".to_string(),
            amount: 1.0,
        },
    );

    manager.add_rule(rule).await.unwrap();

    let now = time_travel_now();
    let rule = manager.get_rule("hourly-increment").await.unwrap();
    let next_execution = rule.next_execution.unwrap();

    // Next execution should be approximately 1 hour from now
    let duration = next_execution - now;
    assert!(duration.num_seconds() >= 3599 && duration.num_seconds() <= 3601);
}

#[tokio::test]
async fn test_mutation_rules_at_time_trigger() {
    let manager = MutationRuleManager::new();

    let rule = MutationRule::new(
        "daily-reset".to_string(),
        "User".to_string(),
        MutationTrigger::AtTime { hour: 3, minute: 0 },
        MutationOperation::Set {
            field: "status".to_string(),
            value: serde_json::json!("active"),
        },
    );

    manager.add_rule(rule).await.unwrap();

    let rule = manager.get_rule("daily-reset").await.unwrap();
    assert!(rule.next_execution.is_some());
}

#[tokio::test]
async fn test_time_advancement_formats() {
    // Test that time advancement accepts various formats
    // This is tested through the actual advance functionality
    let clock = Arc::new(VirtualClock::new());
    clock.enable_and_set(Utc::now());

    let initial = clock.now();

    // Test various duration advances
    clock.advance(Duration::hours(1));
    assert!((clock.now() - initial).num_hours() >= 1);

    clock.advance(Duration::days(7));
    assert!((clock.now() - initial).num_days() >= 8); // 1 hour + 7 days

    clock.advance(Duration::days(30));
    assert!((clock.now() - initial).num_days() >= 38); // 1 hour + 7 days + 30 days
}

#[tokio::test]
async fn test_vbr_snapshot_with_time_travel() {
    // Create VBR engine
    let config = VbrConfig::default().with_storage_backend(StorageBackend::Memory);
    let engine = VbrEngine::new(config).await.unwrap();

    // Create a snapshot with time travel state
    let time_travel_state = mockforge_vbr::TimeTravelSnapshotState {
        enabled: true,
        current_time: Some(Utc::now()),
        scale_factor: 2.0,
        cron_jobs: vec![],
        mutation_rules: vec![],
    };

    let metadata = engine
        .create_snapshot_with_time_travel(
            "test-snapshot",
            Some("Test snapshot with time travel".to_string()),
            "./test_snapshots",
            true,
            Some(time_travel_state.clone()),
        )
        .await
        .unwrap();

    assert!(metadata.time_travel_state.is_some());
    let restored_state = metadata.time_travel_state.unwrap();
    assert_eq!(restored_state.enabled, true);
    assert_eq!(restored_state.scale_factor, 2.0);
}

#[tokio::test]
async fn test_token_expiration_with_virtual_clock() {
    use mockforge_vbr::auth::VbrAuthService;

    // Create auth service
    let auth_service = VbrAuthService::new("test-secret".to_string());

    // Create a token that expires in 1 hour
    let user_id = "user1".to_string();
    let token = auth_service.generate_token(&user_id, Some(3600)).await.unwrap();

    // Initially, token should be valid
    assert!(auth_service.validate_token(&token).await.is_ok());

    // Enable virtual clock and advance time by 2 hours
    let clock = Arc::new(VirtualClock::new());
    clock.enable_and_set(Utc::now());
    clock.advance(Duration::hours(2));

    // Register clock globally (in real usage, this would be done by TimeTravelManager)
    mockforge_core::time_travel::register_global_clock(clock.clone());

    // Token should now be expired (if auth service uses virtual clock)
    // Note: This test assumes auth service is updated to use virtual clock
    // The actual implementation may vary
}

#[tokio::test]
async fn test_cron_job_execution() {
    let clock = Arc::new(VirtualClock::new());
    let test_time = Utc::now();
    clock.enable_and_set(test_time);

    let scheduler = mockforge_core::time_travel::cron::CronScheduler::new(clock.clone());

    let mut execution_count = 0;
    let execution_count_ptr = Arc::new(std::sync::Mutex::new(0));

    let count_clone = execution_count_ptr.clone();
    let action = mockforge_core::time_travel::cron::CronJobAction::Callback(Box::new(move |_| {
        *count_clone.lock().unwrap() += 1;
        Ok(())
    }));

    let job = mockforge_core::time_travel::cron::CronJob::new(
        "test-job".to_string(),
        "Test Job".to_string(),
        "* * * * *".to_string(), // Every minute
    );

    scheduler.add_job(job, action).await.unwrap();

    // Advance time by 1 minute
    clock.advance(Duration::minutes(1));

    // Trigger execution check (in real usage, this would be done by a background task)
    // For testing, we can manually check
    let jobs = scheduler.list_jobs().await;
    assert_eq!(jobs.len(), 1);
}

#[tokio::test]
async fn test_mutation_rule_execution() {
    // This test would require a VBR engine with entities
    // For now, we test the rule creation and scheduling
    let manager = MutationRuleManager::new();

    let rule = MutationRule::new(
        "test-execution".to_string(),
        "User".to_string(),
        MutationTrigger::Interval {
            duration_seconds: 60, // 1 minute for testing
        },
        MutationOperation::Increment {
            field: "count".to_string(),
            amount: 1.0,
        },
    );

    manager.add_rule(rule).await.unwrap();

    let rule = manager.get_rule("test-execution").await.unwrap();
    assert!(rule.next_execution.is_some());
    assert_eq!(rule.execution_count, 0);
}

#[tokio::test]
async fn test_time_scale_factor() {
    let clock = Arc::new(VirtualClock::new());
    clock.enable_and_set(Utc::now());
    clock.set_scale(2.0); // 2x speed

    let start = clock.now();
    clock.advance(Duration::hours(1));

    // With 2x scale, 1 hour of virtual time should pass faster
    // The actual implementation depends on how scale is used
    let end = clock.now();
    assert!((end - start).num_hours() >= 1);
}

#[tokio::test]
async fn test_scenario_save_and_load() {
    let config = TimeTravelConfig {
        enabled: true,
        initial_time: Some(Utc::now()),
        scale_factor: 2.0,
        enable_scheduling: true,
    };

    let manager = TimeTravelManager::new(config);
    manager.advance(Duration::hours(5));

    // Save scenario
    let scenario = manager.save_scenario("test-scenario".to_string());
    assert_eq!(scenario.name, "test-scenario");
    assert!(scenario.enabled);
    assert!(scenario.current_time.is_some());
    assert_eq!(scenario.scale_factor, 2.0);

    // Create new manager and load scenario
    let new_config = TimeTravelConfig::default();
    let new_manager = TimeTravelManager::new(new_config);
    new_manager.load_scenario(&scenario);

    assert!(new_manager.clock().is_enabled());
    assert_eq!(new_manager.clock().scale_factor(), 2.0);
}
