//! WAFBench YAML parser for importing CRS (Core Rule Set) attack patterns
//!
//! This module parses WAFBench YAML test files from the Microsoft WAFBench project
//! (<https://github.com/microsoft/WAFBench>) and converts them into security test payloads
//! compatible with MockForge's security testing framework.
//!
//! # WAFBench YAML Format
//!
//! WAFBench test files follow this structure:
//! ```yaml
//! meta:
//!   author: "author-name"
//!   description: "Tests for rule XXXXXX"
//!   enabled: true
//!   name: "XXXXXX.yaml"
//!
//! tests:
//!   - desc: "Attack scenario description"
//!     test_title: "XXXXXX-N"
//!     stages:
//!       - input:
//!           dest_addr: "127.0.0.1"
//!           headers:
//!             Host: "localhost"
//!             User-Agent: "Mozilla/5.0"
//!           method: "GET"
//!           port: 80
//!           uri: "/path?param=<script>alert(1)</script>"
//!         output:
//!           status: [200, 403, 404]
//! ```
//!
//! # Usage
//!
//! ```bash
//! mockforge bench spec.yaml --wafbench-dir ./wafbench/REQUEST-941-*
//! ```

use crate::error::{BenchError, Result};
use crate::security_payloads::{SecurityCategory, SecurityPayload};
use glob::glob;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// WAFBench test file metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WafBenchMeta {
    /// Author of the test file
    pub author: Option<String>,
    /// Description of what the tests cover
    pub description: Option<String>,
    /// Whether the tests are enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Name of the test file
    pub name: Option<String>,
}

fn default_enabled() -> bool {
    true
}

/// A single WAFBench test case
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WafBenchTest {
    /// Description of the attack scenario
    pub desc: Option<String>,
    /// Unique test identifier (e.g., "941100-1")
    pub test_title: String,
    /// Test stages (request/response pairs)
    #[serde(default)]
    pub stages: Vec<WafBenchStage>,
}

/// A test stage containing input (request) and expected output (response)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WafBenchStage {
    /// The request configuration
    pub input: WafBenchInput,
    /// Expected response
    pub output: Option<WafBenchOutput>,
}

/// Request configuration for a WAFBench test
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WafBenchInput {
    /// Target address
    pub dest_addr: Option<String>,
    /// HTTP headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// HTTP method
    #[serde(default = "default_method")]
    pub method: String,
    /// Target port
    #[serde(default = "default_port")]
    pub port: u16,
    /// Request URI (may contain attack payloads)
    pub uri: Option<String>,
    /// Request body data
    pub data: Option<String>,
    /// Protocol version
    pub version: Option<String>,
}

fn default_method() -> String {
    "GET".to_string()
}

fn default_port() -> u16 {
    80
}

/// Expected response for a WAFBench test
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WafBenchOutput {
    /// Expected HTTP status codes (any match is valid)
    #[serde(default)]
    pub status: Vec<u16>,
    /// Expected response headers
    #[serde(default)]
    pub response_headers: HashMap<String, String>,
    /// Log contains patterns
    #[serde(default)]
    pub log_contains: Vec<String>,
    /// Log does not contain patterns
    #[serde(default)]
    pub no_log_contains: Vec<String>,
}

/// Complete WAFBench test file structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WafBenchFile {
    /// Test file metadata
    pub meta: WafBenchMeta,
    /// Test cases
    #[serde(default)]
    pub tests: Vec<WafBenchTest>,
}

/// A parsed WAFBench test case ready for use in security testing
#[derive(Debug, Clone)]
pub struct WafBenchTestCase {
    /// Test identifier
    pub test_id: String,
    /// Description
    pub description: String,
    /// CRS rule ID (e.g., 941100)
    pub rule_id: String,
    /// Security category
    pub category: SecurityCategory,
    /// HTTP method
    pub method: String,
    /// Attack payloads extracted from the test
    pub payloads: Vec<WafBenchPayload>,
    /// Expected to be blocked (403)
    pub expects_block: bool,
}

/// A specific payload from a WAFBench test
#[derive(Debug, Clone)]
pub struct WafBenchPayload {
    /// The payload location (uri, header, body)
    pub location: PayloadLocation,
    /// The actual payload string
    pub value: String,
    /// Header name if location is Header
    pub header_name: Option<String>,
}

