//! OpenAPI 3.0.0 Conformance Testing
//!
//! Generates k6 scripts that exercise all OpenAPI 3.0.0 features against a target,
//! then reports per-feature pass/fail results.

pub mod custom;
pub mod executor;
pub mod generator;
pub mod har_to_custom;
pub mod report;
pub mod sarif;
pub mod schema_validator;
pub mod spec;
pub mod spec_driven;

pub use custom::CustomConformanceConfig;
pub use executor::{ConformanceProgress, NativeConformanceExecutor};
pub use generator::{ConformanceConfig, ConformanceGenerator};
pub use har_to_custom::{generate_custom_yaml_from_har, HarToCustomOptions};
pub use report::{ConformanceReport, OwaspCoverageEntry};
pub use sarif::ConformanceSarifReport;
pub use schema_validator::SchemaValidatorGenerator;
pub use spec::ConformanceFeature;
pub use spec_driven::SpecDrivenConformanceGenerator;
