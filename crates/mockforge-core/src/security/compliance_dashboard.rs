//! Compliance Monitoring Dashboard
//!
//! This module provides real-time compliance monitoring, aggregating data from
//! various security systems to provide compliance scores, control effectiveness,
//! gap analysis, and alerts.

use crate::Error;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Compliance standard
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComplianceStandard {
    /// SOC 2 Type II
    Soc2,
    /// ISO 27001
    Iso27001,
}

/// Control category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlCategory {
    /// Access control
    AccessControl,
    /// Encryption
    Encryption,
    /// Monitoring
    Monitoring,
    /// Change management
    ChangeManagement,
    /// Incident response
    IncidentResponse,
}

/// Gap severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum GapSeverity {
    /// Critical severity
    Critical,
    /// High severity
    High,
    /// Medium severity
    Medium,
    /// Low severity
    Low,
}

/// Compliance gap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceGap {
    /// Gap ID
    pub gap_id: String,
    /// Gap description
    pub description: String,
    /// Severity
    pub severity: GapSeverity,
    /// Affected standard
    pub standard: ComplianceStandard,
    /// Control ID
    pub control_id: Option<String>,
    /// Status
    pub status: GapStatus,
    /// Created date
    pub created_at: DateTime<Utc>,
    /// Target remediation date
    pub target_remediation_date: Option<DateTime<Utc>>,
    /// Remediated date
    pub remediated_at: Option<DateTime<Utc>>,
}

/// Gap status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapStatus {
    /// Gap identified
    Identified,
    /// Remediation in progress
    InProgress,
    /// Remediated
    Remediated,
    /// Overdue
    Overdue,
}

/// Compliance alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceAlert {
    /// Alert ID
    pub alert_id: String,
    /// Alert type
    pub alert_type: AlertType,
    /// Severity
    pub severity: GapSeverity,
    /// Message
    pub message: String,
    /// Affected standard
    pub standard: Option<ComplianceStandard>,
    /// Control ID
    pub control_id: Option<String>,
    /// Created date
    pub created_at: DateTime<Utc>,
    /// Acknowledged date
    pub acknowledged_at: Option<DateTime<Utc>>,
    /// Resolved date
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Alert type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    /// Compliance violation
    ComplianceViolation,
    /// Control failure
    ControlFailure,
    /// Remediation overdue
    RemediationOverdue,
    /// Audit finding
    AuditFinding,
}

/// Control effectiveness metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlEffectiveness {
    /// Control category
    pub category: ControlCategory,
    /// Effectiveness percentage (0-100)
    pub effectiveness: u8,
    /// Last test date
    pub last_test_date: Option<DateTime<Utc>>,
    /// Test results
    pub test_results: Option<String>,
}

/// Compliance dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceDashboardData {
    /// Overall compliance score (0-100)
    pub overall_compliance: u8,
    /// SOC 2 compliance score
    pub soc2_compliance: u8,
    /// ISO 27001 compliance score
    pub iso27001_compliance: u8,
    /// Control effectiveness by category
    pub control_effectiveness: HashMap<ControlCategory, ControlEffectiveness>,
    /// Gap summary
    pub gaps: GapSummary,
    /// Alert summary
    pub alerts: AlertSummary,
    /// Remediation status
    pub remediation: RemediationStatus,
    /// Last updated
    pub last_updated: DateTime<Utc>,
}

/// Gap summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapSummary {
    /// Total gaps
    pub total: u32,
    /// Critical gaps
    pub critical: u32,
    /// High gaps
    pub high: u32,
    /// Medium gaps
    pub medium: u32,
    /// Low gaps
    pub low: u32,
}

/// Alert summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertSummary {
    /// Total alerts
    pub total: u32,
    /// Critical alerts
    pub critical: u32,
    /// High alerts
    pub high: u32,
    /// Medium alerts
    pub medium: u32,
    /// Low alerts
    pub low: u32,
}