/// Where the payload is injected
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadLocation {
    /// Payload in URI/query string
    Uri,
    /// Payload in HTTP header
    Header,
    /// Payload in request body
    Body,
}

impl std::fmt::Display for PayloadLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Uri => write!(f, "uri"),
            Self::Header => write!(f, "header"),
            Self::Body => write!(f, "body"),
        }
    }
}

/// WAFBench loader and parser
pub struct WafBenchLoader {
    /// Loaded test cases
    test_cases: Vec<WafBenchTestCase>,
    /// Statistics
    stats: WafBenchStats,
}

/// Statistics about loaded WAFBench tests
#[derive(Debug, Clone, Default)]
pub struct WafBenchStats {
    /// Number of files processed
    pub files_processed: usize,
    /// Number of test cases loaded
    pub test_cases_loaded: usize,
    /// Number of payloads extracted
    pub payloads_extracted: usize,
    /// Tests by category
    pub by_category: HashMap<SecurityCategory, usize>,
    /// Files that failed to parse
    pub parse_errors: Vec<String>,
}

impl WafBenchLoader {
    /// Create a new empty loader
    pub fn new() -> Self {
        Self {
            test_cases: Vec::new(),
            stats: WafBenchStats::default(),
        }
    }

    /// Load WAFBench tests from a directory pattern (supports glob)
    ///
    /// # Arguments
    /// * `pattern` - Glob pattern like `./wafbench/REQUEST-941-*` or a direct path
    ///
    /// # Example
    /// ```ignore
    /// let loader = WafBenchLoader::new();
    /// loader.load_from_pattern("./wafbench/REQUEST-941-APPLICATION-ATTACK-XSS/**/*.yaml")?;
    /// ```
    pub fn load_from_pattern(&mut self, pattern: &str) -> Result<()> {
        // If pattern doesn't contain wildcards, treat as directory
        if !pattern.contains('*') && !pattern.contains('?') {
            return self.load_from_directory(Path::new(pattern));
        }

        // Use glob to find matching files
        let entries = glob(pattern).map_err(|e| {
            BenchError::Other(format!("Invalid WAFBench pattern '{}': {}", pattern, e))
        })?;

        for entry in entries {
            match entry {
                Ok(path) => {
                    if path.is_file()
                        && path.extension().is_some_and(|ext| ext == "yaml" || ext == "yml")
                    {
                        if let Err(e) = self.load_file(&path) {
                            self.stats.parse_errors.push(format!("{}: {}", path.display(), e));
                        }
                    } else if path.is_dir() {
                        if let Err(e) = self.load_from_directory(&path) {
                            self.stats.parse_errors.push(format!("{}: {}", path.display(), e));
                        }
                    }
                }
                Err(e) => {
                    self.stats.parse_errors.push(format!("Glob error: {}", e));
                }
            }
        }

        Ok(())
    }

    /// Load WAFBench tests from a directory (recursive)
    pub fn load_from_directory(&mut self, dir: &Path) -> Result<()> {
        if !dir.is_dir() {
            return Err(BenchError::Other(format!(
                "WAFBench path is not a directory: {}",
                dir.display()
            )));
        }

        self.load_directory_recursive(dir)?;
        Ok(())
    }

