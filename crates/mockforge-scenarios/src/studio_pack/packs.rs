//! Pre-built studio packs
//!
//! This module provides pre-built studio packs for common use cases:
//! - Fintech Fraud Lab: Fraud detection and prevention scenarios
//! - E-commerce Peak Day: High-traffic e-commerce scenarios
//! - Healthcare Outage Drill: Healthcare system outage scenarios

use crate::domain_pack::DomainPackManifest;
use serde_json::json;

/// Create the Fintech Fraud Lab studio pack
///
/// This pack includes:
/// - Personas: fraudster, legitimate user, high-value customer
/// - Chaos rules: payment failures, latency spikes, rate limiting
/// - Contract diffs: breaking change scenarios for fraud detection
/// - Reality blends: mix of recorded and synthetic transaction data
pub fn create_fintech_fraud_lab_pack() -> DomainPackManifest {
    let mut pack = DomainPackManifest::new(
        "fintech-fraud-lab".to_string(),
        "1.0.0".to_string(),
        "Fintech Fraud Lab".to_string(),
        "Complete fraud detection and prevention testing environment with personas, chaos rules, and contract diffs".to_string(),
        "finance".to_string(),
        "MockForge Team".to_string(),
    );

    // Add personas
    pack.personas.push(crate::domain_pack::StudioPersona {
        id: "fraudster-1".to_string(),
        name: "Sophisticated Fraudster".to_string(),
        domain: "finance".to_string(),
        traits: [
            ("account_type".to_string(), "suspicious".to_string()),
            ("transaction_pattern".to_string(), "unusual".to_string()),
            ("risk_score".to_string(), "high".to_string()),
        ]
        .iter()
        .cloned()
        .collect(),
        backstory: Some(
            "A sophisticated fraudster attempting to bypass fraud detection systems".to_string(),
        ),
        relationships: std::collections::HashMap::new(),
        metadata: std::collections::HashMap::new(),
    });

    pack.personas.push(crate::domain_pack::StudioPersona {
        id: "legitimate-user-1".to_string(),
        name: "Legitimate User".to_string(),
        domain: "finance".to_string(),
        traits: [
            ("account_type".to_string(), "verified".to_string()),
            ("transaction_pattern".to_string(), "normal".to_string()),
            ("risk_score".to_string(), "low".to_string()),
        ]
        .iter()
        .cloned()
        .collect(),
        backstory: Some("A legitimate user with normal transaction patterns".to_string()),
        relationships: std::collections::HashMap::new(),
        metadata: std::collections::HashMap::new(),
    });

    // Add chaos rules
    pack.chaos_rules.push(crate::domain_pack::StudioChaosRule {
        name: "payment-failure-spike".to_string(),
        description: Some("Simulate payment processing failures".to_string()),
        chaos_config: json!({
            "enabled": true,
            "fault_injection": {
                "error_rate": 0.15,
                "status_codes": [500, 503, 502]
            },
            "latency": {
                "enabled": true,
                "min_ms": 2000,
                "max_ms": 5000
            }
        }),
        duration_seconds: 300,
        tags: vec!["payment".to_string(), "failure".to_string()],
    });

    pack.chaos_rules.push(crate::domain_pack::StudioChaosRule {
        name: "rate-limit-enforcement".to_string(),
        description: Some("Enforce rate limiting on fraud detection endpoints".to_string()),
        chaos_config: json!({
            "enabled": true,
            "fault_injection": {
                "error_rate": 0.05,
                "status_codes": [429]
            }
        }),
        duration_seconds: 0,
        tags: vec!["rate-limit".to_string(), "fraud-detection".to_string()],
    });

    // Add contract diffs
    pack.contract_diffs.push(crate::domain_pack::StudioContractDiff {
        name: "fraud-detection-breaking-change".to_string(),
        description: Some("Breaking change scenario for fraud detection API".to_string()),
        drift_budget: json!({
            "enabled": true,
            "default_budget": {
                "max_breaking_changes": 0,
                "max_non_breaking_changes": 5,
                "severity_threshold": "high",
                "enabled": true
            }
        }),
        endpoint_patterns: vec!["POST /api/v1/fraud/check".to_string()],
    });

    // Add reality blends
    pack.reality_blends.push(crate::domain_pack::StudioRealityBlend {
        name: "transaction-data-blend".to_string(),
        description: Some(
            "Mix recorded transaction data with synthetic fraud patterns".to_string(),
        ),
        reality_ratio: 0.7,
        continuum_config: json!({
            "enabled": true,
            "default_ratio": 0.7,
            "merge_strategy": "field_level"
        }),
        field_rules: vec![
            crate::domain_pack::FieldRealityRule {
                field_pattern: "body.transaction_id".to_string(),
                reality_source: "real".to_string(),
            },
            crate::domain_pack::FieldRealityRule {
                field_pattern: "body.fraud_indicators".to_string(),
                reality_source: "mock".to_string(),
            },
        ],
    });

    pack
}

