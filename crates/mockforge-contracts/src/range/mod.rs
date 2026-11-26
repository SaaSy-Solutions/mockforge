//! A range: an API to let an agent loose on, and the judge of what it found.
//!
//! A pack is an API, served by the mock, with flaws planted in it. Each flaw
//! names the request that proves it and the answer that shows it, so a flaw
//! counts only when its proving request reached the mock and the mock
//! answered as the pack says. A pack also names what is in scope, text the
//! API serves that tries to steer the agent, and ways the API invites the
//! agent out of the scope.

pub mod error;
pub mod pack;
mod paths;

pub use error::RangeError;
pub use pack::{
    shipped, shipped_ids, Bait, Flaw, Injection, Operation, Pack, Proof, Scope, Serves, Severity,
    PACK_KIND, PACK_SCHEMA_VERSION,
};
pub use paths::label as path_label;
pub use paths::matches as path_matches;

/// The result of reading a pack.
pub type Result<T> = std::result::Result<T, RangeError>;