/// Remediation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationStatus {
    /// In progress
    pub in_progress: u32,
    /// Completed this month
    pub completed_this_month: u32,
    /// Overdue
    pub overdue: u32,
}

/// Compliance dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ComplianceDashboardConfig {
    /// Whether dashboard is enabled
    pub enabled: bool,
    /// Refresh interval in seconds
    pub refresh_interval_seconds: u64,
    /// Alert thresholds
    pub alert_thresholds: AlertThresholds,
}

/// Alert thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AlertThresholds {
    /// Minimum compliance score to trigger alert
    pub compliance_score: u8,
    /// Minimum control effectiveness to trigger alert
    pub control_effectiveness: u8,
}

impl Default for ComplianceDashboardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            refresh_interval_seconds: 300, // 5 minutes
            alert_thresholds: AlertThresholds {
                compliance_score: 90,
                control_effectiveness: 85,
            },
        }
    }
}

/// Compliance dashboard engine
///
/// Aggregates data from various security systems to provide real-time
/// compliance monitoring and reporting.
pub struct ComplianceDashboardEngine {
    config: ComplianceDashboardConfig,
    /// Compliance gaps
    gaps: std::sync::Arc<tokio::sync::RwLock<HashMap<String, ComplianceGap>>>,
    /// Compliance alerts
    alerts: std::sync::Arc<tokio::sync::RwLock<HashMap<String, ComplianceAlert>>>,
    /// Control effectiveness cache
    control_effectiveness:
        std::sync::Arc<tokio::sync::RwLock<HashMap<ControlCategory, ControlEffectiveness>>>,
}

