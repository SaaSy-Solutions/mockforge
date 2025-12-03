//! Risk Assessment System
//!
//! This module provides a comprehensive risk assessment framework for identifying,
//! analyzing, evaluating, and treating information security risks.

use crate::Error;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Risk category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskCategory {
    /// Technical risks (vulnerabilities, system failures, data breaches)
    Technical,
    /// Operational risks (process failures, human error, access control)
    Operational,
    /// Compliance risks (regulatory violations, audit findings)
    Compliance,
    /// Business risks (reputation, financial, operational impact)
    Business,
}

/// Likelihood scale (1-5)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Likelihood {
    /// Rare (unlikely to occur)
    Rare = 1,
    /// Unlikely (possible but not expected)
    Unlikely = 2,
    /// Possible (could occur)
    Possible = 3,
    /// Likely (expected to occur)
    Likely = 4,
    /// Almost Certain (very likely to occur)
    AlmostCertain = 5,
}

impl Likelihood {
    /// Get numeric value
    pub fn value(&self) -> u8 {
        *self as u8
    }
}

/// Impact scale (1-5)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Impact {
    /// Negligible (minimal impact)
    Negligible = 1,
    /// Low (minor impact)
    Low = 2,
    /// Medium (moderate impact)
    Medium = 3,
    /// High (significant impact)
    High = 4,
    /// Critical (severe impact)
    Critical = 5,
}

impl Impact {
    /// Get numeric value
    pub fn value(&self) -> u8 {
        *self as u8
    }
}

/// Risk level based on score
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    /// Low risk (1-5): Monitor and review
    Low,
    /// Medium risk (6-11): Action required
    Medium,
    /// High risk (12-19): Urgent action required
    High,
    /// Critical risk (20-25): Immediate action required
    Critical,
}

impl RiskLevel {
    /// Calculate risk level from score
    pub fn from_score(score: u8) -> Self {
        match score {
            1..=5 => RiskLevel::Low,
            6..=11 => RiskLevel::Medium,
            12..=19 => RiskLevel::High,
            20..=25 => RiskLevel::Critical,
            _ => RiskLevel::Low,
        }
    }
}

/// Risk treatment option
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TreatmentOption {
    /// Avoid: Eliminate risk by not performing activity
    Avoid,
    /// Mitigate: Reduce risk through controls
    Mitigate,
    /// Transfer: Transfer risk (insurance, contracts)
    Transfer,
    /// Accept: Accept risk with monitoring
    Accept,
}

/// Treatment status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreatmentStatus {
    /// Not started
    NotStarted,
    /// In progress
    InProgress,
    /// Completed
    Completed,
    /// On hold
    OnHold,
}

/// Risk review frequency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RiskReviewFrequency {
    /// Monthly reviews
    Monthly,
    /// Quarterly reviews
    Quarterly,
    /// Annual reviews
    Annually,
    /// Ad-hoc reviews
    AdHoc,
}

