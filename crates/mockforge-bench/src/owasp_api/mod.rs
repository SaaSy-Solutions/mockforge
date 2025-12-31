//! OWASP API Security Top 10 (2023) Testing Module
//!
//! This module provides automated security testing based on the
//! OWASP API Security Top 10 (2023) categories:
//!
//! - **API1**: Broken Object Level Authorization (BOLA)
//! - **API2**: Broken Authentication
//! - **API3**: Broken Object Property Level Authorization
//! - **API4**: Unrestricted Resource Consumption
//! - **API5**: Broken Function Level Authorization
//! - **API6**: Unrestricted Access to Sensitive Business Flows
//! - **API7**: Server Side Request Forgery (SSRF)
//! - **API8**: Security Misconfiguration
//! - **API9**: Improper Inventory Management
//! - **API10**: Unsafe Consumption of APIs
//!
//! # Usage
//!
//! ```bash
//! # Full OWASP API Top 10 scan
//! mockforge bench --spec api.yaml --target https://api.example.com \
//!   --owasp-api-top10 \
//!   --owasp-auth-header "Authorization"
//!
//! # Specific categories only
//! mockforge bench --spec api.yaml --target https://api.example.com \
//!   --owasp-api-top10 \
//!   --owasp-categories "api1,api2,api7"
//! ```
//!
//! # Example
//!
//! ```ignore
//! use mockforge_bench::owasp_api::{OwaspApiConfig, OwaspCategory};
//!
//! let config = OwaspApiConfig::new()
//!     .with_categories([OwaspCategory::Api1Bola, OwaspCategory::Api7Ssrf])
//!     .with_auth_header("X-Auth-Token")
//!     .with_valid_auth_token("Bearer secret123");
//!
//! // Generate k6 test script
//! let generator = OwaspApiGenerator::new(config, &spec);
//! let script = generator.generate()?;
//! ```

pub mod categories;
pub mod config;
pub mod generator;
pub mod payloads;
pub mod report;
pub mod validators;

// Re-export commonly used types
pub use categories::{OwaspCategory, Severity};
pub use config::{
    AuthToken, DiscoveryConfig, OwaspApiConfig, RateLimitConfig, ReportFormat, SsrfConfig,
};
pub use generator::OwaspApiGenerator;
pub use payloads::{OwaspPayload, OwaspPayloadGenerator};
pub use report::{OwaspFinding, OwaspReport, OwaspScanInfo, OwaspSummary};
pub use validators::{OwaspValidator, ValidationResult};
