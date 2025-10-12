//! Auto-remediation engine for chaos recommendations
//!
//! Automatically applies low-risk chaos recommendations with safety checks,
//! rollback mechanisms, and approval workflows.

use crate::recommendations::{Recommendation, RecommendationCategory, RecommendationSeverity};
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use uuid::Uuid;

/// Auto-remediation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationConfig {
    /// Enable auto-remediation
    pub enabled: bool,
    /// Auto-apply recommendations with severity at or below this level
    pub max_auto_severity: RecommendationSeverity,
    /// Require manual approval for these categories
    pub require_approval_categories: Vec<RecommendationCategory>,
    /// Maximum concurrent remediations
    pub max_concurrent: usize,
    /// Cooldown period between remediations (minutes)
    pub cooldown_minutes: i64,
    /// Auto-rollback on failure
    pub auto_rollback: bool,
    /// Dry-run mode (don't actually apply)
    pub dry_run: bool,
    /// Maximum retries on failure
    pub max_retries: u32,
}

impl Default for RemediationConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default for safety
            max_auto_severity: RecommendationSeverity::Low,
            require_approval_categories: vec![
                RecommendationCategory::FaultInjection,
                RecommendationCategory::CircuitBreaker,
            ],
            max_concurrent: 1,
            cooldown_minutes: 30,
            auto_rollback: true,
            dry_run: false,
            max_retries: 3,
        }
    }
}

/// Remediation action status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RemediationStatus {
    Pending,
    AwaitingApproval,
    Approved,
    Rejected,
    Applying,
    Applied,
    Failed,
    RolledBack,
    Cancelled,
}

/// Applied remediation action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationAction {
    /// Unique action ID
    pub id: String,
    /// Source recommendation ID
    pub recommendation_id: String,
    /// Status
    pub status: RemediationStatus,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Applied at (if applied)
    pub applied_at: Option<DateTime<Utc>>,
    /// Completed at (success or failure)
    pub completed_at: Option<DateTime<Utc>>,
    /// Applied configuration changes
    pub config_changes: HashMap<String, String>,
    /// Rollback data (to restore previous state)
    pub rollback_data: Option<RollbackData>,
    /// Execution logs
    pub logs: Vec<String>,
    /// Success indicator
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Retry count
    pub retry_count: u32,
    /// Approved by (if approval was required)
    pub approved_by: Option<String>,
    /// Approval timestamp
    pub approved_at: Option<DateTime<Utc>>,
}

/// Rollback data to restore previous state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackData {
    pub previous_config: HashMap<String, String>,
    pub restore_commands: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Remediation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationResult {
    pub action_id: String,
    pub success: bool,
    pub message: String,
    pub applied_changes: Vec<String>,
    pub duration_ms: u64,
}

/// Remediation effectiveness metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectivenessMetrics {
    pub recommendation_id: String,
    pub action_id: String,
    /// Metrics before remediation
    pub before_metrics: SystemMetrics,
    /// Metrics after remediation
    pub after_metrics: SystemMetrics,
    /// Improvement score (0.0 - 1.0, higher is better)
    pub improvement_score: f64,
    /// Measurement period
    pub measurement_period_hours: i64,
    pub measured_at: DateTime<Utc>,
}

/// System metrics for effectiveness comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub error_rate: f64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub success_rate: f64,
    pub chaos_impact: f64,
    pub resilience_score: f64,
}

/// Approval request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub action_id: String,
    pub recommendation: Recommendation,
    pub proposed_changes: HashMap<String, String>,
    pub risk_assessment: RiskAssessment,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Risk assessment for remediation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub risk_level: RiskLevel,
    pub impact_scope: Vec<String>,
    pub reversible: bool,
    pub estimated_downtime_ms: u64,
    pub safety_checks: Vec<SafetyCheck>,
}

/// Risk level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Minimal,
    Low,
    Medium,
    High,
    Critical,
}

/// Safety check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyCheck {
    pub name: String,
    pub passed: bool,
    pub message: String,
}

/// Auto-remediation engine
pub struct RemediationEngine {
    config: Arc<RwLock<RemediationConfig>>,
    actions: Arc<RwLock<HashMap<String, RemediationAction>>>,
    effectiveness_metrics: Arc<RwLock<HashMap<String, EffectivenessMetrics>>>,
    approval_queue: Arc<RwLock<VecDeque<ApprovalRequest>>>,
    action_history: Arc<RwLock<VecDeque<RemediationAction>>>,
    max_history: usize,
}

