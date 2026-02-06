//! OWASP API Response Validators
//!
//! This module provides validation logic to detect vulnerabilities
//! based on API responses during security testing.

use super::categories::OwaspCategory;
use super::payloads::ExpectedBehavior;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of validating a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether a vulnerability was detected
    pub vulnerable: bool,
    /// Category being tested
    pub category: OwaspCategory,
    /// Description of what was found
    pub description: String,
    /// Confidence level of the detection
    pub confidence: Confidence,
    /// Additional details
    #[serde(default)]
    pub details: HashMap<String, String>,
}

/// Confidence level in the detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    /// High confidence - clear indicator
    High,
    /// Medium confidence - likely vulnerable
    Medium,
    /// Low confidence - possible but uncertain
    Low,
}

/// Response data for validation
#[derive(Debug, Clone)]
pub struct ResponseData {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: String,
    /// Response time in milliseconds
    pub response_time_ms: u64,
}

/// Baseline response for comparison
#[derive(Debug, Clone)]
pub struct BaselineResponse {
    /// Original response status
    pub status: u16,
    /// Original response body (for comparison)
    pub body: String,
    /// Response time
    pub response_time_ms: u64,
}

/// Validator for OWASP API security testing
pub struct OwaspValidator {
    /// Security headers to check
    required_headers: Vec<(&'static str, Option<&'static str>)>,
    /// Patterns indicating verbose errors
    error_patterns: Vec<&'static str>,
    /// Patterns indicating successful auth bypass
    auth_bypass_patterns: Vec<&'static str>,
}

