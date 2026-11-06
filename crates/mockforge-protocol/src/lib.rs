//! Protocol-neutral contracts, registries, matching, fixture loading and server lifecycles.
//!
//! Authentication, global logging and latency adapters remain in `mockforge-core`.

pub mod fixture_store;
pub mod protocol_abstraction;
pub mod protocol_server;

pub use mockforge_foundation::{Error, Result};
pub use protocol_abstraction::*;
