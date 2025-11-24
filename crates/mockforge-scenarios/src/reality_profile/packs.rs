//! Pre-tuned Reality Profile Packs
//!
//! This module provides pre-configured reality profile packs for common scenarios:
//! - E-commerce Peak Season Pack: High load, cart abandonment patterns
//! - Fintech Fraud Pack: Fraud detection triggers, transaction declines
//! - Healthcare HL7 Pack: HL7 message patterns, insurance edge cases
//! - IoT Device Fleet Chaos Pack: Device disconnections, message bursts

use crate::domain_pack::StudioChaosRule;
use crate::reality_profile::{
    DataMutationBehavior, ErrorDistribution, LatencyCurve, MutationCondition, MutationType,
    ProtocolBehavior,
};
use crate::reality_profile_pack::RealityProfilePackManifest;
use mockforge_core::latency::LatencyDistribution;
use serde_json::json;
use std::collections::HashMap;

/// Create E-commerce Peak Season reality profile pack
///
/// Simulates high load scenarios with:
/// - Increased latency during peak hours
/// - Cart abandonment patterns
/// - Inventory depletion behaviors
/// - Seasonal purchase patterns
pub fn create_ecommerce_peak_season_pack() -> RealityProfilePackManifest {
    let mut pack = RealityProfilePackManifest::new(
        "ecommerce-peak-season".to_string(),
        "1.0.0".to_string(),
        "E-commerce Peak Season".to_string(),
        "Reality profile for e-commerce peak season scenarios with high load, cart abandonment, and inventory depletion patterns".to_string(),
        "ecommerce".to_string(),
        "MockForge Team".to_string(),
    );

    pack.tags = vec![
        "ecommerce".to_string(),
        "peak-season".to_string(),
        "high-load".to_string(),
        "cart-abandonment".to_string(),
    ];

    // Latency curves - higher latency during peak hours
    pack.latency_curves.push(LatencyCurve {
        protocol: "rest".to_string(),
        distribution: LatencyDistribution::Normal,
        params: {
            let mut params = HashMap::new();
            params.insert("mean".to_string(), 300.0); // 300ms mean during peak
            params.insert("std_dev".to_string(), 100.0);
            params
        },
        base_ms: 300,
        endpoint_patterns: vec!["/api/checkout/*".to_string(), "/api/cart/*".to_string()],
        jitter_ms: 50,
        min_ms: 100,
        max_ms: Some(2000),
    });

    // Error distributions - increased errors under load
    pack.error_distributions.push(ErrorDistribution {
        endpoint_pattern: "/api/checkout/*".to_string(),
        error_codes: vec![500, 503, 429],
        probabilities: vec![0.05, 0.03, 0.02], // 5% 500, 3% 503, 2% 429
        pattern: Some(json!({"type": "random", "probability": 0.1})),
        conditions: Some(crate::reality_profile::ErrorCondition {
            load_threshold_rps: Some(100.0),
            latency_threshold_ms: Some(400),
            time_window: Some("peak_hours".to_string()),
            customer_segment: None,
        }),
    });

    // Data mutation behaviors - inventory depletion
    pack.data_mutation_behaviors.push(DataMutationBehavior {
        field_pattern: "body.quantity".to_string(),
        mutation_type: MutationType::Decrement,
        rate: 0.1, // 10% chance per request
        conditions: Some(MutationCondition {
            min_requests: Some(10),
            time_window: Some("peak_hours".to_string()),
            persona_trait: None,
        }),
        params: {
            let mut params = HashMap::new();
            params.insert("min_value".to_string(), json!(0));
            params.insert("decrement_by".to_string(), json!(1));
            params
        },
    });

    // Cart abandonment pattern - cart value decreases over time
    pack.data_mutation_behaviors.push(DataMutationBehavior {
        field_pattern: "body.cart_total".to_string(),
        mutation_type: MutationType::Decrement,
        rate: 0.05, // 5% chance per request
        conditions: Some(MutationCondition {
            min_requests: Some(5),
            time_window: Some("peak_hours".to_string()),
            persona_trait: Some("cart_abandonment_risk".to_string()),
        }),
        params: {
            let mut params = HashMap::new();
            params.insert("decrement_percent".to_string(), json!(0.1)); // 10% reduction
            params
        },
    });

    // Chaos rules - simulate load spikes
    pack.chaos_rules.push(StudioChaosRule {
        name: "peak-load-spike".to_string(),
        description: Some("Simulate load spikes during peak hours".to_string()),
        chaos_config: json!({
            "enabled": true,
            "latency": {
                "enabled": true,
                "fixed_delay_ms": 500,
                "probability": 0.3
            },
            "rate_limit": {
                "enabled": true,
                "requests_per_second": 50
            }
        }),
        duration_seconds: 3600, // 1 hour
        tags: vec!["peak-hours".to_string(), "load-spike".to_string()],
    });

    pack
}

