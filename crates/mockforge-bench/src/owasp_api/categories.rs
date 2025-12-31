//! OWASP API Security Top 10 (2023) Category Definitions
//!
//! This module defines the 10 OWASP API security risk categories
//! with their metadata, severity ratings, and descriptions.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// OWASP API Security Top 10 (2023) Categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OwaspCategory {
    /// API1:2023 - Broken Object Level Authorization
    Api1Bola,
    /// API2:2023 - Broken Authentication
    Api2BrokenAuth,
    /// API3:2023 - Broken Object Property Level Authorization
    Api3BrokenObjectProperty,
    /// API4:2023 - Unrestricted Resource Consumption
    Api4ResourceConsumption,
    /// API5:2023 - Broken Function Level Authorization
    Api5BrokenFunctionAuth,
    /// API6:2023 - Unrestricted Access to Sensitive Business Flows
    Api6SensitiveFlows,
    /// API7:2023 - Server Side Request Forgery
    Api7Ssrf,
    /// API8:2023 - Security Misconfiguration
    Api8Misconfiguration,
    /// API9:2023 - Improper Inventory Management
    Api9ImproperInventory,
    /// API10:2023 - Unsafe Consumption of APIs
    Api10UnsafeConsumption,
}

impl OwaspCategory {
    /// Returns all OWASP API Top 10 categories
    pub fn all() -> Vec<Self> {
        vec![
            Self::Api1Bola,
            Self::Api2BrokenAuth,
            Self::Api3BrokenObjectProperty,
            Self::Api4ResourceConsumption,
            Self::Api5BrokenFunctionAuth,
            Self::Api6SensitiveFlows,
            Self::Api7Ssrf,
            Self::Api8Misconfiguration,
            Self::Api9ImproperInventory,
            Self::Api10UnsafeConsumption,
        ]
    }

    /// Returns the OWASP identifier (e.g., "API1:2023")
    pub fn identifier(&self) -> &'static str {
        match self {
            Self::Api1Bola => "API1:2023",
            Self::Api2BrokenAuth => "API2:2023",
            Self::Api3BrokenObjectProperty => "API3:2023",
            Self::Api4ResourceConsumption => "API4:2023",
            Self::Api5BrokenFunctionAuth => "API5:2023",
            Self::Api6SensitiveFlows => "API6:2023",
            Self::Api7Ssrf => "API7:2023",
            Self::Api8Misconfiguration => "API8:2023",
            Self::Api9ImproperInventory => "API9:2023",
            Self::Api10UnsafeConsumption => "API10:2023",
        }
    }

    /// Returns the short name for the category
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::Api1Bola => "BOLA",
            Self::Api2BrokenAuth => "Broken Authentication",
            Self::Api3BrokenObjectProperty => "Broken Object Property Authorization",
            Self::Api4ResourceConsumption => "Unrestricted Resource Consumption",
            Self::Api5BrokenFunctionAuth => "Broken Function Authorization",
            Self::Api6SensitiveFlows => "Sensitive Business Flows",
            Self::Api7Ssrf => "SSRF",
            Self::Api8Misconfiguration => "Security Misconfiguration",
            Self::Api9ImproperInventory => "Improper Inventory Management",
            Self::Api10UnsafeConsumption => "Unsafe API Consumption",
        }
    }

    /// Returns the full descriptive name
    pub fn full_name(&self) -> &'static str {
        match self {
            Self::Api1Bola => "Broken Object Level Authorization",
            Self::Api2BrokenAuth => "Broken Authentication",
            Self::Api3BrokenObjectProperty => "Broken Object Property Level Authorization",
            Self::Api4ResourceConsumption => "Unrestricted Resource Consumption",
            Self::Api5BrokenFunctionAuth => "Broken Function Level Authorization",
            Self::Api6SensitiveFlows => "Unrestricted Access to Sensitive Business Flows",
            Self::Api7Ssrf => "Server Side Request Forgery",
            Self::Api8Misconfiguration => "Security Misconfiguration",
            Self::Api9ImproperInventory => "Improper Inventory Management",
            Self::Api10UnsafeConsumption => "Unsafe Consumption of APIs",
        }
    }

    /// Returns the severity level for findings in this category
    pub fn severity(&self) -> Severity {
        match self {
            Self::Api1Bola => Severity::High,
            Self::Api2BrokenAuth => Severity::Critical,
            Self::Api3BrokenObjectProperty => Severity::High,
            Self::Api4ResourceConsumption => Severity::Medium,
            Self::Api5BrokenFunctionAuth => Severity::High,
            Self::Api6SensitiveFlows => Severity::Medium,
            Self::Api7Ssrf => Severity::High,
            Self::Api8Misconfiguration => Severity::Medium,
            Self::Api9ImproperInventory => Severity::Low,
            Self::Api10UnsafeConsumption => Severity::Medium,
        }
    }

    /// Returns a description of what this category tests
    pub fn description(&self) -> &'static str {
        match self {
            Self::Api1Bola => "Tests for authorization flaws that allow accessing other users' objects by manipulating resource IDs",
            Self::Api2BrokenAuth => "Tests for authentication bypass, missing auth, and weak token handling",
            Self::Api3BrokenObjectProperty => "Tests for mass assignment vulnerabilities and unauthorized property access",
            Self::Api4ResourceConsumption => "Tests for missing rate limits, pagination abuse, and resource exhaustion",
            Self::Api5BrokenFunctionAuth => "Tests for unauthorized access to admin/privileged functionality",
            Self::Api6SensitiveFlows => "Tests for business logic abuse and flow manipulation",
            Self::Api7Ssrf => "Tests for server-side request forgery via URL injection",
            Self::Api8Misconfiguration => "Tests for missing security headers, CORS issues, and verbose errors",
            Self::Api9ImproperInventory => "Tests for undocumented endpoints and deprecated API versions",
            Self::Api10UnsafeConsumption => "Tests for injection in data passed to third-party APIs",
        }
    }

    /// Returns remediation guidance for this category
    pub fn remediation(&self) -> &'static str {
        match self {
            Self::Api1Bola => "Implement object-level authorization checks that verify the user has access to the specific resource",
            Self::Api2BrokenAuth => "Implement strong authentication, validate tokens properly, and use secure session management",
            Self::Api3BrokenObjectProperty => "Define and enforce strict schemas, whitelist allowed fields for each operation",
            Self::Api4ResourceConsumption => "Implement rate limiting, pagination limits, and resource quotas",
            Self::Api5BrokenFunctionAuth => "Implement role-based access control (RBAC) for all administrative functions",
            Self::Api6SensitiveFlows => "Implement business logic validation, rate limiting, and anti-automation measures",
            Self::Api7Ssrf => "Validate and sanitize all URLs, use allowlists, disable unnecessary URL schemes",
            Self::Api8Misconfiguration => "Enable security headers, configure CORS properly, disable debug endpoints in production",
            Self::Api9ImproperInventory => "Maintain API inventory, deprecate old versions properly, remove debug endpoints",
            Self::Api10UnsafeConsumption => "Validate and sanitize all data before passing to third-party APIs",
        }
    }

    /// Returns the CLI argument name for this category
    pub fn cli_name(&self) -> &'static str {
        match self {
            Self::Api1Bola => "api1",
            Self::Api2BrokenAuth => "api2",
            Self::Api3BrokenObjectProperty => "api3",
            Self::Api4ResourceConsumption => "api4",
            Self::Api5BrokenFunctionAuth => "api5",
            Self::Api6SensitiveFlows => "api6",
            Self::Api7Ssrf => "api7",
            Self::Api8Misconfiguration => "api8",
            Self::Api9ImproperInventory => "api9",
            Self::Api10UnsafeConsumption => "api10",
        }
    }
}