    fn load_directory_recursive(&mut self, dir: &Path) -> Result<()> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| BenchError::Other(format!("Failed to read WAFBench directory: {}", e)))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Recurse into subdirectories
                self.load_directory_recursive(&path)?;
            } else if path.extension().is_some_and(|ext| ext == "yaml" || ext == "yml") {
                if let Err(e) = self.load_file(&path) {
                    self.stats.parse_errors.push(format!("{}: {}", path.display(), e));
                }
            }
        }

        Ok(())
    }

    /// Load a single WAFBench YAML file
    pub fn load_file(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            BenchError::Other(format!("Failed to read WAFBench file {}: {}", path.display(), e))
        })?;

        let wafbench_file: WafBenchFile = serde_yaml::from_str(&content).map_err(|e| {
            BenchError::Other(format!("Failed to parse WAFBench YAML {}: {}", path.display(), e))
        })?;

        // Skip disabled test files
        if !wafbench_file.meta.enabled {
            return Ok(());
        }

        self.stats.files_processed += 1;

        // Determine the rule category from the file path or name
        let category = self.detect_category(path, &wafbench_file.meta);

        // Parse each test case
        for test in wafbench_file.tests {
            if let Some(test_case) = self.parse_test_case(&test, category) {
                self.stats.payloads_extracted += test_case.payloads.len();
                *self.stats.by_category.entry(category).or_insert(0) += 1;
                self.test_cases.push(test_case);
                self.stats.test_cases_loaded += 1;
            }
        }

        Ok(())
    }

    /// Detect the security category from the file path
    fn detect_category(&self, path: &Path, _meta: &WafBenchMeta) -> SecurityCategory {
        let path_str = path.to_string_lossy().to_uppercase();

        if path_str.contains("XSS") || path_str.contains("941") {
            SecurityCategory::Xss
        } else if path_str.contains("SQLI") || path_str.contains("942") {
            SecurityCategory::SqlInjection
        } else if path_str.contains("RCE") || path_str.contains("932") {
            SecurityCategory::CommandInjection
        } else if path_str.contains("LFI") || path_str.contains("930") {
            SecurityCategory::PathTraversal
        } else if path_str.contains("LDAP") {
            SecurityCategory::LdapInjection
        } else if path_str.contains("XXE") || path_str.contains("XML") {
            SecurityCategory::Xxe
        } else if path_str.contains("TEMPLATE") || path_str.contains("SSTI") {
            SecurityCategory::Ssti
        } else {
            // Default to XSS as it's the most common in WAFBench
            SecurityCategory::Xss
        }
    }

    /// Parse a single test case into our format
    fn parse_test_case(
        &self,
        test: &WafBenchTest,
        category: SecurityCategory,
    ) -> Option<WafBenchTestCase> {
        // Extract rule ID from test_title (e.g., "941100-1" -> "941100")
        let rule_id = test.test_title.split('-').next().unwrap_or(&test.test_title).to_string();

        let mut payloads = Vec::new();
        let mut method = "GET".to_string();
        let mut expects_block = false;

        for stage in &test.stages {
            method = stage.input.method.clone();

            // Check if this test expects a block (403)
            if let Some(output) = &stage.output {
                if output.status.contains(&403) {
                    expects_block = true;
                }
            }

            // Extract payload from URI
            if let Some(uri) = &stage.input.uri {
                // Look for attack patterns in the URI
                if self.looks_like_attack(uri) {
                    payloads.push(WafBenchPayload {
                        location: PayloadLocation::Uri,
                        value: uri.clone(),
                        header_name: None,
                    });
                }
            }

            // Extract payloads from headers
            for (header_name, header_value) in &stage.input.headers {
                if self.looks_like_attack(header_value) {
                    payloads.push(WafBenchPayload {
                        location: PayloadLocation::Header,
                        value: header_value.clone(),
                        header_name: Some(header_name.clone()),
                    });
                }
            }

            // Extract payload from body
            if let Some(data) = &stage.input.data {
                if self.looks_like_attack(data) {
                    payloads.push(WafBenchPayload {
                        location: PayloadLocation::Body,
                        value: data.clone(),
                        header_name: None,
                    });
                }
            }
        }

        // If no payloads found, still include the test but with full URI as payload
        if payloads.is_empty() {
            if let Some(stage) = test.stages.first() {
                if let Some(uri) = &stage.input.uri {
                    payloads.push(WafBenchPayload {
                        location: PayloadLocation::Uri,
                        value: uri.clone(),
                        header_name: None,
                    });
                }
            }
        }

        if payloads.is_empty() {
            return None;
        }

        let description = test.desc.clone().unwrap_or_else(|| format!("CRS Rule {} test", rule_id));

        Some(WafBenchTestCase {
            test_id: test.test_title.clone(),
            description,
            rule_id,
            category,
            method,
            payloads,
            expects_block,
        })
    }

    /// Check if a string looks like an attack payload
    fn looks_like_attack(&self, s: &str) -> bool {
        // Common attack patterns
        let attack_patterns = [
            "<script",
            "javascript:",
            "onerror=",
            "onload=",
            "onclick=",
            "onfocus=",
            "onmouseover=",
            "eval(",
            "alert(",
            "document.",
            "window.",
            "'--",
            "' OR ",
            "' AND ",
            "1=1",
            "UNION SELECT",
            "CONCAT(",
            "CHAR(",
            "../",
            "..\\",
            "/etc/passwd",
            "cmd.exe",
            "powershell",
            "; ls",
            "| cat",
            "${",
            "{{",
            "<%",
            "<?",
            "<!ENTITY",
            "SYSTEM \"",
        ];

        let lower = s.to_lowercase();
        attack_patterns.iter().any(|p| lower.contains(&p.to_lowercase()))
    }

    /// Get all loaded test cases
    pub fn test_cases(&self) -> &[WafBenchTestCase] {
        &self.test_cases
    }

    /// Get statistics about loaded tests
    pub fn stats(&self) -> &WafBenchStats {
        &self.stats
    }

    /// Convert loaded tests to SecurityPayload format for use with existing security testing
    pub fn to_security_payloads(&self) -> Vec<SecurityPayload> {
        let mut payloads = Vec::new();

        for test_case in &self.test_cases {
            for payload in &test_case.payloads {
                // Extract just the attack payload part if possible
                let payload_str = self.extract_payload_value(&payload.value);

                payloads.push(
                    SecurityPayload::new(
                        payload_str,
                        test_case.category,
                        format!(
                            "[WAFBench {}] {} ({})",
                            test_case.rule_id, test_case.description, payload.location
                        ),
                    )
                    .high_risk(),
                );
            }
        }

        payloads
    }

    /// Extract the actual attack payload from a URI or value
    fn extract_payload_value(&self, value: &str) -> String {
        // If it's a URI, try to extract query parameter values
        if value.contains('?') {
            if let Some(query) = value.split('?').nth(1) {
                // Get the first parameter value that looks malicious
                for param in query.split('&') {
                    if let Some(val) = param.split('=').nth(1) {
                        let decoded = urlencoding::decode(val).unwrap_or_else(|_| val.into());
                        if self.looks_like_attack(&decoded) {
                            return decoded.to_string();
                        }
                    }
                }
            }
        }

        // Return the full value if we can't extract a specific payload
        value.to_string()
    }
}