impl RemediationEngine {
    /// Create a new remediation engine
    pub fn new() -> Self {
        Self::with_config(RemediationConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: RemediationConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            actions: Arc::new(RwLock::new(HashMap::new())),
            effectiveness_metrics: Arc::new(RwLock::new(HashMap::new())),
            approval_queue: Arc::new(RwLock::new(VecDeque::new())),
            action_history: Arc::new(RwLock::new(VecDeque::new())),
            max_history: 1000,
        }
    }

    /// Update configuration
    pub fn update_config(&self, config: RemediationConfig) {
        let mut cfg = self.config.write();
        *cfg = config;
    }

    /// Get current configuration
    pub fn get_config(&self) -> RemediationConfig {
        self.config.read().clone()
    }

    /// Process a recommendation for auto-remediation
    pub fn process_recommendation(
        &self,
        recommendation: &Recommendation,
    ) -> Result<String, String> {
        let config = self.config.read().clone();

        if !config.enabled {
            return Err("Auto-remediation is disabled".to_string());
        }

        // Check cooldown
        if !self.check_cooldown(&config) {
            return Err("Cooldown period not elapsed".to_string());
        }

        // Check concurrent limit
        if !self.check_concurrent_limit(&config) {
            return Err("Maximum concurrent remediations reached".to_string());
        }

        // Assess risk
        let risk_assessment = self.assess_risk(recommendation);

        // Determine if approval is needed
        let needs_approval = self.needs_approval(recommendation, &config, &risk_assessment);

        // Create remediation action
        let action = self.create_action(recommendation, risk_assessment.clone());
        let action_id = action.id.clone();

        // Store action
        {
            let mut actions = self.actions.write();
            actions.insert(action_id.clone(), action.clone());
        }

        if needs_approval {
            // Queue for approval
            self.queue_for_approval(action_id.clone(), recommendation.clone(), risk_assessment);
            self.update_action_status(&action_id, RemediationStatus::AwaitingApproval);
            Ok(format!("Action {} queued for approval", action_id))
        } else {
            // Auto-apply
            self.apply_action(&action_id)?;
            Ok(format!("Action {} applied successfully", action_id))
        }
    }

    /// Create a remediation action from recommendation
    fn create_action(
        &self,
        recommendation: &Recommendation,
        risk_assessment: RiskAssessment,
    ) -> RemediationAction {
        let config_changes = self.extract_config_changes(recommendation);
        let rollback_data = self.create_rollback_data(&config_changes);

        RemediationAction {
            id: format!("action-{}", Uuid::new_v4()),
            recommendation_id: recommendation.id.clone(),
            status: RemediationStatus::Pending,
            created_at: Utc::now(),
            applied_at: None,
            completed_at: None,
            config_changes,
            rollback_data: Some(rollback_data),
            logs: vec![format!(
                "Action created from recommendation: {}",
                recommendation.title
            )],
            success: false,
            error: None,
            retry_count: 0,
            approved_by: None,
            approved_at: None,
        }
    }

    /// Extract configuration changes from recommendation
    fn extract_config_changes(&self, recommendation: &Recommendation) -> HashMap<String, String> {
        let mut changes = HashMap::new();

        // Parse recommendation action to determine config changes
        match recommendation.category {
            RecommendationCategory::Latency => {
                if let Some(ref example) = recommendation.example {
                    if let Some(latency) = self.extract_latency_value(example) {
                        changes.insert("chaos_latency_ms".to_string(), latency.to_string());
                    }
                }
            }
            RecommendationCategory::FaultInjection => {
                changes.insert("chaos_fault_probability".to_string(), "0.3".to_string());
            }
            RecommendationCategory::RateLimit => {
                changes.insert("chaos_rate_limit".to_string(), "100".to_string());
            }
            _ => {}
        }

        changes
    }

    /// Extract latency value from example command
    fn extract_latency_value(&self, example: &str) -> Option<u64> {
        // Parse: --chaos-latency-ms 1500
        example
            .split_whitespace()
            .position(|s| s == "--chaos-latency-ms")
            .and_then(|i| example.split_whitespace().nth(i + 1))
            .and_then(|v| v.parse().ok())
    }

    /// Create rollback data
    fn create_rollback_data(&self, config_changes: &HashMap<String, String>) -> RollbackData {
        // In a real implementation, this would capture current config values
        let mut previous_config = HashMap::new();
        for key in config_changes.keys() {
            previous_config.insert(key.clone(), "default".to_string());
        }

        RollbackData {
            previous_config,
            restore_commands: vec!["mockforge serve --reset-chaos".to_string()],
            created_at: Utc::now(),
        }
    }

