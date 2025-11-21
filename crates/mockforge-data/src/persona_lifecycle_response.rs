//! Lifecycle state response modifiers
//!
//! This module provides utilities for modifying response data based on persona lifecycle states.
//! It ensures that endpoints like billing and support reflect the current lifecycle state
//! of the persona.

use crate::persona_lifecycle::{LifecycleState, PersonaLifecycle};
use serde_json::Value;

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

/// Apply lifecycle state effects for user engagement endpoints
///
/// Modifies user profile, activity, and notification fields based on the persona's lifecycle state.
/// Affects endpoints like:
/// - User profile endpoints (status, engagement_level, last_active)
/// - Activity endpoints (recent_activity, engagement_score)
/// - Notification endpoints (notification_preferences, engagement_alerts)
pub fn apply_user_engagement_lifecycle_effects(response: &mut Value, lifecycle: &PersonaLifecycle) {
    if let Some(obj) = response.as_object_mut() {
        match lifecycle.current_state {
            LifecycleState::NewSignup => {
                // New users have minimal engagement
                obj.insert("status".to_string(), Value::String("new".to_string()));
                obj.insert("engagement_level".to_string(), Value::String("low".to_string()));
                obj.insert("last_active".to_string(), Value::String("recent".to_string()));
                obj.insert("engagement_score".to_string(), Value::Number(10.into()));
                obj.insert("recent_activity".to_string(), Value::Array(vec![]));
                obj.insert("notification_preferences".to_string(), Value::Object(serde_json::Map::new()));
                obj.insert("engagement_alerts".to_string(), Value::Bool(false));
            }
            LifecycleState::Active => {
                // Active users have good engagement
                obj.insert("status".to_string(), Value::String("active".to_string()));
                obj.insert("engagement_level".to_string(), Value::String("medium".to_string()));
                obj.insert("last_active".to_string(), Value::String("recent".to_string()));
                obj.insert("engagement_score".to_string(), Value::Number(75.into()));
                obj.insert("recent_activity".to_string(), Value::Array(vec![
                    serde_json::json!({"type": "login", "timestamp": "recent"}),
                    serde_json::json!({"type": "action", "timestamp": "recent"}),
                ]));
                obj.insert("notification_preferences".to_string(), Value::Object({
                    let mut prefs = serde_json::Map::new();
                    prefs.insert("email".to_string(), Value::Bool(true));
                    prefs.insert("push".to_string(), Value::Bool(true));
                    prefs
                }));
                obj.insert("engagement_alerts".to_string(), Value::Bool(false));
            }
            LifecycleState::ChurnRisk => {
                // Churn risk users have declining engagement
                obj.insert("status".to_string(), Value::String("at_risk".to_string()));
                obj.insert("engagement_level".to_string(), Value::String("low".to_string()));
                obj.insert("last_active".to_string(), Value::String("30_days_ago".to_string()));
                obj.insert("engagement_score".to_string(), Value::Number(25.into()));
                obj.insert("recent_activity".to_string(), Value::Array(vec![]));
                obj.insert("notification_preferences".to_string(), Value::Object({
                    let mut prefs = serde_json::Map::new();
                    prefs.insert("email".to_string(), Value::Bool(true));
                    prefs.insert("push".to_string(), Value::Bool(false));
                    prefs
                }));
                obj.insert("engagement_alerts".to_string(), Value::Bool(true));
                obj.insert("churn_risk_reason".to_string(), Value::String("inactivity".to_string()));
            }
            LifecycleState::Churned => {
                // Churned users have no engagement
                obj.insert("status".to_string(), Value::String("churned".to_string()));
                obj.insert("engagement_level".to_string(), Value::String("none".to_string()));
                obj.insert("last_active".to_string(), Value::String("90_days_ago".to_string()));
                obj.insert("engagement_score".to_string(), Value::Number(0.into()));
                obj.insert("recent_activity".to_string(), Value::Array(vec![]));
                obj.insert("notification_preferences".to_string(), Value::Object(serde_json::Map::new()));
                obj.insert("engagement_alerts".to_string(), Value::Bool(false));
                obj.insert("churned_at".to_string(), Value::String("recent".to_string()));
            }
            _ => {
                // For other states, use default active behavior
                obj.insert("status".to_string(), Value::String("active".to_string()));
                obj.insert("engagement_level".to_string(), Value::String("medium".to_string()));
            }
        }
    }
}

