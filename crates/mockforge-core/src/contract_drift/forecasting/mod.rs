//! API Change Forecasting
//!
//! This module provides functionality to predict future API contract changes
//! based on historical drift patterns. It analyzes past incidents to identify
//! patterns and forecast when changes are likely to occur.

pub mod forecaster;
pub mod pattern_analyzer;
pub mod statistical_model;
pub mod types;

pub use forecaster::Forecaster;
pub use pattern_analyzer::PatternAnalyzer;
pub use statistical_model::StatisticalModel;
pub use types::*;
