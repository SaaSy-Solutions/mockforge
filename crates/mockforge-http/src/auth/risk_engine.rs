//! Risk assessment engine for authentication
//!
//! This module provides risk-based authentication challenges including
//! MFA prompts, device challenges, and blocked login simulation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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

/// Risk engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskEngineConfig {
    /// MFA threshold (0.0 - 1.0)
    pub mfa_threshold: f64,
    /// Device challenge threshold (0.0 - 1.0)
    pub device_challenge_threshold: f64,
    /// Blocked login threshold (0.0 - 1.0)
    pub blocked_login_threshold: f64,
    /// Risk factor weights
    pub risk_factors: HashMap<String, f64>,
    /// Risk rules (conditions -> actions)
    pub risk_rules: Vec<RiskRule>,
}

/// Risk rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskRule {
    /// Condition (e.g., "risk_score > 0.9")
    pub condition: String,
    /// Action to take
    pub action: RiskAction,
}

impl Default for RiskEngineConfig {
    fn default() -> Self {
        let mut risk_factors = HashMap::new();
        risk_factors.insert("new_device".to_string(), 0.3);
        risk_factors.insert("unusual_location".to_string(), 0.4);
        risk_factors.insert("suspicious_activity".to_string(), 0.5);
        
        let mut risk_rules = Vec::new();
        risk_rules.push(RiskRule {
            condition: "risk_score > 0.9".to_string(),
            action: RiskAction::Block,
        });
        risk_rules.push(RiskRule {
            condition: "risk_score > 0.7".to_string(),
            action: RiskAction::RequireMfa,
        });
        risk_rules.push(RiskRule {
            condition: "risk_score > 0.5".to_string(),
            action: RiskAction::DeviceChallenge,
        });
        
        Self {
            mfa_threshold: 0.7,
            device_challenge_threshold: 0.5,
            blocked_login_threshold: 0.9,
            risk_factors,
            risk_rules,
        }
    }
}

/// Risk engine state
#[derive(Debug, Clone)]
pub struct RiskEngine {
    /// Configuration
    pub config: RiskEngineConfig,
    /// Simulated risk scores (user_id -> risk_score override)
    pub simulated_risks: Arc<RwLock<HashMap<String, Option<f64>>>>,
    /// Simulated risk factors (user_id -> risk_factors override)
    pub simulated_factors: Arc<RwLock<HashMap<String, HashMap<String, f64>>>>,
}

impl RiskEngine {
    /// Create new risk engine
    pub fn new(config: RiskEngineConfig) -> Self {
        Self {
            config,
            simulated_risks: Arc::new(RwLock::new(HashMap::new())),
            simulated_factors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Assess risk for an authentication request
    pub async fn assess_risk(
        &self,
        user_id: &str,
        risk_factors: &HashMap<String, f64>,
    ) -> RiskAssessment {
        // Check for simulated risk score override
        let simulated_risk = {
            let risks = self.simulated_risks.read().await;
            risks.get(user_id).copied().flatten()
        };
        
        if let Some(risk_score) = simulated_risk {
            return self.create_assessment_from_score(risk_score);
        }
        
        // Check for simulated risk factors override
        let factors_to_use = {
            let simulated = self.simulated_factors.read().await;
            if let Some(simulated_factors) = simulated.get(user_id) {
                simulated_factors.clone()
            } else {
                risk_factors.clone()
            }
        };
        
        // Calculate risk score from factors
        let mut risk_factors_vec = Vec::new();
        let mut total_score = 0.0;
        
        for (name, value) in factors_to_use {
            let weight = self.config.risk_factors.get(&name).copied().unwrap_or(0.0);
            let contribution = weight * value;
            total_score += contribution;
            
            risk_factors_vec.push(RiskFactor {
                name: name.clone(),
                weight,
                value,
                contribution,
            });
        }
        
        // Clamp score to 0.0 - 1.0
        let risk_score = total_score.min(1.0).max(0.0);
        
        // Determine recommended action
        let recommended_action = self.determine_action(risk_score);
        
        RiskAssessment {
            risk_score,
            risk_factors: risk_factors_vec,
            recommended_action,
        }
    }

    /// Create assessment from a risk score (for simulation)
    fn create_assessment_from_score(&self, risk_score: f64) -> RiskAssessment {
        let recommended_action = self.determine_action(risk_score);
        
        RiskAssessment {
            risk_score,
            risk_factors: vec![],
            recommended_action,
        }
    }

    /// Determine action based on risk score
    fn determine_action(&self, risk_score: f64) -> RiskAction {
        // Check risk rules first
        for rule in &self.config.risk_rules {
            if self.evaluate_condition(&rule.condition, risk_score) {
                return rule.action.clone();
            }
        }
        
        // Fallback to threshold-based logic
        if risk_score >= self.config.blocked_login_threshold {
            RiskAction::Block
        } else if risk_score >= self.config.mfa_threshold {
            RiskAction::RequireMfa
        } else if risk_score >= self.config.device_challenge_threshold {
            RiskAction::DeviceChallenge
        } else {
            RiskAction::Allow
        }
    }

    /// Evaluate a risk condition
    fn evaluate_condition(&self, condition: &str, risk_score: f64) -> bool {
        // Simple condition evaluation
        // In production, use a proper expression evaluator
        if condition.contains(">") {
            let parts: Vec<&str> = condition.split('>').collect();
            if parts.len() == 2 {
                if let Ok(threshold) = parts[1].trim().parse::<f64>() {
                    return risk_score > threshold;
                }
            }
        } else if condition.contains("<") {
            let parts: Vec<&str> = condition.split('<').collect();
            if parts.len() == 2 {
                if let Ok(threshold) = parts[1].trim().parse::<f64>() {
                    return risk_score < threshold;
                }
            }
        } else if condition.contains(">=") {
            let parts: Vec<&str> = condition.split(">=").collect();
            if parts.len() == 2 {
                if let Ok(threshold) = parts[1].trim().parse::<f64>() {
                    return risk_score >= threshold;
                }
            }
        } else if condition.contains("<=") {
            let parts: Vec<&str> = condition.split("<=").collect();
            if parts.len() == 2 {
                if let Ok(threshold) = parts[1].trim().parse::<f64>() {
                    return risk_score <= threshold;
                }
            }
        } else if condition.contains("==") {
            let parts: Vec<&str> = condition.split("==").collect();
            if parts.len() == 2 {
                if let Ok(threshold) = parts[1].trim().parse::<f64>() {
                    return (risk_score - threshold).abs() < 0.001;
                }
            }
        }
        
        false
    }

    /// Set simulated risk score for a user
    pub async fn set_simulated_risk(&self, user_id: String, risk_score: Option<f64>) {
        let mut risks = self.simulated_risks.write().await;
        if let Some(score) = risk_score {
            risks.insert(user_id, Some(score));
        } else {
            risks.remove(&user_id);
        }
    }

    /// Set simulated risk factors for a user
    pub async fn set_simulated_factors(&self, user_id: String, factors: HashMap<String, f64>) {
        let mut simulated = self.simulated_factors.write().await;
        simulated.insert(user_id, factors);
    }

    /// Clear simulated risk for a user
    pub async fn clear_simulated_risk(&self, user_id: &str) {
        let mut risks = self.simulated_risks.write().await;
        risks.remove(user_id);
        let mut factors = self.simulated_factors.write().await;
        factors.remove(user_id);
    }
}

impl Default for RiskEngine {
    fn default() -> Self {
        Self::new(RiskEngineConfig::default())
    }
}