/// Apply lifecycle state effects for order fulfillment endpoints
///
/// Modifies order-related fields based on the persona's lifecycle state.
/// Maps lifecycle states to order fulfillment states:
/// - NewSignup -> PENDING
/// - Active -> PROCESSING
/// - PowerUser -> SHIPPED
/// - UpgradePending -> DELIVERED
/// - Churned -> COMPLETED
pub fn apply_order_fulfillment_lifecycle_effects(
    response: &mut Value,
    lifecycle: &PersonaLifecycle,
) {
    if let Some(obj) = response.as_object_mut() {
        match lifecycle.current_state {
            LifecycleState::NewSignup => {
                obj.insert("status".to_string(), Value::String("pending".to_string()));
                obj.insert("estimated_delivery".to_string(), Value::Null);
            }
            LifecycleState::Active => {
                obj.insert("status".to_string(), Value::String("processing".to_string()));
                obj.insert("estimated_delivery".to_string(), Value::Null);
            }
            LifecycleState::PowerUser => {
                obj.insert("status".to_string(), Value::String("shipped".to_string()));
                obj.insert("tracking_number".to_string(), Value::String("TRACK123456".to_string()));
                obj.insert(
                    "shipped_at".to_string(),
                    Value::String(chrono::Utc::now().to_rfc3339()),
                );
            }
            LifecycleState::UpgradePending => {
                obj.insert("status".to_string(), Value::String("delivered".to_string()));
                obj.insert(
                    "delivered_at".to_string(),
                    Value::String(chrono::Utc::now().to_rfc3339()),
                );
            }
            LifecycleState::Churned => {
                obj.insert("status".to_string(), Value::String("completed".to_string()));
                obj.insert(
                    "completed_at".to_string(),
                    Value::String(chrono::Utc::now().to_rfc3339()),
                );
            }
            _ => {
                // Default to processing for other states
                obj.insert("status".to_string(), Value::String("processing".to_string()));
            }
        }
    }
}

/// Apply lifecycle state effects for loan endpoints
///
/// Modifies loan-related fields based on the persona's lifecycle state.
/// Maps lifecycle states to loan states:
/// - NewSignup -> APPLICATION
/// - Active -> APPROVED/ACTIVE
/// - PaymentFailed -> PAST_DUE
/// - Churned -> DEFAULTED
pub fn apply_loan_lifecycle_effects(response: &mut Value, lifecycle: &PersonaLifecycle) {
    if let Some(obj) = response.as_object_mut() {
        match lifecycle.current_state {
            LifecycleState::NewSignup => {
                obj.insert("status".to_string(), Value::String("application".to_string()));
                obj.insert("approved".to_string(), Value::Bool(false));
            }
            LifecycleState::Active => {
                obj.insert("status".to_string(), Value::String("active".to_string()));
                obj.insert("approved".to_string(), Value::Bool(true));
                obj.insert(
                    "approved_at".to_string(),
                    Value::String(chrono::Utc::now().to_rfc3339()),
                );
            }
            LifecycleState::PaymentFailed => {
                obj.insert("status".to_string(), Value::String("past_due".to_string()));
                obj.insert(
                    "days_past_due".to_string(),
                    Value::Number(
                        lifecycle
                            .get_metadata("payment_failed_count")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(1)
                            .into(),
                    ),
                );
            }
            LifecycleState::Churned => {
                obj.insert("status".to_string(), Value::String("defaulted".to_string()));
                obj.insert(
                    "defaulted_at".to_string(),
                    Value::String(chrono::Utc::now().to_rfc3339()),
                );
            }
            _ => {
                // Default to application for other states
                obj.insert("status".to_string(), Value::String("application".to_string()));
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
        "order" | "orders" | "fulfillment" | "shipment" | "delivery" => {
            apply_order_fulfillment_lifecycle_effects(response, lifecycle);
        }
        "loan" | "loans" | "credit" | "application" => {
            apply_loan_lifecycle_effects(response, lifecycle);
        }
        "profile" | "user" | "users" | "activity" | "engagement" | "notifications" | "notification" => {
            apply_user_engagement_lifecycle_effects(response, lifecycle);
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
