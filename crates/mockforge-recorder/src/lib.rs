//! MockForge API Flight Recorder
//!
//! Records all API requests and responses for analysis, replay, and debugging.
//! Provides a queryable SQLite database of all interactions.

pub mod api;
pub mod database;
pub mod diff;
pub mod har_export;
pub mod integration_testing;
pub mod middleware;
pub mod models;
pub mod protocols;
pub mod query;
pub mod recorder;
pub mod replay;
pub mod scrubbing;
pub mod sync;
pub mod test_generation;

pub use api::create_api_router;
pub use database::RecorderDatabase;
pub use diff::{ComparisonResult, Difference, DifferenceType, ResponseComparator};
pub use har_export::export_to_har;
pub use integration_testing::{
    IntegrationTestGenerator, IntegrationWorkflow, StepCondition, StepRequest, StepValidation,
    VariableExtraction, WorkflowSetup, WorkflowStep,
};
pub use middleware::recording_middleware;
pub use models::{Protocol, RecordedRequest, RecordedResponse};
pub use query::{QueryFilter, QueryResult};
pub use recorder::Recorder;
pub use replay::ReplayEngine;
pub use scrubbing::{
    CaptureFilter, CaptureFilterConfig, ScrubConfig, ScrubRule, ScrubTarget, Scrubber,
};
pub use sync::{SyncConfig, SyncService, SyncStatus};
pub use test_generation::{
    GeneratedTest, LlmConfig, TestFormat, TestGenerationConfig, TestGenerationResult,
    TestGenerator, TestSuiteMetadata,
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
