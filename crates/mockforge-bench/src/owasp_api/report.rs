//! OWASP API Security Report Structures
//!
//! This module defines the output formats for OWASP API security test results,
//! including JSON and SARIF report formats.

use super::categories::{OwaspCategory, Severity};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Complete OWASP API Security scan report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwaspReport {
    /// Scan metadata
    pub scan_info: OwaspScanInfo,
    /// All findings from the scan
    pub findings: Vec<OwaspFinding>,
    /// Summary statistics
    pub summary: OwaspSummary,
}

/// Metadata about the scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwaspScanInfo {
    /// Timestamp when the scan started
    pub timestamp: DateTime<Utc>,
    /// Timestamp when the scan completed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Target URL(s) that were scanned
    pub target: String,
    /// OpenAPI spec file used
    pub spec: String,
    /// MockForge version
    pub mockforge_version: String,
    /// Categories that were tested
    pub categories_tested: Vec<OwaspCategory>,
    /// Scan configuration summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_summary: Option<ConfigSummary>,
}

/// Summary of scan configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSummary {
    /// Auth header used
    pub auth_header: String,
    /// Whether valid auth token was provided
    pub has_valid_token: bool,
    /// Number of admin paths tested
    pub admin_paths_count: usize,
    /// Concurrency level
    pub concurrency: usize,
}

/// A single security finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwaspFinding {
    /// Unique ID for this finding
    pub id: String,
    /// OWASP category
    pub category: OwaspCategory,
    /// Full category name
    pub category_name: String,
    /// Severity of the finding
    pub severity: Severity,
    /// The endpoint where the vulnerability was found
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Human-readable description
    pub description: String,
    /// Evidence of the vulnerability
    pub evidence: FindingEvidence,
    /// Remediation guidance
    pub remediation: String,
    /// CWE ID if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwe_id: Option<String>,
    /// CVSS score if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cvss_score: Option<f32>,
    /// Additional tags/labels
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Evidence supporting a finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingEvidence {
    /// The request that triggered the finding
    pub request: RequestEvidence,
    /// The response received
    pub response: ResponseEvidence,
    /// The payload that was used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    /// Additional notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Request evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestEvidence {
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Selected request headers (sensitive values redacted)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
    /// Request body preview (truncated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_preview: Option<String>,
}

/// Response evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseEvidence {
    /// HTTP status code
    pub status: u16,
    /// Selected response headers
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
    /// Response body preview (truncated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_preview: Option<String>,
    /// Response time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_time_ms: Option<u64>,
}

/// Summary statistics for the scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwaspSummary {
    /// Total number of endpoints tested
    pub total_endpoints_tested: usize,
    /// Total number of requests made
    pub total_requests: usize,
    /// Total number of findings
    pub total_findings: usize,
    /// Findings broken down by category
    pub findings_by_category: HashMap<String, usize>,
    /// Findings broken down by severity
    pub findings_by_severity: HashMap<String, usize>,
    /// Pass/fail status for each category
    pub category_status: HashMap<String, CategoryStatus>,
    /// Scan duration in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
}

/// Status for a category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CategoryStatus {
    /// All tests passed (no findings)
    Pass,
    /// Findings were detected
    Fail,
    /// Category was skipped
    Skipped,
    /// Error during testing
    Error,
}

impl OwaspReport {
    /// Create a new empty report
    pub fn new(target: String, spec: String, categories: Vec<OwaspCategory>) -> Self {
        Self {
            scan_info: OwaspScanInfo {
                timestamp: Utc::now(),
                completed_at: None,
                target,
                spec,
                mockforge_version: env!("CARGO_PKG_VERSION").to_string(),
                categories_tested: categories,
                config_summary: None,
            },
            findings: Vec::new(),
            summary: OwaspSummary {
                total_endpoints_tested: 0,
                total_requests: 0,
                total_findings: 0,
                findings_by_category: HashMap::new(),
                findings_by_severity: HashMap::new(),
                category_status: HashMap::new(),
                duration_seconds: None,
            },
        }
    }