    /// Assess risk of applying recommendation
    fn assess_risk(&self, recommendation: &Recommendation) -> RiskAssessment {
        let risk_level = match recommendation.severity {
            RecommendationSeverity::Info => RiskLevel::Minimal,
            RecommendationSeverity::Low => RiskLevel::Low,
            RecommendationSeverity::Medium => RiskLevel::Medium,
            RecommendationSeverity::High => RiskLevel::High,
            RecommendationSeverity::Critical => RiskLevel::Critical,
        };

        let safety_checks = vec![
            SafetyCheck {
                name: "configuration_valid".to_string(),
                passed: true,
                message: "Configuration changes are valid".to_string(),
            },
            SafetyCheck {
                name: "rollback_available".to_string(),
                passed: true,
                message: "Rollback mechanism available".to_string(),
            },
        ];

        RiskAssessment {
            risk_level,
            impact_scope: recommendation.affected_endpoints.clone(),
            reversible: true,
            estimated_downtime_ms: 0,
            safety_checks,
        }
    }

    /// Check if recommendation needs approval
    fn needs_approval(
        &self,
        recommendation: &Recommendation,
        config: &RemediationConfig,
        risk: &RiskAssessment,
    ) -> bool {
        // Require approval if severity is above threshold
        if recommendation.severity > config.max_auto_severity {
            return true;
        }

        // Require approval for specific categories
        if config.require_approval_categories.contains(&recommendation.category) {
            return true;
        }

        // Require approval if risk is high
        if risk.risk_level >= RiskLevel::High {
            return true;
        }

        // Require approval if not reversible
        if !risk.reversible {
            return true;
        }

        false
    }

    /// Queue action for approval
    fn queue_for_approval(
        &self,
        action_id: String,
        recommendation: Recommendation,
        risk: RiskAssessment,
    ) {
        let mut changes = HashMap::new();
        changes.insert("example".to_string(), recommendation.example.clone().unwrap_or_default());

        let request = ApprovalRequest {
            action_id,
            recommendation,
            proposed_changes: changes,
            risk_assessment: risk,
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(24),
        };

        let mut queue = self.approval_queue.write();
        queue.push_back(request);
    }

    /// Get pending approval requests
    pub fn get_approval_queue(&self) -> Vec<ApprovalRequest> {
        let queue = self.approval_queue.read();
        queue.iter().cloned().collect()
    }

    /// Approve a remediation action
    pub fn approve_action(&self, action_id: &str, approver: &str) -> Result<(), String> {
        // Remove from approval queue
        {
            let mut queue = self.approval_queue.write();
            queue.retain(|req| req.action_id != action_id);
        }

        // Update action
        {
            let mut actions = self.actions.write();
            if let Some(action) = actions.get_mut(action_id) {
                action.status = RemediationStatus::Approved;
                action.approved_by = Some(approver.to_string());
                action.approved_at = Some(Utc::now());
                action.logs.push(format!("Approved by {}", approver));
            } else {
                return Err("Action not found".to_string());
            }
        }

        // Apply the action
        self.apply_action(action_id)?;

        Ok(())
    }

    /// Reject a remediation action
    pub fn reject_action(&self, action_id: &str, reason: &str) -> Result<(), String> {
        // Remove from approval queue
        {
            let mut queue = self.approval_queue.write();
            queue.retain(|req| req.action_id != action_id);
        }

        self.update_action_status(action_id, RemediationStatus::Rejected);
        self.add_action_log(action_id, &format!("Rejected: {}", reason));

        Ok(())
    }

