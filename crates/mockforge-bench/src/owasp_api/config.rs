//! OWASP API Security Top 10 Configuration
//!
//! This module defines the configuration for running OWASP API Top 10 security tests.

use super::categories::{OwaspCategory, Severity};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// Configuration for OWASP API Security Top 10 testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwaspApiConfig {
    /// Categories to test (empty = all categories)
    #[serde(default)]
    pub categories: HashSet<OwaspCategory>,

    /// Authorization header name for auth bypass tests
    #[serde(default = "default_auth_header")]
    pub auth_header: String,

    /// File containing admin/privileged paths to test
    #[serde(default)]
    pub admin_paths_file: Option<PathBuf>,

    /// List of admin/privileged paths to test
    #[serde(default)]
    pub admin_paths: Vec<String>,

    /// Fields containing resource IDs for BOLA testing
    #[serde(default = "default_id_fields")]
    pub id_fields: Vec<String>,

    /// Valid authorization token for baseline requests
    #[serde(default)]
    pub valid_auth_token: Option<String>,

    /// Alternative authorization tokens for testing (e.g., different user roles)
    #[serde(default)]
    pub alt_auth_tokens: Vec<AuthToken>,

    /// Output report file path
    #[serde(default = "default_report_path")]
    pub report_path: PathBuf,

    /// Report format (json, sarif)
    #[serde(default)]
    pub report_format: ReportFormat,

    /// Minimum severity level to report
    #[serde(default)]
    pub min_severity: Severity,

    /// Rate limiting configuration for API4 tests
    #[serde(default)]
    pub rate_limit_config: RateLimitConfig,

    /// SSRF-specific configuration for API7 tests
    #[serde(default)]
    pub ssrf_config: SsrfConfig,

    /// Discovery-specific configuration for API9 tests
    #[serde(default)]
    pub discovery_config: DiscoveryConfig,

    /// Enable verbose output during testing
    #[serde(default)]
    pub verbose: bool,

    /// Number of concurrent test requests
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,

    /// Request timeout in milliseconds
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    /// Skip TLS certificate verification (for testing with self-signed certs)
    #[serde(default)]
    pub insecure: bool,
}

fn default_auth_header() -> String {
    "Authorization".to_string()
}

fn default_id_fields() -> Vec<String> {
    vec![
        "id".to_string(),
        "uuid".to_string(),
        "user_id".to_string(),
        "userId".to_string(),
        "account_id".to_string(),
        "accountId".to_string(),
        "resource_id".to_string(),
        "resourceId".to_string(),
    ]
}

fn default_report_path() -> PathBuf {
    PathBuf::from("owasp-report.json")
}

fn default_concurrency() -> usize {
    10
}

fn default_timeout() -> u64 {
    30000
}

impl Default for OwaspApiConfig {
    fn default() -> Self {
        Self {
            categories: HashSet::new(),
            auth_header: default_auth_header(),
            admin_paths_file: None,
            admin_paths: Vec::new(),
            id_fields: default_id_fields(),
            valid_auth_token: None,
            alt_auth_tokens: Vec::new(),
            report_path: default_report_path(),
            report_format: ReportFormat::default(),
            min_severity: Severity::Low,
            rate_limit_config: RateLimitConfig::default(),
            ssrf_config: SsrfConfig::default(),
            discovery_config: DiscoveryConfig::default(),
            verbose: false,
            concurrency: default_concurrency(),
            timeout_ms: default_timeout(),
            insecure: false,
        }
    }
}

impl OwaspApiConfig {
    /// Create a new configuration with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the categories to test (all if none specified)
    pub fn categories_to_test(&self) -> Vec<OwaspCategory> {
        if self.categories.is_empty() {
            OwaspCategory::all()
        } else {
            self.categories.iter().copied().collect()
        }
    }

    /// Check if a specific category should be tested
    pub fn should_test_category(&self, category: OwaspCategory) -> bool {
        self.categories.is_empty() || self.categories.contains(&category)
    }