impl Default for OwaspValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl OwaspValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self {
            required_headers: vec![
                ("x-content-type-options", Some("nosniff")),
                ("x-frame-options", None), // DENY or SAMEORIGIN
                ("strict-transport-security", None),
                ("content-security-policy", None),
                ("x-xss-protection", None),
            ],
            error_patterns: vec![
                "stack trace",
                "stacktrace",
                "at line",
                "syntax error",
                "undefined variable",
                "undefined method",
                "null pointer",
                "nullpointerexception",
                "segmentation fault",
                "internal server error",
                "debug mode",
                "DEBUG=",
                "password=",
                "secret=",
                "api_key=",
                "connection string",
                "jdbc:",
                "mysql:",
                "postgres:",
                "mongodb:",
                "redis:",
                "Exception in thread",
                "Traceback (most recent call last)",
                "File \"",
                ".java:",
                ".py:",
                ".js:",
                ".rb:",
                ".go:",
                "at Object.",
                "at Module.",
            ],
            auth_bypass_patterns: vec![
                "\"authenticated\":true",
                "\"authenticated\": true",
                "\"logged_in\":true",
                "\"logged_in\": true",
                "\"success\":true",
                "\"success\": true",
                "\"authorized\":true",
                "\"authorized\": true",
                "welcome",
                "dashboard",
                "profile",
            ],
        }
    }

    /// Validate a response based on the expected behavior
    pub fn validate(
        &self,
        category: OwaspCategory,
        response: &ResponseData,
        expected: &ExpectedBehavior,
        baseline: Option<&BaselineResponse>,
    ) -> ValidationResult {
        match expected {
            ExpectedBehavior::SuccessWhenShouldFail => {
                self.check_success_when_should_fail(category, response)
            }
            ExpectedBehavior::UnauthorizedDataAccess => {
                self.check_unauthorized_access(category, response, baseline)
            }
            ExpectedBehavior::FieldAccepted => {
                self.check_field_accepted(category, response, baseline)
            }
            ExpectedBehavior::NoRateLimiting => self.check_no_rate_limiting(category, response),
            ExpectedBehavior::InternalDataExposure => {
                self.check_internal_exposure(category, response)
            }
            ExpectedBehavior::EndpointExists => self.check_endpoint_exists(category, response),
            ExpectedBehavior::MissingSecurityHeaders => {
                self.check_missing_headers(category, response)
            }
            ExpectedBehavior::VerboseErrors => self.check_verbose_errors(category, response),
            ExpectedBehavior::Custom(desc) => self.check_custom(category, response, desc, baseline),
        }
    }

    /// Check if the request succeeded when it should have failed
    fn check_success_when_should_fail(
        &self,
        category: OwaspCategory,
        response: &ResponseData,
    ) -> ValidationResult {
        let is_success = (200..300).contains(&response.status);

        if is_success {
            // Check for additional auth bypass indicators
            let body_lower = response.body.to_lowercase();
            let has_bypass_indicator =
                self.auth_bypass_patterns.iter().any(|p| body_lower.contains(&p.to_lowercase()));

            ValidationResult {
                vulnerable: true,
                category,
                description: if has_bypass_indicator {
                    format!(
                        "Request succeeded (HTTP {}) with authentication bypass indicators",
                        response.status
                    )
                } else {
                    format!(
                        "Request succeeded (HTTP {}) when it should have been rejected",
                        response.status
                    )
                },
                confidence: if has_bypass_indicator {
                    Confidence::High
                } else {
                    Confidence::Medium
                },
                details: HashMap::new(),
            }
        } else {
            ValidationResult {
                vulnerable: false,
                category,
                description: format!("Request properly rejected (HTTP {})", response.status),
                confidence: Confidence::High,
                details: HashMap::new(),
            }
        }
    }

    /// Check for unauthorized data access (BOLA)
    fn check_unauthorized_access(
        &self,
        category: OwaspCategory,
        response: &ResponseData,
        baseline: Option<&BaselineResponse>,
    ) -> ValidationResult {
        let is_success = (200..300).contains(&response.status);

        if !is_success {
            return ValidationResult {
                vulnerable: false,
                category,
                description: format!("Access denied (HTTP {})", response.status),
                confidence: Confidence::High,
                details: HashMap::new(),
            };
        }

        // If we have a baseline, check if we got different data
        if let Some(baseline) = baseline {
            if response.body != baseline.body && !response.body.is_empty() {
                // Got different data - this is a BOLA vulnerability
                return ValidationResult {
                    vulnerable: true,
                    category,
                    description: "Accessed different user's data by manipulating resource ID"
                        .to_string(),
                    confidence: Confidence::High,
                    details: {
                        let mut d = HashMap::new();
                        d.insert("baseline_length".to_string(), baseline.body.len().to_string());
                        d.insert("response_length".to_string(), response.body.len().to_string());
                        d
                    },
                };
            }
        }

        // No baseline - check if we got any data at all
        if !response.body.is_empty() {
            ValidationResult {
                vulnerable: true,
                category,
                description: format!(
                    "Resource accessed with manipulated ID (HTTP {})",
                    response.status
                ),
                confidence: Confidence::Medium,
                details: HashMap::new(),
            }
        } else {
            ValidationResult {
                vulnerable: false,
                category,
                description: "No data returned".to_string(),
                confidence: Confidence::Medium,
                details: HashMap::new(),
            }
        }
    }

    /// Check if a field was accepted (mass assignment)
    fn check_field_accepted(
        &self,
        category: OwaspCategory,
        response: &ResponseData,
        baseline: Option<&BaselineResponse>,
    ) -> ValidationResult {
        let is_success = (200..300).contains(&response.status);

        if !is_success {
            return ValidationResult {
                vulnerable: false,
                category,
                description: format!("Field rejected (HTTP {})", response.status),
                confidence: Confidence::High,
                details: HashMap::new(),
            };
        }

        // Check if the response body contains indicators of field acceptance
        let body_lower = response.body.to_lowercase();

        // Look for privilege escalation indicators in response
        let privilege_indicators = [
            "\"role\":\"admin\"",
            "\"role\": \"admin\"",
            "\"is_admin\":true",
            "\"is_admin\": true",
            "\"isadmin\":true",
            "\"isadmin\": true",
            "\"verified\":true",
            "\"verified\": true",
            "\"permissions\":",
            "\"balance\":",
            "\"credits\":",
        ];

        let has_indicator =
            privilege_indicators.iter().any(|p| body_lower.contains(&p.to_lowercase()));

        if has_indicator {
            ValidationResult {
                vulnerable: true,
                category,
                description: "Unauthorized field was accepted and reflected in response"
                    .to_string(),
                confidence: Confidence::High,
                details: HashMap::new(),
            }
        } else if let Some(baseline) = baseline {
            // Check if response differs from baseline (field might have been accepted)
            if response.body != baseline.body {
                ValidationResult {
                    vulnerable: true,
                    category,
                    description: "Response differs after injecting unauthorized fields".to_string(),
                    confidence: Confidence::Medium,
                    details: HashMap::new(),
                }
            } else {
                ValidationResult {
                    vulnerable: false,
                    category,
                    description: "Field appears to have been ignored".to_string(),
                    confidence: Confidence::Medium,
                    details: HashMap::new(),
                }
            }
        } else {
            ValidationResult {
                vulnerable: true,
                category,
                description: "Request accepted, field may have been processed".to_string(),
                confidence: Confidence::Low,
                details: HashMap::new(),
            }
        }
    }

    /// Check for missing rate limiting
    fn check_no_rate_limiting(
        &self,
        category: OwaspCategory,
        response: &ResponseData,
    ) -> ValidationResult {
        // Check for rate limit headers
        let rate_limit_headers = [
            "x-ratelimit-limit",
            "x-ratelimit-remaining",
            "x-rate-limit-limit",
            "x-rate-limit-remaining",
            "ratelimit-limit",
            "ratelimit-remaining",
            "retry-after",
        ];

        let headers_lower: HashMap<String, String> =
            response.headers.iter().map(|(k, v)| (k.to_lowercase(), v.clone())).collect();

        let has_rate_limit_headers =
            rate_limit_headers.iter().any(|h| headers_lower.contains_key(*h));

        // Check if we got rate limited (429)
        if response.status == 429 {
            return ValidationResult {
                vulnerable: false,
                category,
                description: "Rate limiting is active (HTTP 429)".to_string(),
                confidence: Confidence::High,
                details: HashMap::new(),
            };
        }

        // Success with no rate limit headers
        if (200..300).contains(&response.status) && !has_rate_limit_headers {
            ValidationResult {
                vulnerable: true,
                category,
                description: "No rate limiting detected - request succeeded without limits"
                    .to_string(),
                confidence: Confidence::Medium,
                details: HashMap::new(),
            }
        } else if has_rate_limit_headers {
            ValidationResult {
                vulnerable: false,
                category,
                description: "Rate limit headers present".to_string(),
                confidence: Confidence::High,
                details: HashMap::new(),
            }
        } else {
            ValidationResult {
                vulnerable: false,
                category,
                description: format!("Request returned HTTP {}", response.status),
                confidence: Confidence::Medium,
                details: HashMap::new(),
            }
        }
    }

    /// Check for internal data exposure (SSRF)
    fn check_internal_exposure(
        &self,
        category: OwaspCategory,
        response: &ResponseData,
    ) -> ValidationResult {
        let body_lower = response.body.to_lowercase();

        // Indicators of internal/cloud metadata exposure
        let exposure_indicators = [
            "instance-id",
            "ami-id",
            "instance-type",
            "local-hostname",
            "public-hostname",
            "iam/",
            "security-credentials",
            "access-key",
            "secret-key",
            "token",
            "root:",
            "/bin/bash",
            "/bin/sh",
            "127.0.0.1",
            "localhost",
            "internal",
            "private",
            "metadata",
            "computemetadata",
        ];

        let has_exposure = exposure_indicators.iter().any(|p| body_lower.contains(*p));

        // Check for non-error responses with content
        let is_success = (200..300).contains(&response.status);

        if is_success && has_exposure {
            ValidationResult {
                vulnerable: true,
                category,
                description: "Internal data or metadata exposed through SSRF".to_string(),
                confidence: Confidence::High,
                details: HashMap::new(),
            }
        } else if is_success && !response.body.is_empty() {
            ValidationResult {
                vulnerable: true,
                category,
                description: "Response received from internal URL - potential SSRF".to_string(),
                confidence: Confidence::Medium,
                details: HashMap::new(),
            }
        } else {
            ValidationResult {
                vulnerable: false,
                category,
                description: "Internal URL request blocked or failed".to_string(),
                confidence: Confidence::High,
                details: HashMap::new(),
            }
        }
    }

    /// Check if an undocumented endpoint exists
    fn check_endpoint_exists(
        &self,
        category: OwaspCategory,
        response: &ResponseData,
    ) -> ValidationResult {
        // 404 = not found (good)
        // 401/403 = exists but protected (finding)
        // 200/other = exists (finding)
        match response.status {
            404 => ValidationResult {
                vulnerable: false,
                category,
                description: "Endpoint not found (HTTP 404)".to_string(),
                confidence: Confidence::High,
                details: HashMap::new(),
            },
            401 | 403 => ValidationResult {
                vulnerable: true,
                category,
                description: format!(
                    "Undocumented endpoint exists but is protected (HTTP {})",
                    response.status
                ),
                confidence: Confidence::Medium,
                details: HashMap::new(),
            },
            _ if (200..300).contains(&response.status) => ValidationResult {
                vulnerable: true,
                category,
                description: format!(
                    "Undocumented endpoint exists and is accessible (HTTP {})",
                    response.status
                ),
                confidence: Confidence::High,
                details: HashMap::new(),
            },
            _ => ValidationResult {
                vulnerable: false,
                category,
                description: format!("Endpoint returned HTTP {}", response.status),
                confidence: Confidence::Medium,
                details: HashMap::new(),
            },
        }
    }

    /// Check for missing security headers
    fn check_missing_headers(
        &self,
        category: OwaspCategory,
        response: &ResponseData,
    ) -> ValidationResult {
        let headers_lower: HashMap<String, String> =
            response.headers.iter().map(|(k, v)| (k.to_lowercase(), v.clone())).collect();

        let mut missing = Vec::new();
        let mut misconfigured = Vec::new();

        for (header, expected_value) in &self.required_headers {
            if let Some(actual) = headers_lower.get(*header) {
                // Check if value matches expected
                if let Some(expected) = expected_value {
                    if !actual.to_lowercase().contains(&expected.to_lowercase()) {
                        misconfigured
                            .push(format!("{}: {} (expected {})", header, actual, expected));
                    }
                }
            } else {
                missing.push(header.to_string());
            }
        }

        // Check CORS
        if let Some(acao) = headers_lower.get("access-control-allow-origin") {
            if acao == "*" {
                misconfigured.push("access-control-allow-origin: * (wildcard)".to_string());
            }
        }

        if !missing.is_empty() || !misconfigured.is_empty() {
            let mut details = HashMap::new();
            if !missing.is_empty() {
                details.insert("missing_headers".to_string(), missing.join(", "));
            }
            if !misconfigured.is_empty() {
                details.insert("misconfigured_headers".to_string(), misconfigured.join("; "));
            }

            ValidationResult {
                vulnerable: true,
                category,
                description: format!(
                    "Security headers missing or misconfigured: {} missing, {} misconfigured",
                    missing.len(),
                    misconfigured.len()
                ),
                confidence: Confidence::High,
                details,
            }
        } else {
            ValidationResult {
                vulnerable: false,
                category,
                description: "All required security headers present".to_string(),
                confidence: Confidence::High,
                details: HashMap::new(),
            }
        }
    }

    /// Check for verbose error messages
    fn check_verbose_errors(
        &self,
        category: OwaspCategory,
        response: &ResponseData,
    ) -> ValidationResult {
        let body_lower = response.body.to_lowercase();

        let found_patterns: Vec<&str> = self
            .error_patterns
            .iter()
            .filter(|p| body_lower.contains(&p.to_lowercase()))
            .copied()
            .collect();

        if !found_patterns.is_empty() {
            let mut details = HashMap::new();
            details.insert("patterns_found".to_string(), found_patterns.join(", "));

            ValidationResult {
                vulnerable: true,
                category,
                description: "Verbose error information exposed".to_string(),
                confidence: Confidence::High,
                details,
            }
        } else {
            ValidationResult {
                vulnerable: false,
                category,
                description: "No verbose errors detected".to_string(),
                confidence: Confidence::Medium,
                details: HashMap::new(),
            }
        }
    }

    /// Custom validation based on description
    fn check_custom(
        &self,
        category: OwaspCategory,
        response: &ResponseData,
        expected_desc: &str,
        baseline: Option<&BaselineResponse>,
    ) -> ValidationResult {
        let is_success = (200..300).contains(&response.status);
        let body_lower = response.body.to_lowercase();

        // Try to detect based on the expected description
        let vulnerable = match expected_desc.to_lowercase().as_str() {
            s if s.contains("negative") && s.contains("accepted") => {
                is_success && (body_lower.contains("success") || body_lower.contains("created"))
            }
            s if s.contains("zero") && s.contains("accepted") => {
                is_success && (body_lower.contains("success") || body_lower.contains("created"))
            }
            s if s.contains("cors") || s.contains("acao") => {
                response.headers.iter().any(|(k, v)| {
                    k.to_lowercase() == "access-control-allow-origin"
                        && (v == "*" || v.contains("evil"))
                })
            }
            s if s.contains("redirect") => {
                response.status == 302 || response.status == 301 || body_lower.contains("redirect")
            }
            s if s.contains("debug") || s.contains("trace") => {
                response.body.len() > 1000
                    || body_lower.contains("debug")
                    || body_lower.contains("trace")
            }
            _ => {
                // Generic check - success when different from baseline
                if let Some(baseline) = baseline {
                    is_success && response.body != baseline.body
                } else {
                    is_success
                }
            }
        };

        ValidationResult {
            vulnerable,
            category,
            description: if vulnerable {
                expected_desc.to_string()
            } else {
                format!("Expected behavior not observed: {}", expected_desc)
            },
            confidence: if vulnerable {
                Confidence::Medium
            } else {
                Confidence::Medium
            },
            details: HashMap::new(),
        }
    }

    /// Validate response for a specific category
    pub fn validate_category(
        &self,
        category: OwaspCategory,
        response: &ResponseData,
        baseline: Option<&BaselineResponse>,
    ) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        match category {
            OwaspCategory::Api1Bola => {
                results.push(self.check_unauthorized_access(category, response, baseline));
            }
            OwaspCategory::Api2BrokenAuth => {
                results.push(self.check_success_when_should_fail(category, response));
            }
            OwaspCategory::Api3BrokenObjectProperty => {
                results.push(self.check_field_accepted(category, response, baseline));
            }
            OwaspCategory::Api4ResourceConsumption => {
                results.push(self.check_no_rate_limiting(category, response));
            }
            OwaspCategory::Api5BrokenFunctionAuth => {
                results.push(self.check_success_when_should_fail(category, response));
            }
            OwaspCategory::Api6SensitiveFlows => {
                results.push(self.check_no_rate_limiting(category, response));
            }
            OwaspCategory::Api7Ssrf => {
                results.push(self.check_internal_exposure(category, response));
            }
            OwaspCategory::Api8Misconfiguration => {
                results.push(self.check_missing_headers(category, response));
                results.push(self.check_verbose_errors(category, response));
            }
            OwaspCategory::Api9ImproperInventory => {
                results.push(self.check_endpoint_exists(category, response));
            }
            OwaspCategory::Api10UnsafeConsumption => {
                results.push(self.check_internal_exposure(category, response));
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_response(status: u16, body: &str) -> ResponseData {
        ResponseData {
            status,
            headers: HashMap::new(),
            body: body.to_string(),
            response_time_ms: 100,
        }
    }

    #[test]
    fn test_success_when_should_fail() {
        let validator = OwaspValidator::new();

        // Should detect vulnerability when request succeeds
        let response = make_response(200, r#"{"authenticated": true}"#);
        let result =
            validator.check_success_when_should_fail(OwaspCategory::Api2BrokenAuth, &response);
        assert!(result.vulnerable);
        assert_eq!(result.confidence, Confidence::High);

        // Should not detect vulnerability when request fails
        let response = make_response(401, r#"{"error": "unauthorized"}"#);
        let result =
            validator.check_success_when_should_fail(OwaspCategory::Api2BrokenAuth, &response);
        assert!(!result.vulnerable);
    }

    #[test]
    fn test_missing_headers() {
        let validator = OwaspValidator::new();

        // Response with no security headers
        let response = make_response(200, "OK");
        let result =
            validator.check_missing_headers(OwaspCategory::Api8Misconfiguration, &response);
        assert!(result.vulnerable);
        assert!(result.details.contains_key("missing_headers"));

        // Response with all headers
        let mut headers = HashMap::new();
        headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
        headers.insert("X-Frame-Options".to_string(), "DENY".to_string());
        headers.insert("Strict-Transport-Security".to_string(), "max-age=31536000".to_string());
        headers.insert("Content-Security-Policy".to_string(), "default-src 'self'".to_string());
        headers.insert("X-XSS-Protection".to_string(), "1; mode=block".to_string());

        let response = ResponseData {
            status: 200,
            headers,
            body: "OK".to_string(),
            response_time_ms: 100,
        };
        let result =
            validator.check_missing_headers(OwaspCategory::Api8Misconfiguration, &response);
        assert!(!result.vulnerable);
    }

    #[test]
    fn test_verbose_errors() {
        let validator = OwaspValidator::new();

        // Response with stack trace
        let response = make_response(500, r#"{"error": "NullPointerException at line 42"}"#);
        let result = validator.check_verbose_errors(OwaspCategory::Api8Misconfiguration, &response);
        assert!(result.vulnerable);

        // Clean error response
        let response = make_response(500, r#"{"error": "Internal server error"}"#);
        let result = validator.check_verbose_errors(OwaspCategory::Api8Misconfiguration, &response);
        // "internal server error" is in the patterns
        assert!(result.vulnerable);

        // Very clean error
        let response = make_response(500, r#"{"error": "Something went wrong"}"#);
        let result = validator.check_verbose_errors(OwaspCategory::Api8Misconfiguration, &response);
        assert!(!result.vulnerable);
    }

    #[test]
    fn test_endpoint_exists() {
        let validator = OwaspValidator::new();

        // 404 = not found (good)
        let response = make_response(404, "Not Found");
        let result =
            validator.check_endpoint_exists(OwaspCategory::Api9ImproperInventory, &response);
        assert!(!result.vulnerable);

        // 403 = exists but protected (finding)
        let response = make_response(403, "Forbidden");
        let result =
            validator.check_endpoint_exists(OwaspCategory::Api9ImproperInventory, &response);
        assert!(result.vulnerable);

        // 200 = exists and accessible (finding)
        let response = make_response(200, "Swagger UI");
        let result =
            validator.check_endpoint_exists(OwaspCategory::Api9ImproperInventory, &response);
        assert!(result.vulnerable);
    }

    #[test]
    fn test_rate_limiting() {
        let validator = OwaspValidator::new();

        // 429 = rate limited (good)
        let response = make_response(429, "Too Many Requests");
        let result =
            validator.check_no_rate_limiting(OwaspCategory::Api4ResourceConsumption, &response);
        assert!(!result.vulnerable);

        // 200 with no rate limit headers (bad)
        let response = make_response(200, "OK");
        let result =
            validator.check_no_rate_limiting(OwaspCategory::Api4ResourceConsumption, &response);
        assert!(result.vulnerable);

        // 200 with rate limit headers (good)
        let mut headers = HashMap::new();
        headers.insert("X-RateLimit-Limit".to_string(), "100".to_string());
        headers.insert("X-RateLimit-Remaining".to_string(), "99".to_string());
        let response = ResponseData {
            status: 200,
            headers,
            body: "OK".to_string(),
            response_time_ms: 100,
        };
        let result =
            validator.check_no_rate_limiting(OwaspCategory::Api4ResourceConsumption, &response);
        assert!(!result.vulnerable);
    }
}
