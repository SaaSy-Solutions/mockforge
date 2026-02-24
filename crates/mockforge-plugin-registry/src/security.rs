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

    async fn scan_for_malware(&self, package_path: &Path) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        let suspicious_patterns = [
            "backdoor",
            "keylogger",
            "trojan",
            "ransomware",
            "cryptominer",
            "rootkit",
            "exploit",
        ];

        Self::walk_files(package_path, self.config.max_file_size, &mut |path| {
            let file_name =
                path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();

            for pattern in &suspicious_patterns {
                if file_name.contains(pattern) {
                    findings.push(Finding {
                        id: format!("MAL-FILENAME-{}", uuid::Uuid::new_v4()),
                        severity: Severity::High,
                        category: Category::Malware,
                        title: format!("Suspicious file name: {}", file_name),
                        description: format!(
                            "File name contains suspicious pattern '{}'. This may indicate malicious content.",
                            pattern
                        ),
                        location: Some(path.display().to_string()),
                        recommendation: "Review the file contents and remove if malicious.".to_string(),
                        references: vec!["CWE-506: Embedded Malicious Code".to_string()],
                    });
                }
            }
        });

        Ok(findings)
    }

    async fn scan_dependencies(&self, package_path: &Path) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        // Check Cargo.toml for suspicious dependency patterns
        let cargo_toml_path = package_path.join("Cargo.toml");
        if cargo_toml_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml_path) {
                // Check for git dependencies pointing to non-standard registries
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.contains("git = \"")
                        && !trimmed.contains("github.com")
                        && !trimmed.contains("gitlab.com")
                    {
                        findings.push(Finding {
                            id: format!("DEP-GIT-{}", uuid::Uuid::new_v4()),
                            severity: Severity::Medium,
                            category: Category::SupplyChain,
                            title: "Non-standard git dependency source".to_string(),
                            description: format!(
                                "Dependency uses a non-standard git repository: {}",
                                trimmed
                            ),
                            location: Some(cargo_toml_path.display().to_string()),
                            recommendation: "Verify the dependency source is trusted.".to_string(),
                            references: vec![
                                "CWE-829: Inclusion of Functionality from Untrusted Control Sphere"
                                    .to_string(),
                            ],
                        });
                    }
                    // Check for path dependencies that escape the package
                    if trimmed.contains("path = \"") && trimmed.contains("..") {
                        findings.push(Finding {
                            id: format!("DEP-PATH-{}", uuid::Uuid::new_v4()),
                            severity: Severity::Low,
                            category: Category::SupplyChain,
                            title: "Path dependency with parent traversal".to_string(),
                            description: format!(
                                "Dependency uses a relative path that traverses parent directories: {}",
                                trimmed
                            ),
                            location: Some(cargo_toml_path.display().to_string()),
                            recommendation: "Ensure path dependencies don't reference files outside the package.".to_string(),
                            references: vec![],
                        });
                    }
                }
            }
        }

        // Check package.json for npm dependency issues
        let package_json_path = package_path.join("package.json");
        if package_json_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&package_json_path) {
                // Check for install scripts (common vector for supply chain attacks)
                if content.contains("\"preinstall\"") || content.contains("\"postinstall\"") {
                    findings.push(Finding {
                        id: format!("DEP-SCRIPT-{}", uuid::Uuid::new_v4()),
                        severity: Severity::Medium,
                        category: Category::SupplyChain,
                        title: "Package contains install scripts".to_string(),
                        description: "Package defines preinstall or postinstall scripts which can execute arbitrary code during installation.".to_string(),
                        location: Some(package_json_path.display().to_string()),
                        recommendation: "Review install scripts for malicious behavior.".to_string(),
                        references: vec!["CWE-829: Inclusion of Functionality from Untrusted Control Sphere".to_string()],
                    });
                }
            }
        }

        Ok(findings)
    }

    async fn static_analysis(&self, package_path: &Path) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        let secret_patterns = [
            ("password", "Hardcoded password"),
            ("secret_key", "Hardcoded secret key"),
            ("api_key", "Hardcoded API key"),
            ("private_key", "Hardcoded private key"),
            ("access_token", "Hardcoded access token"),
        ];

        Self::walk_files(package_path, self.config.max_file_size, &mut |path| {
            let ext =
                path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();

            // Only scan source code files
            if !matches!(ext.as_str(), "rs" | "js" | "ts" | "py" | "go" | "java" | "rb" | "sh") {
                return;
            }

            let Ok(content) = std::fs::read_to_string(path) else {
                return;
            };

            // Check for unsafe blocks in Rust
            if ext == "rs" && content.contains("unsafe {") {
                let unsafe_count = content.matches("unsafe {").count();
                findings.push(Finding {
                    id: format!("SA-UNSAFE-{}", uuid::Uuid::new_v4()),
                    severity: Severity::Medium,
                    category: Category::InsecureCoding,
                    title: format!("Contains {} unsafe block(s)", unsafe_count),
                    description: "Unsafe code can lead to memory safety issues. Each unsafe block should be carefully reviewed.".to_string(),
                    location: Some(path.display().to_string()),
                    recommendation: "Ensure all unsafe blocks have SAFETY comments and are truly necessary.".to_string(),
                    references: vec!["CWE-119: Buffer Overflow".to_string()],
                });
            }

            // Check for hardcoded credentials
            for (pattern, description) in &secret_patterns {
                for (line_num, line) in content.lines().enumerate() {
                    let lower = line.to_lowercase();
                    // Look for assignment patterns like: password = "...", api_key: "..."
                    if lower.contains(pattern)
                        && (line.contains("= \"") || line.contains(": \"") || line.contains("=\""))
                        && !line.trim_start().starts_with("//")
                        && !line.trim_start().starts_with('#')
                        && !line.trim_start().starts_with("///")
                    {
                        findings.push(Finding {
                            id: format!("SA-SECRET-{}", uuid::Uuid::new_v4()),
                            severity: Severity::High,
                            category: Category::DataExfiltration,
                            title: format!("{} detected", description),
                            description: format!(
                                "Possible hardcoded credential at line {}. Secrets should be loaded from environment variables or a secret manager.",
                                line_num + 1
                            ),
                            location: Some(format!("{}:{}", path.display(), line_num + 1)),
                            recommendation: "Move credentials to environment variables or a secrets manager.".to_string(),
                            references: vec!["CWE-798: Use of Hard-coded Credentials".to_string()],
                        });
                        break; // Only report once per pattern per file
                    }
                }
            }

            // Check for command injection patterns
            if ext == "rs" && (content.contains("Command::new") && content.contains(".arg(")) {
                // Only flag if user input might flow in (heuristic: function parameters used in Command)
                if content.contains("std::process::Command") {
                    findings.push(Finding {
                        id: format!("SA-CMDINJ-{}", uuid::Uuid::new_v4()),
                        severity: Severity::Low,
                        category: Category::InsecureCoding,
                        title: "External command execution detected".to_string(),
                        description: "Code executes external commands. Ensure arguments are properly sanitized.".to_string(),
                        location: Some(path.display().to_string()),
                        recommendation: "Validate and sanitize all inputs passed to external commands.".to_string(),
                        references: vec!["CWE-78: OS Command Injection".to_string()],
                    });
                }
            }
        });

        Ok(findings)
    }

    async fn check_license_compliance(&self, package_path: &Path) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();

        // Check for license file presence
        let has_license = package_path.join("LICENSE").exists()
            || package_path.join("LICENSE.md").exists()
            || package_path.join("LICENSE.txt").exists()
            || package_path.join("LICENCE").exists();

        if !has_license {
            findings.push(Finding {
                id: format!("LIC-MISSING-{}", uuid::Uuid::new_v4()),
                severity: Severity::Medium,
                category: Category::Licensing,
                title: "No LICENSE file found".to_string(),
                description:
                    "Package does not contain a LICENSE file. License must be clearly specified."
                        .to_string(),
                location: Some(package_path.display().to_string()),
                recommendation: "Add a LICENSE file with an approved open source license."
                    .to_string(),
                references: vec![],
            });
        }

        // Check Cargo.toml for license field
        let cargo_toml_path = package_path.join("Cargo.toml");
        if cargo_toml_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml_path) {
                let has_license_field = content.lines().any(|line| {
                    let trimmed = line.trim();
                    trimmed.starts_with("license ")
                        || trimmed.starts_with("license=")
                        || trimmed.starts_with("license-file")
                });

                if !has_license_field {
                    findings.push(Finding {
                        id: format!("LIC-CARGO-{}", uuid::Uuid::new_v4()),
                        severity: Severity::Low,
                        category: Category::Licensing,
                        title: "No license field in Cargo.toml".to_string(),
                        description: "Cargo.toml does not specify a license or license-file field."
                            .to_string(),
                        location: Some(cargo_toml_path.display().to_string()),
                        recommendation:
                            "Add a 'license' field to Cargo.toml with an SPDX identifier."
                                .to_string(),
                        references: vec![],
                    });
                } else {
                    // Check if license is in allowed list
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if (trimmed.starts_with("license ") || trimmed.starts_with("license="))
                            && !trimmed.starts_with("license-file")
                        {
                            let license_value = trimmed
                                .split('=')
                                .nth(1)
                                .unwrap_or("")
                                .trim()
                                .trim_matches('"')
                                .trim_matches('\'');

                            // Check each license in an OR expression
                            let all_allowed = license_value.split(" OR ").all(|l| {
                                self.config.allowed_licenses.iter().any(|a| a == l.trim())
                            });

                            if !all_allowed && !license_value.is_empty() {
                                findings.push(Finding {
                                    id: format!("LIC-UNAPPROVED-{}", uuid::Uuid::new_v4()),
                                    severity: Severity::Medium,
                                    category: Category::Licensing,
                                    title: format!("License '{}' may not be approved", license_value),
                                    description: format!(
                                        "The license '{}' is not in the approved license list: {:?}",
                                        license_value, self.config.allowed_licenses
                                    ),
                                    location: Some(cargo_toml_path.display().to_string()),
                                    recommendation: "Use an approved license or request an exception.".to_string(),
                                    references: vec![],
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(findings)
    }

    /// Walk files in a directory, calling the callback for each file within the size limit.
    fn walk_files(dir: &Path, max_size: u64, callback: &mut dyn FnMut(&Path)) {
        let mut stack = vec![dir.to_path_buf()];
        while let Some(current) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&current) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Ok(meta) = std::fs::metadata(&path) {
                    if meta.len() <= max_size {
                        callback(&path);
                    }
                }
            }
        }
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