/// Create Fintech Fraud reality profile pack
///
/// Simulates fraud detection scenarios with:
/// - Transaction decline patterns
/// - Risk score-based behaviors
/// - Multi-factor authentication triggers
/// - Suspicious activity patterns
pub fn create_fintech_fraud_pack() -> RealityProfilePackManifest {
    let mut pack = RealityProfilePackManifest::new(
        "fintech-fraud".to_string(),
        "1.0.0".to_string(),
        "Fintech Fraud Detection".to_string(),
        "Reality profile for fintech fraud detection scenarios with transaction declines, risk scoring, and suspicious activity patterns".to_string(),
        "fintech".to_string(),
        "MockForge Team".to_string(),
    );

    pack.tags = vec![
        "fintech".to_string(),
        "fraud".to_string(),
        "security".to_string(),
        "risk-detection".to_string(),
    ];

    // Error distributions - transaction declines
    pack.error_distributions.push(ErrorDistribution {
        endpoint_pattern: "/api/transactions/*".to_string(),
        error_codes: vec![402, 403, 429],
        probabilities: vec![0.03, 0.02, 0.01], // 3% payment required, 2% forbidden, 1% rate limit
        pattern: Some(json!({"type": "random", "probability": 0.06})),
        conditions: Some(crate::reality_profile::ErrorCondition {
            load_threshold_rps: None,
            latency_threshold_ms: None,
            time_window: None,
            customer_segment: Some("high_risk".to_string()),
        }),
    });

    // Latency curves - fraud checks add latency
    pack.latency_curves.push(LatencyCurve {
        protocol: "rest".to_string(),
        distribution: LatencyDistribution::Normal,
        params: {
            let mut params = HashMap::new();
            params.insert("mean".to_string(), 200.0);
            params.insert("std_dev".to_string(), 50.0);
            params
        },
        base_ms: 200,
        endpoint_patterns: vec!["/api/fraud-check/*".to_string()],
        jitter_ms: 30,
        min_ms: 100,
        max_ms: Some(1000),
    });

    // Data mutation behaviors - risk score increases
    pack.data_mutation_behaviors.push(DataMutationBehavior {
        field_pattern: "body.risk_score".to_string(),
        mutation_type: MutationType::Increment,
        rate: 0.15, // 15% chance per suspicious transaction
        conditions: Some(MutationCondition {
            min_requests: Some(3),
            time_window: None,
            persona_trait: Some("suspicious_activity".to_string()),
        }),
        params: {
            let mut params = HashMap::new();
            params.insert("increment_by".to_string(), json!(10));
            params.insert("max_value".to_string(), json!(100));
            params
        },
    });

    // Protocol behaviors - MFA required
    let mut mfa_behavior = HashMap::new();
    mfa_behavior.insert("require_mfa".to_string(), json!(true));
    mfa_behavior.insert("mfa_threshold".to_string(), json!(0.7));
    pack.protocol_behaviors.insert(
        "rest".to_string(),
        ProtocolBehavior {
            protocol: "rest".to_string(),
            behaviors: mfa_behavior,
            description: Some("Require MFA for high-risk transactions".to_string()),
        },
    );

    // Chaos rules - simulate fraud detection delays
    pack.chaos_rules.push(StudioChaosRule {
        name: "fraud-check-delay".to_string(),
        description: Some("Simulate fraud check processing delays".to_string()),
        chaos_config: json!({
            "enabled": true,
            "latency": {
                "enabled": true,
                "random_delay_range_ms": [100, 500],
                "probability": 0.2
            }
        }),
        duration_seconds: 0, // Infinite
        tags: vec!["fraud-check".to_string(), "security".to_string()],
    });

    pack
}

