//! Lifecycle time manager for automatic lifecycle state updates
//!
//! This module provides integration between time travel and persona lifecycle states.
//! When virtual time advances, it automatically checks and updates persona lifecycle
//! states based on transition rules.

use crate::time_travel::{get_global_clock, VirtualClock};
use chrono::{DateTime, Utc};
#[cfg(feature = "data")]
use mockforge_data::persona_lifecycle::PersonaLifecycle;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Manager for updating persona lifecycles when time changes
///
/// This manager registers callbacks with the virtual clock to automatically
/// update persona lifecycle states when virtual time advances.
pub struct LifecycleTimeManager {
    /// Callback to update persona lifecycles
    /// Takes (old_time, new_time) and returns list of updated personas
    #[allow(clippy::type_complexity)]
    update_callback: Arc<dyn Fn(DateTime<Utc>, DateTime<Utc>) -> Vec<String> + Send + Sync>,
}

impl LifecycleTimeManager {
    /// Create a new lifecycle time manager
    ///
    /// # Arguments
    /// * `update_callback` - Function that updates persona lifecycles and returns list of updated persona IDs
    pub fn new<F>(update_callback: F) -> Self
    where
        F: Fn(DateTime<Utc>, DateTime<Utc>) -> Vec<String> + Send + Sync + 'static,
    {
        Self {
            update_callback: Arc::new(update_callback),
        }
    }

    /// Register with the global virtual clock
    ///
    /// This will automatically update persona lifecycles whenever time changes.
    pub fn register_with_clock(&self) {
        if let Some(clock) = get_global_clock() {
            self.register_with_clock_instance(&clock);
        } else {
            warn!("No global virtual clock found, lifecycle time manager not registered");
        }
    }

    /// Register with a specific virtual clock instance
    ///
    /// This allows registering with a clock that may not be in the global registry.
    pub fn register_with_clock_instance(&self, clock: &VirtualClock) {
        let callback = self.update_callback.clone();
        clock.on_time_change(move |old_time, new_time| {
            debug!("Time changed from {} to {}, updating persona lifecycles", old_time, new_time);
            let updated = callback(old_time, new_time);
            if !updated.is_empty() {
                info!("Updated {} persona lifecycle states: {:?}", updated.len(), updated);
            }
        });
        info!("LifecycleTimeManager registered with virtual clock");
    }
}

/// Check if a persona lifecycle should transition based on elapsed time
///
/// # Arguments
/// * `lifecycle` - The persona lifecycle to check
/// * `current_time` - The current virtual time
///
/// # Returns
/// `true` if the lifecycle state was updated, `false` otherwise
pub fn check_and_update_lifecycle_transitions(
    lifecycle: &mut PersonaLifecycle,
    current_time: DateTime<Utc>,
) -> bool {
    let old_state = lifecycle.current_state;
    let elapsed = current_time - lifecycle.state_entered_at;

    // Check each transition rule
    for rule in &lifecycle.transition_rules {
        // Check if enough time has passed
        if let Some(after_days) = rule.after_days {
            let required_duration = chrono::Duration::days(after_days as i64);
            if elapsed < required_duration {
                continue; // Not enough time has passed
            }
        }

        // Evaluate condition against persona metadata if present
        if let Some(condition) = &rule.condition {
            if !evaluate_lifecycle_condition(condition, &lifecycle.metadata) {
                debug!(
                    "Condition '{}' not met for persona {}, skipping transition",
                    condition, lifecycle.persona_id
                );
                continue;
            }
        }

        // Transition to the new state
        lifecycle.current_state = rule.to;
        lifecycle.state_entered_at = current_time;
        lifecycle.state_history.push((current_time, rule.to));

        info!(
            "Persona {} lifecycle transitioned: {:?} -> {:?}",
            lifecycle.persona_id, old_state, rule.to
        );

        return true; // State was updated
    }

    false // No transition occurred
}

/// Evaluate a lifecycle condition expression against persona metadata
///
/// Supports simple comparison expressions like:
/// - `"payment_failed_count > 2"`
/// - `"login_count >= 10"`
/// - `"subscription_tier == premium"`
/// - `"active == true"`
///
/// The left side is looked up as a key in the metadata map.
/// Numeric comparisons use f64; string comparisons use equality/inequality.
fn evaluate_lifecycle_condition(
    condition: &str,
    metadata: &std::collections::HashMap<String, serde_json::Value>,
) -> bool {
    let expr = condition.trim();

    // Handle literal true/false
    if expr.eq_ignore_ascii_case("true") {
        return true;
    }
    if expr.eq_ignore_ascii_case("false") {
        return false;
    }

    // Parse "variable operator value" expressions
    // Try two-character operators first (>=, <=, ==, !=), then single-character (>, <)
    let operators = [">=", "<=", "!=", "==", ">", "<"];
    let mut parts: Option<(&str, &str, &str)> = None;

    for op in &operators {
        if let Some(idx) = expr.find(op) {
            let var = expr[..idx].trim();
            let val = expr[idx + op.len()..].trim();
            if !var.is_empty() && !val.is_empty() {
                parts = Some((var, op, val));
                break;
            }
        }
    }

    let (variable, operator, threshold_str) = match parts {
        Some(p) => p,
        None => {
            debug!(expression = expr, "Unrecognized condition expression, defaulting to true");
            return true;
        }
    };

    // Look up the variable in metadata
    let meta_value = match metadata.get(variable) {
        Some(val) => val,
        None => {
            debug!(
                variable = variable,
                "Condition variable not found in persona metadata, defaulting to false"
            );
            return false;
        }
    };

    // Try numeric comparison first
    if let Some(actual_num) = meta_value.as_f64() {
        if let Ok(threshold_num) = threshold_str.parse::<f64>() {
            return match operator {
                ">" => actual_num > threshold_num,
                "<" => actual_num < threshold_num,
                ">=" => actual_num >= threshold_num,
                "<=" => actual_num <= threshold_num,
                "==" => (actual_num - threshold_num).abs() < f64::EPSILON,
                "!=" => (actual_num - threshold_num).abs() >= f64::EPSILON,
                _ => true,
            };
        }
    }

    // Fall back to string comparison
    let actual_str = match meta_value {
        serde_json::Value::String(s) => s.as_str(),
        serde_json::Value::Bool(b) => {
            if *b {
                "true"
            } else {
                "false"
            }
        }
        _ => {
            debug!(variable = variable, "Cannot compare non-string/non-numeric metadata value");
            return false;
        }
    };

    match operator {
        "==" => actual_str == threshold_str,
        "!=" => actual_str != threshold_str,
        _ => {
            debug!(operator = operator, "Operator not supported for string comparison");
            false
        }
    }
}