/// Risk entry in the risk register
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Risk {
    /// Risk ID (e.g., "RISK-001")
    pub risk_id: String,
    /// Risk title
    pub title: String,
    /// Risk description
    pub description: String,
    /// Risk category
    pub category: RiskCategory,
    /// Risk subcategory (optional)
    pub subcategory: Option<String>,
    /// Likelihood (1-5)
    pub likelihood: Likelihood,
    /// Impact (1-5)
    pub impact: Impact,
    /// Risk score (likelihood × impact, 1-25)
    pub risk_score: u8,
    /// Risk level
    pub risk_level: RiskLevel,
    /// Threat description
    pub threat: Option<String>,
    /// Vulnerability description
    pub vulnerability: Option<String>,
    /// Affected asset
    pub asset: Option<String>,
    /// Existing controls
    pub existing_controls: Vec<String>,
    /// Treatment option
    pub treatment_option: TreatmentOption,
    /// Treatment plan
    pub treatment_plan: Vec<String>,
    /// Treatment owner
    pub treatment_owner: Option<String>,
    /// Treatment deadline
    pub treatment_deadline: Option<DateTime<Utc>>,
    /// Treatment status
    pub treatment_status: TreatmentStatus,
    /// Residual likelihood (after treatment)
    pub residual_likelihood: Option<Likelihood>,
    /// Residual impact (after treatment)
    pub residual_impact: Option<Impact>,
    /// Residual risk score (after treatment)
    pub residual_risk_score: Option<u8>,
    /// Residual risk level (after treatment)
    pub residual_risk_level: Option<RiskLevel>,
    /// Last reviewed date
    pub last_reviewed: Option<DateTime<Utc>>,
    /// Next review date
    pub next_review: Option<DateTime<Utc>>,
    /// Review frequency
    pub review_frequency: RiskReviewFrequency,
    /// Compliance requirements
    pub compliance_requirements: Vec<String>,
    /// Created date
    pub created_at: DateTime<Utc>,
    /// Updated date
    pub updated_at: DateTime<Utc>,
    /// Created by user ID
    pub created_by: Uuid,
}

impl Risk {
    /// Create a new risk
    pub fn new(
        risk_id: String,
        title: String,
        description: String,
        category: RiskCategory,
        likelihood: Likelihood,
        impact: Impact,
        created_by: Uuid,
    ) -> Self {
        let risk_score = likelihood.value() * impact.value();
        let risk_level = RiskLevel::from_score(risk_score);

        Self {
            risk_id,
            title,
            description,
            category,
            subcategory: None,
            likelihood,
            impact,
            risk_score,
            risk_level,
            threat: None,
            vulnerability: None,
            asset: None,
            existing_controls: Vec::new(),
            treatment_option: TreatmentOption::Accept,
            treatment_plan: Vec::new(),
            treatment_owner: None,
            treatment_deadline: None,
            treatment_status: TreatmentStatus::NotStarted,
            residual_likelihood: None,
            residual_impact: None,
            residual_risk_score: None,
            residual_risk_level: None,
            last_reviewed: None,
            next_review: None,
            review_frequency: RiskReviewFrequency::Quarterly,
            compliance_requirements: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by,
        }
    }

    /// Recalculate risk score and level
    pub fn recalculate(&mut self) {
        self.risk_score = self.likelihood.value() * self.impact.value();
        self.risk_level = RiskLevel::from_score(self.risk_score);

        if let (Some(res_likelihood), Some(res_impact)) =
            (self.residual_likelihood, self.residual_impact)
        {
            self.residual_risk_score = Some(res_likelihood.value() * res_impact.value());
            self.residual_risk_level = self.residual_risk_score.map(RiskLevel::from_score);
        }
    }

    /// Calculate next review date based on frequency
    pub fn calculate_next_review(&mut self) {
        let now = Utc::now();
        let next = match self.review_frequency {
            RiskReviewFrequency::Monthly => now + chrono::Duration::days(30),
            RiskReviewFrequency::Quarterly => now + chrono::Duration::days(90),
            RiskReviewFrequency::Annually => now + chrono::Duration::days(365),
            RiskReviewFrequency::AdHoc => now + chrono::Duration::days(90), // Default to quarterly
        };
        self.next_review = Some(next);
    }
}

/// Risk register summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskSummary {
    /// Total risks
    pub total_risks: u32,
    /// Critical risks
    pub critical: u32,
    /// High risks
    pub high: u32,
    /// Medium risks
    pub medium: u32,
    /// Low risks
    pub low: u32,
    /// Risks by category
    pub by_category: HashMap<RiskCategory, u32>,
    /// Risks by treatment status
    pub by_treatment_status: HashMap<TreatmentStatus, u32>,
}

/// Risk assessment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RiskAssessmentConfig {
    /// Whether risk assessment is enabled
    pub enabled: bool,
    /// Default review frequency
    pub default_review_frequency: RiskReviewFrequency,
    /// Risk tolerance thresholds
    pub risk_tolerance: RiskTolerance,
}