    /// Add a finding to the report
    pub fn add_finding(&mut self, finding: OwaspFinding) {
        // Update summary stats
        *self
            .summary
            .findings_by_category
            .entry(finding.category.cli_name().to_string())
            .or_insert(0) += 1;

        *self
            .summary
            .findings_by_severity
            .entry(finding.severity.as_str().to_string())
            .or_insert(0) += 1;

        self.summary.total_findings += 1;

        // Mark category as failed
        self.summary
            .category_status
            .insert(finding.category.cli_name().to_string(), CategoryStatus::Fail);

        self.findings.push(finding);
    }

    /// Mark scan as completed
    pub fn complete(&mut self) {
        self.scan_info.completed_at = Some(Utc::now());
        if let Some(start) = self.scan_info.timestamp.timestamp_millis().checked_sub(0) {
            let end = Utc::now().timestamp_millis();
            self.summary.duration_seconds = Some((end - start) as f64 / 1000.0);
        }
    }

    /// Set category status to pass if no findings
    pub fn finalize_category_status(&mut self) {
        for category in &self.scan_info.categories_tested {
            let key = category.cli_name().to_string();
            self.summary.category_status.entry(key).or_insert(CategoryStatus::Pass);
        }
    }

    /// Write report to JSON file
    pub fn write_json(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    /// Write report to SARIF format
    pub fn write_sarif(&self, path: &Path) -> std::io::Result<()> {
        let sarif = self.to_sarif();
        let json = serde_json::to_string_pretty(&sarif).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    /// Convert to SARIF format
    fn to_sarif(&self) -> SarifReport {
        let mut results = Vec::new();
        let mut rules = Vec::new();
        let mut rule_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        for finding in &self.findings {
            let rule_id = format!("OWASP-{}", finding.category.cli_name().to_uppercase());

            // Add rule if not already present
            if rule_ids.insert(rule_id.clone()) {
                rules.push(SarifRule {
                    id: rule_id.clone(),
                    name: finding.category.short_name().to_string(),
                    short_description: SarifMessage {
                        text: finding.category.full_name().to_string(),
                    },
                    full_description: SarifMessage {
                        text: finding.category.description().to_string(),
                    },
                    help: SarifMessage {
                        text: finding.category.remediation().to_string(),
                    },
                    default_configuration: SarifConfiguration {
                        level: severity_to_sarif_level(finding.severity),
                    },
                });
            }

            results.push(SarifResult {
                rule_id: rule_id.clone(),
                level: severity_to_sarif_level(finding.severity),
                message: SarifMessage {
                    text: finding.description.clone(),
                },
                locations: vec![SarifLocation {
                    physical_location: SarifPhysicalLocation {
                        artifact_location: SarifArtifactLocation {
                            uri: finding.endpoint.clone(),
                        },
                    },
                }],
            });
        }

        SarifReport {
            schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json".to_string(),
            version: "2.1.0".to_string(),
            runs: vec![SarifRun {
                tool: SarifTool {
                    driver: SarifDriver {
                        name: "MockForge OWASP API Scanner".to_string(),
                        version: self.scan_info.mockforge_version.clone(),
                        information_uri: "https://mockforge.dev".to_string(),
                        rules,
                    },
                },
                results,
            }],
        }
    }

    /// Get count of findings by severity
    pub fn count_by_severity(&self, severity: Severity) -> usize {
        self.findings.iter().filter(|f| f.severity == severity).count()
    }

    /// Check if there are any critical or high severity findings
    pub fn has_critical_findings(&self) -> bool {
        self.findings
            .iter()
            .any(|f| f.severity == Severity::Critical || f.severity == Severity::High)
    }
}

impl OwaspFinding {
    /// Create a new finding
    pub fn new(
        category: OwaspCategory,
        endpoint: String,
        method: String,
        description: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            category,
            category_name: category.full_name().to_string(),
            severity: category.severity(),
            endpoint,
            method,
            description,
            evidence: FindingEvidence {
                request: RequestEvidence {
                    method: String::new(),
                    path: String::new(),
                    headers: HashMap::new(),
                    body_preview: None,
                },
                response: ResponseEvidence {
                    status: 0,
                    headers: HashMap::new(),
                    body_preview: None,
                    response_time_ms: None,
                },
                payload: None,
                notes: None,
            },
            remediation: category.remediation().to_string(),
            cwe_id: category_to_cwe(category),
            cvss_score: None,
            tags: Vec::new(),
        }
    }

    /// Set evidence for this finding
    pub fn with_evidence(mut self, evidence: FindingEvidence) -> Self {
        self.evidence = evidence;
        self
    }

    /// Add a tag to this finding
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Override the severity
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }
}