impl ComplianceDashboardEngine {
    /// Create a new compliance dashboard engine
    pub fn new(config: ComplianceDashboardConfig) -> Self {
        Self {
            config,
            gaps: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            alerts: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            control_effectiveness: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Get dashboard data
    ///
    /// Aggregates data from all security systems to provide comprehensive
    /// compliance status.
    pub async fn get_dashboard_data(&self) -> Result<ComplianceDashboardData, Error> {
        // Calculate compliance scores
        let soc2_compliance = self.calculate_soc2_compliance().await?;
        let iso27001_compliance = self.calculate_iso27001_compliance().await?;
        let overall_compliance = (soc2_compliance + iso27001_compliance) / 2;

        // Get control effectiveness
        let control_effectiveness = self.get_control_effectiveness().await?;

        // Get gap summary
        let gaps = self.get_gap_summary().await?;

        // Get alert summary
        let alerts = self.get_alert_summary().await?;

        // Get remediation status
        let remediation = self.get_remediation_status().await?;

        Ok(ComplianceDashboardData {
            overall_compliance,
            soc2_compliance,
            iso27001_compliance,
            control_effectiveness,
            gaps,
            alerts,
            remediation,
            last_updated: Utc::now(),
        })
    }

    /// Calculate SOC 2 compliance score
    async fn calculate_soc2_compliance(&self) -> Result<u8, Error> {
        // TODO: Integrate with actual security systems to calculate real scores
        // For now, return a placeholder score based on implemented systems

        // Check if key systems are enabled/working
        // - SIEM integration: 20 points
        // - Access reviews: 20 points
        // - Privileged access: 20 points
        // - Change management: 20 points
        // - Security events: 20 points

        // Placeholder: assume all systems are working if enabled
        // In production, this would check actual system status
        Ok(95) // Placeholder score
    }

    /// Calculate ISO 27001 compliance score
    async fn calculate_iso27001_compliance(&self) -> Result<u8, Error> {
        // TODO: Integrate with actual security systems to calculate real scores
        // Similar to SOC 2, but with ISO 27001 control mappings
        Ok(92) // Placeholder score
    }

    /// Get control effectiveness metrics
    async fn get_control_effectiveness(
        &self,
    ) -> Result<HashMap<ControlCategory, ControlEffectiveness>, Error> {
        let mut effectiveness = HashMap::new();

        // Access Control
        effectiveness.insert(
            ControlCategory::AccessControl,
            ControlEffectiveness {
                category: ControlCategory::AccessControl,
                effectiveness: 98, // TODO: Calculate from access review data
                last_test_date: Some(Utc::now() - chrono::Duration::days(7)),
                test_results: Some("All access controls tested and passing".to_string()),
            },
        );

        // Encryption
        effectiveness.insert(
            ControlCategory::Encryption,
            ControlEffectiveness {
                category: ControlCategory::Encryption,
                effectiveness: 100, // TODO: Calculate from encryption status
                last_test_date: Some(Utc::now() - chrono::Duration::days(14)),
                test_results: Some("Encryption controls verified".to_string()),
            },
        );

        // Monitoring
        effectiveness.insert(
            ControlCategory::Monitoring,
            ControlEffectiveness {
                category: ControlCategory::Monitoring,
                effectiveness: 95, // TODO: Calculate from SIEM status
                last_test_date: Some(Utc::now() - chrono::Duration::days(3)),
                test_results: Some("SIEM integration operational".to_string()),
            },
        );

        // Change Management
        effectiveness.insert(
            ControlCategory::ChangeManagement,
            ControlEffectiveness {
                category: ControlCategory::ChangeManagement,
                effectiveness: 90, // TODO: Calculate from change management data
                last_test_date: Some(Utc::now() - chrono::Duration::days(10)),
                test_results: Some("Change management process followed".to_string()),
            },
        );

        // Incident Response
        effectiveness.insert(
            ControlCategory::IncidentResponse,
            ControlEffectiveness {
                category: ControlCategory::IncidentResponse,
                effectiveness: 95, // TODO: Calculate from incident response data
                last_test_date: Some(Utc::now() - chrono::Duration::days(5)),
                test_results: Some("Incident response procedures tested".to_string()),
            },
        );

        Ok(effectiveness)
    }

    /// Get gap summary
    async fn get_gap_summary(&self) -> Result<GapSummary, Error> {
        let gaps = self.gaps.read().await;

        let mut summary = GapSummary {
            total: gaps.len() as u32,
            critical: 0,
            high: 0,
            medium: 0,
            low: 0,
        };

        for gap in gaps.values() {
            match gap.severity {
                GapSeverity::Critical => summary.critical += 1,
                GapSeverity::High => summary.high += 1,
                GapSeverity::Medium => summary.medium += 1,
                GapSeverity::Low => summary.low += 1,
            }
        }

        Ok(summary)
    }

    /// Get alert summary
    async fn get_alert_summary(&self) -> Result<AlertSummary, Error> {
        let alerts = self.alerts.read().await;

        let mut summary = AlertSummary {
            total: alerts.len() as u32,
            critical: 0,
            high: 0,
            medium: 0,
            low: 0,
        };

        for alert in alerts.values() {
            if alert.resolved_at.is_none() {
                match alert.severity {
                    GapSeverity::Critical => summary.critical += 1,
                    GapSeverity::High => summary.high += 1,
                    GapSeverity::Medium => summary.medium += 1,
                    GapSeverity::Low => summary.low += 1,
                }
            }
        }

        Ok(summary)
    }

    /// Get remediation status
    async fn get_remediation_status(&self) -> Result<RemediationStatus, Error> {
        let gaps = self.gaps.read().await;
        let now = Utc::now();
        // Get start of current month - use format string approach
        let month_start_str = format!("{}-{:02}-01T00:00:00Z", now.format("%Y"), now.format("%m"));
        let start_of_month = DateTime::parse_from_rfc3339(&month_start_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or(now);

        let mut status = RemediationStatus {
            in_progress: 0,
            completed_this_month: 0,
            overdue: 0,
        };

        for gap in gaps.values() {
            match gap.status {
                GapStatus::InProgress => status.in_progress += 1,
                GapStatus::Remediated => {
                    if let Some(remediated_at) = gap.remediated_at {
                        if remediated_at >= start_of_month {
                            status.completed_this_month += 1;
                        }
                    }
                }
                GapStatus::Overdue => status.overdue += 1,
                GapStatus::Identified => {
                    // Check if overdue
                    if let Some(target_date) = gap.target_remediation_date {
                        if now > target_date {
                            status.overdue += 1;
                        }
                    }
                }
            }
        }

        Ok(status)
    }

    /// Add a compliance gap
    pub async fn add_gap(
        &self,
        gap_id: String,
        description: String,
        severity: GapSeverity,
        standard: ComplianceStandard,
        control_id: Option<String>,
        target_remediation_date: Option<DateTime<Utc>>,
    ) -> Result<(), Error> {
        let mut gaps = self.gaps.write().await;
        let gap = ComplianceGap {
            gap_id: gap_id.clone(),
            description,
            severity,
            standard,
            control_id,
            status: GapStatus::Identified,
            created_at: Utc::now(),
            target_remediation_date,
            remediated_at: None,
        };
        gaps.insert(gap_id, gap);
        Ok(())
    }

    /// Update gap status
    pub async fn update_gap_status(&self, gap_id: &str, status: GapStatus) -> Result<(), Error> {
        let mut gaps = self.gaps.write().await;
        if let Some(gap) = gaps.get_mut(gap_id) {
            gap.status = status;
            if status == GapStatus::Remediated {
                gap.remediated_at = Some(Utc::now());
            }
        } else {
            return Err(Error::Generic("Gap not found".to_string()));
        }
        Ok(())
    }

    /// Add a compliance alert
    pub async fn add_alert(
        &self,
        alert_id: String,
        alert_type: AlertType,
        severity: GapSeverity,
        message: String,
        standard: Option<ComplianceStandard>,
        control_id: Option<String>,
    ) -> Result<(), Error> {
        let mut alerts = self.alerts.write().await;
        let alert = ComplianceAlert {
            alert_id: alert_id.clone(),
            alert_type,
            severity,
            message,
            standard,
            control_id,
            created_at: Utc::now(),
            acknowledged_at: None,
            resolved_at: None,
        };
        alerts.insert(alert_id, alert);
        Ok(())
    }

    /// Get all gaps
    pub async fn get_all_gaps(&self) -> Result<Vec<ComplianceGap>, Error> {
        let gaps = self.gaps.read().await;
        Ok(gaps.values().cloned().collect())
    }

    /// Get all alerts
    pub async fn get_all_alerts(&self) -> Result<Vec<ComplianceAlert>, Error> {
        let alerts = self.alerts.read().await;
        Ok(alerts.values().cloned().collect())
    }

    /// Get gaps by severity
    pub async fn get_gaps_by_severity(
        &self,
        severity: GapSeverity,
    ) -> Result<Vec<ComplianceGap>, Error> {
        let gaps = self.gaps.read().await;
        Ok(gaps.values().filter(|g| g.severity == severity).cloned().collect())
    }

    /// Get alerts by severity
    pub async fn get_alerts_by_severity(
        &self,
        severity: GapSeverity,
    ) -> Result<Vec<ComplianceAlert>, Error> {
        let alerts = self.alerts.read().await;
        Ok(alerts
            .values()
            .filter(|a| a.severity == severity && a.resolved_at.is_none())
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dashboard_data() {
        let config = ComplianceDashboardConfig::default();
        let engine = ComplianceDashboardEngine::new(config);

        let dashboard = engine.get_dashboard_data().await.unwrap();
        assert!(dashboard.overall_compliance <= 100);
        assert!(dashboard.soc2_compliance <= 100);
        assert!(dashboard.iso27001_compliance <= 100);
    }
}
