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

    // Apply MockAI configuration
    config.mockai.enabled = reality_config.mockai.enabled;
    config.mockai.intelligent_behavior = reality_config.mockai.clone();
}