/// Create the E-commerce Peak Day studio pack
///
/// This pack includes:
/// - Personas: high-volume buyer, first-time customer, returning customer
/// - Chaos rules: inventory shortages, checkout failures, cart abandonment
/// - Contract diffs: breaking changes for product catalog
/// - Reality blends: mix of recorded and synthetic product data
pub fn create_ecommerce_peak_day_pack() -> DomainPackManifest {
    let mut pack = DomainPackManifest::new(
        "ecommerce-peak-day".to_string(),
        "1.0.0".to_string(),
        "E-commerce Peak Day".to_string(),
        "High-traffic e-commerce scenarios for Black Friday, Cyber Monday, and peak shopping events".to_string(),
        "ecommerce".to_string(),
        "MockForge Team".to_string(),
    );

    // Add personas
    pack.personas.push(crate::domain_pack::StudioPersona {
        id: "high-volume-buyer-1".to_string(),
        name: "High Volume Buyer".to_string(),
        domain: "ecommerce".to_string(),
        traits: [
            ("purchase_frequency".to_string(), "high".to_string()),
            ("average_order_value".to_string(), "premium".to_string()),
            ("loyalty_status".to_string(), "vip".to_string()),
        ]
        .iter()
        .cloned()
        .collect(),
        backstory: Some("A high-value customer who makes frequent large purchases".to_string()),
        relationships: std::collections::HashMap::new(),
        metadata: std::collections::HashMap::new(),
    });

    // Add chaos rules
    pack.chaos_rules.push(crate::domain_pack::StudioChaosRule {
        name: "inventory-shortage".to_string(),
        description: Some("Simulate inventory shortages during peak traffic".to_string()),
        chaos_config: json!({
            "enabled": true,
            "fault_injection": {
                "error_rate": 0.10,
                "status_codes": [409]
            }
        }),
        duration_seconds: 3600,
        tags: vec!["inventory".to_string(), "peak-traffic".to_string()],
    });

    pack.chaos_rules.push(crate::domain_pack::StudioChaosRule {
        name: "checkout-latency".to_string(),
        description: Some("Simulate high latency during checkout process".to_string()),
        chaos_config: json!({
            "enabled": true,
            "latency": {
                "enabled": true,
                "min_ms": 3000,
                "max_ms": 8000
            }
        }),
        duration_seconds: 3600,
        tags: vec!["checkout".to_string(), "latency".to_string()],
    });

    // Add reality blends
    pack.reality_blends.push(crate::domain_pack::StudioRealityBlend {
        name: "product-catalog-blend".to_string(),
        description: Some("Mix real product data with synthetic inventory levels".to_string()),
        reality_ratio: 0.6,
        continuum_config: json!({
            "enabled": true,
            "default_ratio": 0.6,
            "merge_strategy": "field_level"
        }),
        field_rules: vec![
            crate::domain_pack::FieldRealityRule {
                field_pattern: "body.product_id".to_string(),
                reality_source: "real".to_string(),
            },
            crate::domain_pack::FieldRealityRule {
                field_pattern: "body.stock_quantity".to_string(),
                reality_source: "mock".to_string(),
            },
        ],
    });

    pack
}

