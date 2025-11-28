//! MockForge Integration Tests
//!
//! This package provides integration tests for the MockForge workspace.
//! Test files are in the `tests/` directory and are automatically discovered by Cargo.

// Common utilities for integration tests
pub mod integration_test_common;

// Re-export commonly used types
pub use integration_test_common::*;
