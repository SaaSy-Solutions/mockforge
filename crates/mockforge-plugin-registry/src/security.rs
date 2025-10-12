//! Plugin security scanning and validation

use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Security scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// Overall status
    pub status: ScanStatus,

    /// Security score (0-100)
    pub score: u8,

    /// Findings by severity
    pub findings: Vec<Finding>,

    /// Scan metadata
    pub metadata: ScanMetadata,
}

/// Scan status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ScanStatus {
    Pass,
    Warning,
    Fail,
}

/// Security finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Finding ID
    pub id: String,

    /// Severity level
    pub severity: Severity,

    /// Finding category
    pub category: Category,

    /// Title
    pub title: String,

    /// Description
    pub description: String,

    /// Location (file path, line number, etc.)
    pub location: Option<String>,

    /// Recommendation
    pub recommendation: String,

    /// References (CVE, CWE, etc.)
    pub references: Vec<String>,
}

/// Severity level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// Finding category
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Malware,
    VulnerableDependency,
    InsecureCoding,
    DataExfiltration,
    SupplyChain,
    Licensing,
    Configuration,
    Obfuscation,
    Other,
}

/// Scan metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanMetadata {
    pub scan_id: String,
    pub scanner_version: String,
    pub scan_started_at: String,
    pub scan_completed_at: String,
    pub duration_ms: u64,
    pub scanned_files: u32,
    pub scanned_bytes: u64,
}

/// Security scanner configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    /// Enable malware detection
    pub enable_malware_scan: bool,

    /// Enable dependency vulnerability scanning
    pub enable_dependency_scan: bool,

    /// Enable static code analysis
    pub enable_static_analysis: bool,

    /// Enable license compliance check
    pub enable_license_check: bool,

    /// Maximum file size to scan (bytes)
    pub max_file_size: u64,

    /// Timeout per file (seconds)
    pub timeout_per_file: u64,

    /// Allowed licenses
    pub allowed_licenses: Vec<String>,

    /// Severity threshold for failure
    pub fail_on_severity: Severity,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            enable_malware_scan: true,
            enable_dependency_scan: true,
            enable_static_analysis: true,
            enable_license_check: true,
            max_file_size: 10 * 1024 * 1024, // 10MB
            timeout_per_file: 30,
            allowed_licenses: vec![
                "MIT".to_string(),
                "Apache-2.0".to_string(),
                "BSD-2-Clause".to_string(),
                "BSD-3-Clause".to_string(),
                "ISC".to_string(),
                "MPL-2.0".to_string(),
            ],
            fail_on_severity: Severity::High,
        }
    }
}

/// Security scanner
pub struct SecurityScanner {
    config: ScannerConfig,
}

impl SecurityScanner {
    /// Create a new scanner with config
    pub fn new(config: ScannerConfig) -> Self {
        Self { config }
    }

    /// Scan a plugin package
    pub async fn scan_plugin(&self, package_path: &Path) -> Result<ScanResult> {
        let start_time = std::time::Instant::now();
        let scan_id = uuid::Uuid::new_v4().to_string();

        let mut findings = Vec::new();
        let scanned_files = 0;
        let scanned_bytes = 0;

        // 1. Malware detection
        if self.config.enable_malware_scan {
            findings.extend(self.scan_for_malware(package_path).await?);
        }

        // 2. Dependency vulnerability scan
        if self.config.enable_dependency_scan {
            findings.extend(self.scan_dependencies(package_path).await?);
        }

        // 3. Static code analysis
        if self.config.enable_static_analysis {
            findings.extend(self.static_analysis(package_path).await?);
        }

        // 4. License compliance
        if self.config.enable_license_check {
            findings.extend(self.check_license_compliance(package_path).await?);
        }

        // Calculate score and status
        let score = self.calculate_security_score(&findings);
        let status = self.determine_status(&findings);

        let duration = start_time.elapsed();

        Ok(ScanResult {
            status,
            score,
            findings,
            metadata: ScanMetadata {
                scan_id,
                scanner_version: env!("CARGO_PKG_VERSION").to_string(),
                scan_started_at: chrono::Utc::now().to_rfc3339(),
                scan_completed_at: chrono::Utc::now().to_rfc3339(),
                duration_ms: duration.as_millis() as u64,
                scanned_files,
                scanned_bytes,
            },
        })
    }

    async fn scan_for_malware(&self, _package_path: &Path) -> Result<Vec<Finding>> {
        let findings = Vec::new();

        // Check for suspicious patterns
        // In production, integrate with actual AV/malware scanning service
        // Examples: ClamAV, VirusTotal API, etc.

        // Placeholder: Check for suspicious file names
        let _suspicious_patterns = [
            "backdoor",
            "keylogger",
            "trojan",
            "ransomware",
            "cryptominer",
            "rootkit",
            "exploit",
        ];

        // This would scan actual files
        // For now, return empty findings

        Ok(findings)
    }

