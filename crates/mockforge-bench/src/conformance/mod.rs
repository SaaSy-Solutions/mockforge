//! OpenAPI 3.0.0 Conformance Testing
//!
//! Generates k6 scripts that exercise all OpenAPI 3.0.0 features against a target,
//! then reports per-feature pass/fail results.

pub mod generator;
pub mod report;
pub mod sarif;
pub mod schema_validator;
pub mod spec;
pub mod spec_driven;

pub use generator::{ConformanceConfig, ConformanceGenerator};
pub use report::ConformanceReport;
pub use sarif::ConformanceSarifReport;
pub use schema_validator::SchemaValidatorGenerator;
pub use spec::ConformanceFeature;
pub use spec_driven::SpecDrivenConformanceGenerator;