impl fmt::Display for OwaspCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - {}", self.identifier(), self.short_name())
    }
}

impl FromStr for OwaspCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "api1" | "bola" => Ok(Self::Api1Bola),
            "api2" | "auth" | "authentication" => Ok(Self::Api2BrokenAuth),
            "api3" | "property" | "mass-assignment" => Ok(Self::Api3BrokenObjectProperty),
            "api4" | "consumption" | "rate-limit" => Ok(Self::Api4ResourceConsumption),
            "api5" | "function" | "privilege" => Ok(Self::Api5BrokenFunctionAuth),
            "api6" | "flows" | "business" => Ok(Self::Api6SensitiveFlows),
            "api7" | "ssrf" => Ok(Self::Api7Ssrf),
            "api8" | "misconfig" | "headers" => Ok(Self::Api8Misconfiguration),
            "api9" | "inventory" | "discovery" => Ok(Self::Api9ImproperInventory),
            "api10" | "unsafe" | "third-party" => Ok(Self::Api10UnsafeConsumption),
            _ => Err(format!("Unknown OWASP category: '{}'. Valid values: api1-api10", s)),
        }
    }
}

/// Severity levels for security findings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    #[default]
    Low,
    Info,
}

impl Severity {
    /// Returns the severity as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Info => "info",
        }
    }

    /// Returns a numeric score (higher = more severe)
    pub fn score(&self) -> u8 {
        match self {
            Self::Critical => 5,
            Self::High => 4,
            Self::Medium => 3,
            Self::Low => 2,
            Self::Info => 1,
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_categories() {
        let all = OwaspCategory::all();
        assert_eq!(all.len(), 10);
    }

    #[test]
    fn test_category_from_str() {
        assert_eq!(OwaspCategory::from_str("api1").unwrap(), OwaspCategory::Api1Bola);
        assert_eq!(OwaspCategory::from_str("bola").unwrap(), OwaspCategory::Api1Bola);
        assert_eq!(OwaspCategory::from_str("API7").unwrap(), OwaspCategory::Api7Ssrf);
        assert_eq!(OwaspCategory::from_str("ssrf").unwrap(), OwaspCategory::Api7Ssrf);
    }

    #[test]
    fn test_category_identifiers() {
        assert_eq!(OwaspCategory::Api1Bola.identifier(), "API1:2023");
        assert_eq!(OwaspCategory::Api10UnsafeConsumption.identifier(), "API10:2023");
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical.score() > Severity::High.score());
        assert!(Severity::High.score() > Severity::Medium.score());
        assert!(Severity::Medium.score() > Severity::Low.score());
    }
}