/// Risk tolerance thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RiskTolerance {
    /// Maximum acceptable risk score
    pub max_acceptable_score: u8,
    /// Require treatment for risks above this score
    pub require_treatment_above: u8,
}

impl Default for RiskAssessmentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_review_frequency: RiskReviewFrequency::Quarterly,
            risk_tolerance: RiskTolerance {
                max_acceptable_score: 5,     // Low risks acceptable
                require_treatment_above: 11, // Medium and above require treatment
            },
        }
    }
}

/// Risk assessment engine
pub struct RiskAssessmentEngine {
    config: RiskAssessmentConfig,
    /// Risk register
    risks: std::sync::Arc<tokio::sync::RwLock<HashMap<String, Risk>>>,
    /// Risk ID counter
    risk_id_counter: std::sync::Arc<tokio::sync::RwLock<u64>>,
    /// Persistence path (optional)
    persistence_path: Option<std::path::PathBuf>,
}

impl RiskAssessmentEngine {
    /// Create a new risk assessment engine
    pub fn new(config: RiskAssessmentConfig) -> Self {
        Self {
            config,
            risks: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            risk_id_counter: std::sync::Arc::new(tokio::sync::RwLock::new(0)),
            persistence_path: None,
        }
    }

    /// Create a new risk assessment engine with persistence
    pub async fn with_persistence<P: AsRef<std::path::Path>>(
        config: RiskAssessmentConfig,
        persistence_path: P,
    ) -> Result<Self, Error> {
        let path = persistence_path.as_ref().to_path_buf();
        let mut engine = Self {
            config,
            risks: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            risk_id_counter: std::sync::Arc::new(tokio::sync::RwLock::new(0)),
            persistence_path: Some(path.clone()),
        };

        // Load existing risks
        engine.load_risks().await?;

        Ok(engine)
    }

    /// Load risks from persistence file
    async fn load_risks(&mut self) -> Result<(), Error> {
        let path = match &self.persistence_path {
            Some(p) => p,
            None => return Ok(()), // No persistence configured
        };

        if !path.exists() {
            return Ok(()); // No file yet, start fresh
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| Error::Generic(format!("Failed to read risk register: {}", e)))?;

        let risks: HashMap<String, Risk> = serde_json::from_str(&content)
            .map_err(|e| Error::Generic(format!("Failed to parse risk register: {}", e)))?;

        // Find max risk ID to set counter
        let max_id = risks
            .keys()
            .filter_map(|id| id.strip_prefix("RISK-").and_then(|num| num.parse::<u64>().ok()))
            .max()
            .unwrap_or(0);

        let mut risk_map = self.risks.write().await;
        *risk_map = risks;
        drop(risk_map);

        let mut counter = self.risk_id_counter.write().await;
        *counter = max_id;
        drop(counter);

        Ok(())
    }