/// Create Healthcare HL7 reality profile pack
///
/// Simulates healthcare scenarios with:
/// - HL7 message patterns
/// - Insurance edge cases
/// - Patient data privacy behaviors
/// - Medical record access patterns
pub fn create_healthcare_hl7_pack() -> RealityProfilePackManifest {
    let mut pack = RealityProfilePackManifest::new(
        "healthcare-hl7".to_string(),
        "1.0.0".to_string(),
        "Healthcare HL7".to_string(),
        "Reality profile for healthcare HL7 message patterns, insurance edge cases, and patient data privacy behaviors".to_string(),
        "healthcare".to_string(),
        "MockForge Team".to_string(),
    );

    pack.tags = vec![
        "healthcare".to_string(),
        "hl7".to_string(),
        "insurance".to_string(),
        "hipaa".to_string(),
    ];

    // Latency curves - HL7 message processing
    pack.latency_curves.push(LatencyCurve {
        protocol: "rest".to_string(),
        distribution: LatencyDistribution::Normal,
        params: {
            let mut params = HashMap::new();
            params.insert("mean".to_string(), 150.0);
            params.insert("std_dev".to_string(), 40.0);
            params
        },
        base_ms: 150,
        endpoint_patterns: vec!["/api/hl7/*".to_string(), "/api/patient/*".to_string()],
        jitter_ms: 20,
        min_ms: 50,
        max_ms: Some(500),
    });

    // Error distributions - insurance edge cases
    pack.error_distributions.push(ErrorDistribution {
        endpoint_pattern: "/api/insurance/*".to_string(),
        error_codes: vec![400, 422, 409],
        probabilities: vec![0.05, 0.03, 0.02], // 5% bad request, 3% validation, 2% conflict
        pattern: Some(json!({"type": "random", "probability": 0.1})),
        conditions: Some(crate::reality_profile::ErrorCondition {
            load_threshold_rps: None,
            latency_threshold_ms: None,
            time_window: None,
            customer_segment: Some("insurance_edge_case".to_string()),
        }),
    });

    // Data mutation behaviors - patient record state transitions
    pack.data_mutation_behaviors.push(DataMutationBehavior {
        field_pattern: "body.record_status".to_string(),
        mutation_type: MutationType::StateTransition,
        rate: 0.1,
        conditions: Some(MutationCondition {
            min_requests: Some(1),
            time_window: None,
            persona_trait: None,
        }),
        params: {
            let mut params = HashMap::new();
            params.insert("states".to_string(), json!(["pending", "active", "archived"]));
            params.insert(
                "transitions".to_string(),
                json!({
                    "pending": [("active", 0.8), ("archived", 0.2)],
                    "active": [("archived", 0.1)]
                }),
            );
            params
        },
    });

    // Protocol behaviors - HL7 message format
    let mut hl7_behavior = HashMap::new();
    hl7_behavior.insert("message_format".to_string(), json!("hl7v2"));
    hl7_behavior.insert("encoding".to_string(), json!("er7"));
    hl7_behavior.insert("batch_mode".to_string(), json!(true));
    pack.protocol_behaviors.insert(
        "rest".to_string(),
        ProtocolBehavior {
            protocol: "rest".to_string(),
            behaviors: hl7_behavior,
            description: Some("HL7 v2 ER7 message format with batch support".to_string()),
        },
    );

    pack
}