    async fn scan_dependencies(&self, _package_path: &Path) -> Result<Vec<Finding>> {
        let findings = Vec::new();

        // In production, integrate with:
        // - RustSec Advisory Database (cargo-audit)
        // - npm audit for JavaScript
        // - pip-audit for Python
        // - OSV (Open Source Vulnerabilities) database

        // Placeholder implementation

        Ok(findings)
    }

    async fn static_analysis(&self, _package_path: &Path) -> Result<Vec<Finding>> {
        let findings = Vec::new();

        // In production, run:
        // - cargo clippy for Rust
        // - eslint for JavaScript
        // - pylint/bandit for Python
        // - semgrep for multi-language

        // Check for common issues:
        // - Unsafe code blocks
        // - Hardcoded credentials
        // - SQL injection patterns
        // - Command injection
        // - Path traversal vulnerabilities

        Ok(findings)
    }

    async fn check_license_compliance(&self, _package_path: &Path) -> Result<Vec<Finding>> {
        let findings = Vec::new();

        // Check if license is in allowed list
        // Scan for license headers in code files
        // Check dependency licenses

        Ok(findings)
    }

    fn calculate_security_score(&self, findings: &[Finding]) -> u8 {
        let mut score: u8 = 100;

        for finding in findings {
            let deduction = match finding.severity {
                Severity::Critical => 30,
                Severity::High => 20,
                Severity::Medium => 10,
                Severity::Low => 5,
                Severity::Info => 0,
            };
            score = score.saturating_sub(deduction);
        }

        score
    }

    fn determine_status(&self, findings: &[Finding]) -> ScanStatus {
        let has_critical = findings.iter().any(|f| f.severity >= self.config.fail_on_severity);

        if has_critical {
            ScanStatus::Fail
        } else if findings.iter().any(|f| f.severity >= Severity::Medium) {
            ScanStatus::Warning
        } else {
            ScanStatus::Pass
        }
    }
}

impl Default for SecurityScanner {
    fn default() -> Self {
        Self::new(ScannerConfig::default())
    }
}

/// Vulnerability database entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: String,
    pub package: String,
    pub versions: Vec<String>,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub cvss_score: Option<f32>,
    pub cve: Option<String>,
    pub patched_versions: Vec<String>,
    pub references: Vec<String>,
}

/// License information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    pub spdx_id: String,
    pub name: String,
    pub approved: bool,
    pub osi_approved: bool,
    pub category: LicenseCategory,
}

/// License category
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LicenseCategory {
    Permissive,
    Copyleft,
    Proprietary,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_score_calculation() {
        let scanner = SecurityScanner::default();

        let findings = vec![
            Finding {
                id: "1".to_string(),
                severity: Severity::High,
                category: Category::Malware,
                title: "Suspicious code".to_string(),
                description: "Test".to_string(),
                location: None,
                recommendation: "Remove".to_string(),
                references: vec![],
            },
            Finding {
                id: "2".to_string(),
                severity: Severity::Medium,
                category: Category::InsecureCoding,
                title: "Weak encryption".to_string(),
                description: "Test".to_string(),
                location: None,
                recommendation: "Use strong encryption".to_string(),
                references: vec![],
            },
        ];

        let score = scanner.calculate_security_score(&findings);
        assert_eq!(score, 70); // 100 - 20 - 10
    }

    #[test]
    fn test_status_determination() {
        let scanner = SecurityScanner::default();

        let critical_findings = vec![Finding {
            id: "1".to_string(),
            severity: Severity::Critical,
            category: Category::Malware,
            title: "Malware detected".to_string(),
            description: "Test".to_string(),
            location: None,
            recommendation: "Remove".to_string(),
            references: vec![],
        }];

        assert_eq!(scanner.determine_status(&critical_findings), ScanStatus::Fail);

        let medium_findings = vec![Finding {
            id: "1".to_string(),
            severity: Severity::Medium,
            category: Category::InsecureCoding,
            title: "Code issue".to_string(),
            description: "Test".to_string(),
            location: None,
            recommendation: "Fix".to_string(),
            references: vec![],
        }];

        assert_eq!(scanner.determine_status(&medium_findings), ScanStatus::Warning);

        let low_findings = vec![Finding {
            id: "1".to_string(),
            severity: Severity::Low,
            category: Category::Configuration,
            title: "Config issue".to_string(),
            description: "Test".to_string(),
            location: None,
            recommendation: "Update".to_string(),
            references: vec![],
        }];

        assert_eq!(scanner.determine_status(&low_findings), ScanStatus::Pass);
    }
}