    /// Load admin paths from file if specified
    pub fn load_admin_paths(&mut self) -> Result<(), std::io::Error> {
        if let Some(ref path) = self.admin_paths_file {
            let content = std::fs::read_to_string(path)?;
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    self.admin_paths.push(trimmed.to_string());
                }
            }
        }
        Ok(())
    }

    /// Get all admin paths (from file and explicit list)
    pub fn all_admin_paths(&self) -> Vec<&str> {
        let mut paths: Vec<&str> = self.admin_paths.iter().map(String::as_str).collect();

        // Add default admin paths if none specified
        if paths.is_empty() {
            paths.extend(DEFAULT_ADMIN_PATHS.iter().copied());
        }

        paths
    }

    /// Builder method to set categories
    pub fn with_categories(mut self, categories: impl IntoIterator<Item = OwaspCategory>) -> Self {
        self.categories = categories.into_iter().collect();
        self
    }

    /// Builder method to set auth header
    pub fn with_auth_header(mut self, header: impl Into<String>) -> Self {
        self.auth_header = header.into();
        self
    }

    /// Builder method to set valid auth token
    pub fn with_valid_auth_token(mut self, token: impl Into<String>) -> Self {
        self.valid_auth_token = Some(token.into());
        self
    }

    /// Builder method to add admin paths
    pub fn with_admin_paths(mut self, paths: impl IntoIterator<Item = String>) -> Self {
        self.admin_paths.extend(paths);
        self
    }

    /// Builder method to set ID fields
    pub fn with_id_fields(mut self, fields: impl IntoIterator<Item = String>) -> Self {
        self.id_fields = fields.into_iter().collect();
        self
    }

    /// Builder method to set report path
    pub fn with_report_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.report_path = path.into();
        self
    }

    /// Builder method to set report format
    pub fn with_report_format(mut self, format: ReportFormat) -> Self {
        self.report_format = format;
        self
    }

    /// Builder method to set verbosity
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Builder method to set insecure TLS mode
    pub fn with_insecure(mut self, insecure: bool) -> Self {
        self.insecure = insecure;
        self
    }
}

/// Authentication token with optional metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    /// The token value (including type prefix like "Bearer ")
    pub value: String,
    /// Role or description of this token
    #[serde(default)]
    pub role: Option<String>,
    /// User ID associated with this token
    #[serde(default)]
    pub user_id: Option<String>,
}

impl AuthToken {
    /// Create a new auth token
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            role: None,
            user_id: None,
        }
    }

    /// Create with role information
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    /// Create with user ID
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }
}

/// Report output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    /// JSON format with detailed findings
    #[default]
    Json,
    /// SARIF format for IDE/CI integration
    Sarif,
}

impl std::str::FromStr for ReportFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "sarif" => Ok(Self::Sarif),
            _ => Err(format!("Unknown report format: '{}'. Valid values: json, sarif", s)),
        }
    }
}

/// Configuration for rate limit testing (API4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Number of rapid requests to send
    #[serde(default = "default_burst_size")]
    pub burst_size: usize,
    /// Maximum pagination limit to test
    #[serde(default = "default_max_limit")]
    pub max_limit: usize,
    /// Large payload size in bytes for resource exhaustion
    #[serde(default = "default_large_payload_size")]
    pub large_payload_size: usize,
    /// Maximum nesting depth for JSON bodies
    #[serde(default = "default_max_nesting")]
    pub max_nesting_depth: usize,
}

fn default_burst_size() -> usize {
    100
}

fn default_max_limit() -> usize {
    100000
}

fn default_large_payload_size() -> usize {
    10 * 1024 * 1024 // 10MB
}

fn default_max_nesting() -> usize {
    100
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            burst_size: default_burst_size(),
            max_limit: default_max_limit(),
            large_payload_size: default_large_payload_size(),
            max_nesting_depth: default_max_nesting(),
        }
    }
}

/// Configuration for SSRF testing (API7)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsrfConfig {
    /// Internal URLs to test for SSRF
    #[serde(default = "default_internal_urls")]
    pub internal_urls: Vec<String>,
    /// Cloud metadata URLs to test
    #[serde(default = "default_metadata_urls")]
    pub metadata_urls: Vec<String>,
    /// Additional URL fields to test beyond defaults
    #[serde(default)]
    pub url_fields: Vec<String>,
}

fn default_internal_urls() -> Vec<String> {
    vec![
        "http://localhost/".to_string(),
        "http://127.0.0.1/".to_string(),
        "http://[::1]/".to_string(),
        "http://0.0.0.0/".to_string(),
        "http://localhost:8080/".to_string(),
        "http://localhost:3000/".to_string(),
        "http://localhost:9000/".to_string(),
        "http://internal/".to_string(),
        "http://backend/".to_string(),
    ]
}

fn default_metadata_urls() -> Vec<String> {
    vec![
        // AWS
        "http://169.254.169.254/latest/meta-data/".to_string(),
        "http://169.254.169.254/latest/user-data/".to_string(),
        // GCP
        "http://metadata.google.internal/computeMetadata/v1/".to_string(),
        // Azure
        "http://169.254.169.254/metadata/instance".to_string(),
        // DigitalOcean
        "http://169.254.169.254/metadata/v1/".to_string(),
        // Alibaba Cloud
        "http://100.100.100.200/latest/meta-data/".to_string(),
    ]
}

