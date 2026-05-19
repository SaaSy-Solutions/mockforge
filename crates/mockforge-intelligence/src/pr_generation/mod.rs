//! Pull request generation for contract changes
//!
//! This module provides functionality for automatically generating pull requests
//! when contract changes are detected, including updates to OpenAPI specs,
//! mock fixtures, generated clients, and example tests.

pub mod generator;
pub mod github;
pub mod gitlab;
pub mod templates;
pub mod types;

pub use generator::PRGenerator;
pub use github::GitHubPRClient;
pub use gitlab::GitLabPRClient;
pub use templates::{PRTemplate, PRTemplateContext};
pub use types::{
    PRFileChange, PRFileChangeType, PRGenerationConfig, PRProvider, PRRequest, PRResult,
};