    /// Save risks to persistence file
    async fn save_risks(&self) -> Result<(), Error> {
        let path = match &self.persistence_path {
            Some(p) => p,
            None => return Ok(()), // No persistence configured
        };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::Generic(format!("Failed to create directory: {}", e)))?;
        }

        let risks = self.risks.read().await;
        let content = serde_json::to_string_pretty(&*risks)
            .map_err(|e| Error::Generic(format!("Failed to serialize risk register: {}", e)))?;

        tokio::fs::write(path, content)
            .await
            .map_err(|e| Error::Generic(format!("Failed to write risk register: {}", e)))?;

        Ok(())
    }

    /// Generate next risk ID
    async fn generate_risk_id(&self) -> String {
        let mut counter = self.risk_id_counter.write().await;
        *counter += 1;
        format!("RISK-{:03}", *counter)
    }

    /// Create a new risk
    pub async fn create_risk(
        &self,
        title: String,
        description: String,
        category: RiskCategory,
        likelihood: Likelihood,
        impact: Impact,
        created_by: Uuid,
    ) -> Result<Risk, Error> {
        let risk_id = self.generate_risk_id().await;
        let mut risk = Risk::new(
            risk_id.clone(),
            title,
            description,
            category,
            likelihood,
            impact,
            created_by,
        );
        risk.review_frequency = self.config.default_review_frequency;
        risk.calculate_next_review();

        let mut risks = self.risks.write().await;
        risks.insert(risk_id, risk.clone());
        drop(risks);

        // Persist to disk
        self.save_risks().await?;

        Ok(risk)
    }

    /// Get risk by ID
    pub async fn get_risk(&self, risk_id: &str) -> Result<Option<Risk>, Error> {
        let risks = self.risks.read().await;
        Ok(risks.get(risk_id).cloned())
    }

    /// Get all risks
    pub async fn get_all_risks(&self) -> Result<Vec<Risk>, Error> {
        let risks = self.risks.read().await;
        Ok(risks.values().cloned().collect())
    }

    /// Get risks by level
    pub async fn get_risks_by_level(&self, level: RiskLevel) -> Result<Vec<Risk>, Error> {
        let risks = self.risks.read().await;
        Ok(risks.values().filter(|r| r.risk_level == level).cloned().collect())
    }

    /// Get risks by category
    pub async fn get_risks_by_category(&self, category: RiskCategory) -> Result<Vec<Risk>, Error> {
        let risks = self.risks.read().await;
        Ok(risks.values().filter(|r| r.category == category).cloned().collect())
    }

    /// Get risks by treatment status
    pub async fn get_risks_by_treatment_status(
        &self,
        status: TreatmentStatus,
    ) -> Result<Vec<Risk>, Error> {
        let risks = self.risks.read().await;
        Ok(risks.values().filter(|r| r.treatment_status == status).cloned().collect())
    }

    /// Update risk
    pub async fn update_risk(&self, risk_id: &str, mut risk: Risk) -> Result<(), Error> {
        risk.recalculate();
        risk.updated_at = Utc::now();

        let mut risks = self.risks.write().await;
        if risks.contains_key(risk_id) {
            risks.insert(risk_id.to_string(), risk);
            drop(risks);
            // Persist to disk
            self.save_risks().await?;
            Ok(())
        } else {
            Err(Error::Generic("Risk not found".to_string()))
        }
    }

    /// Update risk likelihood and impact
    pub async fn update_risk_assessment(
        &self,
        risk_id: &str,
        likelihood: Option<Likelihood>,
        impact: Option<Impact>,
    ) -> Result<(), Error> {
        let mut risks = self.risks.write().await;
        if let Some(risk) = risks.get_mut(risk_id) {
            if let Some(l) = likelihood {
                risk.likelihood = l;
            }
            if let Some(i) = impact {
                risk.impact = i;
            }
            risk.recalculate();
            risk.updated_at = Utc::now();
            drop(risks);
            // Persist to disk
            self.save_risks().await?;
            Ok(())
        } else {
            Err(Error::Generic("Risk not found".to_string()))
        }
    }

    /// Update treatment plan
    pub async fn update_treatment_plan(
        &self,
        risk_id: &str,
        treatment_option: TreatmentOption,
        treatment_plan: Vec<String>,
        treatment_owner: Option<String>,
        treatment_deadline: Option<DateTime<Utc>>,
    ) -> Result<(), Error> {
        let mut risks = self.risks.write().await;
        if let Some(risk) = risks.get_mut(risk_id) {
            risk.treatment_option = treatment_option;
            risk.treatment_plan = treatment_plan;
            risk.treatment_owner = treatment_owner;
            risk.treatment_deadline = treatment_deadline;
            risk.updated_at = Utc::now();
            drop(risks);
            // Persist to disk
            self.save_risks().await?;
            Ok(())
        } else {
            Err(Error::Generic("Risk not found".to_string()))
        }
    }

    /// Update treatment status
    pub async fn update_treatment_status(
        &self,
        risk_id: &str,
        status: TreatmentStatus,
    ) -> Result<(), Error> {
        let mut risks = self.risks.write().await;
        if let Some(risk) = risks.get_mut(risk_id) {
            risk.treatment_status = status;
            risk.updated_at = Utc::now();
            drop(risks);
            // Persist to disk
            self.save_risks().await?;
            Ok(())
        } else {
            Err(Error::Generic("Risk not found".to_string()))
        }
    }

    /// Set residual risk
    pub async fn set_residual_risk(
        &self,
        risk_id: &str,
        residual_likelihood: Likelihood,
        residual_impact: Impact,
    ) -> Result<(), Error> {
        let mut risks = self.risks.write().await;
        if let Some(risk) = risks.get_mut(risk_id) {
            risk.residual_likelihood = Some(residual_likelihood);
            risk.residual_impact = Some(residual_impact);
            risk.recalculate();
            risk.updated_at = Utc::now();
            drop(risks);
            // Persist to disk
            self.save_risks().await?;
            Ok(())
        } else {
            Err(Error::Generic("Risk not found".to_string()))
        }
    }

    /// Review risk
    pub async fn review_risk(&self, risk_id: &str, reviewed_by: Uuid) -> Result<(), Error> {
        let mut risks = self.risks.write().await;
        if let Some(risk) = risks.get_mut(risk_id) {
            risk.last_reviewed = Some(Utc::now());
            risk.calculate_next_review();
            risk.updated_at = Utc::now();
            let _ = reviewed_by; // TODO: Store reviewer
            drop(risks);
            // Persist to disk
            self.save_risks().await?;
            Ok(())
        } else {
            Err(Error::Generic("Risk not found".to_string()))
        }
    }

    /// Get risk summary
    pub async fn get_risk_summary(&self) -> Result<RiskSummary, Error> {
        let risks = self.risks.read().await;

        let mut summary = RiskSummary {
            total_risks: risks.len() as u32,
            critical: 0,
            high: 0,
            medium: 0,
            low: 0,
            by_category: HashMap::new(),
            by_treatment_status: HashMap::new(),
        };

        for risk in risks.values() {
            match risk.risk_level {
                RiskLevel::Critical => summary.critical += 1,
                RiskLevel::High => summary.high += 1,
                RiskLevel::Medium => summary.medium += 1,
                RiskLevel::Low => summary.low += 1,
            }

            *summary.by_category.entry(risk.category).or_insert(0) += 1;
            let count = summary.by_treatment_status.entry(risk.treatment_status).or_insert(0);
            *count += 1;
        }

        Ok(summary)
    }

    /// Get risks due for review
    pub async fn get_risks_due_for_review(&self) -> Result<Vec<Risk>, Error> {
        let risks = self.risks.read().await;
        let now = Utc::now();

        Ok(risks
            .values()
            .filter(|r| r.next_review.map(|next| next <= now).unwrap_or(false))
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_risk_creation() {
        let config = RiskAssessmentConfig::default();
        let engine = RiskAssessmentEngine::new(config);

        let risk = engine
            .create_risk(
                "Test Risk".to_string(),
                "Test description".to_string(),
                RiskCategory::Technical,
                Likelihood::Possible,
                Impact::High,
                Uuid::new_v4(),
            )
            .await
            .unwrap();

        assert_eq!(risk.risk_score, 12); // 3 * 4
        assert_eq!(risk.risk_level, RiskLevel::High);
    }

    #[test]
    fn test_risk_level_calculation() {
        assert_eq!(RiskLevel::from_score(3), RiskLevel::Low);
        assert_eq!(RiskLevel::from_score(9), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_score(15), RiskLevel::High);
        assert_eq!(RiskLevel::from_score(22), RiskLevel::Critical);
    }
}
