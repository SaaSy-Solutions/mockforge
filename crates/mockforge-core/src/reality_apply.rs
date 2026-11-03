//! Bridge from `mockforge_intelligence::reality::RealityEngine` to
//! `mockforge_core::config::ServerConfig`.
//!
//! This helper lives in core because it pokes at concrete `ServerConfig`
//! sub-structs (`observability.chaos`, `core.default_latency`, `mockai`). It
//! was an inherent method on `RealityEngine` before Issue #562 phase 6 moved
//! `reality` to `mockforge-intelligence`; keeping it in intelligence would
//! have forced an intelligence → core dep and re-introduced the cycle that
//! phase 1 broke.

use crate::config::ServerConfig;
use mockforge_intelligence::reality::RealityEngine;

/// Apply the current reality level's chaos / latency / mockai configuration
/// to a `ServerConfig`. Called by the CLI when initializing the server or
/// when the reality level changes.
pub async fn apply_reality_to_server_config(engine: &RealityEngine, config: &mut ServerConfig) {
    let reality_config = engine.get_config().await;

    if !config.reality.enabled {
        return;
    }

    // Apply chaos configuration
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
                (reality_config.latency.jitter_ms as f64 / reality_config.latency.base_ms as f64)
                    .min(1.0)
            } else {
                0.0
            };
        }
    }

    // Apply latency configuration
    config.core.default_latency = reality_config.latency.clone();
    config.core.latency_enabled = reality_config.latency.base_ms > 0;

    // Reality controls activation, not which provider/model the operator trusts.
    let behavior_model = config.mockai.intelligent_behavior.behavior_model.clone();

    // Apply MockAI configuration
    config.mockai.enabled = reality_config.mockai.enabled;
    config.mockai.intelligent_behavior = reality_config.mockai.clone();
    config.mockai.intelligent_behavior.behavior_model = behavior_model;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn reality_levels_preserve_the_selected_provider_model_and_seed() {
        for level in 1..=5 {
            let mut config = ServerConfig::default();
            config.mockai.intelligent_behavior.behavior_model.llm_provider = "ollama".into();
            config.mockai.intelligent_behavior.behavior_model.model = "local-fixture".into();
            config.mockai.intelligent_behavior.behavior_model.api_endpoint =
                Some("http://127.0.0.1:11434/api/chat".into());
            config.mockai.intelligent_behavior.behavior_model.seed = Some(42);
            let expected =
                serde_json::to_value(&config.mockai.intelligent_behavior.behavior_model).unwrap();
            let engine = RealityEngine::with_level(crate::RealityLevel::from_value(level).unwrap());
            let expected_enabled = engine.get_config().await.mockai.enabled;
            apply_reality_to_server_config(&engine, &mut config).await;
            assert_eq!(
                serde_json::to_value(&config.mockai.intelligent_behavior.behavior_model).unwrap(),
                expected
            );
            assert_eq!(config.mockai.enabled, expected_enabled);
        }
    }
    #[tokio::test]
    async fn disabled_reality_cannot_enable_runtime_ai() {
        let mut config = ServerConfig::default();
        config.reality.enabled = false;
        config.mockai.enabled = false;
        let before = serde_json::to_value(&config.mockai).unwrap();
        let engine = RealityEngine::with_level(crate::RealityLevel::from_value(5).unwrap());
        apply_reality_to_server_config(&engine, &mut config).await;
        assert_eq!(serde_json::to_value(&config.mockai).unwrap(), before);
        assert!(!config.mockai.enabled);
    }
}