impl Default for WafBenchLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wafbench_yaml() {
        let yaml = r#"
meta:
  author: test
  description: Test XSS rules
  enabled: true
  name: test.yaml

tests:
  - desc: "XSS in URI parameter"
    test_title: "941100-1"
    stages:
      - input:
          dest_addr: "127.0.0.1"
          headers:
            Host: "localhost"
            User-Agent: "Mozilla/5.0"
          method: "GET"
          port: 80
          uri: "/test?param=<script>alert(1)</script>"
        output:
          status: [403]
"#;

        let file: WafBenchFile = serde_yaml::from_str(yaml).unwrap();
        assert!(file.meta.enabled);
        assert_eq!(file.tests.len(), 1);
        assert_eq!(file.tests[0].test_title, "941100-1");
    }

    #[test]
    fn test_detect_category() {
        let loader = WafBenchLoader::new();
        let meta = WafBenchMeta {
            author: None,
            description: None,
            enabled: true,
            name: None,
        };

        assert_eq!(
            loader.detect_category(Path::new("/wafbench/REQUEST-941-XSS/test.yaml"), &meta),
            SecurityCategory::Xss
        );

        assert_eq!(
            loader.detect_category(Path::new("/wafbench/REQUEST-942-SQLI/test.yaml"), &meta),
            SecurityCategory::SqlInjection
        );
    }

    #[test]
    fn test_looks_like_attack() {
        let loader = WafBenchLoader::new();

        assert!(loader.looks_like_attack("<script>alert(1)</script>"));
        assert!(loader.looks_like_attack("' OR '1'='1"));
        assert!(loader.looks_like_attack("../../../etc/passwd"));
        assert!(loader.looks_like_attack("; ls -la"));
        assert!(!loader.looks_like_attack("normal text"));
        assert!(!loader.looks_like_attack("hello world"));
    }

    #[test]
    fn test_extract_payload_value() {
        let loader = WafBenchLoader::new();

        let uri = "/test?param=%3Cscript%3Ealert(1)%3C/script%3E";
        let payload = loader.extract_payload_value(uri);
        assert!(payload.contains("<script>") || payload.contains("script"));
    }
}
