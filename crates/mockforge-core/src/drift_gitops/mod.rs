//! GitOps integration for drift budget violations
//!
//! This module provides functionality to automatically generate pull requests
//! when drift budgets are exceeded, updating OpenAPI specs, fixtures, and
//! optionally triggering client generation.

pub mod handler;

pub use handler::DriftGitOpsHandler;

