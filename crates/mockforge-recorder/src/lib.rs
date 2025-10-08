//! MockForge API Flight Recorder
//!
//! Records all API requests and responses for analysis, replay, and debugging.
//! Provides a queryable SQLite database of all interactions.

pub mod database;
pub mod models;
pub mod recorder;
pub mod query;
pub mod replay;
pub mod har_export;
pub mod middleware;
pub mod protocols;
pub mod api;
pub mod diff;
pub mod test_generation;
pub mod integration_testing;

pub use database::RecorderDatabase;
pub use models::{RecordedRequest, RecordedResponse, Protocol};
pub use recorder::Recorder;
pub use query::{QueryFilter, QueryResult};
pub use replay::ReplayEngine;
pub use har_export::export_to_har;
pub use middleware::recording_middleware;
pub use api::create_api_router;
pub use diff::{ComparisonResult, Difference, DifferenceType, ResponseComparator};
pub use test_generation::{
    TestGenerator, TestGenerationConfig, TestFormat, TestGenerationResult,
    GeneratedTest, LlmConfig, TestSuiteMetadata,
};
pub use integration_testing::{
    IntegrationWorkflow, IntegrationTestGenerator, WorkflowStep, WorkflowSetup,
    StepRequest, StepValidation, VariableExtraction, StepCondition,
};

use thiserror::Error;

/// Recorder errors
#[derive(Error, Debug)]
pub enum RecorderError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Request not found: {0}")]
    NotFound(String),

    #[error("Invalid filter: {0}")]
    InvalidFilter(String),

    #[error("Replay error: {0}")]
    Replay(String),

    #[error("Test generation error: {0}")]
    TestGeneration(String),
}

pub type Result<T> = std::result::Result<T, RecorderError>;