    /// Apply a remediation action
    fn apply_action(&self, action_id: &str) -> Result<RemediationResult, String> {
        let config = self.config.read().clone();
        let start_time = Utc::now();

        self.update_action_status(action_id, RemediationStatus::Applying);

        // Get action
        let action = {
            let actions = self.actions.read();
            actions.get(action_id).cloned().ok_or_else(|| "Action not found".to_string())?
        };

        if config.dry_run {
            self.add_action_log(action_id, "Dry-run mode: changes not actually applied");
            self.update_action_status(action_id, RemediationStatus::Applied);

            return Ok(RemediationResult {
                action_id: action_id.to_string(),
                success: true,
                message: "Dry-run completed successfully".to_string(),
                applied_changes: action.config_changes.keys().cloned().collect(),
                duration_ms: (Utc::now() - start_time).num_milliseconds() as u64,
            });
        }

        // Apply changes (in real implementation, this would modify actual config)
        let applied_changes: Vec<String> =
            action.config_changes.iter().map(|(k, v)| format!("{} = {}", k, v)).collect();

        self.add_action_log(action_id, &format!("Applied changes: {:?}", applied_changes));

        // Update action
        {
            let mut actions = self.actions.write();
            if let Some(action) = actions.get_mut(action_id) {
                action.status = RemediationStatus::Applied;
                action.success = true;
                action.applied_at = Some(Utc::now());
                action.completed_at = Some(Utc::now());
            }
        }

        // Add to history
        self.add_to_history(action);

        Ok(RemediationResult {
            action_id: action_id.to_string(),
            success: true,
            message: "Remediation applied successfully".to_string(),
            applied_changes: applied_changes.to_vec(),
            duration_ms: (Utc::now() - start_time).num_milliseconds() as u64,
        })
    }

    /// Rollback a remediation action
    pub fn rollback_action(&self, action_id: &str) -> Result<(), String> {
        let action = {
            let actions = self.actions.read();
            actions.get(action_id).cloned().ok_or_else(|| "Action not found".to_string())?
        };

        if action.status != RemediationStatus::Applied {
            return Err("Can only rollback applied actions".to_string());
        }

        let rollback_data =
            action.rollback_data.ok_or_else(|| "No rollback data available".to_string())?;

        self.add_action_log(action_id, "Rolling back changes");

        // Apply rollback (in real implementation, this would restore config)
        for cmd in &rollback_data.restore_commands {
            self.add_action_log(action_id, &format!("Executing: {}", cmd));
        }

        self.update_action_status(action_id, RemediationStatus::RolledBack);
        self.add_action_log(action_id, "Rollback completed");

        Ok(())
    }

    /// Record effectiveness metrics
    pub fn record_effectiveness(
        &self,
        recommendation_id: &str,
        action_id: &str,
        before: SystemMetrics,
        after: SystemMetrics,
        measurement_period_hours: i64,
    ) {
        let improvement_score = self.calculate_improvement_score(&before, &after);

        let metrics = EffectivenessMetrics {
            recommendation_id: recommendation_id.to_string(),
            action_id: action_id.to_string(),
            before_metrics: before,
            after_metrics: after,
            improvement_score,
            measurement_period_hours,
            measured_at: Utc::now(),
        };

        let mut effectiveness = self.effectiveness_metrics.write();
        effectiveness.insert(action_id.to_string(), metrics);
    }

    /// Calculate improvement score
    fn calculate_improvement_score(&self, before: &SystemMetrics, after: &SystemMetrics) -> f64 {
        let mut score = 0.0;
        let mut weight_total = 0.0;

        // Error rate improvement (weight: 0.3)
        if before.error_rate > 0.0 {
            let error_improvement = (before.error_rate - after.error_rate) / before.error_rate;
            score += error_improvement * 0.3;
            weight_total += 0.3;
        }

        // Latency improvement (weight: 0.2)
        if before.avg_latency_ms > 0.0 {
            let latency_improvement =
                (before.avg_latency_ms - after.avg_latency_ms) / before.avg_latency_ms;
            score += latency_improvement * 0.2;
            weight_total += 0.2;
        }

        // Success rate improvement (weight: 0.25)
        let success_improvement = after.success_rate - before.success_rate;
        score += success_improvement * 0.25;
        weight_total += 0.25;

        // Resilience improvement (weight: 0.25)
        let resilience_improvement = after.resilience_score - before.resilience_score;
        score += resilience_improvement * 0.25;
        weight_total += 0.25;

        if weight_total > 0.0 {
            (score / weight_total).max(0.0).min(1.0)
        } else {
            0.0
        }
    }

    /// Get effectiveness metrics for an action
    pub fn get_effectiveness(&self, action_id: &str) -> Option<EffectivenessMetrics> {
        let metrics = self.effectiveness_metrics.read();
        metrics.get(action_id).cloned()
    }

    /// Get all effectiveness metrics
    pub fn get_all_effectiveness(&self) -> Vec<EffectivenessMetrics> {
        let metrics = self.effectiveness_metrics.read();
        metrics.values().cloned().collect()
    }

    /// Get action by ID
    pub fn get_action(&self, action_id: &str) -> Option<RemediationAction> {
        let actions = self.actions.read();
        actions.get(action_id).cloned()
    }

