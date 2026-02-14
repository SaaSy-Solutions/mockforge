//! OpenAPI 3.0.0 Conformance Testing
//!
//! Generates k6 scripts that exercise all OpenAPI 3.0.0 features against a target,
//! then reports per-feature pass/fail results.

pub mod generator;
pub mod report;
pub mod spec;

pub use generator::{ConformanceConfig, ConformanceGenerator};
pub use report::ConformanceReport;
pub use spec::ConformanceFeature;
