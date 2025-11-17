//! Lifecycle state response modifiers
//!
//! This module provides utilities for modifying response data based on persona lifecycle states.
//! It ensures that endpoints like billing and support reflect the current lifecycle state
//! of the persona.

use crate::persona_lifecycle::{LifecycleState, PersonaLifecycle};
use serde_json::Value;
use std::collections::HashMap;

/// Apply lifecycle state effects to a response for billing endpoints
///
/// Modifies billing-related fields based on the persona's lifecycle state.
/// For example, ChurnRisk personas might have payment issues, while
/// PaymentFailed personas have failed payment attempts.
pub fn apply_billing_lifecycle_effects(response: &mut Value, lifecycle: &PersonaLifecycle) {
    if let Some(obj) = response.as_object_mut() {
        match lifecycle.current_state {
            LifecycleState::NewSignup => {
                // New signups have no billing history
                obj.insert("payment_method".to_string(), Value::String("none".to_string()));
                obj.insert("billing_status".to_string(), Value::String("pending".to_string()));
                obj.insert("subscription_status".to_string(), Value::String("trial".to_string()));
            }
            LifecycleState::Active => {
                // Active users have normal billing
                obj.insert("billing_status".to_string(), Value::String("active".to_string()));
                obj.insert("subscription_status".to_string(), Value::String("active".to_string()));
                obj.insert("payment_method".to_string(), Value::String("credit_card".to_string()));
            }
            LifecycleState::PowerUser => {
                // Power users have premium billing
                obj.insert("billing_status".to_string(), Value::String("active".to_string()));
                obj.insert("subscription_status".to_string(), Value::String("premium".to_string()));
                obj.insert("payment_method".to_string(), Value::String("credit_card".to_string()));
            }
            LifecycleState::ChurnRisk => {
                // Churn risk users may have payment issues
                obj.insert("billing_status".to_string(), Value::String("warning".to_string()));
                obj.insert("subscription_status".to_string(), Value::String("at_risk".to_string()));
                obj.insert("payment_method".to_string(), Value::String("credit_card".to_string()));
                obj.insert("last_payment_failed".to_string(), Value::Bool(true));
            }
            LifecycleState::Churned => {
                // Churned users have cancelled billing
                obj.insert("billing_status".to_string(), Value::String("cancelled".to_string()));
                obj.insert(
                    "subscription_status".to_string(),
                    Value::String("cancelled".to_string()),
                );
                obj.insert("payment_method".to_string(), Value::String("none".to_string()));
            }
            LifecycleState::UpgradePending => {
                // Upgrade pending users have pending billing changes
                obj.insert(
                    "billing_status".to_string(),
                    Value::String("pending_upgrade".to_string()),
                );
                obj.insert(
                    "subscription_status".to_string(),
                    Value::String("upgrading".to_string()),
                );
            }
            LifecycleState::PaymentFailed => {
                // Payment failed users have failed payment attempts
                obj.insert("billing_status".to_string(), Value::String("failed".to_string()));
                obj.insert(
                    "subscription_status".to_string(),
                    Value::String("suspended".to_string()),
                );
                obj.insert("payment_method".to_string(), Value::String("credit_card".to_string()));
                obj.insert("last_payment_failed".to_string(), Value::Bool(true));
                obj.insert(
                    "failed_payment_count".to_string(),
                    Value::Number(
                        lifecycle
                            .get_metadata("payment_failed_count")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(1)
                            .into(),
                    ),
                );
            }
        }
    }
}