/// Create the Healthcare Outage Drill studio pack
///
/// This pack includes:
/// - Personas: patient, doctor, administrator
/// - Chaos rules: system outages, data unavailability, emergency scenarios
/// - Contract diffs: breaking changes for patient data access
/// - Reality blends: mix of recorded and synthetic patient data (with PII redaction)
pub fn create_healthcare_outage_drill_pack() -> DomainPackManifest {
    let mut pack = DomainPackManifest::new(
        "healthcare-outage-drill".to_string(),
        "1.0.0".to_string(),
        "Healthcare Outage Drill".to_string(),
        "Healthcare system outage scenarios for disaster recovery and resilience testing"
            .to_string(),
        "healthcare".to_string(),
        "MockForge Team".to_string(),
    );

    // Add personas
    pack.personas.push(crate::domain_pack::StudioPersona {
        id: "patient-1".to_string(),
        name: "Emergency Patient".to_string(),
        domain: "healthcare".to_string(),
        traits: [
            ("patient_type".to_string(), "emergency".to_string()),
            ("priority_level".to_string(), "critical".to_string()),
        ]
        .iter()
        .cloned()
        .collect(),
        backstory: Some("A patient requiring emergency care during system outage".to_string()),
        relationships: std::collections::HashMap::new(),
        metadata: std::collections::HashMap::new(),
    });

    // Add chaos rules
    pack.chaos_rules.push(crate::domain_pack::StudioChaosRule {
        name: "system-outage".to_string(),
        description: Some("Simulate complete system outage".to_string()),
        chaos_config: json!({
            "enabled": true,
            "fault_injection": {
                "error_rate": 1.0,
                "status_codes": [503]
            }
        }),
        duration_seconds: 600,
        tags: vec!["outage".to_string(), "disaster-recovery".to_string()],
    });

    pack.chaos_rules.push(crate::domain_pack::StudioChaosRule {
        name: "partial-data-availability".to_string(),
        description: Some("Simulate partial data availability during outage".to_string()),
        chaos_config: json!({
            "enabled": true,
            "fault_injection": {
                "error_rate": 0.30,
                "status_codes": [503, 404]
            }
        }),
        duration_seconds: 1200,
        tags: vec![
            "partial-outage".to_string(),
            "data-availability".to_string(),
        ],
    });

    // Add contract diffs
    pack.contract_diffs.push(crate::domain_pack::StudioContractDiff {
        name: "patient-data-access-breaking-change".to_string(),
        description: Some("Breaking change scenario for patient data access API".to_string()),
        drift_budget: json!({
            "enabled": true,
            "default_budget": {
                "max_breaking_changes": 0,
                "max_non_breaking_changes": 3,
                "severity_threshold": "critical",
                "enabled": true
            }
        }),
        endpoint_patterns: vec!["GET /api/v1/patients/*".to_string()],
    });

    // Add reality blends with PII redaction
    pack.reality_blends.push(crate::domain_pack::StudioRealityBlend {
        name: "patient-data-blend".to_string(),
        description: Some(
            "Mix real patient data with synthetic PII for privacy compliance".to_string(),
        ),
        reality_ratio: 0.4,
        continuum_config: json!({
            "enabled": true,
            "default_ratio": 0.4,
            "merge_strategy": "field_level"
        }),
        field_rules: vec![
            crate::domain_pack::FieldRealityRule {
                field_pattern: "body.patient_id".to_string(),
                reality_source: "real".to_string(),
            },
            crate::domain_pack::FieldRealityRule {
                field_pattern: "body.name".to_string(),
                reality_source: "mock".to_string(),
            },
            crate::domain_pack::FieldRealityRule {
                field_pattern: "body.email".to_string(),
                reality_source: "mock".to_string(),
            },
            crate::domain_pack::FieldRealityRule {
                field_pattern: "body.medical_history".to_string(),
                reality_source: "real".to_string(),
            },
        ],
    });

    pack
}