impl Default for SsrfConfig {
    fn default() -> Self {
        Self {
            internal_urls: default_internal_urls(),
            metadata_urls: default_metadata_urls(),
            url_fields: vec![
                "url".to_string(),
                "uri".to_string(),
                "link".to_string(),
                "href".to_string(),
                "callback".to_string(),
                "redirect".to_string(),
                "return_url".to_string(),
                "webhook".to_string(),
                "image_url".to_string(),
                "fetch_url".to_string(),
            ],
        }
    }
}

/// Configuration for endpoint discovery (API9)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// API versions to probe
    #[serde(default = "default_api_versions")]
    pub api_versions: Vec<String>,
    /// Common debug/internal endpoints to discover
    #[serde(default = "default_discovery_paths")]
    pub discovery_paths: Vec<String>,
    /// Check for deprecated endpoints
    #[serde(default = "default_true")]
    pub check_deprecated: bool,
}

fn default_api_versions() -> Vec<String> {
    vec![
        "v1".to_string(),
        "v2".to_string(),
        "v3".to_string(),
        "v4".to_string(),
        "api/v1".to_string(),
        "api/v2".to_string(),
        "api/v3".to_string(),
    ]
}

fn default_discovery_paths() -> Vec<String> {
    vec![
        "/swagger".to_string(),
        "/swagger-ui".to_string(),
        "/swagger.json".to_string(),
        "/swagger.yaml".to_string(),
        "/api-docs".to_string(),
        "/openapi".to_string(),
        "/openapi.json".to_string(),
        "/openapi.yaml".to_string(),
        "/graphql".to_string(),
        "/graphiql".to_string(),
        "/playground".to_string(),
        "/debug".to_string(),
        "/debug/".to_string(),
        "/actuator".to_string(),
        "/actuator/health".to_string(),
        "/actuator/info".to_string(),
        "/actuator/env".to_string(),
        "/metrics".to_string(),
        "/health".to_string(),
        "/healthz".to_string(),
        "/status".to_string(),
        "/info".to_string(),
        "/.env".to_string(),
        "/config".to_string(),
        "/admin".to_string(),
        "/internal".to_string(),
        "/test".to_string(),
        "/dev".to_string(),
    ]
}

fn default_true() -> bool {
    true
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            api_versions: default_api_versions(),
            discovery_paths: default_discovery_paths(),
            check_deprecated: default_true(),
        }
    }
}

/// Default admin paths to test for privilege escalation
pub const DEFAULT_ADMIN_PATHS: &[&str] = &[
    "/admin",
    "/admin/",
    "/admin/users",
    "/admin/settings",
    "/admin/config",
    "/api/admin",
    "/api/admin/",
    "/api/admin/users",
    "/api/v1/admin",
    "/api/v2/admin",
    "/management",
    "/manage",
    "/internal",
    "/internal/",
    "/system",
    "/system/config",
    "/settings",
    "/config",
    "/users/admin",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OwaspApiConfig::default();
        assert!(config.categories.is_empty());
        assert_eq!(config.auth_header, "Authorization");
        assert!(!config.id_fields.is_empty());
    }

    #[test]
    fn test_categories_to_test() {
        let config = OwaspApiConfig::default();
        assert_eq!(config.categories_to_test().len(), 10);

        let config = OwaspApiConfig::default()
            .with_categories([OwaspCategory::Api1Bola, OwaspCategory::Api7Ssrf]);
        assert_eq!(config.categories_to_test().len(), 2);
    }

    #[test]
    fn test_should_test_category() {
        let config = OwaspApiConfig::default();
        assert!(config.should_test_category(OwaspCategory::Api1Bola));

        let config = OwaspApiConfig::default().with_categories([OwaspCategory::Api1Bola]);
        assert!(config.should_test_category(OwaspCategory::Api1Bola));
        assert!(!config.should_test_category(OwaspCategory::Api2BrokenAuth));
    }

    #[test]
    fn test_builder_pattern() {
        let config = OwaspApiConfig::new()
            .with_auth_header("X-Auth-Token")
            .with_valid_auth_token("secret123")
            .with_verbose(true);

        assert_eq!(config.auth_header, "X-Auth-Token");
        assert_eq!(config.valid_auth_token, Some("secret123".to_string()));
        assert!(config.verbose);
    }

    #[test]
    fn test_report_format_from_str() {
        assert_eq!("json".parse::<ReportFormat>().unwrap(), ReportFormat::Json);
        assert_eq!("sarif".parse::<ReportFormat>().unwrap(), ReportFormat::Sarif);
        assert!("invalid".parse::<ReportFormat>().is_err());
    }
}