/// Create IoT Device Fleet Chaos reality profile pack
///
/// Simulates IoT scenarios with:
/// - Device disconnections
/// - Message bursts
/// - Network instability
/// - MQTT-specific behaviors
pub fn create_iot_fleet_chaos_pack() -> RealityProfilePackManifest {
    let mut pack = RealityProfilePackManifest::new(
        "iot-fleet-chaos".to_string(),
        "1.0.0".to_string(),
        "IoT Device Fleet Chaos".to_string(),
        "Reality profile for IoT device fleet scenarios with disconnections, message bursts, and network instability".to_string(),
        "iot".to_string(),
        "MockForge Team".to_string(),
    );

    pack.tags = vec![
        "iot".to_string(),
        "mqtt".to_string(),
        "device-fleet".to_string(),
        "chaos".to_string(),
    ];

    // Latency curves - MQTT message delivery
    pack.latency_curves.push(LatencyCurve {
        protocol: "mqtt".to_string(),
        distribution: LatencyDistribution::Pareto,
        params: {
            let mut params = HashMap::new();
            params.insert("shape".to_string(), 1.5); // Heavy-tailed distribution
            params.insert("scale".to_string(), 50.0);
            params
        },
        base_ms: 50,
        endpoint_patterns: vec!["device/*/telemetry".to_string()],
        jitter_ms: 10,
        min_ms: 10,
        max_ms: Some(5000), // Can spike to 5 seconds
    });

    // Error distributions - device disconnections
    pack.error_distributions.push(ErrorDistribution {
        endpoint_pattern: "device/*/status".to_string(),
        error_codes: vec![503, 504],
        probabilities: vec![0.05, 0.03], // 5% service unavailable, 3% gateway timeout
        pattern: Some(json!({
            "type": "burst",
            "count": 5,
            "interval_ms": 10000
        })),
        conditions: Some(crate::reality_profile::ErrorCondition {
            load_threshold_rps: Some(1000.0), // High message rate
            latency_threshold_ms: None,
            time_window: None,
            customer_segment: None,
        }),
    });

    // Data mutation behaviors - device state transitions
    pack.data_mutation_behaviors.push(DataMutationBehavior {
        field_pattern: "body.device_status".to_string(),
        mutation_type: MutationType::StateTransition,
        rate: 0.2, // 20% chance per message
        conditions: Some(MutationCondition {
            min_requests: Some(1),
            time_window: None,
            persona_trait: Some("unstable_connection".to_string()),
        }),
        params: {
            let mut params = HashMap::new();
            params.insert("states".to_string(), json!(["online", "offline", "degraded"]));
            params.insert(
                "transitions".to_string(),
                json!({
                    "online": [("offline", 0.1), ("degraded", 0.05)],
                    "degraded": [("online", 0.3), ("offline", 0.2)],
                    "offline": [("online", 0.5), ("degraded", 0.1)]
                }),
            );
            params
        },
    });

    // Protocol behaviors - MQTT specific
    let mut mqtt_behavior = HashMap::new();
    mqtt_behavior.insert("qos_level".to_string(), json!(1));
    mqtt_behavior.insert("retain_messages".to_string(), json!(true));
    mqtt_behavior.insert("clean_session".to_string(), json!(false));
    mqtt_behavior.insert("keep_alive_secs".to_string(), json!(60));
    pack.protocol_behaviors.insert(
        "mqtt".to_string(),
        ProtocolBehavior {
            protocol: "mqtt".to_string(),
            behaviors: mqtt_behavior,
            description: Some(
                "MQTT QoS 1 with retained messages and persistent sessions".to_string(),
            ),
        },
    );

    // Chaos rules - device disconnection simulation
    pack.chaos_rules.push(StudioChaosRule {
        name: "device-disconnection-burst".to_string(),
        description: Some("Simulate device disconnection bursts".to_string()),
        chaos_config: json!({
            "enabled": true,
            "fault_injection": {
                "enabled": true,
                "connection_errors": true,
                "connection_error_probability": 0.1
            }
        }),
        duration_seconds: 0, // Infinite
        tags: vec!["device-disconnect".to_string(), "network-chaos".to_string()],
    });

    pack
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecommerce_pack_creation() {
        let pack = create_ecommerce_peak_season_pack();
        assert_eq!(pack.name, "ecommerce-peak-season");
        assert!(!pack.latency_curves.is_empty());
        assert!(!pack.error_distributions.is_empty());
    }

    #[test]
    fn test_fintech_pack_creation() {
        let pack = create_fintech_fraud_pack();
        assert_eq!(pack.name, "fintech-fraud");
        assert!(!pack.error_distributions.is_empty());
        assert!(pack.protocol_behaviors.contains_key("rest"));
    }

    #[test]
    fn test_healthcare_pack_creation() {
        let pack = create_healthcare_hl7_pack();
        assert_eq!(pack.name, "healthcare-hl7");
        assert!(!pack.data_mutation_behaviors.is_empty());
    }

    #[test]
    fn test_iot_pack_creation() {
        let pack = create_iot_fleet_chaos_pack();
        assert_eq!(pack.name, "iot-fleet-chaos");
        assert!(!pack.latency_curves.is_empty());
        assert!(pack.protocol_behaviors.contains_key("mqtt"));
    }

    #[test]
    fn test_pack_validation() {
        let pack = create_ecommerce_peak_season_pack();
        assert!(pack.validate().is_ok());
    }
}