/// Apply lifecycle state effects to a response for support endpoints
///
/// Modifies support-related fields based on the persona's lifecycle state.
/// For example, ChurnRisk personas might have support tickets, while
/// PaymentFailed personas have billing-related support issues.
pub fn apply_support_lifecycle_effects(response: &mut Value, lifecycle: &PersonaLifecycle) {
    if let Some(obj) = response.as_object_mut() {
        match lifecycle.current_state {
            LifecycleState::NewSignup => {
                // New signups have onboarding support
                obj.insert("support_tier".to_string(), Value::String("onboarding".to_string()));
                obj.insert("open_tickets".to_string(), Value::Number(0.into()));
                obj.insert("priority".to_string(), Value::String("normal".to_string()));
            }
            LifecycleState::Active => {
                // Active users have normal support
                obj.insert("support_tier".to_string(), Value::String("standard".to_string()));
                obj.insert("open_tickets".to_string(), Value::Number(0.into()));
                obj.insert("priority".to_string(), Value::String("normal".to_string()));
            }
            LifecycleState::PowerUser => {
                // Power users have premium support
                obj.insert("support_tier".to_string(), Value::String("premium".to_string()));
                obj.insert("open_tickets".to_string(), Value::Number(0.into()));
                obj.insert("priority".to_string(), Value::String("high".to_string()));
            }
            LifecycleState::ChurnRisk => {
                // Churn risk users have retention support
                obj.insert("support_tier".to_string(), Value::String("retention".to_string()));
                obj.insert("open_tickets".to_string(), Value::Number(1.into()));
                obj.insert("priority".to_string(), Value::String("high".to_string()));
                obj.insert("last_contact_days_ago".to_string(), Value::Number(30.into()));
            }
            LifecycleState::Churned => {
                // Churned users have no active support
                obj.insert("support_tier".to_string(), Value::String("none".to_string()));
                obj.insert("open_tickets".to_string(), Value::Number(0.into()));
                obj.insert("priority".to_string(), Value::String("low".to_string()));
            }
            LifecycleState::UpgradePending => {
                // Upgrade pending users have sales support
                obj.insert("support_tier".to_string(), Value::String("sales".to_string()));
                obj.insert("open_tickets".to_string(), Value::Number(1.into()));
                obj.insert("priority".to_string(), Value::String("high".to_string()));
            }
            LifecycleState::PaymentFailed => {
                // Payment failed users have billing support
                obj.insert("support_tier".to_string(), Value::String("billing".to_string()));
                obj.insert("open_tickets".to_string(), Value::Number(1.into()));
                obj.insert("priority".to_string(), Value::String("urgent".to_string()));
                obj.insert("ticket_type".to_string(), Value::String("payment_issue".to_string()));
            }
        }
    }
}

/// Apply lifecycle state effects to a response based on endpoint type
///
/// This is a convenience function that routes to the appropriate lifecycle
/// effect function based on the endpoint path or type.
pub fn apply_lifecycle_effects(
    response: &mut Value,
    lifecycle: &PersonaLifecycle,
    endpoint_type: &str,
) {
    match endpoint_type {
        "billing" | "billing_status" | "payment" | "subscription" => {
            apply_billing_lifecycle_effects(response, lifecycle);
        }
        "support" | "support_tickets" | "tickets" | "help" => {
            apply_support_lifecycle_effects(response, lifecycle);
        }
        _ => {
            // For other endpoints, apply basic lifecycle traits
            if let Some(obj) = response.as_object_mut() {
                let effects = lifecycle.apply_lifecycle_effects();
                for (key, value) in effects {
                    obj.insert(key, Value::String(value));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persona_lifecycle::PersonaLifecycle;
    use serde_json::json;

    #[test]
    fn test_apply_billing_lifecycle_effects_new_signup() {
        let lifecycle = PersonaLifecycle::new("user123".to_string(), LifecycleState::NewSignup);
        let mut response = json!({});
        apply_billing_lifecycle_effects(&mut response, &lifecycle);

        assert_eq!(response["billing_status"], "pending");
        assert_eq!(response["subscription_status"], "trial");
    }

    #[test]
    fn test_apply_billing_lifecycle_effects_payment_failed() {
        let lifecycle = PersonaLifecycle::new("user123".to_string(), LifecycleState::PaymentFailed);
        let mut response = json!({});
        apply_billing_lifecycle_effects(&mut response, &lifecycle);

        assert_eq!(response["billing_status"], "failed");
        assert_eq!(response["subscription_status"], "suspended");
        assert_eq!(response["last_payment_failed"], true);
    }

    #[test]
    fn test_apply_support_lifecycle_effects_churn_risk() {
        let lifecycle = PersonaLifecycle::new("user123".to_string(), LifecycleState::ChurnRisk);
        let mut response = json!({});
        apply_support_lifecycle_effects(&mut response, &lifecycle);

        assert_eq!(response["support_tier"], "retention");
        assert_eq!(response["open_tickets"], 1);
        assert_eq!(response["priority"], "high");
    }

    #[test]
    fn test_apply_lifecycle_effects_routing() {
        let lifecycle = PersonaLifecycle::new("user123".to_string(), LifecycleState::Active);
        let mut billing_response = json!({});
        apply_lifecycle_effects(&mut billing_response, &lifecycle, "billing");
        assert_eq!(billing_response["billing_status"], "active");

        let mut support_response = json!({});
        apply_lifecycle_effects(&mut support_response, &lifecycle, "support");
        assert_eq!(support_response["support_tier"], "standard");
    }
}
