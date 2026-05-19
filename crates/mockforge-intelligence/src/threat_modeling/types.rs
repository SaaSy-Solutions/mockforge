//! Types for contract threat modeling
//!
//! Re-exported from `mockforge-foundation::threat_modeling_types` so consumers
//! can use these data types without depending on the deprecated
//! `contract_drift` module.

pub use mockforge_foundation::threat_modeling_types::{
    AggregationLevel, RemediationSuggestion, ThreatAssessment, ThreatCategory, ThreatFinding,
    ThreatLevel, ThreatModelingConfig,
};
