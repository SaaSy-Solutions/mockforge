//! Cargo integration-test binary that mounts the protocol-level E2E test tree.
//!
//! Without this file Cargo would never compile `tests/tests/e2e/protocols/*` — the
//! `e2e/` directory is a module tree, not a `tests/foo.rs` integration-test binary.
//! That made the workflow filter `cargo test ... -- e2e::protocols::http_e2e_tests`
//! match zero tests and exit 0 silently.
//!
//! The `#[path]` attributes are required because Cargo integration-test crate
//! roots look up submodules relative to the file's own directory
//! (`tests/tests/`), not a child directory named after the file.

#[path = "e2e/helpers/mod.rs"]
pub mod helpers;

#[path = "e2e/protocols/mod.rs"]
pub mod protocols;
