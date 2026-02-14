//! Load and performance testing module for MockForge
//!
//! This module provides functionality to run load tests against real API endpoints
//! using OpenAPI specifications to generate realistic traffic patterns.

pub mod command;
pub mod conformance;
pub mod crud_flow;
pub mod data_driven;
pub mod dynamic_params;
pub mod error;
pub mod executor;
pub mod invalid_data;
pub mod k6_gen;
pub mod mock_integration;
pub mod owasp_api;
pub mod parallel_executor;
pub mod parallel_requests;
pub mod param_overrides;
pub mod reporter;
pub mod request_gen;
pub mod scenarios;
pub mod security_payloads;
pub mod spec_dependencies;
pub mod spec_parser;
pub mod target_parser;
pub mod wafbench;

pub use command::BenchCommand;
pub use crud_flow::{CrudFlow, CrudFlowConfig, CrudFlowDetector, FlowStep};
pub use data_driven::{
    DataDistribution, DataDrivenConfig, DataDrivenGenerator, DataMapping, PerUriColumns,
};
pub use error::{BenchError, Result};
pub use invalid_data::{InvalidDataConfig, InvalidDataGenerator, InvalidDataType};
pub use mock_integration::{MockIntegrationConfig, MockIntegrationGenerator, MockServerDetector};
pub use parallel_executor::{AggregatedResults, TargetResult};
pub use parallel_requests::{ParallelConfig, ParallelRequestGenerator};
pub use param_overrides::{OperationOverrides, ParameterOverrides};
pub use scenarios::LoadScenario;
pub use security_payloads::{SecurityCategory, SecurityPayloads, SecurityTestConfig};
pub use spec_dependencies::{
    DependencyDetector, ExtractedValues, SpecDependency, SpecDependencyConfig, SpecGroup,
};
pub use target_parser::{parse_targets_file, TargetConfig};
pub use wafbench::{WafBenchLoader, WafBenchStats, WafBenchTestCase};

// OpenAPI 3.0.0 Conformance Testing exports
pub use conformance::{
    ConformanceConfig, ConformanceFeature, ConformanceGenerator, ConformanceReport,
};

// OWASP API Security Top 10 exports
pub use owasp_api::{
    AuthToken, DiscoveryConfig, OwaspApiConfig, OwaspApiGenerator, OwaspCategory, OwaspFinding,
    OwaspPayload, OwaspPayloadGenerator, OwaspReport, OwaspScanInfo, OwaspSummary, OwaspValidator,
    RateLimitConfig, ReportFormat, Severity, SsrfConfig, ValidationResult,
};
