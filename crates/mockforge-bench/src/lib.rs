//! Load and performance testing module for MockForge
//!
//! This module provides functionality to run load tests against real API endpoints
//! using OpenAPI specifications to generate realistic traffic patterns.

pub mod command;
pub mod error;
pub mod executor;
pub mod k6_gen;
pub mod parallel_executor;
pub mod param_overrides;
pub mod reporter;
pub mod request_gen;
pub mod scenarios;
pub mod spec_parser;
pub mod target_parser;

pub use command::BenchCommand;
pub use error::{BenchError, Result};
pub use parallel_executor::{AggregatedResults, TargetResult};
pub use param_overrides::{OperationOverrides, ParameterOverrides};
pub use scenarios::LoadScenario;
pub use target_parser::{parse_targets_file, TargetConfig};
