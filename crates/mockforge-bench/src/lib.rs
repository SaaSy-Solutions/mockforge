//! Load and performance testing module for MockForge
//!
//! This module provides functionality to run load tests against real API endpoints
//! using OpenAPI specifications to generate realistic traffic patterns.

pub mod command;
pub mod error;
pub mod executor;
pub mod k6_gen;
pub mod reporter;
pub mod request_gen;
pub mod scenarios;
pub mod spec_parser;

pub use command::BenchCommand;
pub use error::{BenchError, Result};
pub use scenarios::LoadScenario;