/// Map OWASP category to CWE ID
fn category_to_cwe(category: OwaspCategory) -> Option<String> {
    match category {
        OwaspCategory::Api1Bola => Some("CWE-639".to_string()), // Authorization Bypass Through User-Controlled Key
        OwaspCategory::Api2BrokenAuth => Some("CWE-287".to_string()), // Improper Authentication
        OwaspCategory::Api3BrokenObjectProperty => Some("CWE-915".to_string()), // Mass Assignment
        OwaspCategory::Api4ResourceConsumption => Some("CWE-770".to_string()), // Allocation Without Limits
        OwaspCategory::Api5BrokenFunctionAuth => Some("CWE-285".to_string()), // Improper Authorization
        OwaspCategory::Api6SensitiveFlows => Some("CWE-840".to_string()), // Business Logic Errors
        OwaspCategory::Api7Ssrf => Some("CWE-918".to_string()),           // SSRF
        OwaspCategory::Api8Misconfiguration => Some("CWE-16".to_string()), // Configuration
        OwaspCategory::Api9ImproperInventory => Some("CWE-1059".to_string()), // Insufficient Documentation
        OwaspCategory::Api10UnsafeConsumption => Some("CWE-20".to_string()), // Improper Input Validation
    }
}

// SARIF format structures

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SarifReport {
    #[serde(rename = "$schema")]
    schema: String,
    version: String,
    runs: Vec<SarifRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SarifDriver {
    name: String,
    version: String,
    information_uri: String,
    rules: Vec<SarifRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SarifRule {
    id: String,
    name: String,
    short_description: SarifMessage,
    full_description: SarifMessage,
    help: SarifMessage,
    default_configuration: SarifConfiguration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SarifConfiguration {
    level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SarifResult {
    rule_id: String,
    level: String,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SarifMessage {
    text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SarifLocation {
    physical_location: SarifPhysicalLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SarifPhysicalLocation {
    artifact_location: SarifArtifactLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SarifArtifactLocation {
    uri: String,
}

/// Convert severity to SARIF level
fn severity_to_sarif_level(severity: Severity) -> String {
    match severity {
        Severity::Critical | Severity::High => "error".to_string(),
        Severity::Medium => "warning".to_string(),
        Severity::Low | Severity::Info => "note".to_string(),
    }
}

/// Console reporter for real-time output
pub struct ConsoleReporter {
    verbose: bool,
    use_color: bool,
}

impl ConsoleReporter {
    /// Create a new console reporter
    pub fn new(verbose: bool) -> Self {
        Self {
            verbose,
            use_color: atty::is(atty::Stream::Stdout),
        }
    }

    /// Print a finding to the console
    pub fn print_finding(&self, finding: &OwaspFinding) {
        let severity_color = match finding.severity {
            Severity::Critical => "\x1b[91m", // Bright red
            Severity::High => "\x1b[31m",     // Red
            Severity::Medium => "\x1b[33m",   // Yellow
            Severity::Low => "\x1b[36m",      // Cyan
            Severity::Info => "\x1b[37m",     // White
        };
        let reset = "\x1b[0m";

        if self.use_color {
            println!(
                "  {}[FINDING]{} {} {} - {}",
                severity_color, reset, finding.method, finding.endpoint, finding.description
            );
        } else {
            println!(
                "  [FINDING] {} {} - {}",
                finding.method, finding.endpoint, finding.description
            );
        }

        if self.verbose {
            println!("    Severity: {:?}", finding.severity);
            println!("    Remediation: {}", finding.remediation);
            if let Some(payload) = &finding.evidence.payload {
                println!("    Payload: {}", payload);
            }
        }
    }

    /// Print category header
    pub fn print_category_header(&self, category: OwaspCategory) {
        let bold = if self.use_color { "\x1b[1m" } else { "" };
        let reset = if self.use_color { "\x1b[0m" } else { "" };

        println!(
            "{}[{}]{} {}: Testing {}...",
            bold,
            category.cli_name().to_uppercase(),
            reset,
            category.short_name(),
            category.description()
        );
    }

    /// Print category result
    pub fn print_category_result(&self, category: OwaspCategory, finding_count: usize) {
        let green = if self.use_color { "\x1b[32m" } else { "" };
        let red = if self.use_color { "\x1b[31m" } else { "" };
        let reset = if self.use_color { "\x1b[0m" } else { "" };

        if finding_count == 0 {
            println!("  {}[PASS]{} {} - All tests passed", green, reset, category.short_name());
        } else {
            println!(
                "  {}[FAIL]{} {} - {} finding(s)",
                red,
                reset,
                category.short_name(),
                finding_count
            );
        }
    }

    /// Print final summary
    pub fn print_summary(&self, report: &OwaspReport) {
        let bold = if self.use_color { "\x1b[1m" } else { "" };
        let green = if self.use_color { "\x1b[32m" } else { "" };
        let red = if self.use_color { "\x1b[31m" } else { "" };
        let reset = if self.use_color { "\x1b[0m" } else { "" };

        println!();
        println!("{}OWASP API Top 10 Scan Results{}", bold, reset);
        println!("==============================");
        println!("Target: {}", report.scan_info.target);
        println!("Endpoints tested: {}", report.summary.total_endpoints_tested);
        println!("Total requests: {}", report.summary.total_requests);

        if let Some(duration) = report.summary.duration_seconds {
            println!("Duration: {:.2}s", duration);
        }

        println!();

        if report.summary.total_findings == 0 {
            println!("{}No vulnerabilities found!{}", green, reset);
        } else {
            println!(
                "{}Found {} vulnerability/ies across {} categories{}",
                red,
                report.summary.total_findings,
                report.summary.findings_by_category.len(),
                reset
            );

            println!();
            println!("Findings by severity:");
            for severity in [
                Severity::Critical,
                Severity::High,
                Severity::Medium,
                Severity::Low,
            ] {
                let count = report.count_by_severity(severity);
                if count > 0 {
                    println!("  {:?}: {}", severity, count);
                }
            }

            println!();
            println!("Findings by category:");
            for (category, count) in &report.summary.findings_by_category {
                println!("  {}: {}", category, count);
            }
        }
    }
}

impl Default for ConsoleReporter {
    fn default() -> Self {
        Self::new(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_creation() {
        let report = OwaspReport::new(
            "https://api.example.com".to_string(),
            "api.yaml".to_string(),
            vec![OwaspCategory::Api1Bola, OwaspCategory::Api2BrokenAuth],
        );

        assert_eq!(report.scan_info.target, "https://api.example.com");
        assert_eq!(report.scan_info.categories_tested.len(), 2);
        assert_eq!(report.summary.total_findings, 0);
    }

    #[test]
    fn test_add_finding() {
        let mut report = OwaspReport::new(
            "https://api.example.com".to_string(),
            "api.yaml".to_string(),
            vec![OwaspCategory::Api1Bola],
        );

        let finding = OwaspFinding::new(
            OwaspCategory::Api1Bola,
            "/users/123".to_string(),
            "GET".to_string(),
            "ID manipulation accepted".to_string(),
        );

        report.add_finding(finding);

        assert_eq!(report.summary.total_findings, 1);
        assert_eq!(report.findings.len(), 1);
        assert!(report.summary.findings_by_category.contains_key("api1"));
    }

    #[test]
    fn test_sarif_conversion() {
        let mut report = OwaspReport::new(
            "https://api.example.com".to_string(),
            "api.yaml".to_string(),
            vec![OwaspCategory::Api1Bola],
        );

        let finding = OwaspFinding::new(
            OwaspCategory::Api1Bola,
            "/users/123".to_string(),
            "GET".to_string(),
            "Test finding".to_string(),
        );

        report.add_finding(finding);

        let sarif = report.to_sarif();
        assert_eq!(sarif.version, "2.1.0");
        assert_eq!(sarif.runs.len(), 1);
        assert_eq!(sarif.runs[0].results.len(), 1);
    }

    #[test]
    fn test_category_to_cwe() {
        assert_eq!(category_to_cwe(OwaspCategory::Api1Bola), Some("CWE-639".to_string()));
        assert_eq!(category_to_cwe(OwaspCategory::Api7Ssrf), Some("CWE-918".to_string()));
    }
}
