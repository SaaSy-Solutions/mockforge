//! Shared authentication risk assessment data, without an engine dependency.

use serde::{Deserialize, Serialize};

/// Risk assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Overall risk score (0.0 - 1.0)
    pub risk_score: f64,
    /// Risk factors contributing to the score
    pub risk_factors: Vec<RiskFactor>,
    /// Recommended action based on risk
    pub recommended_action: RiskAction,
}

/// Individual risk factor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    /// Factor name
    pub name: String,
    /// Factor weight
    pub weight: f64,
    /// Factor value (0.0 - 1.0)
    pub value: f64,
    /// Contribution to overall risk score
    pub contribution: f64,
}

/// Risk-based action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskAction {
    /// Allow normal authentication
    Allow,
    /// Require device challenge
    DeviceChallenge,
    /// Require MFA
    RequireMfa,
    /// Block login
    Block,
}
