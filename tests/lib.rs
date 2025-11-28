//! MockForge Integration Tests
//!
//! This package provides integration tests for the MockForge workspace.
//! Test files are in the `tests/` directory and are automatically discovered by Cargo.

// Dummy library - this package exists only for integration tests

// Re-export E2E test modules for use in test files
pub mod e2e {
    pub mod helpers;
    pub mod protocols;
}