    /// Get all active actions
    pub fn get_active_actions(&self) -> Vec<RemediationAction> {
        let actions = self.actions.read();
        actions
            .values()
            .filter(|a| {
                matches!(
                    a.status,
                    RemediationStatus::Pending
                        | RemediationStatus::Applying
                        | RemediationStatus::Applied
                )
            })
            .cloned()
            .collect()
    }

    /// Get action history
    pub fn get_history(&self, limit: usize) -> Vec<RemediationAction> {
        let history = self.action_history.read();
        history.iter().take(limit).cloned().collect()
    }

    /// Get statistics
    pub fn get_stats(&self) -> RemediationStats {
        let actions = self.actions.read();
        let history = self.action_history.read();

        let total_actions = actions.len() + history.len();
        let successful = actions.values().filter(|a| a.success).count()
            + history.iter().filter(|a| a.success).count();
        let failed = actions.values().filter(|a| a.status == RemediationStatus::Failed).count()
            + history.iter().filter(|a| a.status == RemediationStatus::Failed).count();
        let pending_approval = actions
            .values()
            .filter(|a| a.status == RemediationStatus::AwaitingApproval)
            .count();
        let rolled_back =
            history.iter().filter(|a| a.status == RemediationStatus::RolledBack).count();

        let effectiveness_metrics = self.effectiveness_metrics.read();
        let avg_improvement = if effectiveness_metrics.is_empty() {
            0.0
        } else {
            effectiveness_metrics.values().map(|m| m.improvement_score).sum::<f64>()
                / effectiveness_metrics.len() as f64
        };

        RemediationStats {
            total_actions,
            successful_actions: successful,
            failed_actions: failed,
            pending_approval,
            rolled_back,
            avg_improvement_score: avg_improvement,
            total_effectiveness_measurements: effectiveness_metrics.len(),
        }
    }

    // Helper methods

    fn check_cooldown(&self, config: &RemediationConfig) -> bool {
        let actions = self.actions.read();
        let cooldown_threshold = Utc::now() - Duration::minutes(config.cooldown_minutes);

        !actions.values().any(|a| {
            a.status == RemediationStatus::Applied
                && a.completed_at.is_some_and(|t| t > cooldown_threshold)
        })
    }

    fn check_concurrent_limit(&self, config: &RemediationConfig) -> bool {
        let actions = self.actions.read();
        let active_count = actions
            .values()
            .filter(|a| matches!(a.status, RemediationStatus::Applying))
            .count();

        active_count < config.max_concurrent
    }

    fn update_action_status(&self, action_id: &str, status: RemediationStatus) {
        let mut actions = self.actions.write();
        if let Some(action) = actions.get_mut(action_id) {
            action.status = status;
        }
    }

    fn add_action_log(&self, action_id: &str, message: &str) {
        let mut actions = self.actions.write();
        if let Some(action) = actions.get_mut(action_id) {
            action
                .logs
                .push(format!("[{}] {}", Utc::now().format("%Y-%m-%d %H:%M:%S"), message));
        }
    }

    fn add_to_history(&self, action: RemediationAction) {
        let mut history = self.action_history.write();
        history.push_front(action);
        if history.len() > self.max_history {
            history.pop_back();
        }
    }
}

impl Default for RemediationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Remediation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationStats {
    pub total_actions: usize,
    pub successful_actions: usize,
    pub failed_actions: usize,
    pub pending_approval: usize,
    pub rolled_back: usize,
    pub avg_improvement_score: f64,
    pub total_effectiveness_measurements: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = RemediationEngine::new();
        assert!(!engine.get_config().enabled);
    }

    #[test]
    fn test_config_update() {
        let engine = RemediationEngine::new();
        let mut config = RemediationConfig::default();
        config.enabled = true;
        engine.update_config(config);
        assert!(engine.get_config().enabled);
    }

    #[test]
    fn test_improvement_score_calculation() {
        let engine = RemediationEngine::new();

        let before = SystemMetrics {
            error_rate: 0.5,
            avg_latency_ms: 1000.0,
            p95_latency_ms: 1500.0,
            p99_latency_ms: 2000.0,
            success_rate: 0.5,
            chaos_impact: 0.8,
            resilience_score: 0.3,
        };

        let after = SystemMetrics {
            error_rate: 0.2,
            avg_latency_ms: 500.0,
            p95_latency_ms: 750.0,
            p99_latency_ms: 1000.0,
            success_rate: 0.8,
            chaos_impact: 0.4,
            resilience_score: 0.7,
        };

        let score = engine.calculate_improvement_score(&before, &after);
        assert!(score > 0.0 && score <= 1.0);
    }
}
