//! End-to-end test that the Reality Slider (#672) actually propagates a level
//! change through to the chaos / latency / MockAI subsystems — not just to
//! the in-engine `RealityConfig` cache.
//!
//! The audit that filed #672 read REALITY_SLIDER_HOT_RELOAD_PLAN.md and
//! concluded Phase 4 (subsystem propagation) was unimplemented. In fact it
//! lives in `crates/mockforge-ui/src/handlers.rs::set_reality_level` and
//! has lived there since the slider shipped. This test pins the behaviour
//! so the auditor's reading is true going forward: changing the level
//! mutates the subsystem Arcs the middleware actually reads from.

use axum::{extract::State, Json};
use mockforge_chaos::api::create_chaos_api_router;
use mockforge_chaos::config::ChaosConfig;
use mockforge_core::intelligent_behavior::{config::IntelligentBehaviorConfig, MockAI};
use mockforge_foundation::latency::{FaultConfig, LatencyInjector, LatencyProfile};
use mockforge_ui::handlers::{set_reality_level, AdminState, SetRealityLevelRequest};
use std::sync::Arc;
use tokio::sync::RwLock;

struct Harness {
    state: AdminState,
    chaos_api_state: Arc<mockforge_chaos::api::ChaosApiState>,
    latency: Arc<RwLock<LatencyInjector>>,
    mockai: Arc<RwLock<MockAI>>,
}

fn make_state_with_subsystems() -> Harness {
    let (_router, _config, _latency_tracker, chaos_api_state) =
        create_chaos_api_router(ChaosConfig::default(), None);

    // Latency injector starts effectively quiet so we can observe the change.
    let latency = Arc::new(RwLock::new(LatencyInjector::new(
        LatencyProfile::new(0, 0),
        FaultConfig::default(),
    )));

    let mockai = Arc::new(RwLock::new(MockAI::new(IntelligentBehaviorConfig::default())));

    let state = AdminState::new(
        Some("127.0.0.1:3000".parse().unwrap()),
        Some("127.0.0.1:3001".parse().unwrap()),
        Some("127.0.0.1:50051".parse().unwrap()),
        None,
        true,
        9080,
        Some(chaos_api_state.clone()),
        Some(latency.clone()),
        Some(mockai.clone()),
        None,
        None,
        None,
        None,
        None,
    );

    Harness {
        state,
        chaos_api_state,
        latency,
        mockai,
    }
}

#[tokio::test]
async fn setting_level_5_enables_chaos_subsystem() {
    let h = make_state_with_subsystems();
    assert!(
        !h.chaos_api_state.config.read().await.enabled,
        "preconditions: chaos starts disabled"
    );

    let resp = set_reality_level(State(h.state), Json(SetRealityLevelRequest { level: 5 })).await;
    assert!(resp.0.success, "set_reality_level should succeed: {:?}", resp.0.error);

    // Phase 4 propagation: chaos config should have been swapped to the
    // level-5 (Production Chaos) profile, which has fault_injection populated.
    let chaos_config = h.chaos_api_state.config.read().await;
    assert!(chaos_config.enabled, "level 5 must enable chaos");
    assert!(chaos_config.fault_injection.is_some(), "level 5 must populate fault_injection");
    let fi = chaos_config.fault_injection.as_ref().unwrap();
    assert!(fi.enabled);
    assert!(
        fi.http_error_probability > 0.0,
        "level 5 should set a non-zero error probability, got {}",
        fi.http_error_probability
    );
}

#[tokio::test]
async fn setting_level_5_enables_mockai_subsystem() {
    let h = make_state_with_subsystems();
    assert!(
        !h.mockai.read().await.get_config().enabled,
        "preconditions: MockAI starts disabled"
    );

    let resp = set_reality_level(State(h.state), Json(SetRealityLevelRequest { level: 5 })).await;
    assert!(resp.0.success);

    // Phase 4 propagation: MockAI::update_config_async should have been called.
    assert!(
        h.mockai.read().await.get_config().enabled,
        "level 5 must hot-reload MockAI to enabled"
    );
}

#[tokio::test]
async fn setting_level_5_swaps_latency_profile() {
    let h = make_state_with_subsystems();
    {
        let guard = h.latency.read().await;
        assert_eq!(guard.profile().base_ms, 0, "preconditions: latency starts at 0ms");
    }

    let resp = set_reality_level(State(h.state), Json(SetRealityLevelRequest { level: 5 })).await;
    assert!(resp.0.success);

    let guard = h.latency.read().await;
    let base_ms = guard.profile().base_ms;
    drop(guard);
    // Level 5 = Production Chaos which uses a Pareto-tailed 200–2000ms profile.
    assert!(
        base_ms >= 200,
        "level 5 must hot-reload latency to >=200ms base, got {}",
        base_ms
    );
}

#[tokio::test]
async fn setting_level_1_disables_chaos_and_mockai() {
    let h = make_state_with_subsystems();

    // First go to 5 so we have something to roll back from.
    let resp =
        set_reality_level(State(h.state.clone()), Json(SetRealityLevelRequest { level: 5 })).await;
    assert!(resp.0.success);
    assert!(h.chaos_api_state.config.read().await.enabled);

    // Now drop to 1 → everything should quiesce.
    let resp = set_reality_level(State(h.state), Json(SetRealityLevelRequest { level: 1 })).await;
    assert!(resp.0.success);

    let chaos_config = h.chaos_api_state.config.read().await;
    assert!(!chaos_config.enabled, "level 1 must disable chaos");
    assert!(!h.mockai.read().await.get_config().enabled, "level 1 must disable MockAI");
    {
        let guard = h.latency.read().await;
        assert_eq!(guard.profile().base_ms, 0, "level 1 must zero out latency base");
    }
}

#[tokio::test]
async fn invalid_level_returns_error() {
    let h = make_state_with_subsystems();
    let resp = set_reality_level(State(h.state), Json(SetRealityLevelRequest { level: 99 })).await;
    assert!(!resp.0.success, "level 99 must be rejected");
}
